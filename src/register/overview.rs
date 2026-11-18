//! The state of each registered project, for `todo overview` (T-093; R-PROJ-2).
//!
//! It reads the project's `TODO.md` and nothing else: it never takes a lock, never writes, never
//! opens the state folder and never searches folders. A project whose file is gone, cannot be read
//! or needs a newer todo is reported with its problem, and the other projects are still read.

use std::fs;
use std::io;
use std::path::Path;

use chrono::{DateTime, FixedOffset};
use serde::Serialize;

use crate::atomic::validate_utf8;
use crate::doc::{CURRENT_FORMAT_VERSION, Document, EntryKind, Status};
use crate::error::{ErrorCode, Result, TodoError};
use crate::find::fs_path;
use crate::ops::{OpsConfig, file_label, require_known_format};
use crate::register::Project;
use crate::version::RUNNING_VERSION;

/// Tasks per status (R-KIND-3): tasks only; milestones, questions and ideas are counted apart.
/// `total` is the sum of the five statuses.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct TaskCounts {
    /// Tasks that are open.
    pub open: usize,
    /// Tasks that are claimed, stale or not.
    pub claimed: usize,
    /// Tasks that are blocked.
    pub blocked: usize,
    /// Tasks that are done.
    pub done: usize,
    /// Tasks that are dropped.
    pub dropped: usize,
    /// All tasks: `open + claimed + blocked + done + dropped`.
    pub total: usize,
}

impl TaskCounts {
    fn count(&mut self, status: Status) {
        match status {
            Status::Open => self.open += 1,
            Status::Claimed => self.claimed += 1,
            Status::Blocked => self.blocked += 1,
            Status::Done => self.done += 1,
            Status::Dropped => self.dropped += 1,
        }
        self.total += 1;
    }
}

/// The top-level milestones of a project: how many are done, out of how many. A dropped milestone
/// is left out of both (R-MILE-3).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct MilestoneCount {
    /// Top-level milestones that are done.
    pub done: usize,
    /// Top-level milestones that are not dropped.
    pub total: usize,
}

/// The code and message of the problem that stops a project from being read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProblemJson {
    /// The stable error code (R-OUT-3), such as `NO_FILE` or `TOOL_TOO_OLD`.
    pub code: ErrorCode,
    /// The plain-English message, without the file name.
    pub message: String,
}

/// One registered project as `todo overview` shows it. When `problem` is set, every figure is `None`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectOverview {
    /// The name the project is registered under.
    pub name: String,
    /// The absolute path of the project's `TODO.md`.
    pub path: String,
    /// When the project was added.
    pub added: String,
    /// Why the project cannot be read, or `None`.
    pub problem: Option<ProblemJson>,
    /// The format version in the file (1 when the format line is missing).
    pub format_version: Option<u32>,
    /// The todo version stamped in the file (R-VER-1), as written; `None` when there is none.
    pub tool_stamp: Option<String>,
    /// `true` when the file is in an older format, or was last written by an older todo than the
    /// running one (or has no stamp): `todo migrate --yes` brings it up to date.
    pub needs_migration: Option<bool>,
    /// Tasks per status.
    pub tasks: Option<TaskCounts>,
    /// Top-level milestones, done and total.
    pub milestones: Option<MilestoneCount>,
    /// Claimed tasks whose claim is stale (R-CLAIM-4).
    pub stale_claims: Option<usize>,
    /// Questions that are not done or dropped (R-KIND-3).
    pub open_questions: Option<usize>,
}

/// The figures read from one project's `TODO.md`.
struct Figures {
    format_version: u32,
    tool_stamp: Option<String>,
    needs_migration: bool,
    tasks: TaskCounts,
    milestones: MilestoneCount,
    stale_claims: usize,
    open_questions: usize,
}

/// The overview of `project` at `now`. Reads its `TODO.md` only.
#[must_use]
pub fn overview_of(project: &Project, now: DateTime<FixedOffset>) -> ProjectOverview {
    let base = ProjectOverview {
        name: project.name.clone(),
        path: project.path.clone(),
        added: project.added.clone(),
        problem: None,
        format_version: None,
        tool_stamp: None,
        needs_migration: None,
        tasks: None,
        milestones: None,
        stale_claims: None,
        open_questions: None,
    };
    match measure(Path::new(&project.path), now) {
        Ok(figures) => ProjectOverview {
            format_version: Some(figures.format_version),
            tool_stamp: figures.tool_stamp,
            needs_migration: Some(figures.needs_migration),
            tasks: Some(figures.tasks),
            milestones: Some(figures.milestones),
            stale_claims: Some(figures.stale_claims),
            open_questions: Some(figures.open_questions),
            ..base
        },
        Err(err) => ProjectOverview {
            problem: Some(ProblemJson {
                code: err.code,
                message: err.message,
            }),
            ..base
        },
    }
}

/// Reads and counts the `TODO.md` at `todo_path`, refusing what the other read commands refuse:
/// a file that is not UTF-8, has a problem that blocks all writes, or is in a newer format.
fn measure(todo_path: &Path, now: DateTime<FixedOffset>) -> Result<Figures> {
    let label = file_label(todo_path);
    let cfg = OpsConfig::load_next_to(todo_path)?;
    let bytes = match fs::read(fs_path(todo_path).as_ref()) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(TodoError::new(ErrorCode::NoFile, "TODO.md not found")
                .with_file(todo_path.display().to_string()));
        }
        Err(e) => {
            return Err(TodoError::new(
                ErrorCode::WriteFailed,
                format!("failed reading {}: {e}", todo_path.display()),
            )
            .with_file(todo_path.display().to_string()));
        }
    };
    let _ = validate_utf8(&bytes, &label)?;
    let doc = Document::parse_bytes_with_config(&bytes, cfg.id_config.clone()).map_err(|e| {
        TodoError::new(ErrorCode::UnsafeFile, e.to_string())
            .with_file(label.clone())
            .with_line(e.line)
    })?;
    // The format is checked first, as the other read commands do, so a newer file says TOOL_TOO_OLD.
    require_known_format(&doc, &label)?;
    if let Some(problem) = doc.problems.iter().find(|p| p.blocks_all_writes()) {
        return Err(
            TodoError::new(ErrorCode::UnsafeFile, problem.message.clone())
                .with_file(label)
                .with_line(problem.line),
        );
    }

    let mut tasks = TaskCounts::default();
    let mut milestones = MilestoneCount::default();
    let mut stale_claims = 0;
    let mut open_questions = 0;
    for task in &doc.tasks {
        match task.kind() {
            EntryKind::Task => {
                tasks.count(task.status);
                if task.status == Status::Claimed && cfg.is_claim_stale(task, now) {
                    stale_claims += 1;
                }
            }
            EntryKind::Milestone => {
                let nested = doc.milestone_parent(task).is_some_and(|p| p.is_milestone());
                if !nested && task.status != Status::Dropped {
                    milestones.total += 1;
                    if task.status == Status::Done {
                        milestones.done += 1;
                    }
                }
            }
            EntryKind::Question => {
                if !matches!(task.status, Status::Done | Status::Dropped) {
                    open_questions += 1;
                }
            }
            EntryKind::Idea => {}
        }
    }

    let stamp_version = doc.tool_stamp.as_ref().and_then(|stamp| stamp.version);
    let needs_migration = doc.effective_format_version() < CURRENT_FORMAT_VERSION
        || stamp_version.is_none_or(|version| version < RUNNING_VERSION);

    Ok(Figures {
        format_version: doc.effective_format_version(),
        tool_stamp: doc.tool_stamp.as_ref().map(|stamp| stamp.value.clone()),
        needs_migration,
        tasks,
        milestones,
        stale_claims,
        open_questions,
    })
}
