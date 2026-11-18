//! Kinds of entry and the milestone tree (`docs/format.md` §16, §17; R-MILE-1, R-MILE-2, R-KIND-3).
//!
//! The tree is logical: an entry's `milestone` field names its parent, and only that field links
//! entries. Walks up the tree stop at a missing parent, at an entry that is not a milestone, and at
//! a cycle, so a damaged file never makes them loop.

use serde::Serialize;

use crate::doc::id::TaskId;
use crate::doc::parser::{Document, Status, Task};
use crate::error::{ErrorCode, Result, TodoError};

/// The kind of an entry, taken from its `type` (`docs/format.md` §16).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    /// The ordinary unit of work; handed out by `next`.
    Task,
    /// A piece of work with a goal, divided into tasks; never claimed.
    Milestone,
    /// A question; never handed out.
    Question,
    /// An idea; never handed out.
    Idea,
}

impl EntryKind {
    /// The kind for a `type` value. Matching ignores ASCII case, as the list filters do.
    #[must_use]
    pub fn from_type(task_type: Option<&str>) -> Self {
        match task_type {
            Some(t) if t.eq_ignore_ascii_case("milestone") => Self::Milestone,
            Some(t) if t.eq_ignore_ascii_case("question") => Self::Question,
            Some(t) if t.eq_ignore_ascii_case("idea") => Self::Idea,
            _ => Self::Task,
        }
    }

    /// The name used in messages and JSON.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Milestone => "milestone",
            Self::Question => "question",
            Self::Idea => "idea",
        }
    }

    /// Parses a comma-separated `--kind` list such as `milestone,task` (`R-KIND-3`). Matching ignores
    /// ASCII case. An unknown kind, or an empty list, is `BAD_VALUE`.
    pub fn parse_kind_list(raw: &str) -> Result<Vec<Self>> {
        let mut kinds: Vec<Self> = Vec::new();
        for part in raw.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            let kind = match part.to_ascii_lowercase().as_str() {
                "task" => Self::Task,
                "milestone" => Self::Milestone,
                "question" => Self::Question,
                "idea" => Self::Idea,
                _ => {
                    return Err(TodoError::new(
                        ErrorCode::BadValue,
                        format!("unknown kind `{part}`; use milestone, task, question or idea"),
                    ));
                }
            };
            if !kinds.contains(&kind) {
                kinds.push(kind);
            }
        }
        if kinds.is_empty() {
            return Err(TodoError::new(
                ErrorCode::BadValue,
                "--kind needs one or more of: milestone, task, question, idea",
            ));
        }
        Ok(kinds)
    }
}

/// Progress of a milestone: `done` tasks out of `total` tasks below it (`R-MILE-3`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Progress {
    /// Tasks below the milestone that are done.
    pub done: usize,
    /// Tasks below the milestone that are not dropped.
    pub total: usize,
}

impl Task {
    /// The kind of this entry (`docs/format.md` §16).
    #[must_use]
    pub fn kind(&self) -> EntryKind {
        EntryKind::from_type(self.task_type.as_deref())
    }

    /// Returns `true` for a milestone (`type: milestone`).
    #[must_use]
    pub fn is_milestone(&self) -> bool {
        self.kind() == EntryKind::Milestone
    }
}

impl Document {
    /// The ID named by `task`'s `milestone` field, if the field names a valid ID.
    #[must_use]
    pub fn milestone_parent_id(&self, task: &Task) -> Option<TaskId> {
        self.id_config.parse_user_id(task.milestone.as_deref()?)
    }

    /// The entry named by `task`'s `milestone` field, if this document holds it.
    #[must_use]
    pub fn milestone_parent(&self, task: &Task) -> Option<&Task> {
        self.task_by_id(self.milestone_parent_id(task)?)
    }

    /// The milestones above `task`, nearest first (`R-MILE-4`).
    ///
    /// The walk stops at a missing parent, at an entry that is not a milestone, and at a cycle.
    #[must_use]
    pub fn milestone_ancestors(&self, task: &Task) -> Vec<&Task> {
        let mut chain: Vec<&Task> = Vec::new();
        let mut current = task;
        while let Some(parent) = self.milestone_parent(current) {
            if !parent.is_milestone()
                || parent.id == task.id
                || chain.iter().any(|t| t.id == parent.id)
            {
                break;
            }
            chain.push(parent);
            current = parent;
        }
        chain
    }

    /// Every entry below milestone `id`, at any depth, in file order (`R-MILE-2`).
    #[must_use]
    pub fn entries_below(&self, id: TaskId) -> Vec<&Task> {
        self.tasks
            .iter()
            .filter(|t| t.id != id && self.milestone_ancestors(t).iter().any(|a| a.id == id))
            .collect()
    }

    /// The entries whose `milestone` field names `id` directly, in file order (`R-MILE-2`).
    #[must_use]
    pub fn children_of(&self, id: TaskId) -> Vec<&Task> {
        self.tasks
            .iter()
            .filter(|t| t.id != id && self.milestone_parent_id(t) == Some(id))
            .collect()
    }

    /// The entries below milestone `id` that are open, claimed or blocked (`R-MILE-2`).
    #[must_use]
    pub fn unfinished_below(&self, id: TaskId) -> Vec<&Task> {
        self.entries_below(id)
            .into_iter()
            .filter(|t| matches!(t.status, Status::Open | Status::Claimed | Status::Blocked))
            .collect()
    }

    /// The progress of milestone `id` (`R-MILE-3`): `done` counts the done tasks below it at any
    /// depth, `total` every task below it that isn't dropped. Milestones, questions and ideas are
    /// not counted.
    #[must_use]
    pub fn progress(&self, id: TaskId) -> Progress {
        let mut progress = Progress { done: 0, total: 0 };
        for task in self.entries_below(id) {
            if task.kind() != EntryKind::Task {
                continue;
            }
            match task.status {
                Status::Dropped => {}
                Status::Done => {
                    progress.done += 1;
                    progress.total += 1;
                }
                Status::Open | Status::Claimed | Status::Blocked => progress.total += 1,
            }
        }
        progress
    }
}
