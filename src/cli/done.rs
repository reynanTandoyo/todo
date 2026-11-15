//! `todo done` command: mark a task done with optional note and commit (T-024; R-WRITE-4, NFR-5).

use std::io::ErrorKind;
use std::path::Path;
use std::process::Command as ProcessCommand;

use clap::Args as ClapArgs;

use crate::cli::{CommandKind, CommandMeta, Context};
use crate::doc::Status;
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::{
    OpsConfig, OwnershipDecision, StatusCommand, TaskEditBuilder, edit_task_and_write,
    forced_note_text, validate_transition,
};
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

    let cfg = OpsConfig::default();
    let res = edit_task_and_write(
        ctx,
        &cfg,
        META.name,
        &args.task,
        out,
        |doc, task, _archive, agent, now| {
            let id_str = cfg.id_config.format_id(task.id);
            let (_, ownership) =
                validate_transition(task, StatusCommand::Done, agent, ctx.force, &id_str)?;

            let mut builder = TaskEditBuilder::new(doc, task);
            builder.set_status(Status::Done);
            builder.set_owner(Some(agent));
            builder.remove_claimed();
            if let Some(ref commit_hash) = resolved_commit {
                builder.set_field("commit", commit_hash);
            }

            let date = OpsConfig::format_note_date(now);
            if let OwnershipDecision::Forced { previous_owner } = ownership {
                builder.add_note(&date, agent, &forced_note_text("done", &previous_owner));
            }
            let note_text = match args.note.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                Some(text) => format!("done \u{2014} {text}"),
                None => "done".to_string(),
            };
            builder.add_note(&date, agent, &note_text);

            let (edits, _) = builder.build();
            Ok((edits, ()))
        },
    )?;

    if out.is_json() {
        out.emit_json_ok(META.name, &res.task);
    } else {
        out.write_line(&format!("{}: done ok", res.task_id));
        if let Some(ref accept) = res.task.accept {
            out.write_line(&format!("accept: {accept}"));
        }
    }
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
