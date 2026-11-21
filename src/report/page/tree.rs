//! The work tree of the v1.0 page: every task and milestone of one document in its place, with the
//! overall counts (T-212; R-REPORT-1, "Overall line", "All work" and "More"; section 5.23).
//!
//! [`build_tree`] places every task and milestone of a document whatever its state, so finished
//! work stays where it was. Each entry is read through [`entry_view`], so it reads the same as on
//! the rest of the page. [`build_earlier`] builds the same tree from `TODO-archive.md` for the
//! closed part "Earlier work". Read-only: nothing here changes a document.

use std::collections::BTreeSet;

use chrono::NaiveDate;

use super::entry::{Ctx, EntryView, State, entry_view};
use crate::doc::{Document, EntryKind, MilestoneIndex, Status, Task, TaskId};

/// The em dash that separates a note's words from its text (`docs/format.md` §9).
const DASH: char = '\u{2014}';

/// How many tasks are in each state (R-REPORT-1, "Overall line").
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    /// Tasks that are done.
    pub done: usize,
    /// Tasks in progress (claimed).
    pub in_progress: usize,
    /// Tasks waiting (blocked, or open with an unanswered question).
    pub waiting: usize,
    /// Tasks to do (open).
    pub to_do: usize,
    /// Tasks cancelled (dropped).
    pub cancelled: usize,
}

impl Counts {
    /// The M of "N of M tasks done": every counted task except the cancelled ones.
    #[must_use]
    pub const fn total_counted(&self) -> usize {
        self.done + self.in_progress + self.waiting + self.to_do
    }

    fn record(&mut self, state: State) {
        match state {
            State::Done => self.done += 1,
            State::InProgress => self.in_progress += 1,
            State::Waiting => self.waiting += 1,
            State::ToDo => self.to_do += 1,
            State::Cancelled => self.cancelled += 1,
        }
    }
}

/// The overall line: the counts over every task of a document (R-REPORT-1, "Overall line").
/// Milestones, questions and ideas are not tasks and are not counted (R-KIND-3).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Overall {
    pub counts: Counts,
}

impl Overall {
    /// The overall line of `doc`, with each task's state read through `ctx`.
    #[must_use]
    pub fn of(doc: &Document, ctx: &Ctx<'_>) -> Self {
        let mut counts = Counts::default();
        for task in doc.tasks.iter().filter(|t| t.kind() == EntryKind::Task) {
            counts.record(entry_view(task, ctx).state);
        }
        Self { counts }
    }
}

/// The state of a milestone (R-REPORT-1, "All work").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MilestoneState {
    /// No task below it is done, in progress or waiting.
    NotStarted,
    /// A task below it is done, in progress or waiting, and it is not finished or cancelled itself.
    InProgress,
    /// Done: the date of its latest `done` note, else of its latest note. `None` when it has no note.
    Finished(Option<NaiveDate>),
    /// Dropped: the date of its latest `dropped` note, else of its latest note. `None` when it has
    /// no note.
    Cancelled(Option<NaiveDate>),
}

/// A milestone in the work tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MilestoneView {
    /// The canonical ID (`T-007`).
    pub id: String,
    pub title: String,
    /// `goal`, shown when the milestone is opened; `None` when it is empty.
    pub goal: Option<String>,
    /// `summary`, shown with the goal; `None` when it is empty.
    pub summary: Option<String>,
    pub state: MilestoneState,
    /// The tasks below it, at any depth, that are done.
    pub done: usize,
    /// The tasks below it, at any depth, that are not cancelled.
    pub total: usize,
    /// The milestones directly below it, in file order.
    pub milestones: Vec<MilestoneView>,
    /// The tasks directly below it, in file order.
    pub tasks: Vec<EntryView>,
}

/// The work tree of one document (R-REPORT-1, "All work").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tree {
    /// The top-level milestones: in progress first, then not started, then finished and cancelled,
    /// with file order inside each group.
    pub milestones: Vec<MilestoneView>,
    /// The tasks below no milestone, in file order ("Other work").
    pub other: Vec<EntryView>,
    /// The counts over every task of the document.
    pub counts: Counts,
}

/// The closed part "Earlier work" (R-REPORT-1, "More"): the same tree, built from `TODO-archive.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Earlier {
    pub tree: Tree,
    /// The top-level milestones of the archive.
    pub finished_milestones: usize,
    /// The tasks of the archive, in every state.
    pub tasks: usize,
}

/// The work tree of `doc`: every task and milestone in its place, whatever its state (R-REPORT-1).
#[must_use]
pub fn build_tree(doc: &Document, ctx: &Ctx<'_>) -> Tree {
    let index = doc.milestone_index();
    let mut visited = BTreeSet::new();
    let mut milestones = Vec::new();
    let mut other = Vec::new();

    // The top level: milestones with no milestone above them, each with what is below it.
    for task in doc
        .tasks
        .iter()
        .filter(|t| t.is_milestone() && doc.milestone_ancestors(t).is_empty())
    {
        milestones.push(milestone_view(task, ctx, &index, &mut visited));
    }
    // Other work: tasks with no milestone above them.
    for task in doc
        .tasks
        .iter()
        .filter(|t| t.kind() == EntryKind::Task && doc.milestone_ancestors(t).is_empty())
    {
        visited.insert(task.id);
        other.push(entry_view(task, ctx));
    }
    // A milestone in a cycle has a milestone above it, so no walk from the top reaches it. It is
    // shown at the top rather than hidden, walked from the farthest milestone above it, so that
    // each cycle is shown whole.
    for task in doc.tasks.iter().filter(|t| t.is_milestone()) {
        let root = doc
            .milestone_ancestors(task)
            .last()
            .copied()
            .unwrap_or(task);
        if !visited.contains(&root.id) {
            milestones.push(milestone_view(root, ctx, &index, &mut visited));
        }
        if !visited.contains(&task.id) {
            milestones.push(milestone_view(task, ctx, &index, &mut visited));
        }
    }
    // Every task is reached above; one that no walk reached is shown as Other work, not hidden.
    let stray: Vec<&Task> = doc
        .tasks
        .iter()
        .filter(|t| t.kind() == EntryKind::Task && !visited.contains(&t.id))
        .collect();
    for task in stray {
        visited.insert(task.id);
        other.push(entry_view(task, ctx));
    }

    milestones.sort_by_key(|milestone| group(milestone.state));
    Tree {
        milestones,
        other,
        counts: Overall::of(doc, ctx).counts,
    }
}

/// The earlier work of `archive`: the same tree, with its top-level milestones and its tasks
/// counted (R-REPORT-1, "More").
#[must_use]
pub fn build_earlier(archive: &Document, ctx: &Ctx<'_>) -> Earlier {
    let tree = build_tree(archive, ctx);
    Earlier {
        finished_milestones: tree.milestones.len(),
        tasks: archive
            .tasks
            .iter()
            .filter(|t| t.kind() == EntryKind::Task)
            .count(),
        tree,
    }
}

/// The group of a top-level milestone: in progress, then not started, then finished and cancelled.
fn group(state: MilestoneState) -> u8 {
    match state {
        MilestoneState::InProgress => 0,
        MilestoneState::NotStarted => 1,
        MilestoneState::Finished(_) | MilestoneState::Cancelled(_) => 2,
    }
}

/// The view of milestone `task`: the milestones and tasks below it, and its counts and state. The
/// walk marks every entry it shows in `visited`, so a cycle of milestones cannot loop.
fn milestone_view(
    task: &Task,
    ctx: &Ctx<'_>,
    index: &MilestoneIndex<'_>,
    visited: &mut BTreeSet<TaskId>,
) -> MilestoneView {
    visited.insert(task.id);
    let mut milestones = Vec::new();
    let mut tasks = Vec::new();
    for child in index.children_of(task.id).iter().copied() {
        if visited.contains(&child.id) {
            continue;
        }
        match child.kind() {
            EntryKind::Milestone => milestones.push(milestone_view(child, ctx, index, visited)),
            EntryKind::Task => {
                visited.insert(child.id);
                tasks.push(entry_view(child, ctx));
            }
            EntryKind::Question | EntryKind::Idea => {}
        }
    }

    let below = Below::of(&milestones, &tasks);
    let state = match task.status {
        Status::Done => MilestoneState::Finished(closing_date(task, "done")),
        Status::Dropped => MilestoneState::Cancelled(closing_date(task, "dropped")),
        Status::Open | Status::Claimed | Status::Blocked if below.under_way => {
            MilestoneState::InProgress
        }
        Status::Open | Status::Claimed | Status::Blocked => MilestoneState::NotStarted,
    };
    MilestoneView {
        id: ctx.cfg.id_config.format_id(task.id),
        title: task.title.clone(),
        goal: text_of(task.goal.as_deref()),
        summary: text_of(task.summary.as_deref()),
        state,
        done: below.done,
        total: below.total,
        milestones,
        tasks,
    }
}

/// What the tasks below a milestone add up to: its "N of M" and whether work has started below it.
#[derive(Debug, Default)]
struct Below {
    done: usize,
    total: usize,
    under_way: bool,
}

impl Below {
    fn of(milestones: &[MilestoneView], tasks: &[EntryView]) -> Self {
        let mut below = Self::default();
        below.add(milestones, tasks);
        below
    }

    /// Adds `tasks`, and then the tasks below `milestones` at any depth.
    fn add(&mut self, milestones: &[MilestoneView], tasks: &[EntryView]) {
        for task in tasks {
            if task.state != State::Cancelled {
                self.total += 1;
            }
            if task.state == State::Done {
                self.done += 1;
            }
            if matches!(task.state, State::Done | State::InProgress | State::Waiting) {
                self.under_way = true;
            }
        }
        for milestone in milestones {
            self.add(&milestone.milestones, &milestone.tasks);
        }
    }
}

/// The date of the latest note of `task` that starts with `word`; when there is none, the date of
/// its latest note. `None` when `task` has no note.
fn closing_date(task: &Task, word: &str) -> Option<NaiveDate> {
    task.notes
        .iter()
        .rev()
        .find(|note| starts_with_word(&note.text, word))
        .or_else(|| task.notes.last())
        .map(|note| note.date)
}

/// Whether a note's text starts with `word`: its first word, before any dash, is `word`. The same
/// reading as the entry views use (`entry.rs`).
fn starts_with_word(text: &str, word: &str) -> bool {
    let before_dash = text.split(DASH).next().unwrap_or_default();
    before_dash.split_whitespace().next() == Some(word)
}

/// The value without its surrounding spaces, or `None` when it is empty.
fn text_of(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, FixedOffset};

    use super::*;
    use crate::ops::OpsConfig;

    /// The clock of every test: 21 Nov 2026, 15:00, in the +01:00 offset.
    const NOW: &str = "2026-11-21T15:00:00+01:00";

    fn day(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
    }

    /// The lines of a fixture as one text, with a final newline.
    fn text(lines: &[&str]) -> String {
        let mut text = lines.join("\n");
        text.push('\n');
        text
    }

    /// Runs `check` on the invented `fixture`, with a context read at [`NOW`] and no archive.
    fn with_doc<R>(fixture: &str, check: impl FnOnce(&Document, &Ctx<'_>) -> R) -> R {
        let doc = Document::parse_bytes(fixture.as_bytes()).unwrap();
        let cfg = OpsConfig::default();
        let now = DateTime::<FixedOffset>::parse_from_rfc3339(NOW).unwrap();
        let ctx = Ctx {
            doc: &doc,
            archive: None,
            cfg: &cfg,
            now,
        };
        check(&doc, &ctx)
    }

    fn tree_of(fixture: &str) -> Tree {
        with_doc(fixture, build_tree)
    }

    /// The IDs of the tree in the order the page walks it: each milestone, then what is below it,
    /// then Other work.
    fn walk_ids(milestones: &[MilestoneView], tasks: &[EntryView], out: &mut Vec<String>) {
        for milestone in milestones {
            out.push(milestone.id.clone());
            walk_ids(&milestone.milestones, &milestone.tasks, out);
        }
        out.extend(tasks.iter().map(|task| task.id.clone()));
    }

    fn ids(tree: &Tree) -> Vec<String> {
        let mut out = Vec::new();
        walk_ids(&tree.milestones, &tree.other, &mut out);
        out
    }

    /// The milestone with `id`, at any depth.
    fn find<'a>(milestones: &'a [MilestoneView], id: &str) -> Option<&'a MilestoneView> {
        milestones.iter().find_map(|m| {
            (m.id == id)
                .then_some(m)
                .or_else(|| find(&m.milestones, id))
        })
    }

    #[test]
    fn overall_counts_tasks_by_state_and_leaves_out_milestones_questions_and_ideas() {
        let fixture = text(&[
            "### T-001 · Keep the light",
            "- status: open",
            "- type: milestone",
            "### T-002 · Trim the wick",
            "- status: done",
            "- milestone: T-001",
            "- note (2026-11-18, bob): done — trimmed",
            "### T-003 · Fill the oil",
            "- status: claimed",
            "- owner: keeper-1",
            "- claimed: 2026-11-21T09:00+01:00",
            "- milestone: T-001",
            "### T-004 · Clean the glass",
            "- status: blocked",
            "- note (2026-11-19, bob): blocked — waiting for glass",
            "### T-005 · Paint the door",
            "- status: open",
            "### T-006 · Old ladder",
            "- status: dropped",
            "- note (2026-11-20, bob): dropped — ladder sold",
            "### T-007 · Log the hours",
            "- status: open",
            "- depends: T-008",
            "### T-008 · Who writes the log?",
            "- status: open",
            "- type: question",
            "### T-009 · A bigger lamp",
            "- status: open",
            "- type: idea",
            "### T-010 · Finished shelter",
            "- status: done",
            "- type: milestone",
        ]);
        let counts = with_doc(&fixture, |doc, ctx| Overall::of(doc, ctx).counts);
        assert_eq!(
            counts,
            Counts {
                done: 1,
                in_progress: 1,
                waiting: 2,
                to_do: 1,
                cancelled: 1,
            }
        );
        // The cancelled task leaves out of the M of "N of M tasks done".
        assert_eq!(counts.total_counted(), 5);

        let tree = tree_of(&fixture);
        assert_eq!(tree.counts, counts);
        // Every task and milestone is in the tree, cancelled and done ones included; the question
        // and the idea are not.
        assert_eq!(
            ids(&tree),
            [
                "T-001", "T-002", "T-003", "T-010", "T-004", "T-005", "T-006", "T-007"
            ]
        );
    }

    #[test]
    fn a_milestone_counts_done_of_total_over_nested_milestones_and_leaves_cancelled_out() {
        let fixture = text(&[
            "### T-001 · Top",
            "- status: open",
            "- type: milestone",
            "- goal: the top goal",
            "### T-002 · Shipped part",
            "- status: done",
            "- milestone: T-001",
            "- note (2026-11-18, bob): done — shipped",
            "### T-003 · Inner",
            "- status: open",
            "- type: milestone",
            "- milestone: T-001",
            "### T-004 · Inner done",
            "- status: done",
            "- milestone: T-003",
            "### T-005 · Inner open",
            "- status: open",
            "- milestone: T-003",
            "### T-006 · Inner dropped",
            "- status: dropped",
            "- milestone: T-003",
            "### T-007 · A question under the top",
            "- status: open",
            "- type: question",
            "- milestone: T-001",
        ]);
        let tree = tree_of(&fixture);
        let top = find(&tree.milestones, "T-001").unwrap();
        // T-002, T-004 and T-005 count, at any depth; the cancelled T-006 counts in neither number.
        assert_eq!((top.done, top.total), (2, 3));
        assert_eq!(top.goal.as_deref(), Some("the top goal"));
        assert_eq!(top.state, MilestoneState::InProgress);
        let inner = find(&tree.milestones, "T-003").unwrap();
        assert_eq!((inner.done, inner.total), (1, 2));
        assert_eq!(top.milestones.len(), 1);
        assert!(!ids(&tree).contains(&"T-007".to_string()));
    }

    #[test]
    fn each_milestone_state_is_read_with_its_date() {
        let fixture = text(&[
            "### T-001 · Not begun",
            "- status: open",
            "- type: milestone",
            "- note (2026-11-10, bob): added",
            "### T-002 · Open work",
            "- status: open",
            "- milestone: T-001",
            "### T-003 · Begun",
            "- status: open",
            "- type: milestone",
            "### T-004 · Claimed work",
            "- status: claimed",
            "- owner: keeper-1",
            "- claimed: 2026-11-21T09:00+01:00",
            "- milestone: T-003",
            "### T-005 · Finished by a done note",
            "- status: done",
            "- type: milestone",
            "- note (2026-11-18, bob): added",
            "- note (2026-11-20, bob): done — all in",
            "- note (2026-11-21, bob): added",
            "### T-006 · Finished by its latest note",
            "- status: done",
            "- type: milestone",
            "- note (2026-11-18, bob): added",
            "- note (2026-11-19, bob): added",
            "### T-007 · Finished without a note",
            "- status: done",
            "- type: milestone",
            "### T-008 · Cancelled",
            "- status: dropped",
            "- type: milestone",
            "- note (2026-11-19, bob): dropped — the light is not needed",
        ]);
        let tree = tree_of(&fixture);
        let state = |id: &str| find(&tree.milestones, id).unwrap().state;
        assert_eq!(state("T-001"), MilestoneState::NotStarted);
        assert_eq!(state("T-003"), MilestoneState::InProgress);
        // The latest done note gives the date, even when a later note is not a done one.
        assert_eq!(state("T-005"), MilestoneState::Finished(Some(day(20))));
        assert_eq!(state("T-006"), MilestoneState::Finished(Some(day(19))));
        assert_eq!(state("T-007"), MilestoneState::Finished(None));
        assert_eq!(state("T-008"), MilestoneState::Cancelled(Some(day(19))));
    }

    #[test]
    fn top_level_milestones_are_in_progress_then_not_started_then_finished_and_cancelled() {
        let fixture = text(&[
            "### T-001 · Not started, first",
            "- status: open",
            "- type: milestone",
            "### T-002 · Finished, second",
            "- status: done",
            "- type: milestone",
            "### T-003 · In progress, third",
            "- status: open",
            "- type: milestone",
            "### T-004 · Claimed under the third",
            "- status: claimed",
            "- owner: keeper-1",
            "- claimed: 2026-11-21T09:00+01:00",
            "- milestone: T-003",
            "### T-005 · Cancelled, fifth",
            "- status: dropped",
            "- type: milestone",
            "### T-006 · Not started, sixth",
            "- status: open",
            "- type: milestone",
            "### T-007 · In progress, seventh",
            "- status: open",
            "- type: milestone",
            "### T-008 · Done under the seventh",
            "- status: done",
            "- milestone: T-007",
        ]);
        let tree = tree_of(&fixture);
        let order: Vec<&str> = tree.milestones.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(
            order,
            ["T-003", "T-007", "T-001", "T-006", "T-002", "T-005"]
        );
    }

    #[test]
    fn nested_milestones_come_before_tasks_and_each_group_keeps_file_order() {
        let fixture = text(&[
            "### T-001 · Parent",
            "- status: open",
            "- type: milestone",
            "### T-002 · First task",
            "- status: open",
            "- milestone: T-001",
            "### T-003 · Inner one",
            "- status: open",
            "- type: milestone",
            "- milestone: T-001",
            "### T-004 · Second task",
            "- status: open",
            "- milestone: T-001",
            "### T-005 · Inner two",
            "- status: open",
            "- type: milestone",
            "- milestone: T-001",
            "### T-006 · Task inside one",
            "- status: open",
            "- milestone: T-003",
        ]);
        let tree = tree_of(&fixture);
        let parent = find(&tree.milestones, "T-001").unwrap();
        let nested: Vec<&str> = parent.milestones.iter().map(|m| m.id.as_str()).collect();
        let tasks: Vec<&str> = parent.tasks.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(nested, ["T-003", "T-005"]);
        assert_eq!(tasks, ["T-002", "T-004"]);
        assert_eq!(
            ids(&tree),
            ["T-001", "T-003", "T-006", "T-005", "T-002", "T-004"]
        );
    }

    #[test]
    fn tasks_below_no_milestone_are_other_work_in_file_order() {
        let fixture = text(&[
            "### T-001 · Loose task",
            "- status: open",
            "### T-002 · Names a missing milestone",
            "- status: done",
            "- milestone: T-099",
            "### T-003 · Names a task, not a milestone",
            "- status: open",
            "- milestone: T-001",
            "### T-004 · A milestone with nothing below",
            "- status: open",
            "- type: milestone",
            "### T-005 · A question",
            "- status: open",
            "- type: question",
            "### T-006 · An idea",
            "- status: open",
            "- type: idea",
        ]);
        let tree = tree_of(&fixture);
        let other: Vec<&str> = tree.other.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(other, ["T-001", "T-002", "T-003"]);
        let empty = &tree.milestones[0];
        assert_eq!(empty.id, "T-004");
        assert_eq!(empty.state, MilestoneState::NotStarted);
        assert_eq!((empty.done, empty.total), (0, 0));
        assert!(empty.tasks.is_empty());
    }

    #[test]
    fn a_milestone_cycle_loses_no_entry() {
        let fixture = text(&[
            "### T-010 · Cycle A",
            "- status: open",
            "- type: milestone",
            "- milestone: T-011",
            "### T-011 · Cycle B",
            "- status: open",
            "- type: milestone",
            "- milestone: T-010",
            "### T-012 · Task in the cycle",
            "- status: done",
            "- milestone: T-010",
            "### T-013 · Milestone under the cycle",
            "- status: open",
            "- type: milestone",
            "- milestone: T-011",
            "### T-014 · Task under that milestone",
            "- status: open",
            "- milestone: T-013",
        ]);
        let tree = tree_of(&fixture);
        let mut all = ids(&tree);
        all.sort();
        assert_eq!(all, ["T-010", "T-011", "T-012", "T-013", "T-014"]);
        assert!(tree.other.is_empty());
    }

    #[test]
    fn earlier_work_is_the_same_tree_from_the_archive_with_its_numbers() {
        let fixture = text(&[
            "### T-001 · Old milestone",
            "- status: done",
            "- type: milestone",
            "- note (2026-11-02, bob): done — finished",
            "### T-002 · Old task under it",
            "- status: done",
            "- milestone: T-001",
            "### T-003 · Old dropped task under it",
            "- status: dropped",
            "- milestone: T-001",
            "- note (2026-11-03, bob): dropped — not needed",
            "### T-004 · Loose old task",
            "- status: done",
            "- note (2026-11-04, bob): done — shipped",
            "### T-005 · Old loose dropped",
            "- status: dropped",
            "- note (2026-11-05, bob): dropped — gone",
        ]);
        let earlier = with_doc(&fixture, build_earlier);
        assert_eq!(earlier.finished_milestones, 1);
        assert_eq!(earlier.tasks, 4);
        let old = find(&earlier.tree.milestones, "T-001").unwrap();
        assert_eq!(old.state, MilestoneState::Finished(Some(day(2))));
        assert_eq!((old.done, old.total), (1, 1));
        let other: Vec<&str> = earlier.tree.other.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(other, ["T-004", "T-005"]);
        assert_eq!(
            earlier.tree.counts,
            Counts {
                done: 2,
                cancelled: 2,
                ..Counts::default()
            }
        );
    }

    #[test]
    fn a_document_with_no_entries_has_an_empty_tree() {
        let tree = tree_of("# Nothing yet\n");
        assert!(tree.milestones.is_empty());
        assert!(tree.other.is_empty());
        assert_eq!(tree.counts, Counts::default());
        assert_eq!(Counts::default().total_counted(), 0);
    }
}
