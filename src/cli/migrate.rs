//! `todo migrate` command: bring `TODO.md` and its archive to the current format version (`docs/format.md` §18, R-MIG-1).

use std::fs;

use clap::Args as ClapArgs;
use serde::Serialize;
use similar::TextDiff;

use crate::cli::{CommandKind, CommandMeta, Context};
use crate::doc::CURRENT_FORMAT_VERSION;
use crate::error::{ErrorCode, Result, TodoError};
use crate::find::fs_path;
use crate::lock::{WriteOutcomeFull, locked_read_modify_write_full};
use crate::ops::migrate::{MigratedFile, PendingMigration, describe, stamped_bytes};
use crate::ops::{file_label, parse_doc_for_write, read_optional_archive};
use crate::output::Output;
use crate::state::StateDir;
use crate::version::RUNNING_VERSION;

/// Metadata for the `migrate` command.
pub const META: CommandMeta = CommandMeta {
    name: "migrate",
    summary: "Bring TODO.md and its archive up to the current format version",
    kind: CommandKind::Write,
    details: "Applies every pending format migration to TODO.md and TODO-archive.md, the same way the first write after an update does (docs/format.md §18), and raises the tool stamp. Nothing else changes, and a second run changes nothing. `--dry-run` prints the diff. Read commands never migrate.",
    examples: &[
        "todo migrate --dry-run --agent maintainer",
        "todo migrate --agent maintainer",
    ],
};

/// Arguments for `todo migrate` (none).
#[derive(Debug, ClapArgs)]
pub struct Args {}

#[derive(Serialize)]
struct MigrateJson {
    file: String,
    current_format: u32,
    migrated: Vec<MigratedFile>,
    changed: bool,
}

/// Runs `todo migrate`.
pub fn run(_args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let agent = ctx.require_agent()?.to_owned();
    let cfg = ctx.load_config()?;
    let found = ctx.find_todo_with_mode(cfg.worktree, out)?;
    let state = StateDir::from_found(&found)?;
    let label = file_label(&found.path);
    let archive_path = cfg.archive_path(&found.path);

    let mut lock_warnings = Vec::new();
    let mut dry_run_diff: Option<String> = None;
    let mut migrated: Vec<MigratedFile> = Vec::new();
    let changed = locked_read_modify_write_full(
        &found.path,
        &state,
        &agent,
        META.name,
        &cfg.lock,
        Some(&cfg.id_config),
        &mut lock_warnings,
        |bytes, _snap| {
            let parsed = parse_doc_for_write(bytes, &label, &cfg)?;
            let archive = read_optional_archive(&archive_path, cfg.id_config.clone())?;
            let pending =
                PendingMigration::plan(&parsed, &label, archive.as_ref(), &archive_path, &cfg)?;
            migrated = pending.files.clone();
            let archive_write = pending.archive.clone();
            let (migrated_todo, migration) = pending.split();
            let doc = migrated_todo.map_or(parsed, |m| m.doc);
            let new_bytes = stamped_bytes(&doc, &label)?;

            if new_bytes == bytes && migration.is_none() {
                return Ok(WriteOutcomeFull::Unchanged(false));
            }
            if ctx.dry_run {
                let mut diff = text_diff(&label, bytes, &new_bytes);
                if let Some(arch) = &archive_write {
                    let old_archive = fs::read(fs_path(&arch.path).as_ref()).map_err(|e| {
                        TodoError::new(
                            ErrorCode::WriteFailed,
                            format!("failed reading {}: {e}", arch.path.display()),
                        )
                    })?;
                    diff.push_str(&text_diff(&cfg.archive_file, &old_archive, &arch.new_bytes));
                }
                dry_run_diff = Some(diff);
                return Ok(WriteOutcomeFull::Unchanged(false));
            }
            Ok(WriteOutcomeFull::Changed {
                new_bytes,
                touched_tasks: Some(Vec::new()),
                value: true,
                migration,
            })
        },
    )?;

    for warn in lock_warnings {
        out.warn(warn);
    }

    if out.is_json() {
        if let Some(diff) = dry_run_diff {
            out.set_dry_run_diff(diff);
        }
        out.emit_json_ok(
            META.name,
            &MigrateJson {
                file: label,
                current_format: CURRENT_FORMAT_VERSION,
                migrated,
                changed,
            },
        );
        return Ok(());
    }

    if let Some(diff) = dry_run_diff {
        if diff.is_empty() {
            out.write_line("dry-run: no changes");
        } else {
            out.write_str(&diff);
            if !diff.ends_with('\n') {
                out.write_str("\n");
            }
        }
        return Ok(());
    }

    if !migrated.is_empty() {
        out.write_line(&format!("migrated {}", describe(&migrated)));
    } else if changed {
        out.write_line(&format!(
            "{label} is in format {CURRENT_FORMAT_VERSION}; tool stamp raised to {RUNNING_VERSION}"
        ));
    } else {
        out.write_line(&format!(
            "{label} is already in format {CURRENT_FORMAT_VERSION}: nothing to migrate"
        ));
    }
    Ok(())
}

/// The unified diff from `old` to `new`, headed like the other write commands' previews.
fn text_diff(label: &str, old: &[u8], new: &[u8]) -> String {
    let old_text = String::from_utf8_lossy(old);
    let new_text = String::from_utf8_lossy(new);
    TextDiff::from_lines(old_text.as_ref(), new_text.as_ref())
        .unified_diff()
        .header(&format!("a/{label}"), &format!("b/{label}"))
        .to_string()
}
