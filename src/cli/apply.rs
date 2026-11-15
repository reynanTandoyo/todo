//! `todo apply` command: run several operations from JSON lines in one locked step (`T-036`; R-WRITE-15).

use std::collections::BTreeMap;
use std::fs;
use std::io::Read as _;
use std::path::Path;

use clap::Args as ClapArgs;
use serde::{Deserialize, Serialize};
use similar::TextDiff;

use crate::cli::add::resolve_target_section;
use crate::cli::{CommandKind, CommandMeta, Context};
use crate::doc::{
    CANONICAL_HEADING_SEPARATOR, CANONICAL_NOBODY_OWNER, Document, LineEdit, Status,
    TaskChangeIntent, TaskId, verify_task_edits,
};
use crate::error::{ErrorCode, Result, TodoError};
use crate::find::{fs_path, synced_folder_warning};
use crate::lock::{WriteOutcome, current_time, locked_read_modify_write_with_warnings};
use crate::ops::{
    OpsConfig, OwnershipDecision, StatusCommand, TaskCommand, TaskEditBuilder, check_ownership,
    check_task_writable, file_label, forced_note_text, parse_doc_for_write, plan_claim,
    plan_release, read_optional_archive, resolve_task_in_doc, validate_transition,
};
use crate::output::Output;
use crate::state::{IdsState, StateDir};

/// Metadata for the `apply` command.
pub const META: CommandMeta = CommandMeta {
    name: "apply",
    summary: "Apply a batch of operations from JSON lines in one locked step",
    kind: CommandKind::Write,
    details: "Reads JSON lines from `<file>` or `-` (standard input) and applies all operations inside a single write lock: either all operations succeed with one snapshot, or none apply.",
    examples: &[
        "todo apply ops.jsonl --agent agent-1",
        "Get-Content ops.jsonl | todo apply - --agent agent-1",
    ],
};

/// Arguments for `todo apply`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Path to a JSON lines file, or `-` to read standard input.
    #[arg(value_name = "FILE|-")]
    pub input: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum DependsValue {
    One(String),
    Many(Vec<String>),
}

impl DependsValue {
    fn tokens(&self) -> Vec<String> {
        match self {
            Self::One(s) => s
                .split(',')
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(str::to_string)
                .collect(),
            Self::Many(list) => list
                .iter()
                .map(|s| s.trim())
                .filter(|t| !t.is_empty())
                .map(str::to_string)
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
enum ApplyOp {
    Add {
        #[serde(rename = "ref")]
        ref_name: Option<String>,
        title: String,
        section: Option<String>,
        priority: String,
        #[serde(rename = "type")]
        task_type: Option<String>,
        area: Option<String>,
        depends: Option<DependsValue>,
        files: Option<String>,
        problem: Option<String>,
        fix: Option<String>,
        accept: Option<String>,
        #[serde(default)]
        fields: BTreeMap<String, String>,
    },
    Claim {
        task: String,
    },
    Release {
        task: String,
    },
    Done {
        task: String,
        note: Option<String>,
        commit: Option<String>,
    },
    Drop {
        task: String,
        note: String,
    },
    Block {
        task: String,
        note: String,
    },
    Unblock {
        task: String,
        note: Option<String>,
    },
    Note {
        task: String,
        text: Option<String>,
        note: Option<String>,
    },
    Set {
        task: String,
        title: Option<String>,
        priority: Option<String>,
        #[serde(rename = "type")]
        task_type: Option<String>,
        area: Option<String>,
        depends: Option<DependsValue>,
        files: Option<String>,
        problem: Option<String>,
        fix: Option<String>,
        accept: Option<String>,
        #[serde(default)]
        fields: BTreeMap<String, String>,
    },
    Move {
        task: String,
        section: String,
    },
    Reopen {
        task: String,
        note: Option<String>,
    },
    Answer {
        task: String,
        text: Option<String>,
        note: Option<String>,
    },
}

/// Summary of one applied operation (`R-WRITE-15`).
#[derive(Debug, Clone, Serialize)]
pub struct AppliedOpJson {
    /// 1-based line number in the input JSONL.
    pub index: usize,
    /// Operation name (`add`, `claim`, `done`, etc.).
    pub op: String,
    /// Canonical task ID affected.
    pub task: String,
}

#[derive(Serialize)]
struct ApplyJson {
    count: usize,
    applied: Vec<AppliedOpJson>,
}

/// Runs `todo apply <file|->`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let agent = ctx.require_agent()?.to_owned();
    let raw_input = read_apply_input(&args.input)?;
    let ops = parse_apply_ops(&raw_input)?;
    if ops.is_empty() {
        return Err(TodoError::new(
            ErrorCode::Usage,
            "apply input contains no operations",
        ));
    }

    let cfg = ctx.load_config()?;
    let found = ctx.find_todo_with_mode(cfg.worktree, out)?;
    if let Some(warning) = synced_folder_warning(&found.path) {
        out.warn(warning);
    }
    let state = StateDir::from_found(&found)?;
    let label = file_label(&found.path);
    let archive_path = cfg.archive_path(&found.path);

    let mut lock_warnings = Vec::new();
    let mut dry_run_diff: Option<String> = None;
    let applied = locked_read_modify_write_with_warnings(
        &found.path,
        &state,
        &agent,
        META.name,
        &cfg.lock,
        &mut lock_warnings,
        |bytes, _snap| {
            let mut working_bytes = bytes.to_vec();
            let mut doc = parse_doc_for_write(&working_bytes, &label, &cfg)?;
            let archive = read_optional_archive(&archive_path, cfg.id_config.clone())?;
            let now = current_time();

            let max_doc = doc.tasks.iter().map(|t| t.id.number()).max().unwrap_or(0);
            let max_arch = archive
                .as_ref()
                .map_or(0, |a| a.tasks.iter().map(|t| t.id.number()).max().unwrap_or(0));
            let initial_state = state.read_ids()?.map_or(0, |s| s.highest_id);
            let mut highest_id = max_doc.max(max_arch).max(initial_state);

            let mut refs: BTreeMap<String, String> = BTreeMap::new();
            let mut results = Vec::with_capacity(ops.len());

            for (idx, op) in ops.iter().enumerate() {
                let op_num = idx + 1;
                let (op_name, task_id_str, edits, step_intent) = apply_one_op(
                    op,
                    op_num,
                    &doc,
                    archive.as_ref(),
                    &cfg,
                    &label,
                    &agent,
                    ctx.force,
                    now,
                    &mut highest_id,
                    &mut refs,
                )?;

                let applied_step = doc.line_doc.apply_edits(&edits).map_err(|e| {
                    TodoError::new(
                        ErrorCode::WriteFailed,
                        format!("apply operation #{op_num} ({op_name}) failed: {e}"),
                    )
                })?;
                let new_doc =
                    Document::parse_bytes_with_config(&applied_step.bytes, cfg.id_config.clone())
                        .map_err(|e| {
                            TodoError::new(
                                ErrorCode::WriteFailed,
                                format!("apply operation #{op_num} ({op_name}) failed: {e}"),
                            )
                            .with_file(&label)
                            .with_line(e.line)
                        })?;
                verify_task_edits(&doc, &new_doc, &step_intent).map_err(|e| {
                    TodoError::new(
                        ErrorCode::WriteFailed,
                        format!("apply operation #{op_num} ({op_name}) failed: {e}"),
                    )
                })?;

                working_bytes = applied_step.bytes;
                doc = new_doc;
                results.push(AppliedOpJson {
                    index: op_num,
                    op: op_name.to_string(),
                    task: task_id_str,
                });
            }

            if working_bytes == bytes {
                return Ok(WriteOutcome::Unchanged(results));
            }

            if ctx.dry_run {
                let before_str = String::from_utf8_lossy(bytes);
                let after_str = String::from_utf8_lossy(&working_bytes);
                let diff = TextDiff::from_lines(before_str.as_ref(), after_str.as_ref())
                    .unified_diff()
                    .header(&format!("a/{label}"), &format!("b/{label}"))
                    .to_string();
                dry_run_diff = Some(diff);
                return Ok(WriteOutcome::Unchanged(results));
            }

            if highest_id > initial_state {
                state.write_ids(&IdsState { highest_id })?;
            }

            Ok(WriteOutcome::Changed {
                new_bytes: working_bytes,
                value: results,
            })
        },
    )?;

    for w in lock_warnings {
        out.warn(w);
    }

    if let Some(diff) = dry_run_diff {
        if out.is_json() {
            out.set_dry_run_diff(diff);
        } else {
            out.write_str(&diff);
        }
    }

    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &ApplyJson {
                count: applied.len(),
                applied,
            },
        );
    } else if !ctx.dry_run {
        for item in &applied {
            out.write_line(&format!("{}: {} ok", item.task, item.op));
        }
        let noun = if applied.len() == 1 {
            "operation"
        } else {
            "operations"
        };
        out.write_line(&format!("{} {noun} applied", applied.len()));
    }

    Ok(())
}

fn read_apply_input(input: &str) -> Result<String> {
    if input == "-" {
        let mut buf = Vec::new();
        std::io::stdin().read_to_end(&mut buf).map_err(|e| {
            TodoError::new(
                ErrorCode::BadValue,
                format!("failed to read standard input: {e}"),
            )
        })?;
        let text = String::from_utf8(buf).map_err(|e| {
            TodoError::new(
                ErrorCode::BadValue,
                format!("standard input is not valid UTF-8: {e}"),
            )
        })?;
        return Ok(text.strip_prefix('\u{feff}').unwrap_or(&text).to_string());
    }
    let path = Path::new(input);
    let bytes = fs::read(fs_path(path).as_ref()).map_err(|e| {
        TodoError::new(
            ErrorCode::NoFile,
            format!("could not read {}: {e}", path.display()),
        )
        .with_file(path.display().to_string())
    })?;
    let text = String::from_utf8(bytes).map_err(|e| {
        TodoError::new(
            ErrorCode::BadValue,
            format!("file {} is not valid UTF-8: {e}", path.display()),
        )
        .with_file(path.display().to_string())
    })?;
    Ok(text.strip_prefix('\u{feff}').unwrap_or(&text).to_string())
}

fn parse_apply_ops(raw: &str) -> Result<Vec<ApplyOp>> {
    let mut ops = Vec::new();
    for (idx, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let op: ApplyOp = serde_json::from_str(trimmed).map_err(|e| {
            TodoError::new(
                ErrorCode::BadValue,
                format!("invalid JSON operation on line {}: {e}", idx + 1),
            )
            .with_line(idx + 1)
        })?;
        ops.push(op);
    }
    Ok(ops)
}

fn resolve_task_ref(raw: &str, refs: &BTreeMap<String, String>) -> Result<String> {
    if let Some(name) = raw.strip_prefix('@') {
        refs.get(name).cloned().ok_or_else(|| {
            TodoError::new(
                ErrorCode::NotFound,
                format!("unknown task reference @{name}"),
            )
        })
    } else {
        Ok(raw.to_string())
    }
}

fn resolve_depends(
    dep: &DependsValue,
    refs: &BTreeMap<String, String>,
) -> Result<String> {
    let mut resolved = Vec::new();
    for tok in dep.tokens() {
        resolved.push(resolve_task_ref(&tok, refs)?);
    }
    Ok(resolved.join(", "))
}

#[allow(clippy::too_many_arguments)]
fn apply_one_op(
    op: &ApplyOp,
    op_num: usize,
    doc: &Document,
    archive: Option<&Document>,
    cfg: &OpsConfig,
    label: &str,
    agent: &str,
    force: bool,
    now: chrono::DateTime<chrono::FixedOffset>,
    highest_id: &mut u64,
    refs: &mut BTreeMap<String, String>,
) -> Result<(&'static str, String, Vec<LineEdit>, TaskChangeIntent)> {
    let date = OpsConfig::format_note_date(now);

    match op {
        ApplyOp::Add {
            ref_name,
            title,
            section,
            priority,
            task_type,
            area,
            depends,
            files,
            problem,
            fix,
            accept,
            fields,
        } => {
            let clean_title = title.trim();
            if clean_title.is_empty() {
                return Err(TodoError::new(
                    ErrorCode::Usage,
                    format!("operation #{op_num} (add): title cannot be empty"),
                ));
            }
            let section_idx = resolve_target_section(doc, section.as_deref(), cfg)?;
            let sec = &doc.sections[section_idx];
            *highest_id = highest_id
                .checked_add(1)
                .ok_or_else(|| TodoError::new(ErrorCode::WriteFailed, "task ID overflow"))?;
            let new_id = TaskId(*highest_id);
            let id_str = cfg.id_config.format_id(new_id);
            if let Some(r) = ref_name {
                if refs.contains_key(r) {
                    return Err(TodoError::new(
                        ErrorCode::BadValue,
                        format!("duplicate ref @{r} in apply input"),
                    ));
                }
                refs.insert(r.clone(), id_str.clone());
            }

            let mut lines = vec![
                String::new(),
                format!("### {id_str}{CANONICAL_HEADING_SEPARATOR}{clean_title}"),
                "- status: open".to_string(),
                format!("- owner: {CANONICAL_NOBODY_OWNER}"),
            ];
            let mut combined = vec![format!("priority: {}", priority.trim())];
            if let Some(t) = task_type {
                combined.push(format!("type: {}", t.trim()));
            }
            if let Some(a) = area {
                combined.push(format!("area: {}", a.trim()));
            }
            lines.push(format!("- {}", combined.join(" \u{00b7} ")));

            if let Some(dep) = depends {
                let dep_str = resolve_depends(dep, refs)?;
                if !dep_str.is_empty() {
                    lines.push(format!("- depends: {dep_str}"));
                }
            }
            for (k, v) in [
                ("files", files),
                ("problem", problem),
                ("fix", fix),
                ("accept", accept),
            ] {
                if let Some(val) = v {
                    lines.push(format!("- {k}: {}", val.trim()));
                }
            }
            for (k, v) in fields {
                lines.push(format!("- {k}: {}", v.trim()));
            }
            lines.push(format!("- note ({date}, {agent}): added"));

            let edit = LineEdit::insert(sec.content_end, lines);
            Ok((
                "add",
                id_str,
                vec![edit],
                TaskChangeIntent::add_one(new_id),
            ))
        }
        ApplyOp::Claim { task: raw_task } => {
            let task_ref = resolve_task_ref(raw_task, refs)?;
            let task = resolve_task_in_doc(doc, &task_ref)?;
            check_task_writable(doc, task, label, archive)?;
            let id_str = cfg.id_config.format_id(task.id);
            let plan = plan_claim(task, agent, force, now, cfg)?;
            let (edits, _) = crate::ops::build_claim_edits(doc, task, &plan, now);
            Ok((
                "claim",
                id_str,
                edits,
                TaskChangeIntent::modify_one(task.id),
            ))
        }
        ApplyOp::Release { task: raw_task } => {
            let task_ref = resolve_task_ref(raw_task, refs)?;
            let task = resolve_task_in_doc(doc, &task_ref)?;
            check_task_writable(doc, task, label, archive)?;
            let id_str = cfg.id_config.format_id(task.id);
            let plan = plan_release(task, agent, force, cfg)?;
            let (edits, _) = crate::ops::build_release_edits(doc, task, &plan, agent, now);
            Ok((
                "release",
                id_str,
                edits,
                TaskChangeIntent::modify_one(task.id),
            ))
        }
        ApplyOp::Done {
            task: raw_task,
            note,
            commit,
        } => {
            let task_ref = resolve_task_ref(raw_task, refs)?;
            let task = resolve_task_in_doc(doc, &task_ref)?;
            check_task_writable(doc, task, label, archive)?;
            let id_str = cfg.id_config.format_id(task.id);
            let (_, ownership) =
                validate_transition(task, StatusCommand::Done, agent, force, &id_str)?;
            let mut builder = TaskEditBuilder::new(doc, task);
            builder.set_status(Status::Done);
            builder.set_owner(Some(agent));
            builder.remove_claimed();
            if let Some(c) = commit {
                builder.set_field("commit", c.trim());
            }
            if let OwnershipDecision::Forced { previous_owner } = ownership {
                builder.add_note(&date, agent, &forced_note_text("done", &previous_owner));
            }
            let note_text = match note.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                Some(t) => format!("done \u{2014} {t}"),
                None => "done".to_string(),
            };
            builder.add_note(&date, agent, &note_text);
            let (edits, _) = builder.build();
            Ok((
                "done",
                id_str,
                edits,
                TaskChangeIntent::modify_one(task.id),
            ))
        }
        ApplyOp::Drop {
            task: raw_task,
            note,
        } => {
            let reason = note.trim();
            if reason.is_empty() {
                return Err(TodoError::new(
                    ErrorCode::Usage,
                    format!("operation #{op_num} (drop): note cannot be empty"),
                ));
            }
            let task_ref = resolve_task_ref(raw_task, refs)?;
            let task = resolve_task_in_doc(doc, &task_ref)?;
            check_task_writable(doc, task, label, archive)?;
            let id_str = cfg.id_config.format_id(task.id);
            let (_, ownership) =
                validate_transition(task, StatusCommand::Drop, agent, force, &id_str)?;
            let mut builder = TaskEditBuilder::new(doc, task);
            builder.set_status(Status::Dropped);
            builder.remove_claimed();
            if let OwnershipDecision::Forced { previous_owner } = ownership {
                builder.add_note(&date, agent, &forced_note_text("drop", &previous_owner));
            }
            builder.add_note(&date, agent, &format!("dropped \u{2014} {reason}"));
            let (edits, _) = builder.build();
            Ok((
                "drop",
                id_str,
                edits,
                TaskChangeIntent::modify_one(task.id),
            ))
        }
        ApplyOp::Block {
            task: raw_task,
            note,
        } => {
            let reason = note.trim();
            if reason.is_empty() {
                return Err(TodoError::new(
                    ErrorCode::Usage,
                    format!("operation #{op_num} (block): note cannot be empty"),
                ));
            }
            let task_ref = resolve_task_ref(raw_task, refs)?;
            let task = resolve_task_in_doc(doc, &task_ref)?;
            check_task_writable(doc, task, label, archive)?;
            let id_str = cfg.id_config.format_id(task.id);
            let (_, ownership) =
                validate_transition(task, StatusCommand::Block, agent, force, &id_str)?;
            let mut builder = TaskEditBuilder::new(doc, task);
            builder.set_status(Status::Blocked);
            if let OwnershipDecision::Forced { previous_owner } = ownership {
                builder.add_note(&date, agent, &forced_note_text("block", &previous_owner));
            }
            builder.add_note(&date, agent, &format!("blocked \u{2014} {reason}"));
            let (edits, _) = builder.build();
            Ok((
                "block",
                id_str,
                edits,
                TaskChangeIntent::modify_one(task.id),
            ))
        }
        ApplyOp::Unblock {
            task: raw_task,
            note,
        } => {
            let task_ref = resolve_task_ref(raw_task, refs)?;
            let task = resolve_task_in_doc(doc, &task_ref)?;
            check_task_writable(doc, task, label, archive)?;
            let id_str = cfg.id_config.format_id(task.id);
            let (_, ownership) =
                validate_transition(task, StatusCommand::Unblock, agent, force, &id_str)?;
            let mut builder = TaskEditBuilder::new(doc, task);
            let target_status = if task.owner.is_some() {
                Status::Claimed
            } else {
                Status::Open
            };
            builder.set_status(target_status);
            if let OwnershipDecision::Forced { previous_owner } = ownership {
                builder.add_note(&date, agent, &forced_note_text("unblock", &previous_owner));
            }
            let note_text = match note.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                Some(t) => format!("unblocked \u{2014} {t}"),
                None => "unblocked".to_string(),
            };
            builder.add_note(&date, agent, &note_text);
            let (edits, _) = builder.build();
            Ok((
                "unblock",
                id_str,
                edits,
                TaskChangeIntent::modify_one(task.id),
            ))
        }
        ApplyOp::Note {
            task: raw_task,
            text,
            note,
        } => {
            let body = text
                .as_deref()
                .or(note.as_deref())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    TodoError::new(
                        ErrorCode::Usage,
                        format!("operation #{op_num} (note): text cannot be empty"),
                    )
                })?;
            let task_ref = resolve_task_ref(raw_task, refs)?;
            let task = resolve_task_in_doc(doc, &task_ref)?;
            check_task_writable(doc, task, label, archive)?;
            let id_str = cfg.id_config.format_id(task.id);
            let mut builder = TaskEditBuilder::new(doc, task);
            builder.add_note(&date, agent, body);
            let (edits, _) = builder.build();
            Ok((
                "note",
                id_str,
                edits,
                TaskChangeIntent::modify_one(task.id),
            ))
        }
        ApplyOp::Set {
            task: raw_task,
            title,
            priority,
            task_type,
            area,
            depends,
            files,
            problem,
            fix,
            accept,
            fields,
        } => {
            let task_ref = resolve_task_ref(raw_task, refs)?;
            let task = resolve_task_in_doc(doc, &task_ref)?;
            check_task_writable(doc, task, label, archive)?;
            let id_str = cfg.id_config.format_id(task.id);
            let ownership = check_ownership(task, TaskCommand::Set, agent, force, &id_str)?;
            let mut builder = TaskEditBuilder::new(doc, task);
            if let Some(t) = title {
                builder.set_title(t.trim());
            }
            if let Some(p) = priority {
                builder.set_field("priority", p.trim());
            }
            if let Some(t) = task_type {
                builder.set_field("type", t.trim());
            }
            if let Some(a) = area {
                builder.set_field("area", a.trim());
            }
            if let Some(dep) = depends {
                let dep_str = resolve_depends(dep, refs)?;
                if dep_str.is_empty() {
                    builder.unset_field("depends");
                } else {
                    builder.set_field("depends", &dep_str);
                }
            }
            for (k, v) in [
                ("files", files),
                ("problem", problem),
                ("fix", fix),
                ("accept", accept),
            ] {
                if let Some(val) = v {
                    builder.set_field(k, val.trim());
                }
            }
            for (k, v) in fields {
                builder.set_field(k, v.trim());
            }
            if let OwnershipDecision::Forced { previous_owner } = ownership {
                builder.add_note(&date, agent, &forced_note_text("set", &previous_owner));
            }
            let (edits, _) = builder.build();
            Ok((
                "set",
                id_str,
                edits,
                TaskChangeIntent::modify_one(task.id),
            ))
        }
        ApplyOp::Move {
            task: raw_task,
            section,
        } => {
            let task_ref = resolve_task_ref(raw_task, refs)?;
            let task = resolve_task_in_doc(doc, &task_ref)?;
            check_task_writable(doc, task, label, archive)?;
            let id_str = cfg.id_config.format_id(task.id);
            let ownership = check_ownership(task, TaskCommand::Move, agent, force, &id_str)?;
            let target_sec_idx = resolve_target_section(doc, Some(section), cfg)?;
            let target_sec = &doc.sections[target_sec_idx];

            let mut block_lines: Vec<String> = doc.line_doc.lines[task.block_range.clone()]
                .iter()
                .map(|l| l.text.clone())
                .collect();
            if let OwnershipDecision::Forced { previous_owner } = ownership {
                block_lines.push(format!(
                    "- note ({date}, {agent}): {}",
                    forced_note_text("move", &previous_owner)
                ));
            }

            let mut edits = Vec::new();
            let remove_start = if task.block_range.start > 0
                && doc.line_doc.lines[task.block_range.start - 1]
                    .text
                    .trim()
                    .is_empty()
            {
                task.block_range.start - 1
            } else {
                task.block_range.start
            };
            edits.push(LineEdit::remove(remove_start..task.block_range.end));
            let mut insert_lines = vec![String::new()];
            insert_lines.extend(block_lines);
            edits.push(LineEdit::insert(target_sec.content_end, insert_lines));
            Ok((
                "move",
                id_str,
                edits,
                TaskChangeIntent::modify_one(task.id),
            ))
        }
        ApplyOp::Reopen {
            task: raw_task,
            note,
        } => {
            let task_ref = resolve_task_ref(raw_task, refs)?;
            let task = resolve_task_in_doc(doc, &task_ref)?;
            check_task_writable(doc, task, label, archive)?;
            let id_str = cfg.id_config.format_id(task.id);
            let (_, ownership) =
                validate_transition(task, StatusCommand::Reopen, agent, force, &id_str)?;
            let mut builder = TaskEditBuilder::new(doc, task);
            builder.set_status(Status::Open);
            builder.set_owner(None);
            builder.remove_claimed();
            if let OwnershipDecision::Forced { previous_owner } = ownership {
                builder.add_note(&date, agent, &forced_note_text("reopen", &previous_owner));
            }
            let note_text = match note.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                Some(t) => format!("reopened \u{2014} {t}"),
                None => "reopened".to_string(),
            };
            builder.add_note(&date, agent, &note_text);
            let (edits, _) = builder.build();
            Ok((
                "reopen",
                id_str,
                edits,
                TaskChangeIntent::modify_one(task.id),
            ))
        }
        ApplyOp::Answer {
            task: raw_task,
            text,
            note,
        } => {
            let body = text
                .as_deref()
                .or(note.as_deref())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    TodoError::new(
                        ErrorCode::Usage,
                        format!("operation #{op_num} (answer): text cannot be empty"),
                    )
                })?;
            let task_ref = resolve_task_ref(raw_task, refs)?;
            let task = resolve_task_in_doc(doc, &task_ref)?;
            check_task_writable(doc, task, label, archive)?;
            let id_str = cfg.id_config.format_id(task.id);
            let (_, ownership) =
                validate_transition(task, StatusCommand::Done, agent, force, &id_str)?;
            let mut builder = TaskEditBuilder::new(doc, task);
            builder.set_status(Status::Done);
            builder.set_owner(Some(agent));
            builder.remove_claimed();
            if let OwnershipDecision::Forced { previous_owner } = ownership {
                builder.add_note(&date, agent, &forced_note_text("answer", &previous_owner));
            }
            builder.add_note(&date, agent, &format!("answer \u{2014} {body}"));
            let (edits, _) = builder.build();
            Ok((
                "answer",
                id_str,
                edits,
                TaskChangeIntent::modify_one(task.id),
            ))
        }
    }
}
