//! `todo edit` command: take a lease, open an editor, check, snapshot, and release (`T-048`; R-LOCK-8).

use std::process::Command;

use clap::Args as ClapArgs;
use serde::Serialize;
use serde_json::json;

use crate::atomic::read_file_with_snapshot;
use crate::cli::check::collect_problems;
use crate::cli::{CommandKind, CommandMeta, Context};
use crate::error::{ErrorCode, ExitCode, Result, TodoError};
use crate::find::synced_folder_warning;
use crate::lock::{acquire_lease, release_lease_with_file};
use crate::ops::{file_label, require_known_format_bytes};
use crate::output::Output;
use crate::state::{LastWrite, StateDir};

/// Metadata for the `edit` command.
pub const META: CommandMeta = CommandMeta {
    name: "edit",
    summary: "Take a lease, open TODO.md in an editor, check, snapshot, and unlock",
    kind: CommandKind::Write,
    details: "Acquires a manual editing lease, opens `TODO.md` in `VISUAL`, `EDITOR`, or Notepad, waits until the editor exits, runs `check`, records a snapshot if changed, and ends the lease.",
    examples: &[
        "todo edit --agent alice",
        "todo edit --agent alice --minutes 20",
    ],
};

/// Arguments for `todo edit`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Lease length in minutes (default 10, maximum 60).
    #[arg(long, value_name = "N")]
    pub minutes: Option<u64>,
}

#[derive(Serialize)]
struct EditJson {
    file: String,
    editor: String,
    clean: bool,
    problems: usize,
}

/// Runs `todo edit --agent <name>`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let agent = ctx.require_agent()?;
    let (found, ops_cfg) = ctx.find_todo_and_config(out)?;
    if let Some(warning) = synced_folder_warning(&found.path) {
        out.warn(warning);
    }
    // A file in a newer format is refused before the editor opens (R-VER-2).
    let before_edit = read_file_with_snapshot(&found.path)?;
    require_known_format_bytes(&before_edit.bytes, &file_label(&found.path), &ops_cfg.id_config)?;

    let state = StateDir::from_found(&found)?;
    let lock_cfg = ops_cfg.lock;

    if state.read_last_write()?.is_none()
        && let Ok(snap) = read_file_with_snapshot(&found.path)
    {
        let _ = state.write_last_write(&LastWrite::from_snapshot(&snap.snapshot));
    }

    let _lease = acquire_lease(&state, agent, args.minutes, &lock_cfg)?;

    let editor_raw = resolve_editor();
    let (program, extra_args) = parse_editor_command(&editor_raw);
    let editor_res = Command::new(&program)
        .args(&extra_args)
        .arg(&found.path)
        .status();

    let check_res = if editor_res.as_ref().is_ok_and(|s| s.success()) {
        Some(collect_problems(ctx, out))
    } else {
        None
    };

    // Always record any snapshot and release the lease before returning errors.
    let release_res = release_lease_with_file(&found.path, &state, agent, false, &lock_cfg);

    let status = editor_res.map_err(|e| {
        TodoError::new(
            ErrorCode::WriteFailed,
            format!("failed to launch editor {program:?}: {e}"),
        )
    })?;
    if !status.success() {
        let _ = release_res;
        return Err(TodoError::new(
            ErrorCode::WriteFailed,
            format!("editor {program:?} exited with {status}"),
        ));
    }

    release_res?;

    let (label, problems) = match check_res {
        Some(res) => res?,
        None => (String::new(), Vec::new()),
    };

    if !problems.is_empty() {
        if !out.is_json() {
            for p in &problems {
                out.write_line(&format!("{}:{}: {}", p.file, p.line, p.message));
            }
        }
        let count = problems.len();
        return Err(TodoError::new(
            ErrorCode::BadValue,
            format!("{count} problem(s) found after edit"),
        )
        .with_exit_code(ExitCode::Refused)
        .with_detail("count", json!(count))
        .with_detail("problems", json!(problems)));
    }

    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &EditJson {
                file: found.path.display().to_string(),
                editor: editor_raw,
                clean: true,
                problems: 0,
            },
        );
    } else {
        out.write_line(&format!("{label}: edit ok"));
    }

    Ok(())
}

fn resolve_editor() -> String {
    for key in ["VISUAL", "EDITOR"] {
        if let Ok(val) = std::env::var(key) {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    if cfg!(windows) {
        "notepad".to_string()
    } else {
        "vi".to_string()
    }
}

fn parse_editor_command(raw: &str) -> (String, Vec<String>) {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix('"')
        && let Some(end_quote) = rest.find('"')
    {
        let prog = rest[..end_quote].to_string();
        let tail = rest[end_quote + 1..]
            .split_whitespace()
            .map(str::to_string)
            .collect();
        return (prog, tail);
    }
    let mut parts = trimmed.split_whitespace();
    let prog = parts.next().unwrap_or(trimmed).to_string();
    let args = parts.map(str::to_string).collect();
    (prog, args)
}
