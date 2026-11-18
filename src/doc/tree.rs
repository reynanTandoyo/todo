//! Kinds of entry and the milestone tree (`docs/format.md` §16, §17; R-MILE-1, R-MILE-2, R-KIND-3).
//!
//! The tree is logical: an entry's `milestone` field names its parent, and only that field links
//! entries. Walks up the tree stop at a missing parent, at an entry that is not a milestone, and at
//! a cycle, so a damaged file never makes them loop.

use std::collections::{BTreeMap, BTreeSet};

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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
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

    /// The index of this document's milestone tree (see [`MilestoneIndex`]), built in one pass over
    /// the entries. A loop over many milestones should build it once and ask it, not call the
    /// single-milestone methods below, which each build an index of their own.
    #[must_use]
    pub fn milestone_index(&self) -> MilestoneIndex<'_> {
        MilestoneIndex::new(self)
    }

    /// Every entry below milestone `id`, at any depth, in file order (`R-MILE-2`).
    #[must_use]
    pub fn entries_below(&self, id: TaskId) -> Vec<&Task> {
        self.milestone_index().entries_below(id).to_vec()
    }

    /// The entries whose `milestone` field names `id` directly, in file order (`R-MILE-2`).
    #[must_use]
    pub fn children_of(&self, id: TaskId) -> Vec<&Task> {
        self.milestone_index().children_of(id).to_vec()
    }

    /// The entries below milestone `id` that are open, claimed or blocked (`R-MILE-2`).
    #[must_use]
    pub fn unfinished_below(&self, id: TaskId) -> Vec<&Task> {
        self.milestone_index().unfinished_below(id)
    }

    /// The milestones with work started below them: something below at any depth is claimed or
    /// done (`R-MILE-6`). A milestone is never claimed, so a claimed entry here is a task.
    #[must_use]
    pub fn started_milestones(&self) -> BTreeSet<TaskId> {
        let mut started = BTreeSet::new();
        for entry in &self.tasks {
            if matches!(entry.status, Status::Claimed | Status::Done) {
                started.extend(self.milestone_ancestors(entry).iter().map(|m| m.id));
            }
        }
        started
    }

    /// Whether a milestone above `task` is one of `started` (`R-MILE-6`).
    #[must_use]
    pub fn above_started_milestone(&self, task: &Task, started: &BTreeSet<TaskId>) -> bool {
        self.milestone_ancestors(task)
            .iter()
            .any(|m| started.contains(&m.id))
    }

    /// Whether milestone `milestone` is ready to close (`R-MILE-6`): it is open or blocked, it has
    /// entries below it, and every one of them is done or dropped. Only `done` closes it; nothing
    /// closes it automatically.
    #[must_use]
    pub fn is_ready_to_close(&self, milestone: &Task) -> bool {
        self.milestone_index().is_ready_to_close(milestone)
    }

    /// The milestones above `task` that are ready to close, nearest first (`R-MILE-6`).
    #[must_use]
    pub fn ready_to_close_above(&self, task: &Task) -> Vec<&Task> {
        self.milestone_index().ready_to_close_above(task)
    }

    /// Every milestone that is ready to close, in file order (`summary`, `R-MILE-6`).
    #[must_use]
    pub fn milestones_ready_to_close(&self) -> Vec<&Task> {
        self.milestone_index().milestones_ready_to_close()
    }

    /// The progress of milestone `id` (`R-MILE-3`): `done` counts the done tasks below it at any
    /// depth, `total` every task below it that isn't dropped. Milestones, questions and ideas are
    /// not counted.
    #[must_use]
    pub fn progress(&self, id: TaskId) -> Progress {
        self.milestone_index().progress(id)
    }
}

/// The milestone tree of one document, indexed in one pass over its entries (T-125).
///
/// It holds the entries whose `milestone` names each ID, the entries below each milestone at any
/// depth, and the progress of each milestone, all filled by one walk up each entry's chain of
/// milestones. Its queries give the answers of the [`Document`] methods of the same names (which
/// are built on it), without a scan of every entry for each milestone asked about.
pub struct MilestoneIndex<'a> {
    doc: &'a Document,
    /// The entries whose `milestone` field names each ID directly, in file order.
    children: BTreeMap<TaskId, Vec<&'a Task>>,
    /// The entries below each milestone, at any depth, in file order.
    below: BTreeMap<TaskId, Vec<&'a Task>>,
    /// The progress of each milestone (`R-MILE-3`), counted in the same walk.
    progress: BTreeMap<TaskId, Progress>,
}

impl<'a> MilestoneIndex<'a> {
    /// Indexes `doc` in one pass: each entry is linked to its direct parent, and counted once for
    /// every milestone above it.
    fn new(doc: &'a Document) -> Self {
        let mut index = Self {
            doc,
            children: BTreeMap::new(),
            below: BTreeMap::new(),
            progress: BTreeMap::new(),
        };
        for task in &doc.tasks {
            // An entry that names itself is not its own child (`children_of` never lists it).
            if let Some(parent) = doc.milestone_parent_id(task)
                && parent != task.id
            {
                index.children.entry(parent).or_default().push(task);
            }
            // Only tasks count towards progress: milestones, questions and ideas do not (R-MILE-3).
            let counted = task.kind() == EntryKind::Task && task.status != Status::Dropped;
            let done = task.status == Status::Done;
            for milestone in doc.milestone_ancestors(task) {
                index.below.entry(milestone.id).or_default().push(task);
                if counted {
                    let count = index.progress.entry(milestone.id).or_default();
                    count.total += 1;
                    if done {
                        count.done += 1;
                    }
                }
            }
        }
        index
    }

    /// The entries whose `milestone` field names `id` directly, in file order (`R-MILE-2`).
    #[must_use]
    pub fn children_of(&self, id: TaskId) -> &[&'a Task] {
        match self.children.get(&id) {
            Some(list) => list.as_slice(),
            None => &[],
        }
    }

    /// Every entry below milestone `id`, at any depth, in file order (`R-MILE-2`).
    #[must_use]
    pub fn entries_below(&self, id: TaskId) -> &[&'a Task] {
        match self.below.get(&id) {
            Some(list) => list.as_slice(),
            None => &[],
        }
    }

    /// The entries below milestone `id` that are open, claimed or blocked (`R-MILE-2`).
    #[must_use]
    pub fn unfinished_below(&self, id: TaskId) -> Vec<&'a Task> {
        self.entries_below(id)
            .iter()
            .copied()
            .filter(|t| is_unfinished(t.status))
            .collect()
    }

    /// The progress of milestone `id` (`R-MILE-3`); zero for an ID with nothing below it.
    #[must_use]
    pub fn progress(&self, id: TaskId) -> Progress {
        self.progress.get(&id).copied().unwrap_or_default()
    }

    /// Whether milestone `milestone` is ready to close (`R-MILE-6`): it is open or blocked, it has
    /// entries below it, and every one of them is done or dropped.
    #[must_use]
    pub fn is_ready_to_close(&self, milestone: &Task) -> bool {
        let below = self.entries_below(milestone.id);
        milestone.is_milestone()
            && matches!(milestone.status, Status::Open | Status::Blocked)
            && !below.is_empty()
            && below.iter().all(|t| !is_unfinished(t.status))
    }

    /// The milestones above `task` that are ready to close, nearest first (`R-MILE-6`).
    #[must_use]
    pub fn ready_to_close_above(&self, task: &Task) -> Vec<&'a Task> {
        let doc = self.doc;
        doc.milestone_ancestors(task)
            .into_iter()
            .filter(|m| self.is_ready_to_close(m))
            .collect()
    }

    /// Every milestone that is ready to close, in file order (`summary`, `R-MILE-6`).
    #[must_use]
    pub fn milestones_ready_to_close(&self) -> Vec<&'a Task> {
        let doc = self.doc;
        doc.tasks
            .iter()
            .filter(|t| self.is_ready_to_close(t))
            .collect()
    }
}

/// Whether a status leaves an entry unfinished: open, claimed or blocked (`R-MILE-2`).
fn is_unfinished(status: Status) -> bool {
    matches!(status, Status::Open | Status::Claimed | Status::Blocked)
}

#[cfg(test)]
mod tests {
    use super::{EntryKind, Progress};
    use crate::doc::id::TaskId;
    use crate::doc::parser::{Document, Status, Task};

    /// A damaged tree (invented): milestones with work below them at two depths, a finished and an
    /// empty milestone, a cycle of milestones, an entry that names itself, a missing parent, a task
    /// used as a parent, and a question that names a milestone.
    const DAMAGED: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->
## Now

### T-001 · Top milestone
- status: open
- owner: —
- priority: P1 · type: milestone
- goal: the top goal

### T-002 · Middle milestone
- status: open
- owner: —
- priority: P1 · type: milestone
- milestone: T-001
- goal: the middle goal

### T-003 · Finished task
- status: done
- owner: —
- priority: P1
- milestone: T-002

### T-004 · Open task
- status: open
- owner: —
- priority: P2
- milestone: T-002

### T-005 · Dropped task
- status: dropped
- owner: —
- priority: P2
- milestone: T-001

### T-006 · Names itself
- status: open
- owner: —
- priority: P2
- milestone: T-006

### T-007 · Missing parent
- status: open
- owner: —
- priority: P2
- milestone: T-099

### T-008 · Parent is a task
- status: open
- owner: —
- priority: P2
- milestone: T-009

### T-009 · Plain task
- status: done
- owner: —
- priority: P3

### T-010 · Cycle A
- status: open
- owner: —
- priority: P1 · type: milestone
- milestone: T-011
- goal: the first goal of the cycle

### T-011 · Cycle B
- status: open
- owner: —
- priority: P1 · type: milestone
- milestone: T-010
- goal: the second goal of the cycle

### T-012 · Task in the cycle
- status: done
- owner: —
- priority: P2
- milestone: T-010

### T-013 · Question about the top
- status: open
- owner: —
- priority: P2 · type: question
- milestone: T-001

### T-014 · Empty milestone
- status: open
- owner: —
- priority: P1 · type: milestone
- goal: nothing below it yet

### T-015 · Finished milestone
- status: open
- owner: —
- priority: P1 · type: milestone
- goal: everything below it is finished

### T-016 · Done leaf
- status: done
- owner: —
- priority: P2
- milestone: T-015

### T-017 · Dropped leaf
- status: dropped
- owner: —
- priority: P2
- milestone: T-015
";

    fn ids<'a>(tasks: impl IntoIterator<Item = &'a Task>) -> Vec<TaskId> {
        tasks.into_iter().map(|t| t.id).collect()
    }

    /// The entries below `id` by a scan of every entry: the definition the index replaces (T-125).
    fn scan_below(doc: &Document, id: TaskId) -> Vec<TaskId> {
        ids(doc
            .tasks
            .iter()
            .filter(|t| t.id != id && doc.milestone_ancestors(t).iter().any(|a| a.id == id)))
    }

    fn scan_children(doc: &Document, id: TaskId) -> Vec<TaskId> {
        ids(doc
            .tasks
            .iter()
            .filter(|t| t.id != id && doc.milestone_parent_id(t) == Some(id)))
    }

    fn scan_progress(doc: &Document, id: TaskId) -> Progress {
        let mut progress = Progress::default();
        for task in doc
            .tasks
            .iter()
            .filter(|t| t.id != id && doc.milestone_ancestors(t).iter().any(|a| a.id == id))
        {
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

    fn scan_ready(doc: &Document, milestone: &Task) -> bool {
        let below = scan_below(doc, milestone.id);
        milestone.is_milestone()
            && matches!(milestone.status, Status::Open | Status::Blocked)
            && !below.is_empty()
            && doc
                .tasks
                .iter()
                .filter(|t| below.contains(&t.id))
                .all(|t| matches!(t.status, Status::Done | Status::Dropped))
    }

    #[test]
    fn index_answers_as_a_scan_of_every_entry_does() {
        let doc = Document::parse_bytes(DAMAGED.as_bytes()).unwrap();
        let index = doc.milestone_index();
        for task in &doc.tasks {
            let id = task.id;
            assert_eq!(
                ids(index.entries_below(id).iter().copied()),
                scan_below(&doc, id),
                "entries below {id:?}"
            );
            assert_eq!(
                ids(index.children_of(id).iter().copied()),
                scan_children(&doc, id),
                "children of {id:?}"
            );
            assert_eq!(
                index.progress(id),
                scan_progress(&doc, id),
                "progress of {id:?}"
            );
            assert_eq!(
                index.is_ready_to_close(task),
                scan_ready(&doc, task),
                "ready to close {id:?}"
            );
            let expected_above: Vec<TaskId> = doc
                .milestone_ancestors(task)
                .into_iter()
                .filter(|m| scan_ready(&doc, m))
                .map(|m| m.id)
                .collect();
            assert_eq!(
                ids(index.ready_to_close_above(task)),
                expected_above,
                "ready to close above {id:?}"
            );
        }
        let expected_ready: Vec<TaskId> = doc
            .tasks
            .iter()
            .filter(|t| scan_ready(&doc, t))
            .map(|t| t.id)
            .collect();
        assert_eq!(ids(index.milestones_ready_to_close()), expected_ready);
    }

    #[test]
    fn index_gives_the_expected_answers_on_the_damaged_tree() {
        let doc = Document::parse_bytes(DAMAGED.as_bytes()).unwrap();
        let index = doc.milestone_index();
        // Work below the middle milestone: one done, one open.
        assert_eq!(index.progress(TaskId(2)), Progress { done: 1, total: 2 });
        // The top milestone counts the same two tasks: the dropped one and the question do not.
        assert_eq!(index.progress(TaskId(1)), Progress { done: 1, total: 2 });
        assert_eq!(index.progress(TaskId(15)), Progress { done: 1, total: 1 });
        assert_eq!(index.progress(TaskId(14)), Progress::default());
        // Only the finished milestone is ready to close: the empty one and the cycle are not.
        assert_eq!(ids(index.milestones_ready_to_close()), vec![TaskId(15)]);
        // A task that names a task as its parent is listed under that task, as before.
        assert_eq!(
            ids(index.children_of(TaskId(9)).iter().copied()),
            vec![TaskId(8)]
        );
        // An entry that names itself is not its own child and has nothing below it.
        assert!(index.children_of(TaskId(6)).is_empty());
        assert!(index.entries_below(TaskId(6)).is_empty());
    }

    #[test]
    fn reserved_types_set_the_kind_ignoring_case() {
        assert_eq!(EntryKind::from_type(None), EntryKind::Task);
        assert_eq!(EntryKind::from_type(Some("feature")), EntryKind::Task);
        assert_eq!(
            EntryKind::from_type(Some("milestone")),
            EntryKind::Milestone
        );
        assert_eq!(EntryKind::from_type(Some("Question")), EntryKind::Question);
        assert_eq!(EntryKind::from_type(Some("IDEA")), EntryKind::Idea);
    }
}
