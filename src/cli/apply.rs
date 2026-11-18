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
    Document, LineEdit, TaskChangeIntent, TaskId, verify_task_edits,
};
use crate::error::{ErrorCode, Result, TodoError};
use crate::find::{fs_path, synced_folder_warning};
use crate::lock::{WriteOutcome, current_time, locked_read_modify_write_with_warnings};
use crate::ops::{
    NewTask, OpEnv, OpsConfig, OwnershipDecision, SetChange, TaskCommand, TaskOp, check_ownership,
    check_task_writable, file_label, forced_note_text, highest_task_number, parse_doc_for_write, read_optional_archive,
    resolve_task_in_doc,
};
use crate::ops::identity::done_by;
use crate::output::Output;
use crate::state::{IdsState, StateDir};
use crate::version::RUNNING_VERSION;

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
        difficulty: Option<String>,
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
        difficulty: Option<String>,
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

    // Who did the work, per `done` operation (R-WRITE-16); asked of git before the lock is taken.
    let git_dir = found.path.parent().filter(|p| !p.as_os_str().is_empty());
    let mut configured: Option<Option<String>> = None;
    let done_bys: Vec<Option<String>> = ops
        .iter()
        .map(|op| match op {
            ApplyOp::Done { commit, .. } => match commit.as_deref().map(str::trim) {
                Some(c) if !c.is_empty() => done_by(git_dir, Some(c)),
                _ => configured
                    .get_or_insert_with(|| done_by(git_dir, None))
                    .clone(),
            },
            _ => None,
        })
        .collect();

    let mut lock_warnings = Vec::new();
    let mut stamp_warning: Option<String> = None;
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
            stamp_warning = doc.newer_tool_warning(&label);
            let archive = read_optional_archive(&archive_path, cfg.id_config.clone())?;
            let now = current_time();

            let initial_state = state.read_ids()?.map_or(0, |s| s.highest_id);
            let mut highest_id = highest_task_number(&doc, archive.as_ref(), initial_state);

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
                    ctx.session.as_deref(),
                    done_bys[idx].as_deref(),
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

            // The same write adds or raises the tool stamp (R-VER-1).
            if let Some(stamped) = doc.with_tool_stamp(RUNNING_VERSION).map_err(|e| {
                TodoError::new(ErrorCode::WriteFailed, e.to_string()).with_file(&label)
            })? {
                working_bytes = stamped.bytes;
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
    if let Some(warn) = stamp_warning {
        out.warn(warn);
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
    session: Option<&str>,
    done_by: Option<&str>,
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
            difficulty,
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

            let dep_str = match depends {
                Some(dep) => Some(resolve_depends(dep, refs)?).filter(|d| !d.is_empty()),
                None => None,
            };
            let difficulty = match difficulty
                .as_deref()
                .or_else(|| fields.get("difficulty").map(String::as_str))
            {
                Some(raw) => Some(cfg.canonical_difficulty(raw)?),
                None => None,
            };
            let extra: Vec<(String, String)> = fields
                .iter()
                .map(|(k, v)| (k.clone(), v.trim().to_string()))
                .collect();
            let lines = NewTask {
                title: clean_title,
                priority: Some(priority.trim()),
                difficulty: difficulty.as_deref(),
                task_type: trim(task_type),
                area: trim(area),
                depends: dep_str.as_deref(),
                files: trim(files),
                problem: trim(problem),
                fix: trim(fix),
                accept: trim(accept),
                extra: &extra,
            }
            .lines(&id_str, &date, agent);

            let edit = LineEdit::insert(sec.content_end, lines);
            Ok((
                "add",
                id_str,
                vec![edit],
                TaskChangeIntent::add_one(new_id),
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
        other => {
            let env = OpEnv {
                cfg,
                agent,
                force,
                now,
                session,
            };
            apply_task_op(other, op_num, doc, archive, label, &env, refs, done_by)
        }
    }
}

/// A field value trimmed, if present.
fn trim(v: &Option<String>) -> Option<&str> {
    v.as_deref().map(str::trim)
}

/// Trimmed `text`, or the `operation #N (name)` usage error when it is empty.
fn non_empty<'a>(op_num: usize, what: &str, name: &str, text: &'a str) -> Result<&'a str> {
    let text = text.trim();
    if text.is_empty() {
        return Err(TodoError::new(
            ErrorCode::Usage,
            format!("operation #{op_num} ({name}): {what} cannot be empty"),
        ));
    }
    Ok(text)
}

/// The `text` field of `note` and `answer`, falling back to `note`.
fn text_or_note<'a>(
    op_num: usize,
    name: &str,
    text: &'a Option<String>,
    note: &'a Option<String>,
) -> Result<&'a str> {
    non_empty(op_num, "text", name, text.as_deref().or(note.as_deref()).unwrap_or(""))
}

/// Runs every single-task operation through the shared [`TaskOp`] edits (`ops::task_ops`).
///
/// Only the parts specific to `apply` live here: reading the JSON fields, `@ref` names and the
/// `operation #N` error messages.
#[allow(clippy::too_many_arguments)]
fn apply_task_op(
    op: &ApplyOp,
    op_num: usize,
    doc: &Document,
    archive: Option<&Document>,
    label: &str,
    env: &OpEnv<'_>,
    refs: &BTreeMap<String, String>,
    done_by: Option<&str>,
) -> Result<(&'static str, String, Vec<LineEdit>, TaskChangeIntent)> {
    let mut dep_owned = String::new();
    let mut changes: Vec<SetChange<'_>> = Vec::new();
    let (raw_task, task_op) = match op {
        ApplyOp::Claim { task } => (task, TaskOp::Claim),
        ApplyOp::Release { task } => (task, TaskOp::Release),
        ApplyOp::Done { task, note, commit } => (
            task,
            TaskOp::Done {
                note: note.as_deref(),
                commit: commit.as_deref().map(str::trim),
                by: done_by,
            },
        ),
        ApplyOp::Drop { task, note } => (
            task,
            TaskOp::Drop {
                reason: non_empty(op_num, "note", "drop", note)?,
            },
        ),
        ApplyOp::Block { task, note } => (
            task,
            TaskOp::Block {
                reason: non_empty(op_num, "note", "block", note)?,
            },
        ),
        ApplyOp::Unblock { task, note } => (task, TaskOp::Unblock { note: note.as_deref() }),
        ApplyOp::Reopen { task, note } => (task, TaskOp::Reopen { note: note.as_deref() }),
        ApplyOp::Note { task, text, note } => (
            task,
            TaskOp::Note {
                text: text_or_note(op_num, "note", text, note)?,
            },
        ),
        ApplyOp::Answer { task, text, note } => (
            task,
            TaskOp::Answer {
                text: text_or_note(op_num, "answer", text, note)?,
            },
        ),
        ApplyOp::Set {
            task,
            title,
            priority,
            difficulty,
            task_type,
            area,
            depends,
            files,
            problem,
            fix,
            accept,
            fields,
        } => {
            if let Some(dep) = depends {
                dep_owned = resolve_depends(dep, refs)?;
            }
            if let Some(t) = title {
                changes.push(SetChange::Title(t.trim()));
            }
            let named = [
                ("priority", priority),
                ("difficulty", difficulty),
                ("type", task_type),
                ("area", area),
            ];
            for (key, value) in named {
                if let Some(v) = value {
                    changes.push(SetChange::Field(key, v.trim()));
                }
            }
            if depends.is_some() {
                changes.push(if dep_owned.is_empty() {
                    SetChange::Unset("depends")
                } else {
                    SetChange::Field("depends", &dep_owned)
                });
            }
            let rest = [("files", files), ("problem", problem), ("fix", fix), ("accept", accept)];
            for (key, value) in rest {
                if let Some(v) = value {
                    changes.push(SetChange::Field(key, v.trim()));
                }
            }
            for (k, v) in fields {
                changes.push(SetChange::Field(k, v.trim()));
            }
            (task, TaskOp::Set { changes: &changes })
        }
        ApplyOp::Add { .. } | ApplyOp::Move { .. } => {
            return Err(TodoError::new(
                ErrorCode::WriteFailed,
                format!("operation #{op_num}: not a single-task operation"),
            ));
        }
    };

    let task_ref = resolve_task_ref(raw_task, refs)?;
    let task = resolve_task_in_doc(doc, &task_ref)?;
    check_task_writable(doc, task, label, archive)?;
    let id_str = env.cfg.id_config.format_id(task.id);
    let edits = task_op.build_edits(doc, task, env)?;
    Ok((
        task_op.name(),
        id_str,
        edits,
        TaskChangeIntent::modify_one(task.id),
    ))
}
