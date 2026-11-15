//! `todo done` command: mark a task done with optional note and commit (T-024; R-WRITE-4, NFR-5).

use std::io::ErrorKind;
use std::path::Path;
use std::process::Command as ProcessCommand;

use clap::Args as ClapArgs;

use crate::cli::{
    CommandKind, CommandMeta, Context, edit_one_task, report_task_with_ready, resolve_opt_text_arg,
};
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::TaskOp;
use crate::output::Output;

/// Metadata for the `done` command.
pub const META: CommandMeta = CommandMeta {
    name: "done",
    summary: "Mark a task done",
    kind: CommandKind::Write,
    details: "Marks the task `done`, sets the calling agent as `owner`, removes `claimed`, and appends a `done` note. If `--commit` is given, `HEAD` is resolved via `git rev-parse` before the write lock is taken and unknown commits are refused when `git` is available. Prints the task's `accept` line as a reminder.",
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
    let op = TaskOp::Done {
        note: resolved_note.as_deref(),
        commit: resolved_commit.as_deref(),
    };
    let res = edit_one_task(ctx, out, META.name, &args.task, &op, &cfg, Some(found))?;
    report_task_with_ready(out, META.name, &res, true);
    Ok(())
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
