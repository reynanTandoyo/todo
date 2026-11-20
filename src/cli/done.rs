//! `todo done` command: mark a task done with optional note and commit (T-024; R-WRITE-4, NFR-5),
//! after the task's `verify` command passed (T-158; R-VERIFY-1).

use std::io::ErrorKind;
use std::path::Path;
use std::process::Command as ProcessCommand;

use clap::Args as ClapArgs;

use crate::atomic::read_file_with_snapshot;
use crate::cli::{
    CommandKind, CommandMeta, Context, edit_one_task, report_task_with_ready, resolve_opt_text_arg,
};
use crate::error::{ErrorCode, Result, TodoError};
use crate::find::{FoundTodo, verify_folder};
use crate::ops::identity::done_by;
use crate::ops::verify::{self, CheckOutcome};
use crate::ops::{
    DoneCheck, OpsConfig, StatusCommand, TaskOp, VerifyWhen, file_label, parse_doc_for_write,
    resolve_task_in_doc, validate_transition,
};
use crate::output::Output;
use crate::state::StateDir;

/// Metadata for the `done` command.
pub const META: CommandMeta = CommandMeta {
    name: "done",
    summary: "Mark a task done",
    kind: CommandKind::Write,
    details: "Marks the task `done`, sets the calling agent as `owner`, removes `claimed`, and appends a `done` note. If the task has a `verify` command, todo runs it first, outside the write lock, in the folder of TODO.md (in a linked worktree, the matching folder of that worktree); a pass also writes `verified: <date>`, and a failure refuses with `VERIFY_FAILED` (exit 1), leaves the task claimed and adds the note `verify failed — <last line>`; after `verify_attempts` failures the task is given back as `stuck`. With `verify_when = \"wave\"` the command runs later, with `todo verify --wave`. `--no-verify` skips the command and needs `--force`. If `--commit` is given, `HEAD` is resolved via `git rev-parse` before the write lock is taken and unknown commits are refused when `git` is available. When git can tell who did the work, records it as `by` (the author of `--commit`, otherwise the configured `user.name`, in both cases after `.mailmap`; never an email address); without git or a name, `by` is left out. Prints the task's `accept` line as a reminder.",
    examples: &[
        "todo done T-012 --agent agent-1",
        "todo done T-012 --agent agent-1 --note \"all checks pass\" --commit HEAD",
        "todo done T-012 --agent coordinator --force --note \"verified\"",
        "todo done T-012 --agent coordinator --force --no-verify --note \"checked by hand\"",
    ],
};

/// Arguments for `todo done`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Task ID (e.g. `T-012`, `t12`, or `12`).
    #[arg(value_name = "TASK")]
    pub task: String,

    /// Optional note describing what was completed.
    #[arg(long, value_name = "TEXT")]
    pub note: Option<String>,

    /// Optional commit hash or `HEAD` to record in the `commit` field.
    #[arg(long, value_name = "HASH|HEAD")]
    pub commit: Option<String>,

    /// Skip the task's `verify` command; needs `--force`, and the note says the check was skipped.
    #[arg(long)]
    pub no_verify: bool,
}

/// What `done` does about the task's check, decided from the file before the lock is taken.
enum PlannedCheck {
    /// Nothing runs; the done change records this check (`NotChecked` or `Skipped`).
    Recorded(DoneCheck<'static>),
    /// The command `command` of the task `id` runs first.
    Run { id: String, command: String },
}

/// Runs `todo done <task>`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    if args.no_verify && !ctx.force {
        return Err(TodoError::new(
            ErrorCode::Usage,
            "--no-verify skips the task's verify command, so it needs --force",
        ));
    }

    // Resolve/verify `--commit` before taking the write lock (R-WRITE-4, NFR-5).
    let git_dir = ctx
        .file
        .as_deref()
        .and_then(Path::parent)
        .filter(|p| !p.as_os_str().is_empty());
    let resolved_commit = match args.commit.as_deref() {
        Some(raw) => Some(resolve_commit(raw, git_dir)?),
        None => None,
    };

    let resolved_note = resolve_opt_text_arg(args.note.as_deref())?;
    let (found, cfg) = ctx.load_config_with_found()?;
    // Who did the work (R-WRITE-16); looked up before the lock, left out when git can't say.
    let by = done_by(
        found.path.parent().filter(|p| !p.as_os_str().is_empty()),
        resolved_commit.as_deref(),
    );

    // The check runs outside the lock (R-LOCK-4); its result is written by the change below.
    let command: String;
    let check = match plan_check(ctx, &found, &cfg, &args.task, args.no_verify)? {
        PlannedCheck::Recorded(check) => check,
        PlannedCheck::Run { id, command: text } => {
            command = text;
            run_check_before_done(ctx, out, &found, &cfg, &args.task, &id, &command)?;
            DoneCheck::Passed { command: &command }
        }
    };

    let op = TaskOp::Done {
        note: resolved_note.as_deref(),
        commit: resolved_commit.as_deref(),
        by: by.as_deref(),
        check,
    };
    let res = edit_one_task(ctx, out, META.name, &args.task, &op, &cfg, Some(found))?;
    report_task_with_ready(out, META.name, &res, true);
    Ok(())
}

/// Decides, from a read of `TODO.md` that takes no lock, what `done` does about the task's check.
///
/// A change the rules refuse (R-WRITE-9) is refused here, before any command runs, and a file that
/// the write would refuse is refused the same way.
fn plan_check(
    ctx: &Context,
    found: &FoundTodo,
    cfg: &OpsConfig,
    task_input: &str,
    no_verify: bool,
) -> Result<PlannedCheck> {
    let bytes = read_file_with_snapshot(&found.path)?.bytes;
    let label = file_label(&found.path);
    let doc = parse_doc_for_write(&bytes, &label, cfg)?;
    let task = resolve_task_in_doc(&doc, task_input)?;
    let id = cfg.id_config.format_id(task.id);
    validate_transition(task, StatusCommand::Done, ctx.require_agent()?, ctx.force, &id)?;

    let Some(verify_line) = task.verify.as_deref() else {
        return Ok(PlannedCheck::Recorded(DoneCheck::NotChecked));
    };
    if no_verify {
        return Ok(PlannedCheck::Recorded(DoneCheck::Skipped));
    }
    if !cfg.verify_enabled || cfg.verify_when == VerifyWhen::Wave {
        return Ok(PlannedCheck::Recorded(DoneCheck::NotChecked));
    }
    Ok(PlannedCheck::Run {
        id,
        command: verify_line.to_string(),
    })
}

/// Runs the check of a task before `done` writes it. A pass returns `Ok`. A failure is written as
/// its own change (the attempt, or the give-back as `stuck`) and then refused with `VERIFY_FAILED`;
/// a check that could not run is refused with `VERIFY_NOT_RUN`, and nothing is written.
fn run_check_before_done(
    ctx: &Context,
    out: &mut Output,
    found: &FoundTodo,
    cfg: &OpsConfig,
    task_input: &str,
    id: &str,
    command: &str,
) -> Result<()> {
    let cwd = std::env::current_dir().unwrap_or_default();
    let folder = verify_folder(found, &cwd);
    let state = StateDir::from_found(found)?;
    let log = verify::log_path(state.path(), id);
    match verify::run_check(cfg, command, &folder, &log)? {
        CheckOutcome::Passed => Ok(()),
        CheckOutcome::NotRun { reason } => Err(verify::not_run_error(id, &reason)),
        CheckOutcome::Failed(failure) => {
            let op = TaskOp::VerifyFailed {
                command,
                last_line: &failure.last_line,
            };
            let res = edit_one_task(ctx, out, META.name, task_input, &op, cfg, Some(found.clone()))?;
            if !out.is_json() {
                for line in &failure.tail {
                    out.info(line);
                }
            }
            let message = if res.task.status == "blocked" {
                format!(
                    "verify of {id} failed, so {id} is given back to the planner as stuck: {}",
                    failure.last_line
                )
            } else {
                format!(
                    "verify of {id} failed ({}), so {id} is not done and stays claimed: {}",
                    verify::exit_text(&failure),
                    failure.last_line
                )
            };
            Err(verify::failure_error(id, &failure, message))
        }
    }
}

/// Resolves and verifies a `--commit` argument using `git rev-parse` when `git` is available (`NFR-5`).
///
/// If `git` is not on `PATH`, returns the trimmed input unchecked.
fn resolve_commit(raw: &str, dir: Option<&Path>) -> Result<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(TodoError::new(
            ErrorCode::BadValue,
            "--commit value cannot be empty",
        ));
    }

    let mut cmd = ProcessCommand::new("git");
    if let Some(d) = dir {
        cmd.current_dir(d);
    }
    let spec = format!("{trimmed}^{{commit}}");
    let output = match cmd.args(["rev-parse", "--short", "--verify", &spec]).output() {
        Ok(o) => o,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            return Ok(trimmed.to_string());
        }
        Err(e) => {
            return Err(TodoError::new(
                ErrorCode::BadValue,
                format!("failed to run git rev-parse for '{trimmed}': {e}"),
            ));
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.to_ascii_lowercase().contains("not a git repository")
            && !trimmed.eq_ignore_ascii_case("HEAD")
        {
            return Ok(trimmed.to_string());
        }
        return Err(TodoError::new(
            ErrorCode::BadValue,
            format!("unknown git commit '{trimmed}'"),
        ));
    }

    let resolved = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if trimmed.eq_ignore_ascii_case("HEAD") || !is_hex_hash(trimmed) {
        if resolved.is_empty() {
            Ok(trimmed.to_string())
        } else {
            Ok(resolved)
        }
    } else {
        Ok(trimmed.to_string())
    }
}

fn is_hex_hash(s: &str) -> bool {
    s.len() >= 4 && s.bytes().all(|b| b.is_ascii_hexdigit())
}
