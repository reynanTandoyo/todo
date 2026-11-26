//! The chain of a task (T-228; R-REPORT-7): the tasks it waits for and the tasks that wait for it.

use std::collections::HashMap;

use super::entry::{Ctx, EntryView, State, entry_view};
use super::tree::{MilestoneState, MilestoneView, Tree};
use crate::doc::{Document, EntryKind};

/// The most paths a group (the paths before the task, or after it) shows, one line each (R-REPORT-7).
pub const MAX_LINES: usize = 10;

/// The most steps a path shows. A longer path is cut and keeps the steps nearest the task
/// (R-REPORT-7).
pub const MAX_STEPS: usize = 12;

/// How many paths beyond the shown lines are counted, so that a count of `MAX_COUNTED` or more
/// means more than 100 (R-REPORT-7). The walk stops once `MAX_LINES + MAX_COUNTED` paths are found.
pub const MAX_COUNTED: usize = 101;

/// The state of one step of a chain: an entry's state in words, a milestone's state as the tree
/// shows it, or no entry at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepState {
    /// A task, question or idea, in one of the five states.
    Task(State),
    /// A milestone, in the state the tree shows it.
    Milestone(MilestoneState),
    /// An ID that names no entry of `TODO.md` or of the archive.
    NotFound,
}

/// One step of a chain: an entry, by its canonical ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// The canonical ID (`T-005`).
    pub id: String,
    /// The entry's title; empty when the step is not found.
    pub title: String,
    pub state: StepState,
    /// Whether the entry is in `TODO-archive.md`.
    pub archived: bool,
}

/// How a path through `depends` ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathEnd {
    /// The path reached its natural end: its first step waits for nothing, or its last step is
    /// waited for by nothing.
    Open,
    /// The path came back to a step already on it; the string is the ID it came back to.
    Loop(String),
}

/// One path through `depends`, with its steps in the order they are read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    pub steps: Vec<Step>,
    /// True when steps far from the task were left out, so the path starts with "…".
    pub cut: bool,
    pub end: PathEnd,
}

/// The chain of one task: the paths before it and the paths after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chain {
    /// The paths through `depends` that lead to the task.
    pub before: Vec<Path>,
    /// Paths before the task that are not shown.
    pub more_before: usize,
    /// The paths through the entries that wait for the task.
    pub after: Vec<Path>,
    /// Paths after the task that are not shown.
    pub more_after: usize,
}

/// Every entry of `TODO.md` and of the archive, by canonical ID, with the entries that name each ID
/// in `depends`. It is built once for a page and kept in memory: nothing is written to disk.
#[derive(Debug, Default)]
pub struct ChainIndex {
    nodes: HashMap<String, Node>,
    /// For each ID, the IDs of the entries whose `depends` name it, in file order.
    dependents: HashMap<String, Vec<String>>,
}

/// What the index keeps of one entry.
#[derive(Debug, Clone)]
struct Node {
    title: String,
    archived: bool,
    milestone: bool,
    state: State,
    /// The state of a milestone as the tree shows it; set by [`ChainIndex::set_milestone_states`].
    milestone_state: Option<MilestoneState>,
    /// The canonical IDs of its `depends`, in the order written.
    depends: Vec<String>,
}

impl ChainIndex {
    /// The index of `doc` and of its archive. `TODO.md` comes first: an ID that both files hold keeps
    /// the entry of `TODO.md`. Each entry's state is read through `ctx` as the page reads it.
    #[must_use]
    pub fn new(doc: &Document, archive: Option<&Document>, ctx: &Ctx<'_>) -> Self {
        let mut index = Self::default();
        let files = std::iter::once((doc, false)).chain(archive.map(|archive| (archive, true)));
        for (file, archived) in files {
            for task in &file.tasks {
                let id = ctx.cfg.id_config.format_id(task.id);
                if index.nodes.contains_key(&id) {
                    continue;
                }
                let view = entry_view(task, ctx);
                let depends = task
                    .depends
                    .iter()
                    .map(|dep| match dep.id {
                        Some(dep_id) => ctx.cfg.id_config.format_id(dep_id),
                        None => dep.raw.clone(),
                    })
                    .collect();
                index.nodes.insert(
                    id.clone(),
                    Node {
                        title: view.title,
                        archived,
                        milestone: task.kind() == EntryKind::Milestone,
                        state: view.state,
                        milestone_state: None,
                        depends,
                    },
                );
                index.link_dependents(&id);
            }
        }
        index
    }

    /// Sets the state of every milestone of `tree` on its entry, at any depth, so a milestone step
    /// shows the state the tree shows.
    pub fn set_milestone_states(&mut self, tree: &Tree) {
        set_states(&mut self.nodes, &tree.milestones);
    }

    /// The step of `id`: a known entry gives its title, archived flag and state (a milestone with a
    /// tree state gives that state); an ID that names no entry is [`StepState::NotFound`].
    #[must_use]
    pub fn step(&self, id: &str) -> Step {
        let Some(node) = self.nodes.get(id) else {
            return Step {
                id: id.to_string(),
                title: String::new(),
                state: StepState::NotFound,
                archived: false,
            };
        };
        let state = match (node.milestone, node.milestone_state) {
            (true, Some(milestone_state)) => StepState::Milestone(milestone_state),
            _ => StepState::Task(node.state),
        };
        Step {
            id: id.to_string(),
            title: node.title.clone(),
            state,
            archived: node.archived,
        }
    }

    /// The chain of the task `id` (a canonical ID): the paths through `depends` that lead to it and
    /// the paths through `dependents` that lead away from it, each within the limits of R-REPORT-7.
    /// `None` when `id` names no entry, or when it has no `depends` and nothing depends on it.
    #[must_use]
    pub fn chain_of(&self, id: &str) -> Option<Chain> {
        let node = self.nodes.get(id)?;
        if node.depends.is_empty() && self.neighbors(id, Direction::After).is_empty() {
            return None;
        }
        let (before, before_count) = self.paths_from(id, Direction::Before);
        let (after, after_count) = self.paths_from(id, Direction::After);
        Some(Chain {
            more_before: before_count.saturating_sub(before.len()),
            before,
            more_after: after_count.saturating_sub(after.len()),
            after,
        })
    }

    /// The first `MAX_LINES` paths of one direction from `id`, and how many paths there are in all.
    fn paths_from(&self, id: &str, direction: Direction) -> (Vec<Path>, usize) {
        let mut walk = Walk {
            index: self,
            task: id,
            direction,
            lines: Vec::new(),
            count: 0,
        };
        walk.visit(id, &mut Vec::new());
        (walk.lines, walk.count)
    }

    /// The IDs one step from `id` in `direction`: its `depends`, or the entries whose `depends` name
    /// it. Empty for an ID that names no entry.
    fn neighbors(&self, id: &str, direction: Direction) -> &[String] {
        match direction {
            Direction::Before => match self.nodes.get(id) {
                Some(node) => node.depends.as_slice(),
                None => &[],
            },
            Direction::After => match self.dependents.get(id) {
                Some(waiting) => waiting.as_slice(),
                None => &[],
            },
        }
    }

    /// Adds `id` to the dependents of each ID that its node depends on, once per ID.
    fn link_dependents(&mut self, id: &str) {
        let Some(node) = self.nodes.get(id) else {
            return;
        };
        for dep in &node.depends {
            let waiting = self.dependents.entry(dep.clone()).or_default();
            if !waiting.iter().any(|known| known == id) {
                waiting.push(id.to_string());
            }
        }
    }
}

/// Sets the chain of every task of `tree`, in its other work and below its milestones at any depth,
/// from `index` (R-REPORT-7). Milestones have no chain.
pub fn fill_chains(tree: &mut Tree, index: &ChainIndex) {
    fill_in(&mut tree.other, index);
    fill_below(&mut tree.milestones, index);
}

/// Sets the chain of the tasks of each milestone in `milestones`, and then of those below it.
fn fill_below(milestones: &mut [MilestoneView], index: &ChainIndex) {
    for milestone in milestones {
        fill_in(&mut milestone.tasks, index);
        fill_below(&mut milestone.milestones, index);
    }
}

/// Sets the chain of each entry in `entries` from `index`.
fn fill_in(entries: &mut [EntryView], index: &ChainIndex) {
    for entry in entries {
        entry.chain = index.chain_of(&entry.id);
    }
}

/// Sets `milestone_state` on the node of each milestone in `milestones`, and then in those below it.
fn set_states(nodes: &mut HashMap<String, Node>, milestones: &[MilestoneView]) {
    for milestone in milestones {
        if let Some(node) = nodes.get_mut(&milestone.id) {
            node.milestone_state = Some(milestone.state);
        }
        set_states(nodes, &milestone.milestones);
    }
}

/// The way a walk goes from its task: through `depends` (the paths before it) or through
/// `dependents` (the paths after it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Before,
    After,
}

/// One walk from a task, collecting its paths in the order the walk finds them.
struct Walk<'a> {
    index: &'a ChainIndex,
    task: &'a str,
    direction: Direction,
    /// The first `MAX_LINES` paths found.
    lines: Vec<Path>,
    /// Every path found, shown or not.
    count: usize,
}

impl Walk<'_> {
    /// Walks on from `node`, the last ID of `path` (the task itself when `path` is empty). Each
    /// neighbour is the next step: the task or an ID already on the path closes a loop, an ID that
    /// names no entry ends its path, and a path that reaches `MAX_STEPS` IDs is recorded cut
    /// instead of going deeper.
    fn visit(&mut self, node: &str, path: &mut Vec<String>) {
        let index = self.index;
        let next = index.neighbors(node, self.direction);
        if next.is_empty() {
            if !path.is_empty() {
                self.record(path.as_slice(), None, PathEnd::Open, false);
            }
            return;
        }
        for id in next {
            if self.count >= MAX_LINES + MAX_COUNTED {
                return;
            }
            if id.as_str() == self.task || path.contains(id) {
                self.record(
                    path.as_slice(),
                    Some(id.as_str()),
                    PathEnd::Loop(id.clone()),
                    false,
                );
            } else if !index.nodes.contains_key(id) {
                self.record(path.as_slice(), Some(id.as_str()), PathEnd::Open, false);
            } else if path.len() + 1 == MAX_STEPS {
                // A complete entry at the limit is a whole path; one with more beyond it is cut.
                let cut = !index.neighbors(id, self.direction).is_empty();
                self.record(path.as_slice(), Some(id.as_str()), PathEnd::Open, cut);
            } else {
                path.push(id.clone());
                self.visit(id, path);
                path.pop();
            }
        }
    }

    /// Counts the path `path` followed by `last`, and keeps it while fewer than `MAX_LINES` are
    /// kept. A path through `depends` is read from its far end, so the start comes first.
    fn record(&mut self, path: &[String], last: Option<&str>, end: PathEnd, cut: bool) {
        self.count += 1;
        if self.lines.len() >= MAX_LINES {
            return;
        }
        let mut ids: Vec<&str> = path.iter().map(String::as_str).chain(last).collect();
        if self.direction == Direction::Before {
            ids.reverse();
        }
        let steps = ids.into_iter().map(|id| self.index.step(id)).collect();
        self.lines.push(Path { steps, cut, end });
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, FixedOffset, NaiveDate};

    use super::*;
    use crate::ops::OpsConfig;
    use crate::report::page::tree::{build_earlier, build_tree};

    /// The clock of every test: 21 Nov 2026, 15:00, in the +01:00 offset.
    const NOW: &str = "2026-11-21T15:00:00+01:00";

    fn now() -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(NOW).unwrap()
    }

    fn day(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
    }

    /// The lines of an invented task list as one text, with a final newline.
    fn text(lines: &[&str]) -> String {
        let mut text = lines.join("\n");
        text.push('\n');
        text
    }

    /// The index of the invented `todo` lines and of the invented `archive` lines, read at [`NOW`].
    fn index_of(todo: &[&str], archive: Option<&[&str]>) -> ChainIndex {
        let doc = Document::parse_bytes(text(todo).as_bytes()).unwrap();
        let archive = archive.map(|lines| Document::parse_bytes(text(lines).as_bytes()).unwrap());
        let cfg = OpsConfig::default();
        let ctx = Ctx {
            doc: &doc,
            archive: archive.as_ref(),
            cfg: &cfg,
            now: now(),
        };
        ChainIndex::new(&doc, archive.as_ref(), &ctx)
    }

    #[test]
    fn the_index_finds_a_task_of_todo_md_and_one_of_the_archive() {
        let index = index_of(
            &["### T-001 · Light the lamp", "- status: open"],
            Some(&["### T-010 · Older part", "- status: done"]),
        );
        assert_eq!(
            index.step("T-001"),
            Step {
                id: "T-001".to_string(),
                title: "Light the lamp".to_string(),
                state: StepState::Task(State::ToDo),
                archived: false,
            }
        );
        assert_eq!(
            index.step("T-010"),
            Step {
                id: "T-010".to_string(),
                title: "Older part".to_string(),
                state: StepState::Task(State::Done),
                archived: true,
            }
        );
    }

    #[test]
    fn todo_md_wins_when_an_id_is_in_both_files() {
        let index = index_of(
            &["### T-001 · Current title", "- status: open"],
            Some(&["### T-001 · Old title", "- status: done"]),
        );
        let step = index.step("T-001");
        assert_eq!(step.title, "Current title");
        assert_eq!(step.state, StepState::Task(State::ToDo));
        assert!(!step.archived);
    }

    #[test]
    fn dependents_list_the_entries_that_name_an_id_in_file_order() {
        let index = index_of(
            &[
                "### T-001 · Light the lamp",
                "- status: open",
                "### T-002 · Trim the wick",
                "- status: open",
                "- depends: T-001",
                "### T-003 · Fill the oil",
                "- status: open",
                "- depends: T-001, T-002",
            ],
            Some(&[
                "### T-004 · Older part",
                "- status: done",
                "- depends: T-001",
            ]),
        );
        // TODO.md entries come first, then the archive; each dependent is listed once.
        assert_eq!(
            index.dependents.get("T-001"),
            Some(&vec![
                "T-002".to_string(),
                "T-003".to_string(),
                "T-004".to_string(),
            ])
        );
        assert_eq!(
            index.dependents.get("T-002"),
            Some(&vec!["T-003".to_string()])
        );
        assert_eq!(index.dependents.get("T-003"), None);
    }

    #[test]
    fn a_depends_is_read_as_its_canonical_id() {
        let index = index_of(
            &[
                "### T-001 · Light the lamp",
                "- status: open",
                "### T-002 · Trim the wick",
                "- status: open",
                "- depends: t-1",
            ],
            None,
        );
        assert_eq!(
            index.dependents.get("T-001"),
            Some(&vec!["T-002".to_string()])
        );
    }

    #[test]
    fn a_step_of_an_unknown_id_is_not_found() {
        let index = index_of(&["### T-001 · Light the lamp", "- status: open"], None);
        assert_eq!(
            index.step("T-999"),
            Step {
                id: "T-999".to_string(),
                title: String::new(),
                state: StepState::NotFound,
                archived: false,
            }
        );
    }

    #[test]
    fn a_milestone_step_carries_its_tree_state_after_set_milestone_states() {
        let lines = [
            "### T-001 · Keep the light",
            "- status: open",
            "- type: milestone",
            "### T-002 · Fill the oil",
            "- status: claimed",
            "- owner: keeper-1",
            "- claimed: 2026-11-21T09:00+01:00",
            "- milestone: T-001",
            "### T-003 · Inner milestone, done",
            "- status: done",
            "- type: milestone",
            "- milestone: T-001",
            "- note (2026-11-20, bob): done — all in",
        ];
        let doc = Document::parse_bytes(text(&lines).as_bytes()).unwrap();
        let cfg = OpsConfig::default();
        let ctx = Ctx {
            doc: &doc,
            archive: None,
            cfg: &cfg,
            now: now(),
        };
        let tree = build_tree(&doc, &ctx);
        let mut index = ChainIndex::new(&doc, None, &ctx);

        // Before the tree is set, a milestone is a step with the state of its entry.
        assert_eq!(index.step("T-001").state, StepState::Task(State::ToDo));

        index.set_milestone_states(&tree);
        assert_eq!(
            index.step("T-001").state,
            StepState::Milestone(MilestoneState::InProgress)
        );
        // A task is still a task step, and a milestone nested below another gets its state too.
        assert_eq!(
            index.step("T-002").state,
            StepState::Task(State::InProgress)
        );
        assert_eq!(
            index.step("T-003").state,
            StepState::Milestone(MilestoneState::Finished(Some(day(20))))
        );
    }

    /// The IDs of the steps of `path`, in the order they are read.
    fn ids_of(path: &Path) -> Vec<&str> {
        path.steps.iter().map(|step| step.id.as_str()).collect()
    }

    /// The lines of `count` invented tasks, T-001 first, where each task waits for the one before.
    fn chain_lines(count: usize) -> Vec<String> {
        let mut lines = Vec::new();
        for n in 1..=count {
            lines.push(format!("### T-{n:03} · Step {n}"));
            lines.push("- status: open".to_string());
            if n > 1 {
                lines.push(format!("- depends: T-{:03}", n - 1));
            }
        }
        lines
    }

    fn refs(lines: &[String]) -> Vec<&str> {
        lines.iter().map(String::as_str).collect()
    }

    #[test]
    fn a_straight_chain_gives_one_path_each_way() {
        // A <- B <- C: B waits for A and C waits for B.
        let index = index_of(
            &[
                "### T-001 · Light the lamp",
                "- status: open",
                "### T-002 · Trim the wick",
                "- status: open",
                "- depends: T-001",
                "### T-003 · Fill the oil",
                "- status: open",
                "- depends: T-002",
            ],
            None,
        );
        let c = index.chain_of("T-003").unwrap();
        assert_eq!(c.before.len(), 1);
        assert_eq!(ids_of(&c.before[0]), ["T-001", "T-002"]);
        assert!(!c.before[0].cut);
        assert_eq!(c.before[0].end, PathEnd::Open);
        assert!(c.after.is_empty());
        assert_eq!((c.more_before, c.more_after), (0, 0));

        let a = index.chain_of("T-001").unwrap();
        assert!(a.before.is_empty());
        assert_eq!(a.after.len(), 1);
        assert_eq!(ids_of(&a.after[0]), ["T-002", "T-003"]);
        assert_eq!(a.after[0].end, PathEnd::Open);
    }

    #[test]
    fn a_branch_gives_a_path_for_each_way_back() {
        // T-003 waits for T-001 and for T-002, which waits for T-001.
        let index = index_of(
            &[
                "### T-001 · Light the lamp",
                "- status: open",
                "### T-002 · Trim the wick",
                "- status: open",
                "- depends: T-001",
                "### T-003 · Fill the oil",
                "- status: open",
                "- depends: T-001, T-002",
            ],
            None,
        );
        let c = index.chain_of("T-003").unwrap();
        assert_eq!(c.before.len(), 2);
        assert_eq!(ids_of(&c.before[0]), ["T-001"]);
        assert_eq!(ids_of(&c.before[1]), ["T-001", "T-002"]);
    }

    #[test]
    fn a_step_in_the_archive_is_archived() {
        let index = index_of(
            &[
                "### T-011 · Current part",
                "- status: open",
                "- depends: T-010",
            ],
            Some(&["### T-010 · Older part", "- status: done"]),
        );
        let c = index.chain_of("T-011").unwrap();
        assert_eq!(c.before.len(), 1);
        let step = &c.before[0].steps[0];
        assert_eq!(step.id, "T-010");
        assert_eq!(step.title, "Older part");
        assert_eq!(step.state, StepState::Task(State::Done));
        assert!(step.archived);
    }

    #[test]
    fn a_milestone_in_depends_is_a_milestone_step() {
        let lines = [
            "### T-001 · Keep the light",
            "- status: open",
            "- type: milestone",
            "### T-002 · Fill the oil",
            "- status: claimed",
            "- owner: keeper-1",
            "- claimed: 2026-11-21T09:00+01:00",
            "- milestone: T-001",
            "### T-003 · Trim the wick",
            "- status: open",
            "- depends: T-001",
        ];
        let doc = Document::parse_bytes(text(&lines).as_bytes()).unwrap();
        let cfg = OpsConfig::default();
        let ctx = Ctx {
            doc: &doc,
            archive: None,
            cfg: &cfg,
            now: now(),
        };
        let tree = build_tree(&doc, &ctx);
        let mut index = ChainIndex::new(&doc, None, &ctx);
        index.set_milestone_states(&tree);

        let c = index.chain_of("T-003").unwrap();
        assert_eq!(c.before.len(), 1);
        assert_eq!(
            c.before[0].steps[0].state,
            StepState::Milestone(MilestoneState::InProgress)
        );
    }

    #[test]
    fn an_unknown_id_is_a_not_found_step_that_ends_its_path() {
        // T-002 waits for T-999, which names no entry; T-003 waits for T-002.
        let index = index_of(
            &[
                "### T-002 · Trim the wick",
                "- status: open",
                "- depends: T-999",
                "### T-003 · Fill the oil",
                "- status: open",
                "- depends: T-002",
            ],
            None,
        );
        let c = index.chain_of("T-003").unwrap();
        assert_eq!(c.before.len(), 1);
        assert_eq!(ids_of(&c.before[0]), ["T-999", "T-002"]);
        assert_eq!(c.before[0].steps[0].state, StepState::NotFound);
        assert_eq!(c.before[0].steps[0].title, "");
        assert_eq!(c.before[0].end, PathEnd::Open);
    }

    #[test]
    fn a_loop_ends_with_loop_and_names_the_id_it_comes_back_to() {
        // T-001 waits for T-002, and T-002 waits for T-001.
        let index = index_of(
            &[
                "### T-001 · Light the lamp",
                "- status: open",
                "- depends: T-002",
                "### T-002 · Trim the wick",
                "- status: open",
                "- depends: T-001",
            ],
            None,
        );
        let c = index.chain_of("T-001").unwrap();
        assert_eq!(c.before.len(), 1);
        assert_eq!(ids_of(&c.before[0]), ["T-001", "T-002"]);
        assert_eq!(c.before[0].end, PathEnd::Loop("T-001".to_string()));
        assert_eq!(c.after.len(), 1);
        assert_eq!(c.after[0].end, PathEnd::Loop("T-001".to_string()));
    }

    #[test]
    fn fifteen_waiting_tasks_give_ten_lines_and_five_counted() {
        let mut lines = vec![
            "### T-001 · Light the lamp".to_string(),
            "- status: open".to_string(),
        ];
        for n in 2..=16 {
            lines.push(format!("### T-{n:03} · Waiting {n}"));
            lines.push("- status: open".to_string());
            lines.push("- depends: T-001".to_string());
        }
        let index = index_of(&refs(&lines), None);
        let c = index.chain_of("T-001").unwrap();
        assert_eq!(c.after.len(), 10);
        assert_eq!(c.more_after, 5);
        assert!(c.before.is_empty());
        assert_eq!(c.more_before, 0);
    }

    #[test]
    fn more_than_one_hundred_paths_are_counted_up_to_the_limit() {
        let mut lines = vec![
            "### T-001 · Light the lamp".to_string(),
            "- status: open".to_string(),
        ];
        for n in 2..=130 {
            lines.push(format!("### T-{n:03} · Waiting {n}"));
            lines.push("- status: open".to_string());
            lines.push("- depends: T-001".to_string());
        }
        let index = index_of(&refs(&lines), None);
        let c = index.chain_of("T-001").unwrap();
        assert_eq!(c.after.len(), MAX_LINES);
        assert_eq!(c.more_after, MAX_COUNTED);
    }

    #[test]
    fn a_long_chain_is_cut_to_the_twelve_steps_nearest_the_task() {
        // T-001 <- T-002 <- ... <- T-020, so T-020 waits for a chain of 19 tasks.
        let lines = chain_lines(20);
        let index = index_of(&refs(&lines), None);

        let before = index.chain_of("T-020").unwrap();
        assert_eq!(before.before.len(), 1);
        assert!(before.before[0].cut);
        assert_eq!(before.before[0].steps.len(), MAX_STEPS);
        assert_eq!(
            ids_of(&before.before[0]),
            [
                "T-008", "T-009", "T-010", "T-011", "T-012", "T-013", "T-014", "T-015", "T-016",
                "T-017", "T-018", "T-019"
            ]
        );

        let after = index.chain_of("T-001").unwrap();
        assert_eq!(after.after.len(), 1);
        assert!(after.after[0].cut);
        assert_eq!(
            ids_of(&after.after[0]),
            [
                "T-002", "T-003", "T-004", "T-005", "T-006", "T-007", "T-008", "T-009", "T-010",
                "T-011", "T-012", "T-013"
            ]
        );
    }

    #[test]
    fn a_complete_path_of_exactly_the_limit_is_not_cut() {
        // Thirteen tasks in a chain: T-013 waits for twelve tasks, and T-001 waits for nothing.
        let lines = chain_lines(13);
        let index = index_of(&refs(&lines), None);
        let c = index.chain_of("T-013").unwrap();
        assert_eq!(c.before.len(), 1);
        assert_eq!(c.before[0].steps.len(), MAX_STEPS);
        assert!(!c.before[0].cut);
        assert_eq!(ids_of(&c.before[0]).first(), Some(&"T-001"));
        assert_eq!(c.before[0].end, PathEnd::Open);
    }

    #[test]
    fn a_task_without_depends_or_dependents_has_no_chain() {
        let index = index_of(&["### T-001 · Light the lamp", "- status: open"], None);
        assert_eq!(index.chain_of("T-001"), None);
        assert_eq!(index.chain_of("T-999"), None);
    }

    #[test]
    fn fill_chains_sets_the_chain_of_every_task_of_both_trees() {
        let todo = [
            "### T-001 · Keep the light",
            "- status: open",
            "- type: milestone",
            "### T-002 · Trim the wick",
            "- status: open",
            "- milestone: T-001",
            "- depends: T-003",
            "### T-003 · Fill the oil",
            "- status: open",
            "### T-004 · Clean the glass",
            "- status: open",
            "### T-005 · Polish the lens",
            "- status: open",
            "- milestone: T-001",
            "### T-006 · Inner milestone",
            "- status: open",
            "- type: milestone",
            "- milestone: T-001",
            "### T-007 · Check the lens",
            "- status: open",
            "- milestone: T-006",
            "- depends: T-004",
            "### T-008 · Sweep the floor",
            "- status: open",
        ];
        let archive = [
            "### T-020 · Older part",
            "- status: done",
            "### T-021 · Older task",
            "- status: done",
            "- depends: T-020",
            "### T-022 · Lone archived task",
            "- status: done",
        ];
        let doc = Document::parse_bytes(text(&todo).as_bytes()).unwrap();
        let archive_doc = Document::parse_bytes(text(&archive).as_bytes()).unwrap();
        let cfg = OpsConfig::default();
        let ctx = Ctx {
            doc: &doc,
            archive: Some(&archive_doc),
            cfg: &cfg,
            now: now(),
        };
        let mut tree = build_tree(&doc, &ctx);
        let mut earlier = build_earlier(&archive_doc, &ctx);
        let mut index = ChainIndex::new(&doc, Some(&archive_doc), &ctx);
        index.set_milestone_states(&tree);
        fill_chains(&mut tree, &index);
        fill_chains(&mut earlier.tree, &index);

        // T-001 has tasks T-002 and T-005 and the milestone T-006, which holds T-007.
        let top = &tree.milestones[0];
        let inner = &top.milestones[0];
        // T-002 waits for T-003: its chain shows that path before it.
        let waits = top.tasks[0].chain.as_ref().unwrap();
        assert_eq!(ids_of(&waits.before[0]), ["T-003"]);
        // T-005 waits for nothing and nothing waits for it.
        assert_eq!(top.tasks[1].chain, None);
        // T-007 sits below the milestone T-006 and waits for T-004, so it has a chain.
        assert!(inner.tasks[0].chain.is_some());
        // T-003 and T-004 are waited for, so they have a chain; T-008 has none.
        assert!(tree.other[0].chain.is_some());
        assert!(tree.other[1].chain.is_some());
        assert_eq!(tree.other[2].chain, None);

        // The archive's tree: T-021 waits for T-020, which is waited for; T-022 has no link.
        let archived = &earlier.tree.other;
        assert!(archived[0].chain.is_some());
        assert_eq!(
            ids_of(&archived[1].chain.as_ref().unwrap().before[0]),
            ["T-020"]
        );
        assert_eq!(archived[2].chain, None);
    }
}
