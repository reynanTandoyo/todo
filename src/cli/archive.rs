//! `todo archive` command: move done and dropped tasks to `TODO-archive.md` (`R-ARCH-1`, `docs/format.md` §14).

use std::collections::BTreeMap;

use chrono::{DateTime, FixedOffset};
use clap::Args as ClapArgs;
use serde::Serialize;

use crate::atomic::{atomic_replace, atomic_write_bytes, read_file_with_snapshot, validate_utf8};
use crate::version::RUNNING_VERSION;
use crate::cli::{CommandKind, CommandMeta, Context};
use crate::doc::{
    CURRENT_FORMAT_VERSION, Document, LineEdit, SectionLookup, Status, Task, TaskChangeIntent,
    format_line_text,
};
use crate::error::{ErrorCode, Result, TodoError};
use crate::find::fs_path;
use crate::lock::{MigrationWrite, WriteOutcomeFull, current_time, locked_read_modify_write_full};
use crate::ops::migrate::{MigratedFile, describe, migrate_for_write};
use crate::ops::{
    OpsConfig, check_task_writable, file_label, parse_doc_for_write, read_optional_archive,
    require_yes_or_preview,
};
use crate::output::Output;
use crate::state::{IdsState, StateDir};

/// Metadata for the `archive` command.
pub const META: CommandMeta = CommandMeta {
    name: "archive",
    summary: "Move done and dropped tasks to TODO-archive.md",
    kind: CommandKind::Write,
    details: "Moves done and dropped tasks from TODO.md into TODO-archive.md under the same section names, under the write lock. Requires `--yes` to apply.",
    examples: &[
        "todo archive --agent coordinator",
        "todo archive --yes --agent coordinator",
        "todo archive --older-than 14 --yes --agent coordinator",
    ],
};

/// Arguments for `todo archive`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Only archive tasks whose latest note (or claim) is at least DAYS days old.
    #[arg(long, value_name = "DAYS")]
    pub older_than: Option<u64>,
}

#[derive(Serialize)]
struct ArchivedTaskSummary {
    id: String,
    title: String,
    status: &'static str,
    section: String,
}

#[derive(Serialize)]
struct ArchiveJson {
    archive_file: String,
    applied: bool,
    count: usize,
    tasks: Vec<ArchivedTaskSummary>,
}

/// Runs `todo archive`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let agent = ctx.require_agent()?.to_owned();
    let found = ctx.find_todo(out)?;
    if let Some(warning) = crate::find::synced_folder_warning(&found.path) {
        out.warn(warning);
    }
    let cfg = OpsConfig::load_next_to(&found.path)?;
    let state = StateDir::from_found(&found)?;
    let label = file_label(&found.path);
    let archive_path = cfg.archive_path(&found.path);

    if !ctx.yes || ctx.dry_run {
        let snap = read_file_with_snapshot(&found.path)?;
        let doc = parse_doc_for_write(&snap.bytes, &label, &cfg)?;
        let now = current_time();
        let selected = select_tasks_to_archive(&doc, args.older_than, now);
        let summaries = summarize_selected(&doc, &selected);

        if out.is_json() {
            out.emit_json_ok(
                META.name,
                &ArchiveJson {
                    archive_file: cfg.archive_file.clone(),
                    applied: false,
                    count: summaries.len(),
                    tasks: summaries,
                },
            );
            return Ok(());
        }

        let preview = format_preview(&summaries, &cfg.archive_file);
        let _ = require_yes_or_preview(ctx, &preview, out);
        return Ok(());
    }

    let mut lock_warnings = Vec::new();
    let mut stamp_warning: Option<String> = None;
    let mut migrated_notice: Option<String> = None;
    let summaries = locked_read_modify_write_full(
        &found.path,
        &state,
        &agent,
        "archive",
        &cfg.lock,
        Some(&cfg.id_config),
        &mut lock_warnings,
        |bytes, _snap| {
            let parsed = parse_doc_for_write(bytes, &label, &cfg)?;
            stamp_warning = parsed.newer_tool_warning(&label);
            let existing_archive = read_optional_archive(&archive_path, cfg.id_config.clone())?;
            // TODO.md is migrated in this write when it is in an older format (R-MIG-1). The
            // archive is migrated where it is written below.
            let migrated_todo = migrate_for_write(&parsed, &label)?;
            let doc = migrated_todo.as_ref().map_or(&parsed, |m| &m.doc);
            let now = current_time();
            let selected = select_tasks_to_archive(doc, args.older_than, now);
            if selected.is_empty() {
                return Ok(WriteOutcomeFull::Unchanged(Vec::new()));
            }

            for &task in &selected {
                check_task_writable(doc, task, &label, existing_archive.as_ref())?;
            }

            let summaries = summarize_selected(doc, &selected);

            // Ensure highest_id in ids.json never forgets archived IDs.
            let max_selected_id = selected.iter().map(|t| t.id.number()).max().unwrap_or(0);
            let current_ids = state.read_ids()?.unwrap_or_default();
            if max_selected_id > current_ids.highest_id {
                state.write_ids(&IdsState {
                    highest_id: max_selected_id,
                })?;
            }

            // Build updated archive file bytes and write atomically under TODO.md's lock.
            let archive_migration = write_updated_archive(doc, &selected, &archive_path, &cfg)?;

            // Remove archived task blocks from TODO.md (plus one adjacent blank line when safe).
            let mut edits: Vec<LineEdit> = selected
                .iter()
                .map(|t| {
                    let mut del_end = t.block_range.end;
                    if del_end < doc.line_doc.lines.len()
                        && doc.line_doc.lines[del_end].text.trim().is_empty()
                    {
                        del_end += 1;
                    }
                    LineEdit::remove(t.block_range.start..del_end)
                })
                .collect();
            edits.sort_by_key(|e| e.range.start);
            let intent = TaskChangeIntent {
                removed_ids: selected.iter().map(|t| t.id).collect(),
                ..TaskChangeIntent::default()
            };
            let applied = doc
                .apply_and_verify(&edits, &intent)
                .map_err(|e| {
                    TodoError::new(ErrorCode::WriteFailed, e.to_string()).with_file(label.clone())
                })?;
            // The same write adds or raises the tool stamp (R-VER-1).
            let new_bytes = match applied.doc.with_tool_stamp(RUNNING_VERSION).map_err(|e| {
                TodoError::new(ErrorCode::WriteFailed, e.to_string()).with_file(label.clone())
            })? {
                Some(stamped) => stamped.bytes,
                None => applied.bytes,
            };

            let mut files: Vec<MigratedFile> = Vec::new();
            if let Some(m) = &migrated_todo {
                files.push(MigratedFile {
                    file: label.clone(),
                    from: m.from,
                    to: m.to,
                });
            }
            if let Some((from, to)) = archive_migration {
                files.push(MigratedFile {
                    file: cfg.archive_file.clone(),
                    from,
                    to,
                });
            }
            let migration = (!files.is_empty()).then(|| MigrationWrite {
                summary: describe(&files),
                archive: None,
            });
            migrated_notice = migration.as_ref().map(|m| m.summary.clone());
            Ok(WriteOutcomeFull::Changed {
                new_bytes,
                touched_tasks: None,
                value: summaries,
                migration,
            })
        },
    )?;

    for w in lock_warnings {
        out.warn(w);
    }
    if let Some(warn) = stamp_warning {
        out.warn(warn);
    }
    if let Some(summary) = migrated_notice {
        out.notice(format!("migrated {summary}"));
    }

    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &ArchiveJson {
                archive_file: cfg.archive_file,
                applied: true,
                count: summaries.len(),
                tasks: summaries,
            },
        );
        return Ok(());
    }

    if summaries.is_empty() {
        out.write_line("no tasks to archive");
    } else {
        out.write_line(&format!(
            "archived {} task(s) to {}",
            summaries.len(),
            cfg.archive_file
        ));
    }
    Ok(())
}

fn select_tasks_to_archive(
    doc: &Document,
    older_than: Option<u64>,
    now: DateTime<FixedOffset>,
) -> Vec<&Task> {
    doc.tasks
        .iter()
        .filter(|t| matches!(t.status, Status::Done | Status::Dropped))
        .filter(|t| match older_than {
            None => true,
            Some(days) => task_age_days(t, now).is_some_and(|age| age >= days as i64),
        })
        .collect()
}

fn task_age_days(task: &Task, now: DateTime<FixedOffset>) -> Option<i64> {
    let today = now.date_naive();
    if let Some(latest_note_date) = task.notes.iter().map(|n| n.date).max() {
        return Some((today - latest_note_date).num_days());
    }
    if let Some(claimed_dt) = task.parse_claimed_at() {
        return Some((today - claimed_dt.date_naive()).num_days());
    }
    None
}

fn task_section_name(doc: &Document, task: &Task) -> String {
    task.section
        .and_then(|idx| doc.sections.get(idx))
        .map_or_else(|| "Tasks".to_string(), |s| s.name.clone())
}

fn summarize_selected(doc: &Document, selected: &[&Task]) -> Vec<ArchivedTaskSummary> {
    selected
        .iter()
        .map(|t| ArchivedTaskSummary {
            id: doc.id_config.format_id(t.id),
            title: t.title.clone(),
            status: t.status.as_str(),
            section: task_section_name(doc, t),
        })
        .collect()
}

fn format_preview(summaries: &[ArchivedTaskSummary], archive_file: &str) -> String {
    if summaries.is_empty() {
        return "no tasks to archive".to_string();
    }
    let mut out = format!(
        "would archive {} task(s) to {archive_file}:\n",
        summaries.len()
    );
    for s in summaries {
        out.push_str(&format!(
            "  {} [{}] · {} (section: {})\n",
            s.id, s.status, s.title, s.section
        ));
    }
    out.trim_end().to_string()
}

/// Writes the archive file with the selected tasks added. Returns the format versions the archive
/// was migrated from and to, when it was in an older format (R-MIG-1).
fn write_updated_archive(
    doc: &Document,
    selected: &[&Task],
    archive_path: &std::path::Path,
    cfg: &OpsConfig,
) -> Result<Option<(u32, u32)>> {
    let eol = "\n";
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut section_order: Vec<String> = Vec::new();

    for &task in selected {
        let sec = task_section_name(doc, task);
        if !grouped.contains_key(&sec) {
            section_order.push(sec.clone());
        }
        let block = extract_task_block_lines(doc, task);
        let entry = grouped.entry(sec).or_default();
        if !entry.is_empty() {
            entry.push(String::new());
        }
        entry.extend(block);
    }

    let archive_exists = fs_path(archive_path).exists();
    let mut archive_migration = None;
    let (existing_snap, arch_doc) = if archive_exists {
        let snap = read_file_with_snapshot(archive_path)?;
        let _ = validate_utf8(&snap.bytes, &cfg.archive_file)?;
        let parsed = parse_doc_for_write(&snap.bytes, &cfg.archive_file, cfg)?;
        // An archive in an older format is migrated in the same write (R-MIG-1).
        let arch_doc = match migrate_for_write(&parsed, &cfg.archive_file)? {
            Some(migrated) => {
                archive_migration = Some((migrated.from, migrated.to));
                migrated.doc
            }
            None => parsed,
        };
        (Some(snap), arch_doc)
    } else {
        let initial = format!("{}{eol}# Archive{eol}", format_line_text(CURRENT_FORMAT_VERSION));
        let parsed = Document::parse_bytes_with_config(initial.as_bytes(), cfg.id_config.clone())
            .map_err(|e| TodoError::new(ErrorCode::UnsafeFile, e.to_string()))?;
        (None, parsed)
    };

    let mut edits: Vec<LineEdit> = Vec::new();
    let mut append_tail: Vec<String> = Vec::new();

    for sec_name in section_order {
        let block_lines = grouped.remove(&sec_name).unwrap_or_default();
        if let SectionLookup::Found(sec_idx) = arch_doc.find_section(&sec_name) {
            let insert_at = arch_doc.sections[sec_idx].content_end;
            let mut lines_to_insert = Vec::new();
            if insert_at > 0 && !arch_doc.line_doc.lines[insert_at - 1].text.is_empty() {
                lines_to_insert.push(String::new());
            }
            lines_to_insert.extend(block_lines);
            edits.push(LineEdit::insert(insert_at, lines_to_insert));
        } else {
            if !append_tail.is_empty()
                || !arch_doc
                    .line_doc
                    .lines
                    .last()
                    .is_some_and(|l| l.text.is_empty())
            {
                append_tail.push(String::new());
            }
            append_tail.push(format!("## {sec_name}"));
            append_tail.push(String::new());
            append_tail.extend(block_lines);
        }
    }

    if !append_tail.is_empty() {
        edits.push(LineEdit::insert(arch_doc.line_doc.lines.len(), append_tail));
    }

    edits.sort_by_key(|e| e.range.start);
    let intent = TaskChangeIntent {
        added_ids: selected.iter().map(|t| t.id).collect(),
        ..TaskChangeIntent::default()
    };
    let applied = arch_doc
        .apply_and_verify(&edits, &intent)
        .map_err(|e| {
            TodoError::new(ErrorCode::WriteFailed, e.to_string()).with_file(&cfg.archive_file)
        })?;
    // The archive file carries the same tool stamp as TODO.md (R-VER-1).
    let new_bytes = match applied.doc.with_tool_stamp(RUNNING_VERSION).map_err(|e| {
        TodoError::new(ErrorCode::WriteFailed, e.to_string()).with_file(&cfg.archive_file)
    })? {
        Some(stamped) => stamped.bytes,
        None => applied.bytes,
    };

    if let Some(snap) = existing_snap {
        atomic_replace(archive_path, &snap.snapshot, &new_bytes)?;
    } else {
        atomic_write_bytes(archive_path, &new_bytes)?;
    }
    Ok(archive_migration)
}

fn extract_task_block_lines(doc: &Document, task: &Task) -> Vec<String> {
    let mut lines: Vec<String> = doc.line_doc.lines[task.block_range.clone()]
        .iter()
        .map(|l| l.text.clone())
        .collect();
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines
}
