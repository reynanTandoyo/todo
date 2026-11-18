//! `todo migrate` command: bring `TODO.md` and its archive to the current format version
//! (`docs/format.md` §18, R-MIG-1), and the installed project files to the running tool's content
//! (R-MIG-2).

use std::fs;

use clap::Args as ClapArgs;
use serde::Serialize;
use similar::TextDiff;

use crate::agents::project::{self, ItemKind, ProjectUpdate};
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
    details: "Applies every pending format migration to TODO.md and TODO-archive.md, the same way the first write after an update does (docs/format.md §18), and raises the tool stamp. It then lists the installed project files that are out of date: the agent rules block in TODO.md, AGENTS.md and CLAUDE.md, the Claude Code skill and the Claude Code hooks. `--yes` updates exactly those and never installs one that is not there. A second run changes nothing. `--dry-run` prints the diffs of what the same command would write. Read commands never migrate.",
    examples: &[
        "todo migrate --dry-run --agent maintainer",
        "todo migrate --agent maintainer",
        "todo migrate --yes --agent maintainer",
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
    project: Vec<ProjectRow>,
}

/// One installed project file that is out of date, and whether `--yes` updated it (R-MIG-2).
#[derive(Serialize)]
struct ProjectRow {
    file: String,
    kind: ItemKind,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    events: Vec<&'static str>,
    updated: bool,
}

impl ProjectRow {
    fn new(kind: ItemKind, file: &str, events: &[&'static str], updated: bool) -> Self {
        Self {
            file: file.to_string(),
            kind,
            events: events.to_vec(),
            updated,
        }
    }

    fn describe(&self) -> String {
        self.kind.describe(&self.events)
    }
}

/// Runs `todo migrate`.
pub fn run(_args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let agent = ctx.require_agent()?.to_owned();
    let cfg = ctx.load_config()?;
    let found = ctx.find_todo_with_mode(cfg.worktree, out)?;
    let state = StateDir::from_found(&found)?;
    let label = file_label(&found.path);
    let archive_path = cfg.archive_path(&found.path);

    // The project files are looked at before the lock. A file that cannot be checked is a
    // warning and does not stop the migration of TODO.md.
    let project::ProjectPlan { updates, warnings } = project::plan(&found.path);
    for warn in warnings {
        out.warn(warn);
    }
    let (todo_plan, other_plan): (Vec<ProjectUpdate>, Vec<ProjectUpdate>) = updates
        .into_iter()
        .partition(|u| u.path == found.path);
    let apply = ctx.yes && !ctx.dry_run;

    let mut lock_warnings = Vec::new();
    let mut dry_run_diff: Option<String> = None;
    let mut migrated: Vec<MigratedFile> = Vec::new();
    let mut rules_block_fixed = false;
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
            let mut new_bytes = stamped_bytes(&doc, &label)?;

            // The rules block in TODO.md is a project file: it changes only with --yes (R-MIG-2).
            rules_block_fixed = false;
            if ctx.yes
                && let Some(fixed) = project::rules_block_fix(&new_bytes, &label)?
            {
                new_bytes = fixed;
                rules_block_fixed = true;
            }

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

    // The other project files (R-MIG-2). Each is written only with --yes, and only if it is
    // still the file that was planned from.
    let mut rows = Vec::new();
    let mut project_diff = String::new();
    if rules_block_fixed {
        rows.push(ProjectRow::new(
            ItemKind::RulesBlock,
            &label,
            &[],
            apply,
        ));
    } else if !ctx.yes && !todo_plan.is_empty() {
        rows.push(ProjectRow::new(ItemKind::RulesBlock, &label, &[], false));
    }
    for update in &other_plan {
        if apply {
            project::apply(update)?;
        }
        if ctx.dry_run && ctx.yes {
            project_diff.push_str(&update.diff());
        }
        rows.push(ProjectRow::new(update.kind, &update.file, &update.events, apply));
    }

    let mut dry_diff = dry_run_diff.unwrap_or_default();
    dry_diff.push_str(&project_diff);

    if out.is_json() {
        if ctx.dry_run {
            out.set_dry_run_diff(dry_diff);
        }
        out.emit_json_ok(
            META.name,
            &MigrateJson {
                file: label,
                current_format: CURRENT_FORMAT_VERSION,
                migrated,
                changed,
                project: rows,
            },
        );
        return Ok(());
    }

    if ctx.dry_run && !dry_diff.is_empty() {
        out.write_str(&dry_diff);
        if !dry_diff.ends_with('\n') {
            out.write_str("\n");
        }
    } else if let Some(line) = status_line(&label, &migrated, changed, rules_block_fixed) {
        out.write_line(&line);
    }

    if apply {
        for row in &rows {
            out.write_line(&format!("updated {}: {}", row.file, row.describe()));
        }
    } else if !ctx.yes && !rows.is_empty() {
        for row in &rows {
            out.write_line(&format!("{}: out of date ({})", row.file, row.describe()));
        }
        out.write_line("run todo migrate --yes to update them");
    }
    Ok(())
}

/// The line about TODO.md's format and stamp, or `None` when the only change was its rules block,
/// which has its own line.
fn status_line(
    label: &str,
    migrated: &[MigratedFile],
    changed: bool,
    rules_block_fixed: bool,
) -> Option<String> {
    if !migrated.is_empty() {
        Some(format!("migrated {}", describe(migrated)))
    } else if changed && rules_block_fixed {
        None
    } else if changed {
        Some(format!(
            "{label} is in format {CURRENT_FORMAT_VERSION}; tool stamp raised to {RUNNING_VERSION}"
        ))
    } else {
        Some(format!(
            "{label} is already in format {CURRENT_FORMAT_VERSION}: nothing to migrate"
        ))
    }
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
