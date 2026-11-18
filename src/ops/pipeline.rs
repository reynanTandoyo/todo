//! Shared write pipeline used by every mutating `todo` command (`R-LOCK-1..6`, `R-EDIT-7`, `R-EDIT-8`, `R-IO-1..5`).

use chrono::{DateTime, FixedOffset};
use serde::Serialize;
use similar::TextDiff;

use crate::atomic::validate_utf8;
use crate::cli::Context;
use crate::doc::{Document, EditError, LineEdit, Problem, Task, TaskChangeIntent, TaskId};
use crate::error::{ErrorCode, Result, TodoError};
use crate::find::FoundTodo;
use crate::lock::{WriteOutcomeFull, current_time, locked_read_modify_write_full};
use crate::ops::config::OpsConfig;
use crate::ops::edits::TaskEditBuilder;
use crate::ops::query::resolve_task_in_doc;
use crate::ops::read::{
    TaskJson, build_task_json, file_label, is_task_ready, read_optional_archive,
    require_known_format,
};
use crate::ops::rules::{ClaimPlan, ReleasePlan};
use crate::output::Output;
use crate::state::StateDir;
use crate::version::RUNNING_VERSION;

/// Closure that computes a command output payload from a verified post-edit [`Document`].
pub type DocValueBuilder<T> = Box<dyn FnOnce(&Document) -> Result<T>>;

/// Plan returned by a document-level write operation inside [`edit_doc_and_write`].
pub enum DocWritePlan<T> {
    /// No lines need to change; `TODO.md` will not be rewritten (`R-EDIT-6`).
    Unchanged(T),
    /// Apply `edits`, verify against `intent` (`R-EDIT-7`), atomically write, and compute output
    /// value from the post-edit [`Document`].
    Edits {
        /// Non-overlapping whole-line edits.
        edits: Vec<LineEdit>,
        /// Intended task modifications/additions/removals for post-edit verification.
        intent: TaskChangeIntent,
        /// Builds the return value from the verified post-edit [`Document`].
        build_value: DocValueBuilder<T>,
    },
}

/// Result returned by [`edit_doc_and_write`].
#[derive(Debug, Clone)]
pub struct WriteDocResult<T> {
    /// Discovery metadata for `TODO.md`.
    pub found: FoundTodo,
    /// `true` if `TODO.md` was modified on disk.
    pub changed: bool,
    /// Command-specific output payload.
    pub value: T,
}

/// Summary of a task that became ready as a result of a write (`R-DEP-4`).
#[derive(Debug, Clone, Serialize)]
pub struct NewlyReadyTask {
    /// Canonical formatted task ID.
    pub id: String,
    /// Task priority, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    /// Task title.
    pub title: String,
}

/// Result returned by [`edit_task_and_write`].
#[derive(Debug, Clone)]
pub struct WriteTaskResult<T> {
    /// Discovery metadata for `TODO.md`.
    pub found: FoundTodo,
    /// Canonical formatted task ID (e.g. `"T-012"`).
    pub task_id: String,
    /// Post-edit JSON representation of the target task (`R-WRITE`).
    pub task: TaskJson,
    /// Tasks that became ready after this edit (`R-DEP-4`).
    pub newly_ready: Vec<NewlyReadyTask>,
    /// `true` if `TODO.md` was modified on disk.
    pub changed: bool,
    /// Command-specific payload.
    pub value: T,
}

/// Parses `bytes` for writing and refuses any file-wide unsafe condition (`R-EDIT-8`, `format.md` §13).
///
/// A format version newer than this todo is refused first, with `TOOL_TOO_OLD` (R-VER-2).
pub fn parse_doc_for_write(bytes: &[u8], label: &str, cfg: &OpsConfig) -> Result<Document> {
    let _ = validate_utf8(bytes, label)?;
    let doc = Document::parse_bytes_with_config(bytes, cfg.id_config.clone()).map_err(|e| {
        TodoError::new(ErrorCode::UnsafeFile, e.to_string())
            .with_file(label)
            .with_line(e.line)
    })?;
    require_known_format(&doc, label)?;

    if let Some(blocker) = doc.problems.iter().find(|p| p.blocks_all_writes()) {
        return Err(problem_to_unsafe_error(blocker, label));
    }
    Ok(doc)
}

/// Refuses writing to `task` if it has any task-level blocking problem (`R-EDIT-8`, `format.md` §13),
/// including a duplicate ID in `TODO.md` or `TODO-archive.md`, a repeated field, or an unknown status.
pub fn check_task_writable(
    doc: &Document,
    task: &Task,
    label: &str,
    archive: Option<&Document>,
) -> Result<()> {
    let id_str = doc.id_config.format_id(task.id);
    if let Some(problem) = doc.blocking_problems_for_task(task.id).into_iter().next() {
        return Err(problem_to_unsafe_error(problem, label).with_task(id_str));
    }
    if let Some(arch) = archive
        && let Some(dup) = doc
            .check_archive_duplicates(arch)
            .into_iter()
            .find(|p| p.blocks_task(task.id))
    {
        return Err(problem_to_unsafe_error(&dup, label).with_task(id_str));
    }
    Ok(())
}

/// Runs the shared write pipeline for a document-level operation:
/// 1. Require `--agent` / `TODO_AGENT` (`R-CLAIM-5`).
/// 2. Locate `TODO.md` (`R-FIND-1..5`).
/// 3. Take the OS write lock and honour any live manual lease (`R-LOCK-1..6`).
/// 4. Read bytes under the lock, validate UTF-8, parse, and refuse unsafe files (`R-EDIT-8`).
/// 5. Execute `op` to produce [`DocWritePlan`].
/// 6. Apply line edits and run byte + parsed-task verification (`R-EDIT-7`).
/// 7. Atomically replace `TODO.md` only if bytes changed (`R-EDIT-6`, `R-IO-1..5`).
pub fn edit_doc_and_write<T, F>(
    ctx: &Context,
    cfg: &OpsConfig,
    command: &str,
    out: &mut Output,
    mut op: F,
) -> Result<WriteDocResult<T>>
where
    F: FnMut(
        &Document,
        Option<&Document>,
        &str,
        &str,
        DateTime<FixedOffset>,
    ) -> Result<DocWritePlan<T>>,
{
    edit_doc_with_state_and_write(
        ctx,
        cfg,
        command,
        out,
        |doc, archive, _state, label, agent, now| op(doc, archive, label, agent, now),
    )
}

type DocValueWithArchiveBuilder<T> = Box<dyn FnOnce(&Document, Option<&Document>) -> Result<T>>;

enum DocValueCallback<T> {
    Plain(DocValueBuilder<T>),
    WithArchive(DocValueWithArchiveBuilder<T>),
}

impl<T> DocValueCallback<T> {
    fn call(self, doc: &Document, archive: Option<&Document>) -> Result<T> {
        match self {
            Self::Plain(f) => f(doc),
            Self::WithArchive(f) => f(doc, archive),
        }
    }
}

enum DocWritePlanInner<T> {
    Unchanged(T),
    Edits {
        edits: Vec<LineEdit>,
        intent: TaskChangeIntent,
        build_value: DocValueCallback<T>,
    },
}

impl<T> From<DocWritePlan<T>> for DocWritePlanInner<T> {
    fn from(plan: DocWritePlan<T>) -> Self {
        match plan {
            DocWritePlan::Unchanged(val) => Self::Unchanged(val),
            DocWritePlan::Edits {
                edits,
                intent,
                build_value,
            } => Self::Edits {
                edits,
                intent,
                build_value: DocValueCallback::Plain(build_value),
            },
        }
    }
}

/// Same as [`edit_doc_and_write`], but also provides `&StateDir` inside the lock so commands like
/// `add` can read and update `ids.json` atomically under the write lock (`R-ID-2`).
pub fn edit_doc_with_state_and_write<T, F>(
    ctx: &Context,
    cfg: &OpsConfig,
    command: &str,
    out: &mut Output,
    mut op: F,
) -> Result<WriteDocResult<T>>
where
    F: FnMut(
        &Document,
        Option<&Document>,
        &StateDir,
        &str,
        &str,
        DateTime<FixedOffset>,
    ) -> Result<DocWritePlan<T>>,
{
    let find_timer = crate::timing::PhaseTimer::start("write.find_todo");
    let found = ctx.find_todo_with_mode(cfg.worktree, out)?;
    find_timer.finish();
    edit_doc_core(
        ctx,
        found,
        cfg,
        command,
        out,
        |doc, archive, state, label, agent, now| {
            Ok(op(doc, archive, state, label, agent, now)?.into())
        },
    )
}

/// Like [`edit_doc_with_state_and_write`], reusing an already-located [`FoundTodo`] to avoid
/// running `find_todo` twice.
pub(crate) fn edit_doc_with_found_and_state_and_write<T, F>(
    ctx: &Context,
    found: FoundTodo,
    cfg: &OpsConfig,
    command: &str,
    out: &mut Output,
    mut op: F,
) -> Result<WriteDocResult<T>>
where
    F: FnMut(
        &Document,
        Option<&Document>,
        &StateDir,
        &str,
        &str,
        DateTime<FixedOffset>,
    ) -> Result<DocWritePlan<T>>,
{
    crate::cli::emit_find_notices(&found, out);
    edit_doc_core(
        ctx,
        found,
        cfg,
        command,
        out,
        |doc, archive, state, label, agent, now| {
            Ok(op(doc, archive, state, label, agent, now)?.into())
        },
    )
}

fn edit_doc_core<T, F>(
    ctx: &Context,
    found: FoundTodo,
    cfg: &OpsConfig,
    command: &str,
    out: &mut Output,
    mut op: F,
) -> Result<WriteDocResult<T>>
where
    F: FnMut(
        &Document,
        Option<&Document>,
        &StateDir,
        &str,
        &str,
        DateTime<FixedOffset>,
    ) -> Result<DocWritePlanInner<T>>,
{
    let agent = ctx.require_agent()?.to_owned();
    if let Some(warning) = crate::find::synced_folder_warning(&found.path) {
        out.warn(warning);
    }
    let state_timer = crate::timing::PhaseTimer::start("write.state_dir");
    let state = StateDir::from_found(&found)?;
    state_timer.finish();
    let label = file_label(&found.path);
    let archive_path = cfg.archive_path(&found.path);

    let mut dry_run_diff: Option<String> = None;
    let mut lock_warnings: Vec<String> = Vec::new();
    let mut stamp_warning: Option<String> = None;
    let (changed, value) = locked_read_modify_write_full(
        &found.path,
        &state,
        &agent,
        command,
        &cfg.lock,
        Some(&cfg.id_config),
        &mut lock_warnings,
        |bytes, _snap| {
            let parse_timer = crate::timing::PhaseTimer::start("write.parse_doc");
            let doc = parse_doc_for_write(bytes, &label, cfg)?;
            parse_timer.finish();
            stamp_warning = doc.newer_tool_warning(&label);
            let arch_timer = crate::timing::PhaseTimer::start("write.read_archive");
            let archive = read_optional_archive(&archive_path, cfg.id_config.clone())?;
            arch_timer.finish();
            let now = current_time();

            let op_timer = crate::timing::PhaseTimer::start("write.op_plan");
            let plan = op(&doc, archive.as_ref(), &state, &label, &agent, now)?;
            op_timer.finish();
            match plan {
                DocWritePlanInner::Unchanged(val) => {
                    if ctx.dry_run {
                        dry_run_diff = Some(String::new());
                    }
                    Ok(WriteOutcomeFull::Unchanged((false, val)))
                }
                DocWritePlanInner::Edits {
                    edits,
                    intent,
                    build_value,
                } => {
                    let apply_timer = crate::timing::PhaseTimer::start("write.apply_and_verify");
                    let applied = doc
                        .apply_and_verify_with_orig_bytes(bytes, &edits, &intent)
                        .map_err(|e| edit_error_to_todo_error(&e, &label))?;
                    apply_timer.finish();
                    let val_timer = crate::timing::PhaseTimer::start("write.build_value");
                    let val = build_value.call(&applied.doc, archive.as_ref())?;
                    val_timer.finish();
                    if !applied.changed {
                        if ctx.dry_run {
                            dry_run_diff = Some(String::new());
                        }
                        return Ok(WriteOutcomeFull::Unchanged((false, val)));
                    }
                    // The same write adds or raises the tool stamp (R-VER-1).
                    let applied = match applied
                        .doc
                        .with_tool_stamp(RUNNING_VERSION)
                        .map_err(|e| edit_error_to_todo_error(&e, &label))?
                    {
                        Some(stamped) => stamped,
                        None => applied,
                    };
                    if ctx.dry_run {
                        let diff_timer = crate::timing::PhaseTimer::start("write.dry_run_diff");
                        let old_text = String::from_utf8_lossy(bytes);
                        let new_text = String::from_utf8_lossy(&applied.bytes);
                        let diff = TextDiff::from_lines(old_text.as_ref(), new_text.as_ref())
                            .unified_diff()
                            .header(&format!("a/{label}"), &format!("b/{label}"))
                            .to_string();
                        dry_run_diff = Some(diff);
                        diff_timer.finish();
                        return Ok(WriteOutcomeFull::Unchanged((false, val)));
                    }
                    let touched_tasks = crate::history::diff_doc_task_ids(&doc, &applied.doc);
                    Ok(WriteOutcomeFull::Changed {
                        new_bytes: applied.bytes,
                        touched_tasks: Some(touched_tasks),
                        value: (true, val),
                    })
                }
            }
        },
    )?;

    for warn in lock_warnings {
        out.warn(warn);
    }
    if let Some(warn) = stamp_warning {
        out.warn(warn);
    }

    if let Some(diff) = dry_run_diff {
        if out.is_json() {
            out.set_dry_run_diff(diff);
        } else if diff.is_empty() {
            out.write_line("dry-run: no changes");
        } else {
            out.write_str(&diff);
            if !diff.ends_with('\n') {
                out.write_str("\n");
            }
        }
    }

    Ok(WriteDocResult {
        found,
        changed,
        value,
    })
}

/// Checks whether a bulk command may proceed (`R-WRITE-14`).
///
/// When `--yes` is missing (or `--dry-run` is active), prints `preview` (and a hint to pass
/// `--yes`) and returns `Ok(false)` so the caller makes no changes on disk.
pub fn require_yes_or_preview(ctx: &Context, preview: &str, out: &mut Output) -> bool {
    if ctx.yes && !ctx.dry_run {
        return true;
    }
    out.write_line(preview);
    if !ctx.yes {
        out.write_line("dry-run preview: pass --yes to apply these changes");
    }
    false
}

/// Runs the shared write pipeline targeting a single task by ID (`task_input` per `R-ID-3`).
///
/// Enforces file-wide and task-specific `UNSAFE_FILE` refusals (`R-EDIT-8`), runs `op` inside
/// the lock, verifies edits (`R-EDIT-7`), writes atomically (`R-IO-1..5`), and returns the
/// post-edit [`TaskJson`].
pub fn edit_task_and_write<T, F>(
    ctx: &Context,
    cfg: &OpsConfig,
    command: &str,
    task_input: &str,
    out: &mut Output,
    op: F,
) -> Result<WriteTaskResult<T>>
where
    F: FnMut(
        &Document,
        &Task,
        Option<&Document>,
        &str,
        DateTime<FixedOffset>,
    ) -> Result<(Vec<LineEdit>, T)>,
    T: 'static,
{
    let find_timer = crate::timing::PhaseTimer::start("write.find_todo");
    let found = ctx.find_todo_with_mode(cfg.worktree, out)?;
    find_timer.finish();
    edit_task_core(ctx, found, cfg, command, task_input, out, op)
}

/// Like [`edit_task_and_write`], reusing an already-located [`FoundTodo`].
pub(crate) fn edit_task_with_found_and_write<T, F>(
    ctx: &Context,
    found: FoundTodo,
    cfg: &OpsConfig,
    command: &str,
    task_input: &str,
    out: &mut Output,
    op: F,
) -> Result<WriteTaskResult<T>>
where
    F: FnMut(
        &Document,
        &Task,
        Option<&Document>,
        &str,
        DateTime<FixedOffset>,
    ) -> Result<(Vec<LineEdit>, T)>,
    T: 'static,
{
    crate::cli::emit_find_notices(&found, out);
    edit_task_core(ctx, found, cfg, command, task_input, out, op)
}

fn edit_task_core<T, F>(
    ctx: &Context,
    found: FoundTodo,
    cfg: &OpsConfig,
    command: &str,
    task_input: &str,
    out: &mut Output,
    mut op: F,
) -> Result<WriteTaskResult<T>>
where
    F: FnMut(
        &Document,
        &Task,
        Option<&Document>,
        &str,
        DateTime<FixedOffset>,
    ) -> Result<(Vec<LineEdit>, T)>,
    T: 'static,
{
    let label_cfg = cfg.clone();

    let res = edit_doc_core(
        ctx,
        found,
        cfg,
        command,
        out,
        |doc, archive, _state, label, agent, now| {
            let task = resolve_task_in_doc(doc, task_input)?;
            let task_id = task.id;
            let id_str = label_cfg.id_config.format_id(task_id);
            check_task_writable(doc, task, label, archive)?;

            let (edits, payload) = op(doc, task, archive, agent, now)?;
            if edits.is_empty() {
                let task_json = build_task_json(doc, task, archive, false, now, &label_cfg);
                return Ok(DocWritePlanInner::Unchanged((
                    task_json,
                    Vec::new(),
                    payload,
                )));
            }

            // Only another task that directly depends on `task_id` can transition from not-ready to
            // ready when only `task_id` is modified.
            let pre_not_ready_dependents: Vec<TaskId> = doc
                .tasks
                .iter()
                .filter(|t| {
                    t.id != task_id
                        && t.depends.iter().any(|d| d.id == Some(task_id))
                        && !is_task_ready(doc, t, archive, now, &label_cfg)
                })
                .map(|t| t.id)
                .collect();

            let cfg_owned = label_cfg.clone();
            let err_id = id_str.clone();
            let intent = TaskChangeIntent::modify_one(task_id);
            Ok(DocWritePlanInner::Edits {
                edits,
                intent,
                build_value: DocValueCallback::WithArchive(Box::new(
                    move |new_doc, new_archive| {
                        let updated_task = new_doc.task_by_id(task_id).ok_or_else(|| {
                            TodoError::new(
                                ErrorCode::WriteFailed,
                                format!("edited task {err_id} missing after verification"),
                            )
                        })?;
                        let task_json = build_task_json(
                            new_doc,
                            updated_task,
                            new_archive,
                            false,
                            now,
                            &cfg_owned,
                        );
                        let newly_ready = pre_not_ready_dependents
                            .into_iter()
                            .filter_map(|dep_id| {
                                let t = new_doc.task_by_id(dep_id)?;
                                is_task_ready(new_doc, t, new_archive, now, &cfg_owned).then(|| {
                                    NewlyReadyTask {
                                        id: cfg_owned.id_config.format_id(t.id),
                                        priority: t.priority.clone(),
                                        title: t.title.clone(),
                                    }
                                })
                            })
                            .collect();
                        Ok((task_json, newly_ready, payload))
                    },
                )),
            })
        },
    )?;

    Ok(WriteTaskResult {
        found: res.found,
        task_id: res.value.0.id.clone(),
        task: res.value.0,
        newly_ready: res.value.1,
        changed: res.changed,
        value: res.value.2,
    })
}

/// Converts a [`ClaimPlan`] into canonical [`LineEdit`]s and [`TaskChangeIntent`].
///
/// Shared by `todo claim` and `todo next --claim`. `session` is the claiming agent session
/// (`R-CLAIM-7`); `None` removes a leftover `session` line.
#[must_use]
pub fn build_claim_edits(
    doc: &Document,
    task: &Task,
    plan: &ClaimPlan,
    session: Option<&str>,
    now: DateTime<FixedOffset>,
) -> (Vec<LineEdit>, TaskChangeIntent) {
    let mut builder = TaskEditBuilder::new(doc, task);
    builder.set_status(plan.new_status);
    builder.set_owner(Some(&plan.new_owner));
    builder.set_claimed(&plan.claimed_timestamp);
    builder.set_session(session);
    if let Some(ref note_text) = plan.auto_note {
        let date = OpsConfig::format_note_date(now);
        let author = if plan.new_owner.is_empty() {
            "agent"
        } else {
            &plan.new_owner
        };
        builder.add_note(&date, author, note_text);
    }
    builder.build()
}

/// Converts a [`ReleasePlan`] into canonical [`LineEdit`]s and [`TaskChangeIntent`].
#[must_use]
pub fn build_release_edits(
    doc: &Document,
    task: &Task,
    plan: &ReleasePlan,
    agent: &str,
    now: DateTime<FixedOffset>,
) -> (Vec<LineEdit>, TaskChangeIntent) {
    let mut builder = TaskEditBuilder::new(doc, task);
    if task.status != plan.new_status {
        builder.set_status(plan.new_status);
    }
    builder.clear_owner_if_present();
    builder.remove_claimed();
    if let Some(ref note_text) = plan.auto_note {
        let date = OpsConfig::format_note_date(now);
        builder.add_note(&date, agent, note_text);
    }
    builder.build()
}

fn problem_to_unsafe_error(problem: &Problem, file_label: &str) -> TodoError {
    TodoError::new(ErrorCode::UnsafeFile, problem.message.clone())
        .with_file(file_label)
        .with_line(problem.line)
}

fn edit_error_to_todo_error(err: &EditError, file_label: &str) -> TodoError {
    match err {
        EditError::InvalidUtf8(u) => TodoError::new(ErrorCode::UnsafeFile, u.to_string())
            .with_file(file_label)
            .with_line(u.line),
        _ => TodoError::new(ErrorCode::WriteFailed, err.to_string()).with_file(file_label),
    }
}
