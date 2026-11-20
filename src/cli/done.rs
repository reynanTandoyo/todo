//! `todo done` command: mark a task done with optional note and commit (T-024; R-WRITE-4, NFR-5).

use std::path::Path;

use clap::Args as ClapArgs;

use crate::cli::{
    CommandKind, CommandMeta, Context, edit_one_task, report_task_with_ready, resolve_opt_text_arg,
};
use crate::error::Result;
use crate::ops::TaskOp;
use crate::ops::commit_files::{resolve_commit, scope_of_commit};
use crate::ops::identity::done_by;
use crate::output::Output;

/// Metadata for the `done` command.
pub const META: CommandMeta = CommandMeta {
    name: "done",
    summary: "Mark a task done",
    kind: CommandKind::Write,
    details: "Marks the task `done`, sets the calling agent as `owner`, removes `claimed`, and appends a `done` note. If `--commit` is given, `HEAD` is resolved via `git rev-parse` before the write lock is taken and unknown commits are refused when `git` is available. When the task has `files` and git is available, the files that the commit changed must lie inside them (`TODO.md` and the archive file always may), or `done` is refused with `OUT_OF_SCOPE`; `--force` finishes it and records the files in a note. A task at the lowest level with `files` needs `--commit` while git is available. When git can tell who did the work, records it as `by` (the author of `--commit`, otherwise the configured `user.name`, in both cases after `.mailmap`; never an email address); without git or a name, `by` is left out. Prints the task's `accept` line as a reminder.",
    examples: &[
        "todo done T-012 --agent agent-1",
        "todo done T-012 --agent agent-1 --note \"all checks pass\" --commit HEAD",
        "todo done T-012 --agent coordinator --force --note \"verified\"",
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
}

/// Runs `todo done <task>`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
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
    let folder = found.path.parent().filter(|p| !p.as_os_str().is_empty());
    // Who did the work (R-WRITE-16); looked up before the lock, left out when git can't say.
    let by = done_by(folder, resolved_commit.as_deref());
    // Which files the commit changed (R-SCOPE-2), also asked of git before the lock.
    let scope = scope_of_commit(&found.path, &cfg.archive_file, resolved_commit.as_deref())?;
    let op = TaskOp::Done {
        note: resolved_note.as_deref(),
        commit: resolved_commit.as_deref(),
        by: by.as_deref(),
        scope: &scope,
    };
    let res = edit_one_task(ctx, out, META.name, &args.task, &op, &cfg, Some(found))?;
    report_task_with_ready(out, META.name, &res, true);
    Ok(())
}
