//! `todo guard` command: Claude Code `PreToolUse` hook preventing direct edits to `TODO.md` (`T-053`; R-AGENT-4).

use std::io::{IsTerminal, Read as _};
use std::path::{Path, PathBuf};

use clap::Args as ClapArgs;
use serde::Serialize;
use serde_json::Value;

use crate::cli::{CommandKind, CommandMeta, Context};
use crate::error::{ErrorCode, ExitCode, Result, TodoError};
use crate::find::{canonicalize_clean, find_todo, FindOptions, WorktreeMode};
use crate::lock::{current_time, is_lease_live};
use crate::ops::OpsConfig;
use crate::output::Output;
use crate::state::StateDir;

/// Metadata for the `guard` command.
pub const META: CommandMeta = CommandMeta {
    name: "guard",
    summary: "Claude Code PreToolUse hook that blocks direct edits to TODO.md",
    kind: CommandKind::Meta,
    details: "Reads a Claude Code PreToolUse JSON payload from standard input (or `--path`), and blocks edits to `TODO.md` or the archive file unless a manual lease (`todo lock`) is live.",
    examples: &[
        "todo guard --path TODO.md",
        "echo '{\"tool_name\":\"Edit\",\"tool_input\":{\"file_path\":\"TODO.md\"}}' | todo guard",
    ],
};

/// Arguments for `todo guard`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Explicit target file path (used instead of reading Claude Code JSON from stdin).
    #[arg(long, value_name = "PATH")]
    pub path: Option<PathBuf>,
}

#[derive(Serialize)]
struct GuardJson {
    allowed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

/// Runs `todo guard`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let target_path = match args.path {
        Some(p) => Some(p),
        None => read_target_from_stdin()?,
    };

    let Some(target) = target_path else {
        if out.is_json() {
            out.emit_json_ok(
                META.name,
                &GuardJson {
                    allowed: true,
                    target: None,
                    reason: None,
                },
            );
        }
        return Ok(());
    };

    let found = match find_todo(&FindOptions {
        explicit_file: ctx.file.as_deref(),
        explicit_source: ctx.file_source,
        start_dir: None,
        worktree: WorktreeMode::Shared,
    }) {
        Ok(f) => f,
        Err(_) => {
            if out.is_json() {
                out.emit_json_ok(
                    META.name,
                    &GuardJson {
                        allowed: true,
                        target: Some(target.display().to_string()),
                        reason: Some("no TODO.md found".to_string()),
                    },
                );
            }
            return Ok(());
        }
    };

    let cfg = OpsConfig::load_next_to(&found.path)?;
    let archive_path = cfg.archive_path(&found.path);

    let abs_target = make_absolute(&target);
    let abs_todo = make_absolute(&found.path);
    let abs_archive = make_absolute(&archive_path);

    let is_protected = paths_match(&abs_target, &abs_todo) || paths_match(&abs_target, &abs_archive);
    if !is_protected {
        if out.is_json() {
            out.emit_json_ok(
                META.name,
                &GuardJson {
                    allowed: true,
                    target: Some(target.display().to_string()),
                    reason: None,
                },
            );
        }
        return Ok(());
    }

    let state = StateDir::from_found(&found)?;
    if let Some(lease) = state.read_lease()?
        && is_lease_live(&lease, current_time())
    {
        if out.is_json() {
            out.emit_json_ok(
                META.name,
                &GuardJson {
                    allowed: true,
                    target: Some(target.display().to_string()),
                    reason: Some(format!("live lease held by {}", lease.agent)),
                },
            );
        }
        return Ok(());
    }

    let file_name = found
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "TODO.md".to_string());
    let msg = format!(
        "direct edits to {file_name} are blocked; use `todo` commands instead \
         (for example `todo add`, `todo claim`, `todo note`, `todo done`, or `todo edit --agent <name>`)"
    );
    Err(TodoError::new(ErrorCode::Usage, msg)
        .with_exit_code(ExitCode::Usage)
        .with_file(target.display().to_string()))
}

fn read_target_from_stdin() -> Result<Option<PathBuf>> {
    if std::io::stdin().is_terminal() {
        return Ok(None);
    }
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf).map_err(|e| {
        TodoError::new(
            ErrorCode::BadValue,
            format!("failed reading hook input from stdin: {e}"),
        )
    })?;
    let trimmed = buf.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let val: Value = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    let path_str = val
        .get("tool_input")
        .and_then(|ti| ti.get("file_path").or_else(|| ti.get("path")))
        .and_then(|v| v.as_str());
    Ok(path_str.map(PathBuf::from))
}

fn make_absolute(path: &Path) -> PathBuf {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else if let Ok(cwd) = std::env::current_dir() {
        cwd.join(path)
    } else {
        path.to_path_buf()
    };
    canonicalize_clean(&joined).unwrap_or(joined)
}

fn paths_match(a: &Path, b: &Path) -> bool {
    if cfg!(windows) {
        a.to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy())
    } else {
        a == b
    }
}
