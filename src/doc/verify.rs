//! High-level write verification combining byte-level and parsed-task checks (R-EDIT-7).

use std::collections::{BTreeMap, BTreeSet};

use crate::doc::id::TaskId;
use crate::doc::lines::{EditError, LineEdit, verify_byte_edits};
use crate::doc::parser::{Document, FieldEntry, Note, Status, Task};

/// Declares which tasks a write command is allowed to modify, add, or remove (R-EDIT-7).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskChangeIntent {
    /// Existing task IDs whose fields, notes, title, or section may change.
    pub modified_ids: BTreeSet<TaskId>,
    /// New task IDs allowed to appear in the edited document.
    pub added_ids: BTreeSet<TaskId>,
    /// Existing task IDs allowed to disappear from the edited document.
    pub removed_ids: BTreeSet<TaskId>,
}

impl TaskChangeIntent {
    /// Expects only task `id` to be modified.
    #[must_use]
    pub fn modify_one(id: TaskId) -> Self {
        let mut modified_ids = BTreeSet::new();
        modified_ids.insert(id);
        Self {
            modified_ids,
            ..Self::default()
        }
    }

    /// Expects only task `id` to be added.
    #[must_use]
    pub fn add_one(id: TaskId) -> Self {
        let mut added_ids = BTreeSet::new();
        added_ids.insert(id);
        Self {
            added_ids,
            ..Self::default()
        }
    }

    /// Expects only the given task IDs to be modified.
    #[must_use]
    pub fn modify_many(ids: impl IntoIterator<Item = TaskId>) -> Self {
        Self {
            modified_ids: ids.into_iter().collect(),
            ..Self::default()
        }
    }

    /// Expects only the given task IDs to be added.
    #[must_use]
    pub fn add_many(ids: impl IntoIterator<Item = TaskId>) -> Self {
        Self {
            added_ids: ids.into_iter().collect(),
            ..Self::default()
        }
    }

    /// Expects only the given task IDs to be removed (e.g. `archive`).
    #[must_use]
    pub fn remove_many(ids: impl IntoIterator<Item = TaskId>) -> Self {
        Self {
            removed_ids: ids.into_iter().collect(),
            ..Self::default()
        }
    }
}

/// Result of applying and verifying line edits against a [`Document`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedDocEdits {
    /// The newly parsed document.
    pub doc: Document,
    /// Exact bytes ready to write to disk.
    pub bytes: Vec<u8>,
    /// `true` if `bytes` differs from the original document's bytes (R-EDIT-6).
    pub changed: bool,
}

impl Document {
    /// Applies `edits` to this document and verifies both byte preservation and parsed-task
    /// equivalence outside `intent` (R-EDIT-7).
    pub fn apply_and_verify(
        &self,
        edits: &[LineEdit],
        intent: &TaskChangeIntent,
    ) -> Result<AppliedDocEdits, EditError> {
        let orig_bytes = self.to_bytes();
        let applied = self.line_doc.apply_edits(edits)?;
        verify_byte_edits(&orig_bytes, &applied.bytes, edits)?;

        let edited_doc = Document::from_line_doc(applied.doc, self.id_config.clone());
        verify_task_edits(self, &edited_doc, intent)?;

        Ok(AppliedDocEdits {
            doc: edited_doc,
            bytes: applied.bytes,
            changed: applied.changed,
        })
    }
}

/// Verifies that `edited` parses to the exact same tasks as `orig` apart from `intent`, and
/// introduced no file-wide write blockers or new task blockers on touched tasks (R-EDIT-7).
pub fn verify_task_edits(
    orig: &Document,
    edited: &Document,
    intent: &TaskChangeIntent,
) -> Result<(), EditError> {
    if edited.blocks_all_writes() {
        let first = edited
            .problems
            .iter()
            .find(|p| p.blocks_all_writes())
            .map(|p| format!("line {}: {}", p.line, p.message))
            .unwrap_or_else(|| "file became unsafe to write".to_string());
        return Err(EditError::VerificationFailed {
            reason: format!("edited document has file-wide blocker: {first}"),
        });
    }

    let mut orig_by_id: BTreeMap<TaskId, Vec<&Task>> = BTreeMap::new();
    for task in &orig.tasks {
        orig_by_id.entry(task.id).or_default().push(task);
    }

    let mut edited_by_id: BTreeMap<TaskId, Vec<&Task>> = BTreeMap::new();
    for task in &edited.tasks {
        edited_by_id.entry(task.id).or_default().push(task);
    }

    // Check every original task.
    let mut untouched_orig_order = Vec::new();
    for orig_task in &orig.tasks {
        if intent.removed_ids.contains(&orig_task.id) {
            if edited_by_id.contains_key(&orig_task.id) {
                return Err(EditError::VerificationFailed {
                    reason: format!(
                        "task {} was expected to be removed but still exists",
                        orig.id_config.format_id(orig_task.id)
                    ),
                });
            }
            continue;
        }

        let Some(edited_matches) = edited_by_id.get(&orig_task.id) else {
            return Err(EditError::VerificationFailed {
                reason: format!(
                    "untouched task {} disappeared after edit",
                    orig.id_config.format_id(orig_task.id)
                ),
            });
        };

        if intent.modified_ids.contains(&orig_task.id) {
            if edited.blocks_task_write(orig_task.id) {
                return Err(EditError::VerificationFailed {
                    reason: format!(
                        "modified task {} has a blocking problem after edit",
                        orig.id_config.format_id(orig_task.id)
                    ),
                });
            }
            continue;
        }

        // Untouched task must match identically (and keep count if it was unique).
        let orig_count = orig_by_id.get(&orig_task.id).map_or(0, Vec::len);
        if orig_count != edited_matches.len() {
            return Err(EditError::VerificationFailed {
                reason: format!(
                    "occurrence count of untouched task {} changed",
                    orig.id_config.format_id(orig_task.id)
                ),
            });
        }
        if let Some(&edited_task) = edited_matches.first()
            && orig_count == 1
            && task_snapshot(orig, orig_task) != task_snapshot(edited, edited_task)
        {
            return Err(EditError::VerificationFailed {
                reason: format!(
                    "untouched task {} parsed differently after edit",
                    orig.id_config.format_id(orig_task.id)
                ),
            });
        }
        untouched_orig_order.push(orig_task.id);
    }

    // Check every edited task for unexpected additions and untouched order.
    let mut untouched_edited_order = Vec::new();
    for edited_task in &edited.tasks {
        if !orig_by_id.contains_key(&edited_task.id) {
            if !intent.added_ids.contains(&edited_task.id) {
                return Err(EditError::VerificationFailed {
                    reason: format!(
                        "unexpected task {} appeared after edit",
                        edited.id_config.format_id(edited_task.id)
                    ),
                });
            }
            if edited.blocks_task_write(edited_task.id) {
                return Err(EditError::VerificationFailed {
                    reason: format!(
                        "newly added task {} has a blocking problem",
                        edited.id_config.format_id(edited_task.id)
                    ),
                });
            }
        } else if !intent.modified_ids.contains(&edited_task.id)
            && !intent.removed_ids.contains(&edited_task.id)
        {
            untouched_edited_order.push(edited_task.id);
        }
    }

    if untouched_orig_order != untouched_edited_order {
        return Err(EditError::VerificationFailed {
            reason: "order of untouched tasks changed after edit".to_string(),
        });
    }

    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct TaskSnapshot<'a> {
    id: TaskId,
    title: &'a str,
    section: Option<&'a str>,
    status: Status,
    status_raw: Option<&'a str>,
    owner: Option<&'a str>,
    claimed: Option<&'a str>,
    priority: Option<&'a str>,
    task_type: Option<&'a str>,
    area: Option<&'a str>,
    depends: Vec<(Option<TaskId>, &'a str)>,
    files: Option<&'a str>,
    problem: Option<&'a str>,
    fix: Option<&'a str>,
    accept: Option<&'a str>,
    commit: &'a [String],
    unknown_fields: Vec<(&'a str, &'a str)>,
    notes: Vec<(&'a str, &'a str, &'a str)>,
}

fn task_snapshot<'a>(doc: &'a Document, task: &'a Task) -> TaskSnapshot<'a> {
    TaskSnapshot {
        id: task.id,
        title: &task.title,
        section: doc.task_section_name(task),
        status: task.status,
        status_raw: task.status_raw.as_deref(),
        owner: task.owner.as_deref(),
        claimed: task.claimed.as_deref(),
        priority: task.priority.as_deref(),
        task_type: task.task_type.as_deref(),
        area: task.area.as_deref(),
        depends: task
            .depends
            .iter()
            .map(|d| (d.id, d.raw.as_str()))
            .collect(),
        files: task.files.as_deref(),
        problem: task.problem.as_deref(),
        fix: task.fix.as_deref(),
        accept: task.accept.as_deref(),
        commit: &task.commit,
        unknown_fields: task
            .unknown_fields
            .iter()
            .map(|FieldEntry { key, value, .. }| (key.as_str(), value.as_str()))
            .collect(),
        notes: task
            .notes
            .iter()
            .map(
                |Note {
                     date_raw,
                     agent,
                     text,
                     ..
                 }| { (date_raw.as_str(), agent.as_str(), text.as_str()) },
            )
            .collect(),
    }
}
