//! `todo hooks` command: install or uninstall Claude Code hooks in `.claude/settings.json` (`T-053`; R-AGENT-4).

use std::fs;
use std::path::Path;

use clap::{Args as ClapArgs, Subcommand};
use serde::Serialize;
use serde_json::Value;
use similar::TextDiff;

use crate::agents::hooks_file::{install_hook_text, uninstall_hook_text};
use crate::agents::project::{BACKUP_REL_PATH, SETTINGS_REL_PATH};
use crate::atomic::atomic_write_bytes;
use crate::cli::{CommandKind, CommandMeta, Context};
use crate::error::{ErrorCode, Result, TodoError};
use crate::find::fs_path;
use crate::ops::require_yes_or_preview;
use crate::output::Output;

/// Metadata for the `hooks` command.
pub const META: CommandMeta = CommandMeta {
    name: "hooks",
    summary: "Install or uninstall Claude Code hooks in .claude/settings.json",
    kind: CommandKind::Meta,
    details: "`hooks install --yes` adds a `SessionStart` hook (`todo summary`), a `PreToolUse` hook (`todo guard` for `Edit|Write|MultiEdit`) and a `SessionEnd` hook (`todo session-end`, which releases the ending session's claims) to `.claude/settings.json` in the folder that holds `TODO.md`, keeping a backup and preserving formatting. `hooks uninstall --yes` removes only the entries added by `todo`, keeping every other byte of the file.",
    examples: &[
        "todo hooks install",
        "todo hooks install --yes",
        "todo hooks uninstall --yes",
    ],
};

/// Arguments for `todo hooks`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Subcommand (`install` or `uninstall`).
    #[command(subcommand)]
    pub action: HooksAction,
}

/// Subcommands under `todo hooks`.
#[derive(Debug, Subcommand)]
pub enum HooksAction {
    /// Add `todo` SessionStart, PreToolUse and SessionEnd hooks to `.claude/settings.json`.
    Install,
    /// Remove only the `todo`-managed hooks from `.claude/settings.json`.
    Uninstall,
}

#[derive(Serialize)]
struct HooksJson {
    file: String,
    action: &'static str,
    applied: bool,
    changed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    backup: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diff: Option<String>,
}

/// Runs `todo hooks`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    // The settings file is in the folder of TODO.md, not the current folder (T-127).
    let dir = ctx.project_dir(out)?;
    let settings_path = dir.join(SETTINGS_REL_PATH);
    let backup_path = dir.join(BACKUP_REL_PATH);
    let label = SETTINGS_REL_PATH.to_string();

    let old_bytes = match fs::read(fs_path(&settings_path).as_ref()) {
        Ok(b) => Some(b),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            return Err(TodoError::new(
                ErrorCode::WriteFailed,
                format!("could not read {label}: {e}"),
            )
            .with_file(label));
        }
    };

    let old_text = match &old_bytes {
        Some(b) => std::str::from_utf8(b).map_err(|e| {
            TodoError::new(
                ErrorCode::BadValue,
                format!("{label} is not valid UTF-8: {e}"),
            )
            .with_file(label.clone())
        })?,
        None => "",
    };

    if !old_text.trim().is_empty() {
        serde_json::from_str::<Value>(old_text).map_err(|e| {
            TodoError::new(
                ErrorCode::BadValue,
                format!("{label} is not valid JSON: {e}"),
            )
            .with_file(label.clone())
        })?;
    }

    // Both actions edit the text in place, so every byte outside the managed entries stays
    // (T-126, T-130).
    let (action_name, edited) = match args.action {
        HooksAction::Install => ("install", install_hook_text(old_text)),
        HooksAction::Uninstall => ("uninstall", uninstall_hook_text(old_text)),
    };
    let new_text = edited
        .map_err(|msg| TodoError::new(ErrorCode::BadValue, msg).with_file(label.clone()))?;
    let new_bytes = new_text.as_bytes();
    // A missing file has nothing to remove, so uninstall does not create one.
    let changed = match old_bytes.as_deref() {
        Some(old) => old != new_bytes,
        None => !new_bytes.is_empty(),
    };

    if !ctx.yes || ctx.dry_run {
        let diff = TextDiff::from_lines(old_text, &new_text)
            .unified_diff()
            .header(&format!("a/{label}"), &format!("b/{label}"))
            .to_string();

        if out.is_json() {
            out.set_dry_run_diff(diff.clone());
            out.emit_json_ok(
                META.name,
                &HooksJson {
                    file: label,
                    action: action_name,
                    applied: false,
                    changed,
                    backup: None,
                    diff: Some(diff),
                },
            );
            return Ok(());
        }

        if !changed {
            out.write_line(&format!("{label}: already up to date"));
            return Ok(());
        }

        let preview = if diff.is_empty() {
            format!("would update {label}")
        } else {
            diff
        };
        let _ = require_yes_or_preview(ctx, &preview, out);
        return Ok(());
    }

    let mut wrote_backup = None;
    if changed {
        if let Some(ref orig) = old_bytes {
            atomic_write_bytes(&backup_path, orig)?;
            wrote_backup = Some(BACKUP_REL_PATH.to_string());
        }
        if let Some(parent) = settings_path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(fs_path(parent).as_ref()).map_err(|e| {
                TodoError::new(
                    ErrorCode::WriteFailed,
                    format!("could not create {}: {e}", parent.display()),
                )
            })?;
        }
        atomic_write_bytes(&settings_path, new_bytes)?;
    }

    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &HooksJson {
                file: label,
                action: action_name,
                applied: true,
                changed,
                backup: wrote_backup,
                diff: None,
            },
        );
    } else if changed {
        out.write_line(&format!("{label}: {action_name} ok"));
    } else {
        out.write_line(&format!("{label}: already up to date"));
    }

    Ok(())
}

/// Ensures unused Path import warning doesn't trigger if refactored.
#[allow(dead_code)]
fn _settings_exists(dir: &Path) -> bool {
    dir.join(SETTINGS_REL_PATH).exists()
}
