//! `todo apply` command: run several operations from JSON lines in one locked step (`T-036`; R-WRITE-15).

use std::collections::BTreeMap;
use std::fs;
use std::io::Read as _;
use std::path::Path;

use clap::Args as ClapArgs;
use serde::{Deserialize, Serialize};
use similar::TextDiff;

use crate::atomic::read_file_with_snapshot;
use crate::cli::add::resolve_target_section;
use crate::cli::{CommandKind, CommandMeta, Context};
use crate::doc::{
    Document, EntryKind, LineEdit, TaskChangeIntent, TaskId, sanitize_trimmed_value,
    verify_task_edits,
};
use crate::error::{ErrorCode, Result, TodoError};
use crate::find::{FoundTodo, fs_path, synced_folder_warning, verify_folder};
use crate::lock::{WriteOutcomeFull, current_time, locked_read_modify_write_full};
use crate::ops::{
    DoneCheck, NewTask, OpEnv, OpsConfig, OwnershipDecision, SetChange, StepsInput, TaskCommand,
    TaskOp, VerifyWhen, check_ownership, check_task_writable, file_label, forced_note_text,
    highest_task_number, milestone, parse_doc_for_write, read_optional_archive,
    resolve_task_in_doc, steps_from_text,
};
use crate::ops::commit_files::{resolve_commit, scope_of_commit};
use crate::ops::identity::done_by;
use crate::ops::migrate::PendingMigration;
use crate::ops::scope::CommitScope;
use crate::ops::verify::{self, CheckOutcome};
use crate::output::Output;
use crate::state::{IdsState, StateDir};
use crate::version::RUNNING_VERSION;

/// Metadata for the `apply` command.
pub const META: CommandMeta = CommandMeta {
    name: "apply",
    summary: "Apply a batch of operations from JSON lines in one locked step",
    kind: CommandKind::Write,
    details: "Reads JSON lines from `<file>` or `-` (standard input) and applies all operations inside a single write lock: either all operations succeed with one snapshot, or none apply. A `done` with a `commit` is checked against the task's `files` as `todo done` checks it (R-SCOPE-2).",
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
        milestone: Option<String>,
        goal: Option<String>,
        why: Option<String>,
        summary: Option<String>,
        depends: Option<DependsValue>,
        files: Option<String>,
        read: Option<String>,
        problem: Option<String>,
        fix: Option<String>,
        steps: Option<StepsInput>,
        accept: Option<String>,
        verify: Option<String>,
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
        milestone: Option<String>,
        goal: Option<String>,
        why: Option<String>,
        summary: Option<String>,
        depends: Option<DependsValue>,
        files: Option<String>,
        read: Option<String>,
        problem: Option<String>,
        fix: Option<String>,
        steps: Option<StepsInput>,
        accept: Option<String>,
        verify: Option<String>,
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

    // The commit of each `done` operation, resolved as `done` resolves its `--commit` (R-WRITE-4),
    // before the lock is taken. An empty commit is no commit.
    let resolve_dir = ctx
        .file
        .as_deref()
        .and_then(Path::parent)
        .filter(|p| !p.as_os_str().is_empty());
    let done_commits: Vec<Option<String>> = ops
        .iter()
        .map(|op| match op {
            ApplyOp::Done { commit, .. } => {
                match commit.as_deref().map(str::trim).filter(|c| !c.is_empty()) {
                    Some(c) => resolve_commit(c, resolve_dir).map(Some),
                    None => Ok(None),
                }
            }
            _ => Ok(None),
        })
        .collect::<Result<_>>()?;

    // Who did the work, per `done` operation (R-WRITE-16); asked of git before the lock is taken.
    let git_dir = found.path.parent().filter(|p| !p.as_os_str().is_empty());
    let mut configured: Option<Option<String>> = None;
    let done_bys: Vec<Option<String>> = ops
        .iter()
        .zip(&done_commits)
        .map(|(op, commit)| match op {
            ApplyOp::Done { .. } => match commit {
                Some(c) => done_by(git_dir, Some(c.as_str())),
                None => configured
                    .get_or_insert_with(|| done_by(git_dir, None))
                    .clone(),
            },
            _ => None,
        })
        .collect();
    // Which files each `done` commit changed (R-SCOPE-2), asked of git before the lock, as `done`
    // asks it. The scope is checked first: a refusal here runs no verify command.
    let done_scopes: Vec<CommitScope> = ops
        .iter()
        .zip(&done_commits)
        .map(|(op, commit)| match op {
            ApplyOp::Done { .. } => {
                scope_of_commit(&found.path, &cfg.archive_file, commit.as_deref())
            }
            _ => Ok(CommitScope::NoGit),
        })
        .collect::<Result<_>>()?;

    // The `verify` commands of the `done` operations run before the lock is taken, as `done` runs
    // them (T-173, R-LOCK-4). Nothing runs when the verify commands are off or wait for the wave.
    let env = BatchEnv {
        cfg: &cfg,
        label: &label,
        agent: &agent,
        force: ctx.force,
        session: ctx.session.as_deref(),
        done_bys: &done_bys,
        done_commits: &done_commits,
        done_scopes: &done_scopes,
        now: current_time(),
    };
    let required = cfg.verify_enabled && cfg.verify_when == VerifyWhen::Done;
    let checked: Vec<Option<String>> = if required {
        let bytes = read_file_with_snapshot(&found.path)?.bytes;
        check_done_commands(out, &found, &env, &state, &ops, &bytes)?
    } else {
        vec![None; ops.len()]
    };

    let mut lock_warnings = Vec::new();
    let mut stamp_warning: Option<String> = None;
    let mut migrated_notice: Option<String> = None;
    let mut dry_run_diff: Option<String> = None;
    let applied = locked_read_modify_write_full(
        &found.path,
        &state,
        &agent,
        META.name,
        &cfg.lock,
        Some(&cfg.id_config),
        &mut lock_warnings,
        |bytes, _snap| {
            let parsed = parse_doc_for_write(bytes, &label, &cfg)?;
            stamp_warning = parsed.newer_tool_warning(&label);
            let archive = read_optional_archive(&archive_path, cfg.id_config.clone())?;
            // Files in an older format are migrated first, in this same write (R-MIG-1).
            let (migrated_todo, migration) =
                PendingMigration::plan(&parsed, &label, archive.as_ref(), &archive_path, &cfg)?
                    .split();
            let doc = migrated_todo.map_or(parsed, |m| m.doc);
            let initial_state = state.read_ids()?.map_or(0, |s| s.highest_id);
            let mut pass = run_batch(&ops, doc, archive.as_ref(), initial_state, &env, &checked)?;

            // A `done` whose `verify` line appeared after the batch was checked is refused, so no
            // change goes through on a check that never ran (T-173).
            if required {
                for (idx, entry) in pass.done_verify.iter().enumerate() {
                    if let Some((id, _)) = entry
                        && checked[idx].is_none()
                    {
                        return Err(TodoError::new(
                            ErrorCode::ChangedDuringWrite,
                            format!(
                                "the verify line of {id} changed while the batch was checked, so nothing was written; run apply again"
                            ),
                        )
                        .with_task(id.clone()));
                    }
                }
            }

            if pass.working_bytes == pass.start_bytes {
                return Ok(WriteOutcomeFull::Unchanged(pass.results));
            }

            // The same write adds or raises the tool stamp (R-VER-1).
            if let Some(stamped) = pass.doc.with_tool_stamp(RUNNING_VERSION).map_err(|e| {
                TodoError::new(ErrorCode::WriteFailed, e.to_string()).with_file(&label)
            })? {
                pass.working_bytes = stamped.bytes;
            }

            if ctx.dry_run {
                let before_str = String::from_utf8_lossy(bytes);
                let after_str = String::from_utf8_lossy(&pass.working_bytes);
                let diff = TextDiff::from_lines(before_str.as_ref(), after_str.as_ref())
                    .unified_diff()
                    .header(&format!("a/{label}"), &format!("b/{label}"))
                    .to_string();
                dry_run_diff = Some(diff);
                return Ok(WriteOutcomeFull::Unchanged(pass.results));
            }

            if pass.highest_id > initial_state {
                state.write_ids(&IdsState {
                    highest_id: pass.highest_id,
                })?;
            }

            migrated_notice = migration.as_ref().map(|m| m.summary.clone());
            Ok(WriteOutcomeFull::Changed {
                new_bytes: pass.working_bytes,
                touched_tasks: None,
                value: pass.results,
                migration,
                archive: None,
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
    done_commit: Option<&str>,
    done_by: Option<&str>,
    done_scope: &CommitScope,
    now: chrono::DateTime<chrono::FixedOffset>,
    highest_id: &mut u64,
    refs: &mut BTreeMap<String, String>,
    passed: Option<&str>,
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
            milestone,
            goal,
            why,
            summary,
            depends,
            files,
            read,
            problem,
            fix,
            steps,
            accept,
            verify,
            fields,
        } => {
            // Free-text values: line breaks become spaces, as `add` makes them (`docs/format.md` §10).
            let clean_title = sanitize_trimmed_value(title);
            if clean_title.is_empty() {
                return Err(TodoError::new(
                    ErrorCode::Usage,
                    format!("operation #{op_num} (add): title cannot be empty"),
                ));
            }
            // The parent may be an `@ref` to an earlier operation, which is already in `doc`.
            let milestone_src = milestone
                .as_deref()
                .or_else(|| fields.get("milestone").map(String::as_str))
                .map(str::trim)
                .filter(|m| !m.is_empty());
            let goal_src = goal
                .as_deref()
                .or_else(|| fields.get("goal").map(String::as_str))
                .map(sanitize_trimmed_value)
                .filter(|g| !g.is_empty());
            // `why` may also come from `fields`; left in `extra` it would be dropped (known field).
            let why_src = why
                .as_deref()
                .or_else(|| fields.get("why").map(String::as_str))
                .map(sanitize_trimmed_value)
                .filter(|w| !w.is_empty());
            // `summary` may also come from `fields`; line breaks become spaces (R-WRITE-19).
            let summary_src = summary
                .as_deref()
                .or_else(|| fields.get("summary").map(String::as_str))
                .map(sanitize_trimmed_value)
                .filter(|s| !s.is_empty());
            let read_src = read
                .as_deref()
                .or_else(|| fields.get("read").map(String::as_str))
                .map(sanitize_trimmed_value)
                .filter(|r| !r.is_empty());
            let verify_src = verify
                .as_deref()
                .or_else(|| fields.get("verify").map(String::as_str))
                .map(sanitize_trimmed_value)
                .filter(|v| !v.is_empty());
            let steps_list = match steps {
                Some(input) => steps_from_text(&input.text()),
                None => fields
                    .get("steps")
                    .map_or_else(Vec::new, |text| steps_from_text(text)),
            };
            let milestone_id = match milestone_src {
                Some(raw) => {
                    let target = resolve_task_ref(raw, refs)?;
                    Some(
                        milestone::check_parent(doc, cfg, None, &target)
                            .map_err(|e| at_operation(e, op_num, "add"))?,
                    )
                }
                None => None,
            };
            milestone::check_new_entry_fields(
                task_type.as_deref(),
                milestone_id.as_deref(),
                goal_src.as_deref(),
            )
            .map_err(|e| at_operation(e, op_num, "add"))?;
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
            // A task needs a level as `add` requires one (R-LEVEL-1); the whole batch is refused.
            if difficulty.is_none()
                && cfg.requires_field("difficulty")
                && EntryKind::from_type(task_type.as_deref()) == EntryKind::Task
            {
                return Err(at_operation(
                    TodoError::new(
                        ErrorCode::MissingField,
                        "missing required field `difficulty`; give \"difficulty\" in the add operation",
                    ),
                    op_num,
                    "add",
                ));
            }
            // The priority is checked as `add` checks it (T-179).
            let priority = cfg
                .canonical_priority(priority)
                .map_err(|e| at_operation(e, op_num, "add"))?;
            let extra: Vec<(String, String)> = fields
                .iter()
                .map(|(k, v)| (k.clone(), sanitize_trimmed_value(v)))
                .collect();
            let lines = NewTask {
                title: &clean_title,
                priority: Some(priority.as_str()),
                difficulty: difficulty.as_deref(),
                task_type: trim(task_type).as_deref(),
                area: trim(area).as_deref(),
                milestone: milestone_id.as_deref(),
                goal: goal_src.as_deref(),
                why: why_src.as_deref(),
                summary: summary_src.as_deref(),
                depends: dep_str.as_deref(),
                files: trim(files).as_deref(),
                read: read_src.as_deref(),
                problem: trim(problem).as_deref(),
                fix: trim(fix).as_deref(),
                steps: &steps_list,
                accept: trim(accept).as_deref(),
                verify: verify_src.as_deref(),
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
            apply_task_op(
                other,
                op_num,
                doc,
                archive,
                label,
                &env,
                refs,
                done_commit,
                done_by,
                done_scope,
                passed,
            )
        }
    }
}

/// Names the operation in a refusal, as the other `apply` errors do (R-WRITE-15).
fn at_operation(mut err: TodoError, op_num: usize, name: &str) -> TodoError {
    err.message = format!("operation #{op_num} ({name}): {}", err.message);
    err
}

/// A field value with its line breaks turned into spaces and trimmed, if present (R-EDIT-5).
fn trim(v: &Option<String>) -> Option<String> {
    v.as_deref().map(sanitize_trimmed_value)
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
    done_commit: Option<&str>,
    done_by: Option<&str>,
    done_scope: &CommitScope,
    passed: Option<&str>,
) -> Result<(&'static str, String, Vec<LineEdit>, TaskChangeIntent)> {
    let mut dep_owned = String::new();
    let mut milestone_owned = String::new();
    let mut steps_owned = String::new();
    let mut changes: Vec<SetChange<'_>> = Vec::new();
    let (raw_task, task_op) = match op {
        ApplyOp::Claim { task } => (task, TaskOp::Claim),
        ApplyOp::Release { task } => (task, TaskOp::Release),
        // The `verify` command of each `done` ran before the lock was taken, as `done` runs it
        // (T-173); `passed` is the command that passed, which the change checks again.
        ApplyOp::Done { task, note, .. } => (
            task,
            TaskOp::Done {
                note: note.as_deref(),
                commit: done_commit,
                by: done_by,
                scope: done_scope,
                check: passed.map_or(DoneCheck::NotChecked, |command| DoneCheck::Passed {
                    command,
                }),
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
            milestone,
            goal,
            why,
            summary,
            depends,
            files,
            read,
            problem,
            fix,
            steps,
            accept,
            verify,
            fields,
        } => {
            if let Some(input) = steps {
                steps_owned = input.text();
            }
            if let Some(dep) = depends {
                dep_owned = resolve_depends(dep, refs)?;
            }
            if let Some(m) = milestone {
                milestone_owned = resolve_task_ref(m.trim(), refs)?;
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
            if milestone.is_some() {
                changes.push(SetChange::Field("milestone", &milestone_owned));
            }
            if let Some(g) = goal {
                changes.push(SetChange::Field("goal", g.trim()));
            }
            if depends.is_some() {
                changes.push(if dep_owned.is_empty() {
                    SetChange::Unset("depends")
                } else {
                    SetChange::Field("depends", &dep_owned)
                });
            }
            if steps.is_some() {
                changes.push(SetChange::Field("steps", &steps_owned));
            }
            let rest = [
                ("why", why),
                ("summary", summary),
                ("files", files),
                ("read", read),
                ("problem", problem),
                ("fix", fix),
                ("accept", accept),
                ("verify", verify),
            ];
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

/// What `apply` needs for every pass over its batch, apart from the operations and the document
/// (T-173): the settings, the agent and the time of the batch.
struct BatchEnv<'a> {
    cfg: &'a OpsConfig,
    label: &'a str,
    agent: &'a str,
    force: bool,
    session: Option<&'a str>,
    /// The person behind each operation (`R-WRITE-16`), by operation index.
    done_bys: &'a [Option<String>],
    /// The commit of each `done` operation, resolved as `done` resolves it (`R-WRITE-4`).
    done_commits: &'a [Option<String>],
    /// What git says about the commit of each `done` operation (`R-SCOPE-2`).
    done_scopes: &'a [CommitScope],
    now: chrono::DateTime<chrono::FixedOffset>,
}

/// The outcome of one pass over a batch (T-173).
struct BatchPass {
    /// One entry per operation, as the output lists them.
    results: Vec<AppliedOpJson>,
    /// The text of the document before the batch, as migrated.
    start_bytes: Vec<u8>,
    /// The text after the last operation, not yet stamped.
    working_bytes: Vec<u8>,
    /// The document after the last operation.
    doc: Document,
    /// The highest task number after the batch (`R-ID-2`).
    highest_id: u64,
    /// By operation index: the task a `done` finishes, and the `verify` line that task has before it.
    done_verify: Vec<Option<(String, String)>>,
}

/// Applies the operations to `doc`, in order, in memory (T-173). `passed` gives, by operation index,
/// the `verify` command whose check passed before the lock was taken; a `done` with one records it
/// as its check, and its own change checks again that the line is still that command.
fn run_batch(
    ops: &[ApplyOp],
    mut doc: Document,
    archive: Option<&Document>,
    initial_state: u64,
    env: &BatchEnv<'_>,
    passed: &[Option<String>],
) -> Result<BatchPass> {
    let start_bytes = doc.to_bytes();
    let mut working_bytes = start_bytes.clone();
    let mut highest_id = highest_task_number(&doc, archive, initial_state);
    let mut refs: BTreeMap<String, String> = BTreeMap::new();
    let mut results = Vec::with_capacity(ops.len());
    let mut done_verify = Vec::with_capacity(ops.len());

    for (idx, op) in ops.iter().enumerate() {
        let op_num = idx + 1;
        done_verify.push(done_verify_line(op, &doc, &refs, env.cfg));
        let (op_name, task_id_str, edits, step_intent) = apply_one_op(
            op,
            op_num,
            &doc,
            archive,
            env.cfg,
            env.label,
            env.agent,
            env.force,
            env.session,
            env.done_commits[idx].as_deref(),
            env.done_bys[idx].as_deref(),
            &env.done_scopes[idx],
            env.now,
            &mut highest_id,
            &mut refs,
            passed.get(idx).and_then(|command| command.as_deref()),
        )?;

        let applied_step = doc.line_doc.apply_edits(&edits).map_err(|e| {
            TodoError::new(
                ErrorCode::WriteFailed,
                format!("apply operation #{op_num} ({op_name}) failed: {e}"),
            )
        })?;
        let new_doc =
            Document::parse_bytes_with_config(&applied_step.bytes, env.cfg.id_config.clone())
                .map_err(|e| {
                    TodoError::new(
                        ErrorCode::WriteFailed,
                        format!("apply operation #{op_num} ({op_name}) failed: {e}"),
                    )
                    .with_file(env.label)
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
    Ok(BatchPass {
        results,
        start_bytes,
        working_bytes,
        doc,
        highest_id,
        done_verify,
    })
}

/// The task a `done` operation finishes, and the `verify` line that task has just before the
/// operation (T-173). `None` for other operations, for a `done` whose task has no `verify` line,
/// and for a task that does not resolve: applying the operation reports that error.
fn done_verify_line(
    op: &ApplyOp,
    doc: &Document,
    refs: &BTreeMap<String, String>,
    cfg: &OpsConfig,
) -> Option<(String, String)> {
    let ApplyOp::Done { task, .. } = op else {
        return None;
    };
    let task_ref = resolve_task_ref(task, refs).ok()?;
    let target = resolve_task_in_doc(doc, &task_ref).ok()?;
    let command = target.verify.clone()?;
    Some((cfg.id_config.format_id(target.id), command))
}

/// Runs the `verify` command of each `done` in the batch before the lock is taken, as `done` runs its
/// command (T-173). The batch is applied in memory to find the line each `done` finishes with, and
/// each command runs in the folder `done` uses. A failing check, or one that cannot start, refuses
/// the whole batch, and nothing is written. Returns, by operation index, the command that passed.
fn check_done_commands(
    out: &mut Output,
    found: &FoundTodo,
    env: &BatchEnv<'_>,
    state: &StateDir,
    ops: &[ApplyOp],
    bytes: &[u8],
) -> Result<Vec<Option<String>>> {
    let cfg = env.cfg;
    let archive_path = cfg.archive_path(&found.path);
    let parsed = parse_doc_for_write(bytes, env.label, cfg)?;
    let archive = read_optional_archive(&archive_path, cfg.id_config.clone())?;
    let (migrated_todo, _) =
        PendingMigration::plan(&parsed, env.label, archive.as_ref(), &archive_path, cfg)?.split();
    let doc = migrated_todo.map_or(parsed, |m| m.doc);
    let initial_state = state.read_ids()?.map_or(0, |s| s.highest_id);
    let pass = run_batch(ops, doc, archive.as_ref(), initial_state, env, &[])?;

    let cwd = std::env::current_dir().unwrap_or_default();
    let folder = verify_folder(found, &cwd);
    let mut passed: Vec<Option<String>> = vec![None; ops.len()];
    for (idx, entry) in pass.done_verify.iter().enumerate() {
        let Some((id, command)) = entry else {
            continue;
        };
        let log = verify::log_path(state.path(), id);
        match verify::run_check(cfg, command, &folder, &log)? {
            CheckOutcome::Passed => passed[idx] = Some(command.clone()),
            CheckOutcome::NotRun { reason } => return Err(verify::not_run_error(id, &reason)),
            CheckOutcome::Failed(failure) => {
                if !out.is_json() {
                    for line in &failure.tail {
                        out.info(line);
                    }
                }
                let message = format!(
                    "apply refused: verify of {id} failed ({}), so nothing was written: {}",
                    verify::exit_text(&failure),
                    failure.last_line
                );
                return Err(verify::failure_error(id, &failure, message));
            }
        }
    }
    Ok(passed)
}
