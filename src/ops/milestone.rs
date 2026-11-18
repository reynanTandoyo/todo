//! Milestone rules shared by `add`, `set`, `apply`, `done` and `drop` (`docs/format.md` §17;
//! R-MILE-1, R-MILE-2).
//!
//! A milestone is a piece of work with a `goal`, divided into tasks. Tasks and milestones name
//! their parent with `milestone`; questions and ideas are not part of the tree. Every check here
//! runs before anything is written, so a refused change leaves the file byte-identical (R-MILE-1).

use serde_json::Value;

use crate::doc::{Document, EntryKind, Task};
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::config::OpsConfig;
use crate::ops::query::resolve_task_in_doc;
use crate::ops::task_ops::SetChange;

/// Checks the rules a new entry must meet before anything is written (R-MILE-1): a milestone needs a
/// goal, and only tasks and milestones may name a parent milestone.
pub fn check_new_entry_fields(
    task_type: Option<&str>,
    milestone: Option<&str>,
    goal: Option<&str>,
) -> Result<()> {
    let kind = EntryKind::from_type(task_type);
    if milestone.is_some() && !matches!(kind, EntryKind::Task | EntryKind::Milestone) {
        return Err(not_in_tree_error(kind));
    }
    if kind == EntryKind::Milestone && goal.is_none() {
        return Err(TodoError::new(
            ErrorCode::MissingField,
            "a milestone needs a goal: pass --goal <TEXT>",
        )
        .with_detail("field", Value::String("goal".to_string())));
    }
    Ok(())
}

/// Resolves a `milestone` value to the milestone it names in `doc`, and checks that `entry` may be
/// placed under it (R-MILE-1). `entry` is the task being changed, or `None` for a new one.
///
/// Returns the canonical ID of the parent. A missing parent is `NOT_FOUND`; a parent that is not a
/// milestone, an empty value and a change that would make a milestone its own ancestor are
/// `BAD_VALUE`.
pub fn check_parent(
    doc: &Document,
    cfg: &OpsConfig,
    entry: Option<&Task>,
    value: &str,
) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(TodoError::new(
            ErrorCode::BadValue,
            "--milestone needs a task; use --unset milestone to make an entry top-level",
        ));
    }
    let parent = resolve_task_in_doc(doc, value)?;
    let parent_id = cfg.id_config.format_id(parent.id);
    if !parent.is_milestone() {
        return Err(TodoError::new(
            ErrorCode::BadValue,
            format!("task {parent_id} is not a milestone; only a milestone can hold tasks"),
        )
        .with_task(parent_id));
    }
    if let Some(entry) = entry
        && (parent.id == entry.id
            || doc
                .milestone_ancestors(parent)
                .iter()
                .any(|a| a.id == entry.id))
    {
        let entry_id = cfg.id_config.format_id(entry.id);
        return Err(TodoError::new(
            ErrorCode::BadValue,
            format!(
                "{entry_id} cannot go under {parent_id}: that would make {entry_id} its own ancestor"
            ),
        )
        .with_task(entry_id));
    }
    Ok(parent_id)
}

/// Resolves a milestone argument (an ID or a title, as elsewhere) to the milestone it names in `doc`
/// (`R-MILE-3`). A missing task is `NOT_FOUND`; a task that is not a milestone is `BAD_VALUE`,
/// because only a milestone has entries below it.
pub fn resolve_milestone<'a>(doc: &'a Document, input: &str) -> Result<&'a Task> {
    let task = resolve_task_in_doc(doc, input.trim())?;
    if !task.is_milestone() {
        let id_str = doc.id_config.format_id(task.id);
        return Err(TodoError::new(
            ErrorCode::BadValue,
            format!("task {id_str} is not a milestone; only a milestone has entries below it"),
        )
        .with_task(id_str));
    }
    Ok(task)
}

/// The canonical form of a `milestone` value that [`check_parent`] has already accepted.
#[must_use]
pub fn canonical_parent(doc: &Document, cfg: &OpsConfig, value: &str) -> String {
    match resolve_task_in_doc(doc, value.trim()) {
        Ok(parent) => cfg.id_config.format_id(parent.id),
        Err(_) => value.trim().to_string(),
    }
}

/// Checks the milestone rules of `set` (R-MILE-1) against `task` as it will be once `changes` have
/// applied in order: a question or idea cannot hold a milestone, a milestone with entries below it
/// cannot change kind, and a new parent must pass [`check_parent`].
pub fn check_set_changes(
    doc: &Document,
    cfg: &OpsConfig,
    task: &Task,
    changes: &[SetChange<'_>],
) -> Result<()> {
    let mut task_type = task.task_type.clone();
    let mut parent = task.milestone.clone();
    let (mut type_given, mut parent_given) = (false, false);
    for change in changes {
        match *change {
            SetChange::Field(key, value) if key.trim().eq_ignore_ascii_case("type") => {
                type_given = true;
                task_type = Some(value.trim().to_string()).filter(|v| !v.is_empty());
            }
            SetChange::Unset(key) if key.trim().eq_ignore_ascii_case("type") => {
                type_given = true;
                task_type = None;
            }
            SetChange::Field(key, value) if key.trim().eq_ignore_ascii_case("milestone") => {
                parent_given = true;
                parent = Some(value.to_string());
            }
            SetChange::Unset(key) if key.trim().eq_ignore_ascii_case("milestone") => {
                parent_given = true;
                parent = None;
            }
            _ => {}
        }
    }

    let id_str = cfg.id_config.format_id(task.id);
    let kind = EntryKind::from_type(task_type.as_deref());
    if type_given && task.is_milestone() && kind != EntryKind::Milestone {
        let below = doc.entries_below(task.id).len();
        if below > 0 {
            return Err(TodoError::new(
                ErrorCode::BadValue,
                format!(
                    "{id_str} has {below} entries below it; a milestone with entries below it stays a milestone"
                ),
            )
            .with_task(id_str));
        }
    }
    if (type_given || parent_given)
        && !matches!(kind, EntryKind::Task | EntryKind::Milestone)
        && parent.is_some()
    {
        return Err(not_in_tree_error(kind));
    }
    if parent_given && let Some(value) = parent.as_deref() {
        check_parent(doc, cfg, Some(task), value)?;
    }
    Ok(())
}

/// The refusal of `done` or `drop` on a milestone with unfinished entries below it (R-MILE-2).
///
/// Lists the unfinished entries with their status, and names the fix in `how_out`.
#[must_use]
pub fn refuse_unfinished(
    cfg: &OpsConfig,
    milestone: &Task,
    unfinished: &[&Task],
    how_out: &str,
) -> TodoError {
    let id_str = cfg.id_config.format_id(milestone.id);
    let listed: Vec<String> = unfinished
        .iter()
        .map(|t| format!("{} ({})", cfg.id_config.format_id(t.id), t.status.as_str()))
        .collect();
    let ids: Vec<Value> = unfinished
        .iter()
        .map(|t| Value::String(cfg.id_config.format_id(t.id)))
        .collect();
    TodoError::new(
        ErrorCode::BadTransition,
        format!(
            "{id_str} has unfinished entries below it: {}; {how_out}",
            listed.join(", ")
        ),
    )
    .with_task(id_str)
    .with_detail("unfinished", Value::Array(ids))
}

fn not_in_tree_error(kind: EntryKind) -> TodoError {
    TodoError::new(
        ErrorCode::BadValue,
        format!(
            "a {} is not part of the milestone tree; only tasks and milestones can name a milestone",
            kind.as_str()
        ),
    )
}
