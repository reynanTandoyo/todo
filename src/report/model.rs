//! What the HTML report shows, decided from the parsed task lists (T-092; R-REPORT-1).
//!
//! The model is built once per run and holds plain text, so the HTML writer only escapes and lays
//! it out. Read-only: nothing here changes a document.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, FixedOffset};

use crate::doc::{Document, EntryKind, Note, Progress, Status, Task, TaskId};
use crate::ops::OpsConfig;
use crate::ops::read::about_id;

/// The em dash that separates a reason from the words before it (`docs/format.md` §9).
const DASH: &str = "\u{2014}";

/// Shown when no note records why a task is blocked or dropped.
pub const NO_REASON: &str = "no reason recorded";

/// Title used when `TODO.md` has no `# ` heading before its first section.
const DEFAULT_TITLE: &str = "Tasks";

/// Everything the page shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// The first `# ` heading of `TODO.md`, or [`DEFAULT_TITLE`].
    pub title: String,
    /// The file the page is built from, by its name (`TODO.md`).
    pub file_label: String,
    /// The archive's file name when `TODO-archive.md` was read, whether or not it holds entries.
    pub archive_label: Option<String>,
    /// The clock at the build, formatted for people (`2026-11-19 10:00 +01:00`).
    pub generated: String,
    pub counts: Counts,
    /// Unfinished tasks (open, claimed or blocked) per area, in area order; `no_area` is shown last.
    pub areas: Vec<AreaCount>,
    /// The work: milestones with their goals and progress, tasks indented below them, and the
    /// top-level tasks last (`R-REPORT-1`).
    pub work: Vec<WorkNode>,
    /// Questions that are not done or dropped, in file order.
    pub questions: Vec<Entry>,
    /// Ideas that are not done or dropped, in file order.
    pub ideas: Vec<Entry>,
    /// Done entries of every kind that the work tree does not show, in file order.
    pub done: Vec<Entry>,
    /// Dropped entries of every kind that the work tree does not show, in file order.
    pub dropped: Vec<Entry>,
    /// Unfinished tasks that have no area.
    pub no_area: usize,
}

/// The numbers at the top of the page.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    /// Tasks with status `open`.
    pub open: usize,
    /// Tasks with status `claimed`.
    pub claimed: usize,
    /// Tasks with status `blocked`.
    pub blocked: usize,
    /// Milestones that are open, claimed or blocked.
    pub milestones: usize,
    /// Questions that are not done or dropped.
    pub questions: usize,
    /// Ideas that are not done or dropped.
    pub ideas: usize,
    /// Done entries listed on the page.
    pub done: usize,
    /// Dropped entries listed on the page.
    pub dropped: usize,
}

/// Unfinished tasks in one area.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaCount {
    /// The area as the first task in it wrote it.
    pub area: String,
    pub count: usize,
}

/// One entry of the page: a task, a milestone, a question or an idea.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The canonical ID (`T-007`).
    pub id: String,
    pub kind: EntryKind,
    pub title: String,
    pub status: Status,
    pub priority: Option<String>,
    pub difficulty: Option<String>,
    pub area: Option<String>,
    /// Set for a claimed entry.
    pub claim: Option<Claim>,
    /// Set for a blocked or dropped entry: the latest reason, or [`NO_REASON`].
    pub reason: Option<String>,
    /// The goal of a milestone.
    pub goal: Option<String>,
    /// The entry this one is about (`about`), for a question or an idea.
    pub about: Option<About>,
    /// Whether the entry comes from `TODO-archive.md`.
    pub archived: bool,
    /// The `- key: value` lines of the entry, as written, in file order.
    pub fields: Vec<(String, String)>,
    /// The dated notes of the entry, in file order.
    pub notes: Vec<NoteView>,
}

/// Who holds a claim, and how old it is (`R-CLAIM-4`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    /// `None` when the owner is `—` or missing.
    pub owner: Option<String>,
    /// Hours since the claim, or `None` when the claim has no valid time.
    pub age_hours: Option<i64>,
    pub stale: bool,
}

/// The entry a question or an idea is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct About {
    /// The canonical ID of the named entry.
    pub id: String,
    /// Its title, when the task lists hold it.
    pub title: Option<String>,
    /// Whether the page shows that entry, so that a link to it can point at its row.
    pub linked: bool,
}

/// A dated note as the page shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteView {
    pub date: String,
    pub agent: String,
    pub text: String,
}

/// A milestone or task in the work tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkNode {
    pub entry: Entry,
    /// Progress of a milestone (`R-MILE-3`); `None` for a task.
    pub progress: Option<Progress>,
    /// Entries below this milestone that the tree shows, in file order.
    pub children: Vec<WorkNode>,
}

/// Builds the page from the task list, its archive when there is one, and the clock.
///
/// `TODO-archive.md` contributes only its done and dropped entries, which the done and dropped
/// lists show marked as archived; anything else in it is not shown.
#[must_use]
pub fn build_page(
    doc: &Document,
    archive: Option<&Document>,
    cfg: &OpsConfig,
    now: DateTime<FixedOffset>,
    file_label: &str,
) -> Page {
    let builder = Builder {
        doc,
        index: doc.milestone_index(),
        cfg,
        now,
        titles: title_index(doc, archive),
    };

    let mut visited = BTreeSet::new();
    let mut work = Vec::new();
    for task in doc
        .tasks
        .iter()
        .filter(|t| t.is_milestone() && doc.milestone_ancestors(t).is_empty())
    {
        work.extend(builder.work_node(task, &mut visited));
    }
    for task in doc
        .tasks
        .iter()
        .filter(|t| t.kind() == EntryKind::Task && doc.milestone_ancestors(t).is_empty())
    {
        work.extend(builder.work_node(task, &mut visited));
    }
    // Entries in a milestone cycle have an ancestor, so no walk from the top reaches them. Show
    // them as top-level rather than hide them; entries already shown are skipped.
    for task in doc
        .tasks
        .iter()
        .filter(|t| matches!(t.kind(), EntryKind::Task | EntryKind::Milestone))
    {
        work.extend(builder.work_node(task, &mut visited));
    }

    // Entries the work tree shows are not repeated in the done or dropped lists.
    let mut shown = BTreeSet::new();
    collect_ids(&work, &mut shown);
    let placed = shown.clone();

    let mut questions = Vec::new();
    let mut ideas = Vec::new();
    let mut done = Vec::new();
    let mut dropped = Vec::new();
    for (source, archived) in std::iter::once((doc, false)).chain(archive.map(|a| (a, true))) {
        for task in &source.tasks {
            match task.status {
                Status::Done | Status::Dropped => {
                    let id = source.id_config.format_id(task.id);
                    if !archived && placed.contains(&id) {
                        continue;
                    }
                    let entry = builder.entry(source, task, archived);
                    if task.status == Status::Done {
                        done.push(entry);
                    } else {
                        dropped.push(entry);
                    }
                }
                Status::Open | Status::Claimed | Status::Blocked if !archived => {
                    match task.kind() {
                        EntryKind::Question => questions.push(builder.entry(source, task, false)),
                        EntryKind::Idea => ideas.push(builder.entry(source, task, false)),
                        EntryKind::Task | EntryKind::Milestone => {}
                    }
                }
                Status::Open | Status::Claimed | Status::Blocked => {}
            }
        }
    }

    for entry in questions.iter().chain(&ideas).chain(&done).chain(&dropped) {
        shown.insert(entry.id.clone());
    }
    link_work(&mut work, &shown);
    for entry in questions
        .iter_mut()
        .chain(ideas.iter_mut())
        .chain(done.iter_mut())
        .chain(dropped.iter_mut())
    {
        link_entry(entry, &shown);
    }

    let (areas, no_area) = area_counts(doc);
    let counts = count(doc, &questions, &ideas, &done, &dropped);

    Page {
        title: page_title(doc),
        file_label: file_label.to_string(),
        archive_label: archive.map(|_| cfg.archive_file.clone()),
        generated: now.format("%Y-%m-%d %H:%M %:z").to_string(),
        counts,
        areas,
        work,
        questions,
        ideas,
        done,
        dropped,
        no_area,
    }
}

/// Shared state for turning tasks into entries and work nodes.
struct Builder<'a> {
    doc: &'a Document,
    index: crate::doc::MilestoneIndex<'a>,
    cfg: &'a OpsConfig,
    now: DateTime<FixedOffset>,
    /// Titles of every entry in `TODO.md` and the archive, by ID.
    titles: BTreeMap<TaskId, String>,
}

impl<'a> Builder<'a> {
    /// The work node for `task` and what the tree shows below it, or `None` when the tree shows
    /// nothing of it. An unfinished milestone is always shown; a finished or dropped one only when
    /// unfinished work sits below it. `visited` stops a cycle of milestones from looping.
    fn work_node(&self, task: &'a Task, visited: &mut BTreeSet<TaskId>) -> Option<WorkNode> {
        if !visited.insert(task.id) {
            return None;
        }
        if task.is_milestone() {
            let children: Vec<WorkNode> = self
                .index
                .children_of(task.id)
                .iter()
                .copied()
                .filter(|child| matches!(child.kind(), EntryKind::Task | EntryKind::Milestone))
                .filter_map(|child| self.work_node(child, visited))
                .collect();
            if !is_unfinished(task.status) && children.is_empty() {
                return None;
            }
            return Some(WorkNode {
                entry: self.entry(self.doc, task, false),
                progress: Some(self.index.progress(task.id)),
                children,
            });
        }
        if task.kind() == EntryKind::Task && is_unfinished(task.status) {
            return Some(WorkNode {
                entry: self.entry(self.doc, task, false),
                progress: None,
                children: Vec::new(),
            });
        }
        None
    }

    /// The entry for `task`, which belongs to `source` (`TODO.md` or the archive).
    fn entry(&self, source: &Document, task: &Task, archived: bool) -> Entry {
        let kind = task.kind();
        let is_milestone = kind == EntryKind::Milestone;
        let reason = match task.status {
            Status::Blocked => Some(reason_for(task, "blocked")),
            Status::Dropped => Some(reason_for(task, "dropped")),
            Status::Open | Status::Claimed | Status::Done => None,
        };
        let claim = (task.status == Status::Claimed).then(|| Claim {
            owner: task.owner.clone(),
            age_hours: self
                .cfg
                .claim_age(task, self.now)
                .map(|age| age.num_hours()),
            stale: self.cfg.is_claim_stale(task, self.now),
        });
        let about = about_id(task, self.cfg).map(|target| About {
            id: source.id_config.format_id(target),
            title: self.titles.get(&target).cloned(),
            linked: false,
        });
        Entry {
            id: source.id_config.format_id(task.id),
            kind,
            title: task.title.clone(),
            status: task.status,
            priority: task.priority.clone(),
            difficulty: task.difficulty.clone(),
            area: task.area.clone(),
            claim,
            reason,
            goal: if is_milestone {
                task.goal.clone()
            } else {
                None
            },
            about,
            archived,
            fields: task
                .field_lines
                .iter()
                .map(|field| (field.key.clone(), field.value.clone()))
                .collect(),
            notes: task.notes.iter().map(note_view).collect(),
        }
    }
}

fn is_unfinished(status: Status) -> bool {
    matches!(status, Status::Open | Status::Claimed | Status::Blocked)
}

fn note_view(note: &Note) -> NoteView {
    NoteView {
        date: note.date_raw.clone(),
        agent: note.agent.clone(),
        text: note.text.clone(),
    }
}

/// The reason in the latest note that starts with `keyword`, such as `blocked — <reason>` or
/// `dropped with T-002 — <reason>` (`docs/format.md` §9). [`NO_REASON`] when none gives one.
fn reason_for(task: &Task, keyword: &str) -> String {
    task.notes
        .iter()
        .rev()
        .find_map(|note| {
            let (head, reason) = note.text.trim().split_once(DASH)?;
            let head = head.trim();
            let names_keyword = head == keyword || head.starts_with(&format!("{keyword} "));
            let reason = reason.trim();
            (names_keyword && !reason.is_empty()).then(|| reason.to_string())
        })
        .unwrap_or_else(|| NO_REASON.to_string())
}

/// The first `# ` heading in the preamble of `doc`, or [`DEFAULT_TITLE`].
fn page_title(doc: &Document) -> String {
    doc.preamble_range
        .clone()
        .find_map(|idx| {
            doc.line_doc
                .lines
                .get(idx)
                .and_then(|line| line.text.strip_prefix("# "))
        })
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .unwrap_or(DEFAULT_TITLE)
        .to_string()
}

fn title_index(doc: &Document, archive: Option<&Document>) -> BTreeMap<TaskId, String> {
    let mut titles = BTreeMap::new();
    for source in std::iter::once(doc).chain(archive) {
        for task in &source.tasks {
            titles.entry(task.id).or_insert_with(|| task.title.clone());
        }
    }
    titles
}

/// Adds the canonical ID of every entry in `nodes`, at any depth.
fn collect_ids(nodes: &[WorkNode], ids: &mut BTreeSet<String>) {
    for node in nodes {
        ids.insert(node.entry.id.clone());
        collect_ids(&node.children, ids);
    }
}

fn link_work(nodes: &mut [WorkNode], shown: &BTreeSet<String>) {
    for node in nodes {
        link_entry(&mut node.entry, shown);
        link_work(&mut node.children, shown);
    }
}

/// Marks the `about` link of `entry` as working when the page shows the entry it names.
fn link_entry(entry: &mut Entry, shown: &BTreeSet<String>) {
    if let Some(about) = entry.about.as_mut() {
        about.linked = shown.contains(&about.id);
    }
}

/// Unfinished tasks per area, and how many unfinished tasks have no area (`R-REPORT-1`).
fn area_counts(doc: &Document) -> (Vec<AreaCount>, usize) {
    let mut by_area: BTreeMap<String, AreaCount> = BTreeMap::new();
    let mut no_area = 0;
    for task in doc
        .tasks
        .iter()
        .filter(|t| t.kind() == EntryKind::Task && is_unfinished(t.status))
    {
        match task
            .area
            .as_deref()
            .map(str::trim)
            .filter(|a| !a.is_empty())
        {
            Some(area) => {
                by_area
                    .entry(area.to_lowercase())
                    .or_insert_with(|| AreaCount {
                        area: area.to_string(),
                        count: 0,
                    })
                    .count += 1;
            }
            None => no_area += 1,
        }
    }
    (by_area.into_values().collect(), no_area)
}

fn count(
    doc: &Document,
    questions: &[Entry],
    ideas: &[Entry],
    done: &[Entry],
    dropped: &[Entry],
) -> Counts {
    let mut counts = Counts {
        questions: questions.len(),
        ideas: ideas.len(),
        done: done.len(),
        dropped: dropped.len(),
        ..Counts::default()
    };
    for task in doc.tasks.iter().filter(|t| is_unfinished(t.status)) {
        match (task.kind(), task.status) {
            (EntryKind::Task, Status::Open) => counts.open += 1,
            (EntryKind::Task, Status::Claimed) => counts.claimed += 1,
            (EntryKind::Task, Status::Blocked) => counts.blocked += 1,
            (EntryKind::Milestone, _) => counts.milestones += 1,
            (EntryKind::Task | EntryKind::Question | EntryKind::Idea, _) => {}
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reason_comes_from_the_latest_matching_note() {
        let doc = Document::parse_bytes(
            "### T-001 · Blocked task\n\
- status: blocked\n\
- note (2026-11-17, bob): blocked — waiting for the driver\n\
- note (2026-11-18, bob): blocked — the driver crashes\n\
- note (2026-11-19, bob): unblocked — fixed in the driver\n"
                .as_bytes(),
        )
        .unwrap();
        assert_eq!(reason_for(&doc.tasks[0], "blocked"), "the driver crashes");
    }

    #[test]
    fn reason_reads_the_drop_note_that_names_a_milestone() {
        let doc = Document::parse_bytes(
            "### T-002 · Dropped below a milestone\n\
- status: dropped\n\
- note (2026-11-18, bob): dropped with T-001 — the scene no longer needs it\n"
                .as_bytes(),
        )
        .unwrap();
        assert_eq!(
            reason_for(&doc.tasks[0], "dropped"),
            "the scene no longer needs it"
        );
    }

    #[test]
    fn missing_reason_says_so() {
        let doc = Document::parse_bytes(
            "### T-003 · Blocked without a reason\n\
- status: blocked\n\
- note (2026-11-18, bob): added\n\
- note (2026-11-18, bob): blocked\n"
                .as_bytes(),
        )
        .unwrap();
        assert_eq!(reason_for(&doc.tasks[0], "blocked"), NO_REASON);
    }

    #[test]
    fn title_is_the_first_heading_of_the_preamble() {
        let with =
            Document::parse_bytes(b"# Harbour lights \xe2\x80\x94 tasks\n\n## Now\n").unwrap();
        assert_eq!(page_title(&with), "Harbour lights \u{2014} tasks");
        let without = Document::parse_bytes(b"## Now\n").unwrap();
        assert_eq!(page_title(&without), DEFAULT_TITLE);
    }
}
