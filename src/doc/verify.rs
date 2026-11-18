//! High-level write verification combining byte-level and parsed-task checks (R-EDIT-7).

use std::collections::BTreeSet;

use crate::doc::id::{FastU64Map, TaskId};
use crate::doc::lines::{EditError, LineEdit, verify_byte_edits};
use crate::doc::parser::{Document, Task};

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
        let t_orig = crate::timing::PhaseTimer::start("verify.to_bytes");
        let orig_bytes = self.to_bytes();
        t_orig.finish();
        self.apply_and_verify_with_orig_bytes(&orig_bytes, edits, intent)
    }

    /// Like [`Document::apply_and_verify`], reusing caller-supplied `orig_bytes` to avoid
    /// re-serializing the original [`LineDoc`].
    pub(crate) fn apply_and_verify_with_orig_bytes(
        &self,
        orig_bytes: &[u8],
        edits: &[LineEdit],
        intent: &TaskChangeIntent,
    ) -> Result<AppliedDocEdits, EditError> {
        let t_apply = crate::timing::PhaseTimer::start("verify.line_apply");
        let applied = self.line_doc.apply_edits(edits)?;
        t_apply.finish();
        let t_vb = crate::timing::PhaseTimer::start("verify.verify_bytes");
        verify_byte_edits(orig_bytes, &applied.bytes, edits)?;
        t_vb.finish();

        let t_reparse = crate::timing::PhaseTimer::start("verify.reparse");
        let edited_doc = Document::from_line_doc(applied.doc, self.id_config.clone());
        t_reparse.finish();
        let t_vt = crate::timing::PhaseTimer::start("verify.verify_tasks");
        verify_task_edits(self, &edited_doc, intent)?;
        t_vt.finish();

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

    // Store (first_index, occurrence_count) per TaskId without per-task heap allocations.
    let mut orig_by_id: FastU64Map<TaskId, (usize, usize)> =
        FastU64Map::with_capacity_and_hasher(orig.tasks.len(), Default::default());
    for (idx, task) in orig.tasks.iter().enumerate() {
        orig_by_id
            .entry(task.id)
            .and_modify(|e| e.1 += 1)
            .or_insert((idx, 1));
    }

    let mut edited_by_id: FastU64Map<TaskId, (usize, usize)> =
        FastU64Map::with_capacity_and_hasher(edited.tasks.len(), Default::default());
    for (idx, task) in edited.tasks.iter().enumerate() {
        edited_by_id
            .entry(task.id)
            .and_modify(|e| e.1 += 1)
            .or_insert((idx, 1));
    }

    // Check every original task.
    let mut untouched_orig_order = Vec::with_capacity(orig.tasks.len());
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

        let Some(&(edited_first_idx, edited_count)) = edited_by_id.get(&orig_task.id) else {
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
        let orig_count = orig_by_id.get(&orig_task.id).map_or(0, |&(_, c)| c);
        if orig_count != edited_count {
            return Err(EditError::VerificationFailed {
                reason: format!(
                    "occurrence count of untouched task {} changed",
                    orig.id_config.format_id(orig_task.id)
                ),
            });
        }
        if orig_count == 1
            && !tasks_semantically_equal(orig, orig_task, edited, &edited.tasks[edited_first_idx])
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
    let mut untouched_edited_order = Vec::with_capacity(edited.tasks.len());
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

fn tasks_semantically_equal(
    orig_doc: &Document,
    a: &Task,
    edited_doc: &Document,
    b: &Task,
) -> bool {
    a.id == b.id
        && a.title == b.title
        && orig_doc.task_section_name(a) == edited_doc.task_section_name(b)
        && a.status == b.status
        && a.status_raw == b.status_raw
        && a.owner == b.owner
        && a.claimed == b.claimed
        && a.priority == b.priority
        && a.difficulty == b.difficulty
        && a.task_type == b.task_type
        && a.area == b.area
        && a.why == b.why
        && a.files == b.files
        && a.problem == b.problem
        && a.fix == b.fix
        && a.accept == b.accept
        && a.commit == b.commit
        && a.depends.len() == b.depends.len()
        && a.depends
            .iter()
            .zip(&b.depends)
            .all(|(x, y)| x.id == y.id && x.raw == y.raw)
        && a.unknown_fields.len() == b.unknown_fields.len()
        && a.unknown_fields
            .iter()
            .zip(&b.unknown_fields)
            .all(|(x, y)| x.key == y.key && x.value == y.value)
        && a.notes.len() == b.notes.len()
        && a.notes
            .iter()
            .zip(&b.notes)
            .all(|(x, y)| x.date_raw == y.date_raw && x.agent == y.agent && x.text == y.text)
}
