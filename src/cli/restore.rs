//! `todo restore` command: restore a saved snapshot under the write lock (`R-HIST-2`, `R-WRITE-14`).

use clap::Args as ClapArgs;
use serde::Serialize;
use similar::TextDiff;

use crate::atomic::{read_file_with_snapshot, validate_utf8};
use crate::cli::{CommandKind, CommandMeta, Context};
use crate::doc::stamp_bytes;
use crate::error::{ErrorCode, Result, TodoError};
use crate::history::{parse_snapshot_number, read_snapshot_bytes};
use crate::lock::{WriteOutcome, locked_read_modify_write};
use crate::ops::{OpsConfig, file_label, require_known_format_bytes, require_yes_or_preview};
use crate::output::Output;
use crate::state::StateDir;

/// Metadata for the `restore` command.
pub const META: CommandMeta = CommandMeta {
    name: "restore",
    summary: "Restore TODO.md from a saved snapshot",
    kind: CommandKind::Write,
    details: "Replaces TODO.md with snapshot `n` under the write lock, after saving the current file as its own snapshot so the restore can also be undone. Requires `--yes` to apply.",
    examples: &[
        "todo restore 3 --agent agent-1",
        "todo restore 3 --yes --agent agent-1",
    ],
};

/// Arguments for `todo restore`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Snapshot number to restore (e.g. `3` or `000003`).
    #[arg(value_name = "N")]
    pub snapshot: String,
}

#[derive(Serialize)]
struct RestoreJson {
    restored_from: u64,
    applied: bool,
    changed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    diff: Option<String>,
}

/// Runs `todo restore`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let agent = ctx.require_agent()?.to_owned();
    let num = parse_snapshot_number(&args.snapshot)?;
    let found = ctx.find_todo(out)?;
    if let Some(warning) = crate::find::synced_folder_warning(&found.path) {
        out.warn(warning);
    }
    let cfg = OpsConfig::load_next_to(&found.path)?;
    let state = StateDir::from_found(&found)?;
    let label = file_label(&found.path);

    let (entry, snap_bytes) = read_snapshot_bytes(&state, num)?;
    let _ = validate_utf8(&snap_bytes, &entry.file)?;
    // Neither the snapshot nor the file it would replace may be in a newer format (R-VER-2).
    require_known_format_bytes(&snap_bytes, &label, &cfg.id_config)?;

    if !ctx.yes || ctx.dry_run {
        let current = read_file_with_snapshot(&found.path)?;
        require_known_format_bytes(&current.bytes, &label, &cfg.id_config)?;
        let changed = current.bytes != snap_bytes;
        let old_text = String::from_utf8_lossy(&current.bytes);
        let new_text = String::from_utf8_lossy(&snap_bytes);
        let diff = TextDiff::from_lines(old_text.as_ref(), new_text.as_ref())
            .unified_diff()
            .header(&format!("a/{label}"), &format!("b/{label}#{}", entry.number))
            .to_string();

        if out.is_json() {
            out.set_dry_run_diff(diff.clone());
            out.emit_json_ok(
                META.name,
                &RestoreJson {
                    restored_from: entry.number,
                    applied: false,
                    changed,
                    diff: Some(diff),
                },
            );
            return Ok(());
        }

        let preview = if !changed || diff.is_empty() {
            format!(
                "snapshot #{:06} is identical to {}",
                entry.number,
                found.path.display()
            )
        } else {
            diff
        };
        let _ = require_yes_or_preview(ctx, &preview, out);
        return Ok(());
    }

    let changed = locked_read_modify_write(
        &found.path,
        &state,
        &agent,
        "restore",
        &cfg.lock,
        |current_bytes, _snap| {
            require_known_format_bytes(current_bytes, &label, &cfg.id_config)?;
            // The restored file gets the tool stamp too, so an old snapshot never lowers it (R-VER-1).
            let restored = stamp_bytes(&snap_bytes, cfg.id_config.clone()).map_err(|e| {
                TodoError::new(ErrorCode::WriteFailed, e.to_string()).with_file(label.clone())
            })?;
            if current_bytes == restored.as_slice() {
                return Ok(WriteOutcome::Unchanged(false));
            }
            Ok(WriteOutcome::Changed {
                new_bytes: restored,
                value: true,
            })
        },
    )?;

    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &RestoreJson {
                restored_from: entry.number,
                applied: true,
                changed,
                diff: None,
            },
        );
        return Ok(());
    }

    if changed {
        out.write_line(&format!(
            "restored {} from snapshot #{:06}",
            found.path.display(),
            entry.number
        ));
    } else {
        out.write_line(&format!(
            "{} already matches snapshot #{:06}",
            found.path.display(),
            entry.number
        ));
    }
    Ok(())
}
