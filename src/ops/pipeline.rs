//! Shared write pipeline used by every mutating `todo` command (`R-LOCK-1..6`, `R-EDIT-7`, `R-EDIT-8`, `R-IO-1..5`).

use chrono::{DateTime, FixedOffset};
use similar::TextDiff;

use crate::atomic::validate_utf8;
use crate::cli::Context;
use crate::doc::{Document, EditError, LineEdit, Problem, Task, TaskChangeIntent};
use crate::error::{ErrorCode, Result, TodoError};
use crate::find::FoundTodo;
use crate::lock::{WriteOutcome, current_time, locked_read_modify_write_with_warnings};
use crate::ops::config::OpsConfig;
use crate::ops::edits::TaskEditBuilder;
use crate::ops::query::resolve_task_in_doc;
use crate::ops::read::{TaskJson, build_task_json, file_label, read_optional_archive};
use crate::ops::rules::{ClaimPlan, ReleasePlan};
use crate::output::Output;
use crate::state::StateDir;

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

/// Result returned by [`edit_task_and_write`].
#[derive(Debug, Clone)]
pub struct WriteTaskResult<T> {
    /// Discovery metadata for `TODO.md`.
    pub found: FoundTodo,
    /// Canonical formatted task ID (e.g. `"T-012"`).
    pub task_id: String,
    /// Post-edit JSON representation of the target task (`R-WRITE`).
    pub task: TaskJson,
    /// `true` if `TODO.md` was modified on disk.
    pub changed: bool,
    /// Command-specific payload.
    pub value: T,
}

/// Parses `bytes` for writing and refuses any file-wide unsafe condition (`R-EDIT-8`, `format.md` §13).
pub fn parse_doc_for_write(bytes: &[u8], label: &str, cfg: &OpsConfig) -> Result<Document> {
    let _ = validate_utf8(bytes, label)?;
    let doc = Document::parse_bytes_with_config(bytes, cfg.id_config.clone()).map_err(|e| {
        TodoError::new(ErrorCode::UnsafeFile, e.to_string())
            .with_file(label)
            .with_line(e.line)
    })?;

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
    let agent = ctx.require_agent()?.to_owned();
    let found = ctx.find_todo_with_mode(cfg.worktree, out)?;
    if let Some(warning) = crate::find::synced_folder_warning(&found.path) {
        out.warn(warning);
    }
    let state = StateDir::from_found(&found)?;
    let label = file_label(&found.path);
    let archive_path = cfg.archive_path(&found.path);

    let mut dry_run_diff: Option<String> = None;
    let mut lock_warnings: Vec<String> = Vec::new();
    let (changed, value) = locked_read_modify_write_with_warnings(
        &found.path,
        &state,
        &agent,
        command,
        &cfg.lock,
        &mut lock_warnings,
        |bytes, _snap| {
            let doc = parse_doc_for_write(bytes, &label, cfg)?;
            let archive = read_optional_archive(&archive_path, cfg.id_config.clone())?;
            let now = current_time();

            match op(&doc, archive.as_ref(), &state, &label, &agent, now)? {
                DocWritePlan::Unchanged(val) => {
                    if ctx.dry_run {
                        dry_run_diff = Some(String::new());
                    }
                    Ok(WriteOutcome::Unchanged((false, val)))
                }
                DocWritePlan::Edits {
                    edits,
                    intent,
                    build_value,
                } => {
                    let applied = doc
                        .apply_and_verify(&edits, &intent)
                        .map_err(|e| edit_error_to_todo_error(&e, &label))?;
                    let val = build_value(&applied.doc)?;
                    if !applied.changed {
                        if ctx.dry_run {
                            dry_run_diff = Some(String::new());
                        }
                        return Ok(WriteOutcome::Unchanged((false, val)));
                    }
                    if ctx.dry_run {
                        let old_text = String::from_utf8_lossy(bytes);
                        let new_text = String::from_utf8_lossy(&applied.bytes);
                        let diff = TextDiff::from_lines(old_text.as_ref(), new_text.as_ref())
                            .unified_diff()
                            .header(&format!("a/{label}"), &format!("b/{label}"))
                            .to_string();
                        dry_run_diff = Some(diff);
                        return Ok(WriteOutcome::Unchanged((false, val)));
                    }
                    Ok(WriteOutcome::Changed {
                        new_bytes: applied.bytes,
                        value: (true, val),
                    })
                }
            }
        },
    )?;

    for warn in lock_warnings {
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

    let res = edit_doc_and_write(ctx, cfg, command, out, |doc, archive, label, agent, now| {
        let task = resolve_task_in_doc(doc, task_input)?;
        let task_id = task.id;
        let id_str = label_cfg.id_config.format_id(task_id);
        check_task_writable(doc, task, label, archive)?;

        let (edits, payload) = op(doc, task, archive, agent, now)?;
        if edits.is_empty() {
            let task_json = build_task_json(doc, task, archive, false, now, &label_cfg);
            return Ok(DocWritePlan::Unchanged((task_json, payload)));
        }

        let arch_owned = archive.cloned();
        let cfg_owned = label_cfg.clone();
        let err_id = id_str.clone();
        let intent = TaskChangeIntent::modify_one(task_id);
        Ok(DocWritePlan::Edits {
            edits,
            intent,
            build_value: Box::new(move |new_doc| {
                let updated_task = new_doc.task_by_id(task_id).ok_or_else(|| {
                    TodoError::new(
                        ErrorCode::WriteFailed,
                        format!("edited task {err_id} missing after verification"),
                    )
                })?;
                let task_json = build_task_json(
                    new_doc,
                    updated_task,
                    arch_owned.as_ref(),
                    false,
                    now,
                    &cfg_owned,
                );
                Ok((task_json, payload))
            }),
        })
    })?;

    Ok(WriteTaskResult {
        found: res.found,
        task_id: res.value.0.id.clone(),
        task: res.value.0,
        changed: res.changed,
        value: res.value.1,
    })
}

/// Converts a [`ClaimPlan`] into canonical [`LineEdit`]s and [`TaskChangeIntent`].
///
/// Shared by `todo claim` and `todo next --claim`.
#[must_use]
pub fn build_claim_edits(
    doc: &Document,
    task: &Task,
    plan: &ClaimPlan,
    now: DateTime<FixedOffset>,
) -> (Vec<LineEdit>, TaskChangeIntent) {
    let mut builder = TaskEditBuilder::new(doc, task);
    builder.set_status(plan.new_status);
    builder.set_owner(Some(&plan.new_owner));
    builder.set_claimed(&plan.claimed_timestamp);
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
