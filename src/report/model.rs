//! What the HTML report shows, decided from the parsed task lists (T-092; R-REPORT-1).
//!
//! The model is built once per run and holds plain text, so the HTML writer only escapes and lays
//! it out. Read-only: nothing here changes a document.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, FixedOffset};

use crate::doc::{Document, EntryKind, NO_REASON, Note, Progress, Status, Task, TaskId};
use crate::ops::OpsConfig;
use crate::ops::plan::build_plan;
use crate::ops::read::about_id;
use crate::ops::waves::{detail_gaps, plan_waves};

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
    /// Unfinished tasks per level (`R-LEVEL-1`, `R-REPORT-3`), as the planner counts them.
    pub levels: LevelCounts,
    /// The planning state: what the planner still has to do (`R-LEVEL-2`, `R-REPORT-3`).
    pub planning: Planning,
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

/// Unfinished tasks per level, in the order of `difficulties` (`R-LEVEL-1`). Every configured level is
/// listed, even at zero; a task whose level is not configured is counted apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelCounts {
    pub levels: Vec<LevelCount>,
    /// Unfinished tasks without a level.
    pub no_level: usize,
    /// Unfinished tasks whose level is not one of `difficulties`.
    pub other: usize,
}

/// Unfinished tasks at one level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelCount {
    /// The level as `difficulties` spells it.
    pub level: String,
    pub count: usize,
}

/// The planning state of a project, decided by the planner's list and the waves (`R-REPORT-3`), so
/// the page counts a task as `todo plan` and `todo waves` do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planning {
    /// The target level (`target_difficulty`); a task at or below it needs complete detail.
    pub target: String,
    /// Tasks at or below the target level that are not ready for lack of detail (`R-DETAIL-2`), in
    /// file order, each with what is missing.
    pub not_ready: Vec<NotReadyRow>,
    /// Tasks a worker gave back (`R-STUCK-1`), in file order, each with its problem and attempts.
    pub given_back: Vec<GivenBackRow>,
    /// Tasks kept above the target level on purpose (`R-LEVEL-3`), in file order, each with its reason.
    pub kept_on_purpose: Vec<KeptRow>,
    /// The number of waves (`R-WAVE-1`).
    pub waves: usize,
    /// The tasks in wave 1.
    pub wave1_tasks: usize,
    /// The lanes of wave 1 (`R-SCOPE-1`).
    pub wave1_lanes: usize,
}

/// A task in the planning state, with a link to its row when the page shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanRow {
    /// The canonical ID (`T-007`).
    pub id: String,
    pub title: String,
    /// The level as written, `None` when the task has none.
    pub level: Option<String>,
    /// Whether the page shows the task, so a link can point at its row.
    pub linked: bool,
}

/// A task that is not ready for lack of detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotReadyRow {
    pub row: PlanRow,
    /// What the detail rule lacks (`R-DETAIL-2`), as `detail_gaps` names it.
    pub missing: Vec<String>,
}

/// A task a worker gave back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GivenBackRow {
    pub row: PlanRow,
    /// The problem of the latest `stuck — <problem>` note, or [`NO_REASON`].
    pub problem: String,
    /// The `attempts` value as written, `0` when there is none.
    pub attempts: String,
}

/// A task kept above the target level on purpose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeptRow {
    pub row: PlanRow,
    /// The reason of its `final` line, or [`NO_REASON`].
    pub reason: String,
}

/// What a task shows when it is opened about its level and its check (`R-REPORT-3`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Facts {
    /// The level as written, `None` when the task has none.
    pub level: Option<String>,
    /// The `files` value as written, `None` when there is none.
    pub files: Option<String>,
    /// The `verify` command as written, `None` when there is none.
    pub verify: Option<String>,
    /// The date the check last passed (`verified`), `None` when it has not.
    pub verified: Option<String>,
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
    /// The level and the check of a task, shown first when it is opened (`R-REPORT-3`); `None` for
    /// milestones, questions and ideas.
    pub facts: Option<Facts>,
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
    // Built from the planner's list and the waves, once per page; the rows link to the entries the
    // page shows, so the set is complete before they are made.
    let planning = planning_state(doc, archive, cfg, &shown);
    let levels = level_counts(doc, cfg);
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
        generated: generated_text(now),
        counts,
        areas,
        levels,
        planning,
        work,
        questions,
        ideas,
        done,
        dropped,
        no_area,
    }
}

/// The clock at the build, as the pages show it (`2026-11-19 10:00 +01:00`).
#[must_use]
pub fn generated_text(now: DateTime<FixedOffset>) -> String {
    now.format("%Y-%m-%d %H:%M %:z").to_string()
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
        // The reasons are the ones `summary` shows too (T-140; `doc::reason`).
        let reason = match task.status {
            Status::Blocked => Some(task.blocked_reason().unwrap_or(NO_REASON).to_string()),
            Status::Dropped => Some(task.dropped_reason().unwrap_or(NO_REASON).to_string()),
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
            facts: (kind == EntryKind::Task).then(|| Facts {
                level: task.difficulty.clone(),
                files: task.files.clone(),
                verify: task.verify.clone(),
                verified: task.verified.clone(),
            }),
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

/// Unfinished tasks per configured level, and those without a level or with one that is not
/// configured (`R-LEVEL-1`). The same tally as the open tasks of the areas, by level.
fn level_counts(doc: &Document, cfg: &OpsConfig) -> LevelCounts {
    let mut counts = LevelCounts {
        levels: cfg
            .difficulties
            .iter()
            .map(|level| LevelCount {
                level: level.clone(),
                count: 0,
            })
            .collect(),
        no_level: 0,
        other: 0,
    };
    for task in doc
        .tasks
        .iter()
        .filter(|t| t.kind() == EntryKind::Task && is_unfinished(t.status))
    {
        match (
            task.difficulty.as_deref(),
            cfg.difficulty_rank(task.difficulty.as_deref()),
        ) {
            (None, _) => counts.no_level += 1,
            (Some(_), Some(rank)) => {
                if let Some(level) = counts.levels.get_mut(rank) {
                    level.count += 1;
                }
            }
            (Some(_), None) => counts.other += 1,
        }
    }
    counts
}

/// The planning state of `doc` (`R-REPORT-3`), from `ops::plan` and `ops::waves`: the planner's
/// groups and the waves are the ones `todo plan` and `todo waves` show. `shown` holds the IDs the page
/// lists, so a row links only to an entry that has a row.
fn planning_state(
    doc: &Document,
    archive: Option<&Document>,
    cfg: &OpsConfig,
    shown: &BTreeSet<String>,
) -> Planning {
    let plan = build_plan(doc, archive, cfg, None);
    let row = |task: &Task| {
        let id = doc.id_config.format_id(task.id);
        PlanRow {
            linked: shown.contains(&id),
            id,
            title: task.title.clone(),
            level: task.difficulty.clone(),
        }
    };
    let waves = plan_waves(doc, archive, None);
    let first = waves.waves.first();
    Planning {
        target: cfg.target_difficulty().to_string(),
        not_ready: plan
            .to_detail
            .iter()
            .map(|task| NotReadyRow {
                row: row(task),
                missing: detail_gaps(task, doc, archive, cfg),
            })
            .collect(),
        given_back: plan
            .given_back
            .iter()
            .map(|task| GivenBackRow {
                row: row(task),
                problem: task.blocked_reason().unwrap_or(NO_REASON).to_string(),
                attempts: task
                    .attempts
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or("0")
                    .to_string(),
            })
            .collect(),
        kept_on_purpose: plan
            .kept_on_purpose
            .iter()
            .map(|task| KeptRow {
                row: row(task),
                reason: task
                    .final_reason
                    .clone()
                    .unwrap_or_else(|| NO_REASON.to_string()),
            })
            .collect(),
        waves: waves.waves.len(),
        wave1_tasks: first.map_or(0, |wave| wave.lanes.iter().map(Vec::len).sum()),
        wave1_lanes: first.map_or(0, |wave| wave.lanes.len()),
    }
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
    fn title_is_the_first_heading_of_the_preamble() {
        let with =
            Document::parse_bytes(b"# Harbour lights \xe2\x80\x94 tasks\n\n## Now\n").unwrap();
        assert_eq!(page_title(&with), "Harbour lights \u{2014} tasks");
        let without = Document::parse_bytes(b"## Now\n").unwrap();
        assert_eq!(page_title(&without), DEFAULT_TITLE);
    }
}
