//! The chain of a task (T-228; R-REPORT-7): the tasks it waits for and the tasks that wait for it.

use std::collections::HashMap;

use super::entry::{Ctx, State, entry_view};
use super::tree::{MilestoneState, MilestoneView, Tree};
use crate::doc::{Document, EntryKind};

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

/// Sets `milestone_state` on the node of each milestone in `milestones`, and then in those below it.
fn set_states(nodes: &mut HashMap<String, Node>, milestones: &[MilestoneView]) {
    for milestone in milestones {
        if let Some(node) = nodes.get_mut(&milestone.id) {
            node.milestone_state = Some(milestone.state);
        }
        set_states(nodes, &milestone.milestones);
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, FixedOffset, NaiveDate};

    use super::*;
    use crate::ops::OpsConfig;
    use crate::report::page::tree::build_tree;

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
}
