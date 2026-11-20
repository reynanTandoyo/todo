//! `todo done` command: mark a task done with optional note and commit (T-024; R-WRITE-4, NFR-5),
//! after the task's `verify` command passed (T-158; R-VERIFY-1).

use std::path::Path;

use clap::Args as ClapArgs;

use crate::atomic::read_file_with_snapshot;
use crate::cli::{
    CommandKind, CommandMeta, Context, edit_one_task, report_task_with_ready, resolve_opt_text_arg,
};
use crate::error::{ErrorCode, Result, TodoError};
use crate::find::{FoundTodo, verify_folder};
use crate::ops::commit_files::{resolve_commit, scope_of_commit};
use crate::ops::identity::done_by;
use crate::ops::scope::{CommitScope, check_done_commit};
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
    details: "Marks the task `done`, sets the calling agent as `owner`, removes `claimed`, and appends a `done` note. If the task has a `verify` command, todo runs it first, outside the write lock, in the folder of TODO.md (in a linked worktree, the matching folder of that worktree); a pass also writes `verified: <date>`, and a failure refuses with `VERIFY_FAILED` (exit 1), leaves the task claimed and adds the note `verify failed — <last line>`; after `verify_attempts` failures the task is given back as `stuck`. With `verify_when = \"wave\"` the command runs later, with `todo verify --wave`. `--no-verify` skips the command and needs `--force`. When the task has `files` and git is available, the files that a `--commit` changed must lie inside them (`TODO.md` and the archive file always may), or `done` is refused with `OUT_OF_SCOPE` before any command runs; `--force` finishes it and records the files in a note. A task at the lowest level with `files` needs `--commit` while git is available. If `--commit` is given, `HEAD` is resolved via `git rev-parse` before the write lock is taken and unknown commits are refused when `git` is available. When git can tell who did the work, records it as `by` (the author of `--commit`, otherwise the configured `user.name`, in both cases after `.mailmap`; never an email address); without git or a name, `by` is left out. Prints the task's `accept` line as a reminder.",
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
    // Which files the commit changed (R-SCOPE-2), also asked of git before the lock.
    let scope = scope_of_commit(&found.path, &cfg.archive_file, resolved_commit.as_deref())?;

    // The check runs outside the lock (R-LOCK-4); its result is written by the change below.
    let command: String;
    let check = match plan_check(ctx, &found, &cfg, &args.task, args.no_verify, &scope)? {
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
        scope: &scope,
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
    scope: &CommitScope,
) -> Result<PlannedCheck> {
    let bytes = read_file_with_snapshot(&found.path)?.bytes;
    let label = file_label(&found.path);
    let doc = parse_doc_for_write(&bytes, &label, cfg)?;
    let task = resolve_task_in_doc(&doc, task_input)?;
    let id = cfg.id_config.format_id(task.id);
    validate_transition(task, StatusCommand::Done, ctx.require_agent()?, ctx.force, &id)?;
    // R-SCOPE-2 before any command: a commit outside the task's files, or a missing commit, is
    // refused here. The change checks it again under the lock.
    check_done_commit(task, cfg, scope, ctx.force, &id)?;

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
