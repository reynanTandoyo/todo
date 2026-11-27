//! `todo verify` command: run one task's `verify` command without changing anything, or check the
//! whole wave of finished tasks and record the results (T-158; R-VERIFY-1, R-VERIFY-2).

use std::path::{Path, PathBuf};

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::atomic::read_file_with_snapshot;
use crate::cli::{CommandKind, CommandMeta, Context, edit_one_task};
use crate::doc::Status;
use crate::error::{ErrorCode, ExitCode, Result, TodoError};
use crate::find::{FoundTodo, verify_folder};
use crate::ops::verify::{self, CheckOutcome, Failure};
use crate::ops::{OpsConfig, TaskOp, file_label, parse_doc_for_write, resolve_task_in_doc};
use crate::output::Output;
use crate::state::StateDir;

/// Metadata for the `verify` command.
pub const META: CommandMeta = CommandMeta {
    name: "verify",
    summary: "Run a task's verify command, or the checks of a finished wave",
    kind: CommandKind::Write,
    details: "`verify <task>` runs the task's `verify` command through `verify_shell`, outside the write lock, and changes nothing in TODO.md; the output goes to `verify/<ID>.log` in the state folder. `verify --wave` runs the command of every done task that has `verify` and no `verified` line, one after another in file order, and writes each result under its own lock: a pass writes `verified`; a failure reopens the task and gives it back as `stuck`, with the last line of the output as the problem. It prints the tasks that passed, the tasks that failed and the tasks given back, and exits 1 when any failed. It refuses to start while a task is claimed (`ALREADY_CLAIMED`) unless `--force` is given. With `verify = false` in todo.toml, both exit 5 with `VERIFY_NOT_RUN`.",
    examples: &[
        "todo verify T-012 --agent planner",
        "todo verify --wave --agent planner",
    ],
};

/// Arguments for `todo verify`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Task ID whose `verify` command runs (e.g. `T-012`, `t12`, or `12`); TODO.md is not changed.
    #[arg(value_name = "TASK", conflicts_with = "wave", required_unless_present = "wave")]
    pub task: Option<String>,

    /// Check every done task that has a `verify` command and no `verified` line.
    #[arg(long)]
    pub wave: bool,
}

/// JSON of a single check that passed.
#[derive(Serialize)]
struct PassedJson<'a> {
    task: &'a str,
    passed: bool,
    log: String,
}

/// JSON of one failed check of a wave.
#[derive(Serialize)]
struct FailedJson<'a> {
    task: &'a str,
    exit_code: Option<i32>,
    last_line: &'a str,
    timed_out: bool,
    log: String,
}

/// JSON of a whole wave check (`todo.verify.v1`).
#[derive(Serialize)]
struct WaveJson<'a> {
    checked: usize,
    passed: &'a [String],
    failed: Vec<FailedJson<'a>>,
    given_back: &'a [String],
}

/// Runs `todo verify <task>` or `todo verify --wave`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let (found, cfg) = ctx.find_todo_and_config(out)?;
    if !cfg.verify_enabled {
        return Err(TodoError::new(
            ErrorCode::VerifyNotRun,
            "verify commands are off: verify = false in todo.toml",
        ));
    }
    match args.task {
        Some(task) => check_one(out, &found, &cfg, &task),
        None => check_wave(ctx, out, &found, &cfg),
    }
}

/// `todo verify <task>`: runs the command and reports it. Writes nothing to TODO.md.
fn check_one(out: &mut Output, found: &FoundTodo, cfg: &OpsConfig, task_input: &str) -> Result<()> {
    let (id, command) = read_verify_line(found, cfg, task_input)?;
    let log = log_of(found, &id)?;
    match run_in_folder(found, cfg, &command, &log)? {
        CheckOutcome::Passed => {
            if out.is_json() {
                out.emit_json_ok(
                    META.name,
                    &PassedJson {
                        task: &id,
                        passed: true,
                        log: log.display().to_string(),
                    },
                );
            } else {
                out.write_line(&format!("{id}: verify passed (output in {})", log.display()));
            }
            Ok(())
        }
        CheckOutcome::NotRun { reason } => Err(verify::not_run_error(&id, &reason)),
        CheckOutcome::Failed(failure) => {
            if !out.is_json() {
                for line in &failure.tail {
                    out.info(line);
                }
            }
            let message = format!(
                "verify of {id} failed ({}): {}",
                verify::exit_text(&failure),
                failure.last_line
            );
            Err(verify::failure_error(&id, &failure, message))
        }
    }
}

/// `todo verify --wave`: checks every done task with `verify` and no `verified`, in file order.
fn check_wave(ctx: &Context, out: &mut Output, found: &FoundTodo, cfg: &OpsConfig) -> Result<()> {
    crate::cli::emit_find_notices(found, out);
    let quiet = without_notices(found);
    let bytes = read_file_with_snapshot(&found.path)?.bytes;
    let doc = parse_doc_for_write(&bytes, &file_label(&found.path), cfg)?;

    // Refused while a worker may still be editing (R-VERIFY-2); `--force` runs anyway.
    let claimed: Vec<String> = doc
        .tasks
        .iter()
        .filter(|task| task.status == Status::Claimed)
        .map(|task| cfg.id_config.format_id(task.id))
        .collect();
    if !claimed.is_empty() && !ctx.force {
        return Err(TodoError::new(
            ErrorCode::AlreadyClaimed,
            format!(
                "{} task(s) are claimed, so a worker may still be editing: {}; wait for them, or pass --force",
                claimed.len(),
                claimed.join(", ")
            ),
        )
        .with_candidates(claimed));
    }

    let pending: Vec<(String, String)> = doc
        .tasks
        .iter()
        .filter(|task| verify::awaits_verified(task))
        .filter_map(|task| {
            let command = task.verify.clone()?;
            Some((cfg.id_config.format_id(task.id), command))
        })
        .collect();

    let mut passed: Vec<String> = Vec::new();
    let mut failed: Vec<(String, Failure)> = Vec::new();
    for (id, command) in &pending {
        let log = log_of(found, id)?;
        match run_in_folder(found, cfg, command, &log)? {
            CheckOutcome::Passed => {
                let op = TaskOp::VerifyPassed { command };
                edit_one_task(ctx, out, META.name, id, &op, cfg, Some(quiet.clone()))?;
                passed.push(id.clone());
            }
            CheckOutcome::Failed(failure) => {
                let op = TaskOp::VerifyWaveFailed {
                    command,
                    last_line: &failure.last_line,
                };
                edit_one_task(ctx, out, META.name, id, &op, cfg, Some(quiet.clone()))?;
                failed.push((id.clone(), failure));
            }
            CheckOutcome::NotRun { reason } => return Err(verify::not_run_error(id, &reason)),
        }
    }

    let given_back: Vec<String> = failed.iter().map(|(id, _)| id.clone()).collect();
    if out.is_json() {
        let json = WaveJson {
            checked: pending.len(),
            passed: &passed,
            failed: failed
                .iter()
                .map(|(id, failure)| FailedJson {
                    task: id,
                    exit_code: failure.exit_code,
                    last_line: &failure.last_line,
                    timed_out: failure.timed_out,
                    log: failure.log.display().to_string(),
                })
                .collect(),
            given_back: &given_back,
        };
        out.emit_json_ok(META.name, &json);
    } else if pending.is_empty() {
        out.write_line("nothing to check: no done task has a verify command and no verified line");
    } else {
        out.write_line(&format!("checked {} done task(s)", pending.len()));
        if !passed.is_empty() {
            out.write_line(&format!("passed: {}", passed.join(", ")));
        }
        for (id, failure) in &failed {
            out.write_line(&format!("failed: {id}: {}", failure.last_line));
        }
        if !given_back.is_empty() {
            out.write_line(&format!(
                "given back (blocked, as stuck): {}",
                given_back.join(", ")
            ));
        }
    }
    if !failed.is_empty() {
        out.set_exit(ExitCode::Refused);
    }
    Ok(())
}

/// The task's canonical ID and its `verify` command. A task without one has nothing to run.
fn read_verify_line(found: &FoundTodo, cfg: &OpsConfig, task_input: &str) -> Result<(String, String)> {
    let bytes = read_file_with_snapshot(&found.path)?.bytes;
    let doc = parse_doc_for_write(&bytes, &file_label(&found.path), cfg)?;
    let task = resolve_task_in_doc(&doc, task_input)?;
    let id = cfg.id_config.format_id(task.id);
    match task.verify.clone() {
        Some(command) => Ok((id, command)),
        None => Err(TodoError::new(
            ErrorCode::MissingField,
            format!("{id} has no verify command"),
        )
        .with_task(id)),
    }
}

/// Runs one command in the folder of the task's worktree (R-VERIFY-1).
fn run_in_folder(
    found: &FoundTodo,
    cfg: &OpsConfig,
    command: &str,
    log: &Path,
) -> Result<CheckOutcome> {
    let cwd = std::env::current_dir().unwrap_or_default();
    let folder = verify_folder(found, &cwd);
    verify::run_check(cfg, command, &folder, log)
}

/// The log file of a task's check, `verify/<ID>.log` in the state folder (R-VERIFY-1).
fn log_of(found: &FoundTodo, id: &str) -> Result<PathBuf> {
    let state = StateDir::from_found(found)?;
    Ok(verify::log_path(state.path(), id))
}

/// `found` without the notices and warnings of its discovery, which were printed once already.
fn without_notices(found: &FoundTodo) -> FoundTodo {
    let mut quiet = found.clone();
    quiet.notice = None;
    quiet.warnings.clear();
    quiet
}
