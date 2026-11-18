//! `todo archive` command: move done and dropped tasks to `TODO-archive.md` (`R-ARCH-1`,
//! `R-ARCH-2`, `docs/format.md` §14).

use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{DateTime, FixedOffset};
use clap::Args as ClapArgs;
use serde::Serialize;

use crate::atomic::{read_file_with_snapshot, validate_utf8};
use crate::version::RUNNING_VERSION;
use crate::cli::{CommandKind, CommandMeta, Context};
use crate::doc::id::TaskId;
use crate::doc::{
    CURRENT_FORMAT_VERSION, Document, LineEdit, SectionLookup, Task, TaskChangeIntent,
    format_line_text,
};
use crate::error::{ErrorCode, Result, TodoError};
use crate::find::fs_path;
use crate::lock::{
    ArchiveWrite, MigrationWrite, WriteOutcomeFull, current_time, locked_read_modify_write_full,
};
use crate::ops::migrate::{MigratedFile, describe, migrate_for_write};
use crate::ops::{
    OpsConfig, check_task_writable, file_label, parse_doc_for_write, read_optional_archive,
    require_yes_or_preview,
};
use crate::output::Output;
use crate::state::{IdsState, StateDir};

/// Why an entry whose milestone chain does not reach a top-level milestone stays in TODO.md.
const BROKEN_CHAIN_REASON: &str = "its milestone chain does not reach a top-level milestone (a milestone is missing, is not a milestone, or loops); run todo check";

/// Why a finished entry that is not a milestone stays in TODO.md when entries name it as their
/// milestone: moving it would leave them pointing at an archived entry.
const NAMED_NOT_MILESTONE_REASON: &str =
    "entries below it name it as their milestone, but it is not a milestone; run todo check";

/// Metadata for the `archive` command.
pub const META: CommandMeta = CommandMeta {
    name: "archive",
    summary: "Move done and dropped tasks to TODO-archive.md",
    kind: CommandKind::Write,
    details: "Moves done and dropped tasks from TODO.md into TODO-archive.md under the same section names, under the write lock. An entry in a milestone moves only with its whole tree: when the top-level milestone is done or dropped and nothing in the tree is open, claimed or blocked. Finished entries kept back for that reason are listed. Requires `--yes` to apply.",
    examples: &[
        "todo archive --agent coordinator",
        "todo archive --yes --agent coordinator",
        "todo archive --older-than 14 --yes --agent coordinator",
    ],
};

/// Arguments for `todo archive`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Only archive tasks whose latest note (or claim) is at least DAYS days old. For an entry in a
    /// milestone tree, the age of the top-level milestone counts.
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

/// A finished entry that stays in TODO.md, and why (`R-ARCH-2`).
#[derive(Serialize)]
struct KeptBackSummary {
    #[serde(flatten)]
    task: ArchivedTaskSummary,
    reason: String,
}

#[derive(Serialize)]
struct ArchiveJson {
    archive_file: String,
    applied: bool,
    count: usize,
    tasks: Vec<ArchivedTaskSummary>,
    kept_back: Vec<KeptBackSummary>,
}

/// What `archive` moves and keeps back, as printed and as returned in JSON.
struct ArchiveOutcome {
    tasks: Vec<ArchivedTaskSummary>,
    kept_back: Vec<KeptBackSummary>,
}

/// The entries one `archive` run moves, and the finished entries it keeps back (`R-ARCH-1`,
/// `R-ARCH-2`).
#[derive(Default)]
struct ArchivePlan<'a> {
    /// Entries to move, in file order. A milestone tree is listed in full or not at all.
    selected: Vec<&'a Task>,
    /// Finished entries that stay in TODO.md, in file order, each with its reason.
    kept_back: Vec<(&'a Task, String)>,
}

/// Where an entry sits for `archive` (`R-ARCH-2`).
enum Home {
    /// Neither a milestone nor under one: archived on its own, as before.
    Alone,
    /// In the tree of the top-level milestone with this ID, the milestone itself included.
    Tree(TaskId),
    /// Names a milestone that does not reach a top-level one: it stays in TODO.md.
    Broken,
}

/// What `archive` does with one finished entry.
enum Step {
    /// Move it to the archive.
    Move,
    /// Keep it in TODO.md, for this reason.
    Keep(String),
}

/// Whether the tree of one top-level milestone may move (`R-ARCH-2`).
struct TreeState {
    /// Whether the top-level milestone is old enough for `--older-than`.
    aged: bool,
    /// Why the tree stays in TODO.md, if it does.
    blocker: Option<String>,
}

impl TreeState {
    /// What happens to the finished entries of this tree. `None` when the tree is too young, so
    /// nothing is moved and nothing is reported as kept back.
    fn step(&self) -> Option<Step> {
        if !self.aged {
            return None;
        }
        Some(match &self.blocker {
            Some(reason) => Step::Keep(reason.clone()),
            None => Step::Move,
        })
    }
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
        let plan = plan_archive(&doc, args.older_than, now);
        let outcome = summarize(&doc, &plan);

        if out.is_json() {
            out.emit_json_ok(
                META.name,
                &ArchiveJson {
                    archive_file: cfg.archive_file.clone(),
                    applied: false,
                    count: outcome.tasks.len(),
                    tasks: outcome.tasks,
                    kept_back: outcome.kept_back,
                },
            );
            return Ok(());
        }

        let preview = format_preview(&outcome, &cfg.archive_file);
        let _ = require_yes_or_preview(ctx, &preview, out);
        return Ok(());
    }

    let mut lock_warnings = Vec::new();
    let mut stamp_warning: Option<String> = None;
    let mut migrated_notice: Option<String> = None;
    let outcome = locked_read_modify_write_full(
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
            let plan = plan_archive(doc, args.older_than, now);
            let outcome = summarize(doc, &plan);
            if plan.selected.is_empty() {
                return Ok(WriteOutcomeFull::Unchanged(outcome));
            }

            for &task in &plan.selected {
                check_task_writable(doc, task, &label, existing_archive.as_ref())?;
            }

            // Ensure highest_id in ids.json never forgets archived IDs.
            let max_selected_id = plan
                .selected
                .iter()
                .map(|t| t.id.number())
                .max()
                .unwrap_or(0);
            let current_ids = state.read_ids()?.unwrap_or_default();
            if max_selected_id > current_ids.highest_id {
                state.write_ids(&IdsState {
                    highest_id: max_selected_id,
                })?;
            }

            // The archive file is built now and written by the lock, after TODO.md's snapshot is
            // recorded and before TODO.md is replaced (R-MIG-1, R-ARCH-1).
            let archive_update = build_updated_archive(doc, &plan.selected, &archive_path, &cfg)?;

            // Remove archived task blocks from TODO.md (plus one adjacent blank line when safe).
            let mut edits: Vec<LineEdit> = plan
                .selected
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
                removed_ids: plan.selected.iter().map(|t| t.id).collect(),
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
            if let Some((from, to)) = archive_update.migrated {
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
                value: outcome,
                migration,
                archive: Some(archive_update.write),
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
                count: outcome.tasks.len(),
                tasks: outcome.tasks,
                kept_back: outcome.kept_back,
            },
        );
        return Ok(());
    }

    out.write_line(&format_result(&outcome, &cfg.archive_file));
    Ok(())
}

/// Works out which entries `archive` moves and which finished entries it keeps back (`R-ARCH-1`,
/// `R-ARCH-2`).
///
/// An entry with no milestone is moved when it is finished and old enough, as before, unless other
/// entries name it as their milestone. A milestone tree is moved only as a whole: its top-level
/// milestone is finished, nothing in the tree is open, claimed or blocked, and no entry outside the
/// tree still names one of its entries. Its age is the top-level milestone's age. An entry whose
/// milestone chain does not reach a top-level one is kept.
fn plan_archive(doc: &Document, older_than: Option<u64>, now: DateTime<FixedOffset>) -> ArchivePlan<'_> {
    let aged = |task: &Task| {
        older_than.is_none_or(|days| {
            task_age_days(task, now).is_some_and(|age| age >= days as i64)
        })
    };

    let trees: HashMap<TaskId, TreeState> = doc
        .tasks
        .iter()
        .filter(|t| t.milestone.is_none() && t.is_milestone())
        .map(|root| {
            (
                root.id,
                TreeState {
                    aged: aged(root),
                    blocker: tree_blocker(doc, root),
                },
            )
        })
        .collect();

    // Entries that name a parent, whatever it is: a parent that is not a milestone is damage, and
    // its children must not be left pointing at an archived entry.
    let named: HashSet<TaskId> = doc
        .tasks
        .iter()
        .filter_map(|t| doc.milestone_parent_id(t))
        .collect();

    let mut plan = ArchivePlan::default();
    for task in doc.tasks.iter().filter(|t| t.status.is_finished()) {
        let step = match home_of(doc, task) {
            Home::Alone if aged(task) && named.contains(&task.id) => {
                Some(Step::Keep(NAMED_NOT_MILESTONE_REASON.to_string()))
            }
            Home::Alone if aged(task) => Some(Step::Move),
            Home::Alone => None,
            Home::Broken if aged(task) => Some(Step::Keep(BROKEN_CHAIN_REASON.to_string())),
            Home::Broken => None,
            Home::Tree(root) => trees.get(&root).and_then(TreeState::step),
        };
        match step {
            Some(Step::Move) => plan.selected.push(task),
            Some(Step::Keep(reason)) => plan.kept_back.push((task, reason)),
            None => {}
        }
    }
    plan
}

/// Where `task` sits in the milestone tree (`R-ARCH-2`).
fn home_of(doc: &Document, task: &Task) -> Home {
    match task.milestone.as_deref() {
        None if task.is_milestone() => Home::Tree(task.id),
        None => Home::Alone,
        Some(_) => match doc.milestone_ancestors(task).last() {
            // The walk up ends at a top-level milestone only when that milestone has no
            // `milestone` field of its own; a missing, non-milestone or cyclic parent leaves one.
            Some(root) if root.milestone.is_none() => Home::Tree(root.id),
            _ => Home::Broken,
        },
    }
}

/// Why the tree of top-level milestone `root` stays in TODO.md, or `None` when it may move
/// (`R-ARCH-2`). The first failing rule is the reason.
fn tree_blocker(doc: &Document, root: &Task) -> Option<String> {
    let root_id = doc.id_config.format_id(root.id);
    if !root.status.is_finished() {
        return Some(format!(
            "milestone {root_id} is still {}",
            root.status.as_str()
        ));
    }

    let below = doc.entries_below(root.id);
    let unfinished = below.iter().filter(|t| !t.status.is_finished()).count();
    if unfinished > 0 {
        let noun = if unfinished == 1 { "entry" } else { "entries" };
        return Some(format!(
            "milestone {root_id} has {unfinished} unfinished {noun} below it"
        ));
    }

    // An entry outside the tree that still names one of its entries would be left pointing at
    // an archived entry.
    let members: HashSet<TaskId> = below
        .iter()
        .map(|t| t.id)
        .chain(std::iter::once(root.id))
        .collect();
    doc.tasks.iter().find_map(|outside| {
        let parent = doc.milestone_parent_id(outside)?;
        (!members.contains(&outside.id) && members.contains(&parent)).then(|| {
            format!(
                "{} still names {} as its milestone",
                doc.id_config.format_id(outside.id),
                doc.id_config.format_id(parent)
            )
        })
    })
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

fn summary_of(doc: &Document, task: &Task) -> ArchivedTaskSummary {
    ArchivedTaskSummary {
        id: doc.id_config.format_id(task.id),
        title: task.title.clone(),
        status: task.status.as_str(),
        section: task_section_name(doc, task),
    }
}

fn summarize(doc: &Document, plan: &ArchivePlan<'_>) -> ArchiveOutcome {
    ArchiveOutcome {
        tasks: plan.selected.iter().map(|t| summary_of(doc, t)).collect(),
        kept_back: plan
            .kept_back
            .iter()
            .map(|(t, reason)| KeptBackSummary {
                task: summary_of(doc, t),
                reason: reason.clone(),
            })
            .collect(),
    }
}

/// The lines that say how many finished entries were kept back, and why (`R-ARCH-2`).
fn kept_back_lines(kept: &[KeptBackSummary]) -> Vec<String> {
    if kept.is_empty() {
        return Vec::new();
    }
    let noun = if kept.len() == 1 { "entry" } else { "entries" };
    let mut lines = vec![format!("kept back {} finished {noun} in TODO.md:", kept.len())];
    for k in kept {
        lines.push(format!(
            "  {} [{}] · {} — {}",
            k.task.id, k.task.status, k.task.title, k.reason
        ));
    }
    lines
}

fn format_preview(outcome: &ArchiveOutcome, archive_file: &str) -> String {
    let mut lines = Vec::new();
    if outcome.tasks.is_empty() {
        lines.push("no tasks to archive".to_string());
    } else {
        lines.push(format!(
            "would archive {} task(s) to {archive_file}:",
            outcome.tasks.len()
        ));
        for s in &outcome.tasks {
            lines.push(format!(
                "  {} [{}] · {} (section: {})",
                s.id, s.status, s.title, s.section
            ));
        }
    }
    lines.extend(kept_back_lines(&outcome.kept_back));
    lines.join("\n")
}

fn format_result(outcome: &ArchiveOutcome, archive_file: &str) -> String {
    let mut lines = Vec::new();
    if outcome.tasks.is_empty() {
        lines.push("no tasks to archive".to_string());
    } else {
        lines.push(format!(
            "archived {} task(s) to {archive_file}",
            outcome.tasks.len()
        ));
    }
    lines.extend(kept_back_lines(&outcome.kept_back));
    lines.join("\n")
}

/// The archive file as this run leaves it, for the lock to write (R-ARCH-1).
struct ArchiveUpdate {
    /// The new archive text, and the snapshot of the file it replaces (`None` when it is created).
    write: ArchiveWrite,
    /// The format versions the archive was migrated from and to, when it was in an older format
    /// (R-MIG-1).
    migrated: Option<(u32, u32)>,
}

/// Builds the archive file with the selected tasks added. Nothing is written here: the write
/// happens under the lock, after the snapshot of TODO.md (R-MIG-1).
fn build_updated_archive(
    doc: &Document,
    selected: &[&Task],
    archive_path: &std::path::Path,
    cfg: &OpsConfig,
) -> Result<ArchiveUpdate> {
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

    Ok(ArchiveUpdate {
        write: ArchiveWrite {
            path: archive_path.to_path_buf(),
            expected: existing_snap.map(|snap| snap.snapshot),
            new_bytes,
        },
        migrated: archive_migration,
    })
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
