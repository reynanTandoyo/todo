//! `todo agents` command: `install` and `check` the managed agent-rules block (`R-AGENT-3`).

use std::fs;
use std::io;
use std::path::PathBuf;

use clap::{Args as ClapArgs, Subcommand};
use serde::Serialize;
use similar::TextDiff;

use crate::agents::{ManagedBlockStatus, check_managed_block_in_text, upsert_managed_block};
use crate::atomic::{atomic_replace, atomic_write_bytes, read_file_with_snapshot, validate_utf8};
use crate::cli::{CommandKind, CommandMeta, Context};
use crate::error::{ErrorCode, ExitCode, Result, TodoError};
use crate::find::fs_path;
use crate::ops::require_yes_or_preview;
use crate::output::Output;

/// Metadata for the `agents` command.
pub const META: CommandMeta = CommandMeta {
    name: "agents",
    summary: "Install or check the managed agent-rules block in AGENTS.md",
    kind: CommandKind::Meta,
    details: "`agents install [--into FILE] --yes` adds or updates the managed `<!-- todo:begin -->` ... `<!-- todo:end -->` rules block in `AGENTS.md` (or `--into FILE`) without changing any other byte. `agents check [--into FILE]` verifies the block is present and up to date.",
    examples: &[
        "todo agents install",
        "todo agents install --yes",
        "todo agents install --into CLAUDE.md --yes",
        "todo agents check",
        "todo agents check --into CLAUDE.md",
    ],
};

/// Arguments for `todo agents`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Subcommand (`install` or `check`).
    #[command(subcommand)]
    pub action: AgentsAction,
}

/// Subcommands under `todo agents`.
#[derive(Debug, Subcommand)]
pub enum AgentsAction {
    /// Add or update the managed agent-rules block in a Markdown file.
    Install {
        /// Target file path (defaults to `AGENTS.md`).
        #[arg(long, value_name = "FILE", default_value = "AGENTS.md")]
        into: PathBuf,
    },
    /// Verify that the managed agent-rules block is present and current.
    Check {
        /// Target file path (defaults to `AGENTS.md`).
        #[arg(long, value_name = "FILE", default_value = "AGENTS.md")]
        into: PathBuf,
    },
}

#[derive(Serialize)]
struct AgentsInstallJson {
    file: String,
    applied: bool,
    changed: bool,
    created: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    diff: Option<String>,
}

#[derive(Serialize)]
struct AgentsCheckJson {
    file: String,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
}

/// Runs `todo agents`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    match args.action {
        AgentsAction::Install { into } => run_install(into, ctx, out),
        AgentsAction::Check { into } => run_check(into, out),
    }
}

fn run_install(into: PathBuf, ctx: &Context, out: &mut Output) -> Result<()> {
    let label = into.display().to_string();
    let exists = fs_path(&into).exists();
    let existing = if exists {
        Some(read_file_with_snapshot(&into)?)
    } else {
        None
    };

    let old_bytes: &[u8] = existing.as_ref().map_or(&[], |r| r.bytes.as_slice());
    let new_bytes = upsert_managed_block(existing.as_ref().map(|r| r.bytes.as_slice()), &label)?;
    let changed = old_bytes != new_bytes.as_slice();

    if !ctx.yes || ctx.dry_run {
        let old_text = String::from_utf8_lossy(old_bytes);
        let new_text = String::from_utf8_lossy(&new_bytes);
        let diff = TextDiff::from_lines(old_text.as_ref(), new_text.as_ref())
            .unified_diff()
            .header(&format!("a/{label}"), &format!("b/{label}"))
            .to_string();

        if out.is_json() {
            out.set_dry_run_diff(diff.clone());
            out.emit_json_ok(
                META.name,
                &AgentsInstallJson {
                    file: label,
                    applied: false,
                    changed,
                    created: !exists,
                    diff: Some(diff),
                },
            );
            return Ok(());
        }

        let preview = if !changed {
            format!("{label}: agent rules block is already up to date")
        } else if diff.is_empty() {
            format!("would update {label}")
        } else {
            diff
        };
        let _ = require_yes_or_preview(ctx, &preview, out);
        return Ok(());
    }

    if changed {
        if let Some(snap) = existing {
            atomic_replace(&into, &snap.snapshot, &new_bytes)?;
        } else {
            if let Some(parent) = into.parent()
                && !parent.as_os_str().is_empty()
            {
                fs::create_dir_all(fs_path(parent).as_ref()).map_err(|e| {
                    TodoError::new(
                        ErrorCode::WriteFailed,
                        format!("failed creating directory {}: {e}", parent.display()),
                    )
                    .with_file(label.clone())
                })?;
            }
            atomic_write_bytes(&into, &new_bytes)?;
        }
    }

    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &AgentsInstallJson {
                file: label,
                applied: true,
                changed,
                created: !exists,
                diff: None,
            },
        );
        return Ok(());
    }

    if !changed {
        out.write_line(&format!("{label}: agent rules block already up to date"));
    } else if !exists {
        out.write_line(&format!("{label}: created with agent rules block"));
    } else {
        out.write_line(&format!("{label}: updated agent rules block"));
    }
    Ok(())
}

fn run_check(into: PathBuf, out: &mut Output) -> Result<()> {
    let label = into.display().to_string();
    let bytes = match fs::read(fs_path(&into).as_ref()) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            if !out.is_json() {
                out.write_line(&format!("{label}:1: missing file (no agent rules block)"));
            }
            return Err(TodoError::new(
                ErrorCode::NotFound,
                format!("{label}: missing file (no agent rules block)"),
            )
            .with_file(label)
            .with_line(1)
            .with_exit_code(ExitCode::Refused));
        }
        Err(e) => {
            return Err(TodoError::new(
                ErrorCode::WriteFailed,
                format!("failed reading {label}: {e}"),
            )
            .with_file(label));
        }
    };

    let text = validate_utf8(&bytes, &label)?;
    match check_managed_block_in_text(text) {
        ManagedBlockStatus::Current { begin_line, .. } => {
            if out.is_json() {
                out.emit_json_ok(
                    META.name,
                    &AgentsCheckJson {
                        file: label,
                        status: "current",
                        line: Some(begin_line),
                    },
                );
            } else {
                out.write_line(&format!("{label}: ok (agent rules block is current)"));
            }
            Ok(())
        }
        ManagedBlockStatus::Missing => {
            if !out.is_json() {
                out.write_line(&format!("{label}:1: missing agent rules block"));
            }
            Err(TodoError::new(
                ErrorCode::BadValue,
                format!("{label}:1: missing agent rules block"),
            )
            .with_file(label)
            .with_line(1)
            .with_exit_code(ExitCode::Refused))
        }
        ManagedBlockStatus::Outdated { begin_line, .. } => {
            if !out.is_json() {
                out.write_line(&format!("{label}:{begin_line}: outdated agent rules block"));
            }
            Err(TodoError::new(
                ErrorCode::BadValue,
                format!("{label}:{begin_line}: outdated agent rules block"),
            )
            .with_file(label)
            .with_line(begin_line)
            .with_exit_code(ExitCode::Refused))
        }
    }
}
