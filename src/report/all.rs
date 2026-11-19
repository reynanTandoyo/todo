//! What the page for every registered project shows (T-094; R-REPORT-2).
//!
//! Each project's lists come from [`build_page`], the same model as the single-project page. The
//! card at the top of each project is decided here from the same `TODO.md`: the tasks per status
//! (counted as `todo overview` counts them), the top-level milestones with their progress, the
//! blocked tasks and the open questions. A project's files are read as `todo overview` reads them,
//! without a state folder: nothing is written next to a project, and no history snapshot or
//! outside-edit check runs. A project that cannot be read shows its problem instead of its lists.

use std::path::Path;

use chrono::{DateTime, FixedOffset};

use crate::doc::{Document, EntryKind, Progress, Status};
use crate::error::Result;
use crate::ops::{file_label, read_optional_archive};
use crate::register::Project;
use crate::register::overview::{ProblemJson, TaskCounts, load_todo_doc};
use crate::report::model::{Page, build_page, generated_text, reason_for};

/// The page for every registered project, in the order they were added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllPage {
    /// The clock at the build, formatted for people.
    pub generated: String,
    /// One entry per registered project, in register order.
    pub projects: Vec<ProjectPage>,
}

/// One registered project on the page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectPage {
    /// The name the project is registered under.
    pub name: String,
    /// The absolute path of the project's `TODO.md`.
    pub path: String,
    /// The card and lists, or the problem that stops them.
    pub content: Content,
    /// Problems that the single-project report warns about in this project's files, each with the
    /// path of the file. The command prints them on standard error; the page does not show them.
    pub warnings: Vec<String>,
}

impl ProjectPage {
    /// The card and lists of the project, or `None` when it cannot be read.
    #[must_use]
    pub fn lists(&self) -> Option<&Lists> {
        match &self.content {
            Content::Lists(lists) => Some(&**lists),
            Content::Problem(_) => None,
        }
    }
}

/// What the page shows for a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Content {
    /// The project can be read: its card, then its lists.
    Lists(Box<Lists>),
    /// The project cannot be read (gone, not parsed, or in a newer format): its problem only.
    Problem(ProblemJson),
}

/// A project that can be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lists {
    pub card: Card,
    /// The lists, as the single-project page shows them.
    pub page: Page,
}

/// The card at the top of the page for one project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    /// Tasks per status (`R-KIND-3`), as `todo overview` counts them: `TODO.md` only.
    pub tasks: TaskCounts,
    /// Top-level milestones that are not dropped, in file order, with their progress (`R-MILE-3`).
    pub milestones: Vec<MilestoneRow>,
    /// Blocked tasks, in file order, each with the reason of its latest `blocked` note.
    pub blocked: Vec<BlockedRow>,
    /// Open questions, in file order.
    pub questions: Vec<Row>,
}

/// A top-level milestone on a card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MilestoneRow {
    /// The canonical ID (`T-010`).
    pub id: String,
    pub title: String,
    pub status: Status,
    /// Tasks below the milestone that are done, out of those that are not dropped.
    pub progress: Progress,
}

/// A blocked task on a card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockedRow {
    /// The canonical ID (`T-013`).
    pub id: String,
    pub title: String,
    /// The reason of the latest `blocked — <reason>` note, or the page's "no reason recorded".
    pub reason: String,
}

/// An entry named on a card: its canonical ID and title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The canonical ID (`T-020`).
    pub id: String,
    pub title: String,
}

/// The page for `projects` at `now`. Each project is read on its own: one that cannot be read
/// shows its problem, and the others are still shown. Reads only; nothing is written.
#[must_use]
pub fn build_all_page(projects: &[Project], now: DateTime<FixedOffset>) -> AllPage {
    AllPage {
        generated: generated_text(now),
        projects: projects
            .iter()
            .map(|project| project_page(project, now))
            .collect(),
    }
}

fn project_page(project: &Project, now: DateTime<FixedOffset>) -> ProjectPage {
    let (content, warnings) = match read_lists(Path::new(&project.path), now) {
        Ok((lists, warnings)) => (Content::Lists(Box::new(lists)), warnings),
        Err(err) => (
            Content::Problem(ProblemJson {
                code: err.code,
                message: err.message,
            }),
            Vec::new(),
        ),
    };
    ProjectPage {
        name: project.name.clone(),
        path: project.path.clone(),
        content,
        warnings,
    }
}

/// Reads the `TODO.md` at `todo_path` and its archive, and builds the card and lists from them.
/// Returns the warnings the single-project report would print for the same files.
fn read_lists(todo_path: &Path, now: DateTime<FixedOffset>) -> Result<(Lists, Vec<String>)> {
    let (cfg, doc) = load_todo_doc(todo_path)?;
    let archive = read_optional_archive(&cfg.archive_path(todo_path), cfg.id_config.clone())?;

    let shown = todo_path.display().to_string();
    let mut warnings: Vec<String> = doc
        .problems
        .iter()
        .map(|problem| problem.format_with_path(&shown))
        .collect();
    if let Some(archive) = &archive {
        warnings.extend(
            doc.check_archive_duplicates(archive)
                .iter()
                .map(|dup| dup.format_with_path(&shown)),
        );
    }

    let page = build_page(&doc, archive.as_ref(), &cfg, now, &file_label(todo_path));
    let card = card_of(&doc, &page);
    Ok((Lists { card, page }, warnings))
}

fn card_of(doc: &Document, page: &Page) -> Card {
    let index = doc.milestone_index();
    let milestones = doc
        .tasks
        .iter()
        .filter(|t| t.is_milestone() && t.status != Status::Dropped)
        .filter(|t| !doc.milestone_parent(t).is_some_and(|p| p.is_milestone()))
        .map(|t| MilestoneRow {
            id: doc.id_config.format_id(t.id),
            title: t.title.clone(),
            status: t.status,
            progress: index.progress(t.id),
        })
        .collect();
    let blocked = doc
        .tasks
        .iter()
        .filter(|t| t.kind() == EntryKind::Task && t.status == Status::Blocked)
        .map(|t| BlockedRow {
            id: doc.id_config.format_id(t.id),
            title: t.title.clone(),
            reason: reason_for(t, "blocked"),
        })
        .collect();
    let questions = page
        .questions
        .iter()
        .map(|entry| Row {
            id: entry.id.clone(),
            title: entry.title.clone(),
        })
        .collect();
    Card {
        tasks: TaskCounts::of(doc),
        milestones,
        blocked,
        questions,
    }
}
