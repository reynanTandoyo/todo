//! Shared read loading and JSON serialization helpers for task commands.

use std::collections::BTreeMap;
use std::io;
use std::path::Path;

use chrono::{DateTime, FixedOffset};
use serde::Serialize;

use crate::atomic::{read_file_with_snapshot, validate_utf8};
use crate::cli::Context;
use crate::doc::{
    CURRENT_FORMAT_VERSION, Document, EntryKind, IdConfig, Status, Task, TaskId, ToolStamp,
};
use crate::error::{ErrorCode, Result, TodoError};
use crate::find::{FoundTodo, fs_path};
use crate::ops::config::OpsConfig;
use crate::ops::scope::files_busy;
use crate::ops::waves::detail_gaps;
use crate::output::Output;
use crate::version::RUNNING_VERSION;

/// A `TODO.md` (or archive) document loaded for a read command (`R-LOCK-5`, `R-EDIT-8`).
#[derive(Debug, Clone)]
pub struct LoadedRead {
    /// Discovery metadata for `TODO.md`.
    pub found: FoundTodo,
    /// Parsed primary document (either `TODO.md` or `TODO-archive.md` when `from_archive` is set).
    pub doc: Document,
    /// Optional parsed archive document (when `TODO.md` is primary and `TODO-archive.md` exists).
    pub archive: Option<Document>,
    /// Short display label for `file:line: message` warnings/errors.
    pub file_label: String,
    /// Whether `doc` was loaded from the archive file (when present).
    pub archive_present: bool,
}

/// Returns the short filename label used in `file:line: message` diagnostics (`R-OUT-4`).
#[must_use]
pub fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "TODO.md".to_string())
}

/// Returns `true` when `doc` declares a format version newer than this todo knows (R-VER-2).
#[must_use]
pub fn format_is_newer(doc: &Document) -> bool {
    doc.declared_format_version
        .is_some_and(|v| v > CURRENT_FORMAT_VERSION)
}

/// Refuses a document whose format line names a version newer than this todo knows (R-VER-2,
/// `docs/format.md` §4). Every command that interprets or changes task content calls this on the
/// parsed document, so an old todo never hands out tasks it misreads.
///
/// `label` is the file name used in the error (`TODO.md`, `TODO-archive.md`).
pub fn require_known_format(doc: &Document, label: &str) -> Result<()> {
    if !format_is_newer(doc) {
        return Ok(());
    }
    let version = doc.declared_format_version.unwrap_or_default();
    let line = doc.format_line.map_or(1, |idx| idx + 1);
    let written_by = match &doc.tool_stamp {
        Some(ToolStamp {
            version: Some(stamp),
            ..
        }) => format!("was last written by todo {stamp}"),
        Some(ToolStamp { value, .. }) => {
            format!("has the tool line '{value}', which is not a version")
        }
        None => "has no tool line, so it counts as written by todo 0.5.0 or older".to_string(),
    };
    Err(TodoError::new(
        ErrorCode::ToolTooOld,
        format!(
            "format version {version} is newer than this todo {RUNNING_VERSION} supports; the file {written_by}. Update the todo executable first (`todo help update`)."
        ),
    )
    .with_file(label)
    .with_line(line))
}

/// [`require_known_format`] for raw bytes, for commands that check a file before parsing it for
/// their own purpose. Bytes that are not UTF-8 pass here: the command's own checks report them.
pub fn require_known_format_bytes(bytes: &[u8], label: &str, id_config: &IdConfig) -> Result<()> {
    match Document::parse_bytes_with_config(bytes, id_config.clone()) {
        Ok(doc) => require_known_format(&doc, label),
        Err(_) => Ok(()),
    }
}

/// Parses an optional archive file if it exists on disk.
///
/// Returns `Ok(None)` if the file does not exist (`NotFound`). An archive in a format version
/// newer than this todo is refused (R-VER-2).
pub fn read_optional_archive(archive_path: &Path, id_config: IdConfig) -> Result<Option<Document>> {
    match std::fs::read(fs_path(archive_path).as_ref()) {
        Ok(bytes) => {
            let label = file_label(archive_path);
            let _ = validate_utf8(&bytes, &label)?;
            let doc = Document::parse_bytes_with_config(&bytes, id_config).map_err(|e| {
                TodoError::new(ErrorCode::UnsafeFile, e.to_string())
                    .with_file(label.clone())
                    .with_line(e.line)
            })?;
            require_known_format(&doc, &label)?;
            Ok(Some(doc))
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(TodoError::new(
            ErrorCode::WriteFailed,
            format!("failed reading {}: {e}", archive_path.display()),
        )
        .with_file(archive_path.display().to_string())),
    }
}

/// What a read does with `TODO.md` when its format version is newer than this todo (R-VER-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NewerFormat {
    /// Refuse with `TOOL_TOO_OLD`: every command that interprets tasks.
    Refuse,
    /// Load it anyway, for `where`, which only shows the format and the stamp.
    Show,
}

/// Loads `TODO.md` (or `TODO-archive.md` when `from_archive` is `true`) without taking any lock (`R-LOCK-5`).
///
/// Emits any parse problems/warnings on `out` per `R-EDIT-8` and `docs/format.md` §13 so read
/// commands still work on files with write-blocking problems. A file in a format version newer
/// than this todo is refused with `TOOL_TOO_OLD` (R-VER-2).
pub fn load_for_read(
    ctx: &Context,
    cfg: &OpsConfig,
    from_archive: bool,
    load_archive_for_deps: bool,
    out: &mut Output,
) -> Result<LoadedRead> {
    let find_timer = crate::timing::PhaseTimer::start("read.find_todo");
    let found = ctx.find_todo_with_mode(cfg.worktree, out)?;
    find_timer.finish();
    load_for_read_inner(
        found,
        cfg,
        from_archive,
        load_archive_for_deps,
        out,
        NewerFormat::Refuse,
    )
}

/// Loads `TODO.md` for `todo where`, which shows the format version and the tool stamp of a file
/// that this todo cannot read (R-VER-2). The caller must not count or list its tasks when
/// [`format_is_newer`] is `true`. The archive is never loaded here.
pub fn load_for_where(ctx: &Context, cfg: &OpsConfig, out: &mut Output) -> Result<LoadedRead> {
    let found = ctx.find_todo_with_mode(cfg.worktree, out)?;
    load_for_read_inner(found, cfg, false, false, out, NewerFormat::Show)
}

/// Like [`load_for_read`], reusing an already-located [`FoundTodo`] so callers that also loaded
/// `todo.toml` avoid running file discovery twice.
pub(crate) fn load_for_read_with_found(
    found: FoundTodo,
    cfg: &OpsConfig,
    from_archive: bool,
    load_archive_for_deps: bool,
    out: &mut Output,
) -> Result<LoadedRead> {
    crate::cli::emit_find_notices(&found, out);
    load_for_read_inner(
        found,
        cfg,
        from_archive,
        load_archive_for_deps,
        out,
        NewerFormat::Refuse,
    )
}

fn load_for_read_inner(
    found: FoundTodo,
    cfg: &OpsConfig,
    from_archive: bool,
    load_archive_for_deps: bool,
    out: &mut Output,
    newer_format: NewerFormat,
) -> Result<LoadedRead> {
    if from_archive {
        let archive_path = cfg.archive_path(&found.path);
        let label = file_label(&archive_path);
        match read_optional_archive(&archive_path, cfg.id_config.clone())? {
            Some(doc) => {
                for problem in &doc.problems {
                    out.warn(problem.format_with_path(&label));
                }
                return Ok(LoadedRead {
                    found,
                    doc,
                    archive: None,
                    file_label: label,
                    archive_present: true,
                });
            }
            None => {
                let empty = Document::parse_bytes_with_config(b"", cfg.id_config.clone())
                    .map_err(|e| TodoError::new(ErrorCode::UnsafeFile, e.to_string()))?;
                return Ok(LoadedRead {
                    found,
                    doc: empty,
                    archive: None,
                    file_label: label,
                    archive_present: false,
                });
            }
        }
    }

    let label = file_label(&found.path);
    let read_timer = crate::timing::PhaseTimer::start("read.read_file_and_hash");
    let read = read_file_with_snapshot(&found.path)?;
    read_timer.finish();
    let state_timer = crate::timing::PhaseTimer::start("read.state_dir");
    let state = crate::state::StateDir::from_found(&found)?;
    state_timer.finish();
    let outside_timer = crate::timing::PhaseTimer::start("read.outside_edit");
    if let crate::history::OutsideEditOutcome::Recorded {
        warning: Some(warn),
    } =
        crate::history::detect_outside_edit(&found.path, &label, &state, &read.snapshot, &cfg.lock)?
    {
        out.warn(warn);
    }
    outside_timer.finish();
    let _ = validate_utf8(&read.bytes, &label)?;
    let parse_timer = crate::timing::PhaseTimer::start("read.parse_doc");
    let doc =
        Document::parse_bytes_with_config(&read.bytes, cfg.id_config.clone()).map_err(|e| {
            TodoError::new(ErrorCode::UnsafeFile, e.to_string())
                .with_file(label.clone())
                .with_line(e.line)
        })?;
    parse_timer.finish();
    if newer_format == NewerFormat::Refuse {
        require_known_format(&doc, &label)?;
    }

    for problem in &doc.problems {
        out.warn(problem.format_with_path(&label));
    }

    let arch_timer = crate::timing::PhaseTimer::start("read.archive");
    let archive = if load_archive_for_deps {
        let archive_path = cfg.archive_path(&found.path);
        let arch = read_optional_archive(&archive_path, cfg.id_config.clone())?;
        if let Some(ref arch_doc) = arch {
            for dup in doc.check_archive_duplicates(arch_doc) {
                out.warn(dup.format_with_path(&label));
            }
        }
        arch
    } else {
        None
    };
    arch_timer.finish();

    Ok(LoadedRead {
        found,
        doc,
        archive,
        file_label: label,
        archive_present: true,
    })
}

/// Whether a milestone above a task holds the task back: the milestone is blocked, done or dropped
/// (`R-MILE-6`). `next`, `list --ready` and `waves` all apply this one rule.
#[must_use]
pub fn milestone_holds_back(milestone: &Task) -> bool {
    matches!(
        milestone.status,
        Status::Blocked | Status::Done | Status::Dropped
    )
}

/// Whether a task has no level, so `next` and `list --ready` do not hand it out until it has one
/// (`R-LEVEL-1`). `waves` holds such a task back for the same reason.
#[must_use]
pub fn lacks_level(task: &Task) -> bool {
    task.difficulty.is_none()
}

/// Returns `true` if `task` is eligible to be handed out by `next` / `list --ready` (`R-READ-3`).
///
/// Only entries of kind task are handed out, so questions, ideas and milestones never are
/// (`R-KIND-3`).
#[must_use]
pub fn is_task_ready(
    doc: &Document,
    task: &Task,
    archive: Option<&Document>,
    now: DateTime<FixedOffset>,
    cfg: &OpsConfig,
) -> bool {
    let status_eligible = task.status == Status::Open
        || (task.status == Status::Claimed && cfg.is_claim_stale(task, now));
    if !status_eligible {
        return false;
    }
    // Only entries of kind task are handed out: never a milestone (R-MILE-2), a question
    // (R-WRITE-11) or an idea (R-KIND-1, R-KIND-3).
    if task.kind() != EntryKind::Task {
        return false;
    }
    // A task without a level is not handed out until it has one (R-LEVEL-1).
    if lacks_level(task) {
        return false;
    }
    // Work is handed out along the tree (R-MILE-6): no milestone above the task may hold it back,
    // and the dependencies of every milestone above it must be finished too.
    let above = doc.milestone_ancestors(task);
    if above.iter().any(|m| milestone_holds_back(m)) {
        return false;
    }
    // A task at or below the target level is handed out only with its detail complete, and no open
    // question about it (R-DETAIL-2); the same list `show`, `check` and `waves` report.
    doc.dependencies_satisfied(task, archive)
        && above.iter().all(|m| doc.dependencies_satisfied(m, archive))
        && detail_gaps(task, doc, archive, cfg).is_empty()
}

/// Returns `true` if the readiness of `waiting` can change when `changed` changes state: it names
/// `changed` in `depends`, a milestone above it names `changed` in `depends`, `changed` is a
/// milestone above `waiting`, or `changed` is a question about `waiting` (`R-DEP-4`, `R-MILE-6`,
/// `R-DETAIL-2`).
#[must_use]
pub fn waits_on_task(doc: &Document, waiting: &Task, changed: &Task) -> bool {
    if waiting.id == changed.id {
        return false;
    }
    let names_changed = |t: &Task| t.depends.iter().any(|d| d.id == Some(changed.id));
    let above = doc.milestone_ancestors(waiting);
    names_changed(waiting)
        || above.iter().any(|m| names_changed(m))
        || (changed.is_milestone() && above.iter().any(|m| m.id == changed.id))
        || (is_question(changed) && about_id_in(changed, &doc.id_config) == Some(waiting.id))
}

/// JSON representation of a dependency reference with its resolved status (`R-READ-2`, `R-DEP-2`).
#[derive(Debug, Clone, Serialize)]
pub struct DependencyStatusJson {
    /// Raw dependency token as written in `- depends:`.
    pub raw: String,
    /// Canonical task ID if the token parsed as a valid ID.
    pub id: Option<String>,
    /// Whether the target task was found in `TODO.md` or `TODO-archive.md`.
    pub exists: bool,
    /// Whether the target task lives in `TODO-archive.md`.
    pub archived: bool,
    /// Current status of the target task, if found.
    pub status: Option<String>,
    /// Title of the target task, if found.
    pub title: Option<String>,
    /// Whether this dependency is satisfied (`done` or `dropped`).
    pub satisfied: bool,
}

/// JSON representation of a task waiting on the inspected task (`R-READ-2`, `R-DEP-2`).
#[derive(Debug, Clone, Serialize)]
pub struct DependentTaskJson {
    /// Canonical task ID.
    pub id: String,
    /// Task title.
    pub title: String,
    /// Current status.
    pub status: String,
    /// Whether this dependent lives in `TODO-archive.md`.
    pub archived: bool,
}

/// JSON representation of a dated note (`docs/format.md` §9).
#[derive(Debug, Clone, Serialize)]
pub struct NoteJson {
    /// Calendar date (`YYYY-MM-DD`).
    pub date: String,
    /// Agent name.
    pub agent: String,
    /// Note body text.
    pub text: String,
    /// 1-based line number in the file.
    pub line: usize,
}

/// A milestone above an entry, as `show --json` lists it (`R-MILE-4`).
#[derive(Debug, Clone, Serialize)]
pub struct MilestoneRefJson {
    /// Canonical milestone ID.
    pub id: String,
    /// Milestone title.
    pub title: String,
    /// Milestone goal, if present.
    pub goal: Option<String>,
    /// Current status of the milestone.
    pub status: String,
}

/// An entry that names another entry as its milestone (`R-MILE-2`).
#[derive(Debug, Clone, Serialize)]
pub struct ChildJson {
    /// Canonical task ID.
    pub id: String,
    /// Task title.
    pub title: String,
    /// Kind of entry: `task` or `milestone`.
    pub kind: String,
    /// Current status.
    pub status: String,
}

/// A task that holds files the inspected entry names, as `show --json` and `list --json` list it
/// (`R-SCOPE-1`).
#[derive(Debug, Clone, Serialize)]
pub struct FilesBusyJson {
    /// Canonical task ID.
    pub id: String,
    /// Task title.
    pub title: String,
    /// `claimed`, or `blocked` for a task blocked with an owner.
    pub status: String,
    /// Owner agent name, or `null` when unowned.
    pub owner: Option<String>,
}

/// The tasks that hold files `task` names, as [`FilesBusyJson`] entries in file order.
#[must_use]
pub fn files_busy_json(busy: &[&Task], cfg: &OpsConfig) -> Vec<FilesBusyJson> {
    busy.iter()
        .map(|t| FilesBusyJson {
            id: cfg.id_config.format_id(t.id),
            title: t.title.clone(),
            status: t.status.as_str().to_string(),
            owner: t.owner.clone(),
        })
        .collect()
}

/// Full JSON representation of a task (`show`, `next`, and write commands).
#[derive(Debug, Clone, Serialize)]
pub struct TaskJson {
    /// Canonical task ID (e.g. `"T-001"`).
    pub id: String,
    /// Task title.
    pub title: String,
    /// Section name, if the task is under a `##` section.
    pub section: Option<String>,
    /// Whether the task lives in the archive file.
    pub archived: bool,
    /// Effective status.
    pub status: String,
    /// Owner agent name, or `null` when unowned.
    pub owner: Option<String>,
    /// Raw `claimed` timestamp string, if present.
    pub claimed: Option<String>,
    /// Whether the claim is currently stale (`R-CLAIM-4`).
    pub stale: bool,
    /// Task priority, if present.
    pub priority: Option<String>,
    /// Task difficulty, if present (`R-WRITE-18`).
    pub difficulty: Option<String>,
    /// Task type, if present.
    #[serde(rename = "type")]
    pub task_type: Option<String>,
    /// Task area, if present.
    pub area: Option<String>,
    /// Goal of a milestone, if present (`R-MILE-1`).
    pub goal: Option<String>,
    /// Parent milestone as written in `- milestone:`, if present (`R-MILE-1`).
    pub milestone: Option<String>,
    /// The milestones above this entry, from the top down (`R-MILE-4`).
    pub milestones: Vec<MilestoneRefJson>,
    /// The entries whose milestone is this entry (`R-MILE-2`).
    pub children: Vec<ChildJson>,
    /// `- why:` value, if present: what the task is for (R-WRITE-17).
    pub why: Option<String>,
    /// `- final:` value, if present: why the task stays above the target level (R-LEVEL-3).
    #[serde(rename = "final")]
    pub final_reason: Option<String>,
    /// Canonicalized or raw dependency strings from `- depends:`.
    pub depends: Vec<String>,
    /// Detailed dependency statuses (`R-DEP-2`).
    pub dependencies: Vec<DependencyStatusJson>,
    /// Tasks waiting on this task (`R-DEP-2`).
    pub dependents: Vec<DependentTaskJson>,
    /// Questions whose `about` field names this entry, in file order (`R-KIND-2`).
    pub questions: Vec<QuestionAboutJson>,
    /// Whether the task is ready for `next`: nothing holds files it names (`R-SCOPE-1`), and its
    /// detail is complete when it is at or below the target level (`R-DETAIL-2`).
    pub ready: bool,
    /// What the detail rule lacks (`R-DETAIL-2`), in a fixed order; `[]` when complete, or when the
    /// task is above the target level. `ready` is `false` while anything is listed.
    pub missing: Vec<String>,
    /// The tasks of other agents that hold files this task names (`R-SCOPE-1`); `[]` when none.
    pub files_busy: Vec<FilesBusyJson>,
    /// `- files:` value, if present.
    pub files: Option<String>,
    /// `- read:` paths, split at commas: what the worker should look at but not change (R-DETAIL-1).
    pub read: Vec<String>,
    /// `- problem:` value, if present.
    pub problem: Option<String>,
    /// `- fix:` value, if present.
    pub fix: Option<String>,
    /// `- steps:` items in order, without their numbers (R-DETAIL-1); empty when absent.
    pub steps: Vec<String>,
    /// `- accept:` value, if present.
    pub accept: Option<String>,
    /// `- verify:` value, if present: the command that checks the task (R-DETAIL-1).
    pub verify: Option<String>,
    /// `- attempts:` value, if present: how often the task failed (R-STUCK-1).
    pub attempts: Option<String>,
    /// `- verified:` value, if present: the date the `verify` command last passed (R-VERIFY-2).
    pub verified: Option<String>,
    /// Commit hashes from `- commit:`.
    pub commit: Vec<String>,
    /// Unknown `- key: value` fields kept as written (`docs/format.md` §8).
    pub fields: BTreeMap<String, String>,
    /// Dated notes in order of appearance.
    pub notes: Vec<NoteJson>,
    /// 1-based starting line number of the task block.
    pub line_start: usize,
    /// 1-based ending line number of the task block (inclusive).
    pub line_end: usize,
}

/// JSON representation of a question whose `about` field names the inspected entry (`R-KIND-2`).
#[derive(Debug, Clone, Serialize)]
pub struct QuestionAboutJson {
    /// Canonical question ID.
    pub id: String,
    /// Question text.
    pub title: String,
    /// Current status.
    pub status: String,
    /// Whether this question lives in `TODO-archive.md`.
    pub archived: bool,
}

/// Summary JSON representation of a task used in `list --json`.
#[derive(Debug, Clone, Serialize)]
pub struct ListTaskJson {
    /// Canonical task ID (e.g. `"T-001"`).
    pub id: String,
    /// Task title.
    pub title: String,
    /// Section name, if any.
    pub section: Option<String>,
    /// Effective status.
    pub status: String,
    /// Owner agent name, or `null` when unowned.
    pub owner: Option<String>,
    /// Raw `claimed` timestamp string, if present.
    pub claimed: Option<String>,
    /// Whether the claim is currently stale.
    pub stale: bool,
    /// Task priority, if present.
    pub priority: Option<String>,
    /// Task difficulty, if present (`R-WRITE-18`).
    pub difficulty: Option<String>,
    /// Task type, if present.
    #[serde(rename = "type")]
    pub task_type: Option<String>,
    /// Task area, if present.
    pub area: Option<String>,
    /// Dependency tokens from `- depends:`.
    pub depends: Vec<String>,
    /// Whether the task is ready (`next` could hand it out, files included).
    pub ready: bool,
    /// The tasks of other agents that hold files this task names (`R-SCOPE-1`); `[]` when none.
    pub files_busy: Vec<FilesBusyJson>,
    /// Unknown `- key: value` fields (`docs/format.md` §8).
    pub fields: BTreeMap<String, String>,
    /// 1-based starting line number of the task block.
    pub line_start: usize,
    /// 1-based ending line number of the task block (inclusive).
    pub line_end: usize,
}

/// Whether `task` is a question (`type: question`, any case; `docs/format.md` §16).
#[must_use]
pub fn is_question(task: &Task) -> bool {
    task.task_type
        .as_deref()
        .is_some_and(|t| t.eq_ignore_ascii_case("question"))
}

/// The entry `task` names in its `about` field, as an ID (`R-KIND-2`). `None` when the field is
/// absent or does not hold an ID.
#[must_use]
pub fn about_id(task: &Task, cfg: &OpsConfig) -> Option<TaskId> {
    about_id_in(task, &cfg.id_config)
}

/// [`about_id`] with only the ID format, for code that has a document but no configuration.
#[must_use]
pub fn about_id_in(task: &Task, ids: &IdConfig) -> Option<TaskId> {
    let field = task.unknown_fields.iter().find(|f| f.key == "about")?;
    ids.parse_user_id(&field.value)
}

/// The questions whose `about` field names `target` (`R-KIND-2`), in file order, looking first in
/// `primary` and then in `secondary`.
///
/// `archived` means `primary` is `TODO-archive.md`, as for [`build_task_json`].
#[must_use]
pub fn questions_about(
    primary: &Document,
    secondary: Option<&Document>,
    archived: bool,
    target: TaskId,
    cfg: &OpsConfig,
) -> Vec<QuestionAboutJson> {
    let mut found = Vec::new();
    for (doc, is_archive) in [(Some(primary), archived), (secondary, !archived)] {
        let Some(doc) = doc else { continue };
        for candidate in &doc.tasks {
            if is_question(candidate) && about_id(candidate, cfg) == Some(target) {
                found.push(QuestionAboutJson {
                    id: cfg.id_config.format_id(candidate.id),
                    title: candidate.title.clone(),
                    status: candidate.status.as_str().to_string(),
                    archived: is_archive,
                });
            }
        }
    }
    found
}

/// The milestones above `task` in `doc`, from the top down (`R-MILE-4`): each with its ID, title,
/// goal and status. Empty for an entry without a milestone.
#[must_use]
pub fn milestone_chain_json(doc: &Document, task: &Task, cfg: &OpsConfig) -> Vec<MilestoneRefJson> {
    doc.milestone_ancestors(task)
        .into_iter()
        .rev()
        .map(|m| MilestoneRefJson {
            id: cfg.id_config.format_id(m.id),
            title: m.title.clone(),
            goal: m.goal.clone(),
            status: m.status.as_str().to_string(),
        })
        .collect()
}

/// Builds a [`TaskJson`] for `task`, resolving dependencies and dependents across `primary` and `secondary`.
///
/// When `archived` is `false`, `primary` is `TODO.md` and `secondary` is `TODO-archive.md`.
/// When `archived` is `true`, `primary` is `TODO-archive.md` and `secondary` is `TODO.md`.
#[must_use]
pub fn build_task_json(
    primary: &Document,
    task: &Task,
    secondary: Option<&Document>,
    archived: bool,
    now: DateTime<FixedOffset>,
    cfg: &OpsConfig,
) -> TaskJson {
    let dependencies =
        task.depends
            .iter()
            .map(|dep| {
                let id_str = dep.id.map(|id| cfg.id_config.format_id(id));
                let resolved = dep.id.and_then(|id| {
                    primary.task_by_id(id).map(|t| (t, archived)).or_else(|| {
                        secondary.and_then(|s| s.task_by_id(id).map(|t| (t, !archived)))
                    })
                });
                match resolved {
                    Some((target, is_arch)) => DependencyStatusJson {
                        raw: dep.raw.clone(),
                        id: id_str,
                        exists: true,
                        archived: is_arch,
                        status: Some(target.status.as_str().to_string()),
                        title: Some(target.title.clone()),
                        satisfied: target.status.is_finished(),
                    },
                    None => DependencyStatusJson {
                        raw: dep.raw.clone(),
                        id: id_str,
                        exists: false,
                        archived: false,
                        status: None,
                        title: None,
                        satisfied: false,
                    },
                }
            })
            .collect();

    let mut dependents = Vec::new();
    let active_doc = if archived { secondary } else { Some(primary) };
    if let Some(active) = active_doc {
        for candidate in &active.tasks {
            if candidate.id != task.id && candidate.depends.iter().any(|d| d.id == Some(task.id)) {
                dependents.push(DependentTaskJson {
                    id: cfg.id_config.format_id(candidate.id),
                    title: candidate.title.clone(),
                    status: candidate.status.as_str().to_string(),
                    archived: false,
                });
            }
        }
    }

    let questions = questions_about(primary, secondary, archived, task.id, cfg);

    let fields = task
        .unknown_fields
        .iter()
        .map(|f| (f.key.clone(), f.value.clone()))
        .collect();

    let notes = task
        .notes
        .iter()
        .map(|n| NoteJson {
            date: n.date_raw.clone(),
            agent: n.agent.clone(),
            text: n.text.clone(),
            line: n.line + 1,
        })
        .collect();

    let ready_archive = if archived { None } else { secondary };
    // Questions about the task may sit in either file, so both are searched (R-DETAIL-2).
    let missing = detail_gaps(task, primary, secondary, cfg);
    // A finished entry in the archive is not held back by the files of active tasks (R-SCOPE-1).
    let busy = if archived {
        Vec::new()
    } else {
        files_busy(primary, task, task.owner.as_deref(), now, cfg)
    };

    let milestones = milestone_chain_json(primary, task, cfg);
    let children = primary
        .children_of(task.id)
        .into_iter()
        .map(|c| ChildJson {
            id: cfg.id_config.format_id(c.id),
            title: c.title.clone(),
            kind: c.kind().as_str().to_string(),
            status: c.status.as_str().to_string(),
        })
        .collect();

    TaskJson {
        id: cfg.id_config.format_id(task.id),
        title: task.title.clone(),
        section: primary.task_section_name(task).map(str::to_string),
        archived,
        status: task.status.as_str().to_string(),
        owner: task.owner.clone(),
        claimed: task.claimed.clone(),
        stale: cfg.is_claim_stale(task, now),
        priority: task.priority.clone(),
        difficulty: task.difficulty.clone(),
        task_type: task.task_type.clone(),
        area: task.area.clone(),
        goal: task.goal.clone(),
        milestone: task.milestone.clone(),
        milestones,
        children,
        why: task.why.clone(),
        final_reason: task.final_reason.clone(),
        depends: task.depends.iter().map(|d| d.raw.clone()).collect(),
        dependencies,
        dependents,
        questions,
        ready: is_task_ready(primary, task, ready_archive, now, cfg) && busy.is_empty(),
        missing,
        files_busy: files_busy_json(&busy, cfg),
        files: task.files.clone(),
        read: task.read_paths(),
        problem: task.problem.clone(),
        fix: task.fix.clone(),
        steps: task.steps.clone(),
        accept: task.accept.clone(),
        verify: task.verify.clone(),
        attempts: task.attempts.clone(),
        verified: task.verified.clone(),
        commit: task.commit.clone(),
        fields,
        notes,
        line_start: task.block_range.start + 1,
        line_end: task.block_range.end,
    }
}

/// Builds a [`ListTaskJson`] summary for `task`.
#[must_use]
pub fn build_list_task_json(
    doc: &Document,
    task: &Task,
    archive: Option<&Document>,
    now: DateTime<FixedOffset>,
    cfg: &OpsConfig,
) -> ListTaskJson {
    let fields = task
        .unknown_fields
        .iter()
        .map(|f| (f.key.clone(), f.value.clone()))
        .collect();
    let busy = files_busy(doc, task, task.owner.as_deref(), now, cfg);
    ListTaskJson {
        id: cfg.id_config.format_id(task.id),
        title: task.title.clone(),
        section: doc.task_section_name(task).map(str::to_string),
        status: task.status.as_str().to_string(),
        owner: task.owner.clone(),
        claimed: task.claimed.clone(),
        stale: cfg.is_claim_stale(task, now),
        priority: task.priority.clone(),
        difficulty: task.difficulty.clone(),
        task_type: task.task_type.clone(),
        area: task.area.clone(),
        depends: task.depends.iter().map(|d| d.raw.clone()).collect(),
        ready: is_task_ready(doc, task, archive, now, cfg) && busy.is_empty(),
        files_busy: files_busy_json(&busy, cfg),
        fields,
        line_start: task.block_range.start + 1,
        line_end: task.block_range.end,
    }
}
