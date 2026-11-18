//! Shared read loading and JSON serialization helpers for task commands.

use std::collections::BTreeMap;
use std::io;
use std::path::Path;

use chrono::{DateTime, FixedOffset};
use serde::Serialize;

use crate::atomic::{read_file_with_snapshot, validate_utf8};
use crate::cli::Context;
use crate::doc::{Document, IdConfig, Status, Task, TaskId};
use crate::error::{ErrorCode, Result, TodoError};
use crate::find::{FoundTodo, fs_path};
use crate::ops::config::OpsConfig;
use crate::output::Output;

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

/// Parses an optional archive file if it exists on disk.
///
/// Returns `Ok(None)` if the file does not exist (`NotFound`).
pub fn read_optional_archive(archive_path: &Path, id_config: IdConfig) -> Result<Option<Document>> {
    match std::fs::read(fs_path(archive_path).as_ref()) {
        Ok(bytes) => {
            let label = file_label(archive_path);
            let _ = validate_utf8(&bytes, &label)?;
            let doc = Document::parse_bytes_with_config(&bytes, id_config).map_err(|e| {
                TodoError::new(ErrorCode::UnsafeFile, e.to_string())
                    .with_file(label)
                    .with_line(e.line)
            })?;
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

/// Loads `TODO.md` (or `TODO-archive.md` when `from_archive` is `true`) without taking any lock (`R-LOCK-5`).
///
/// Emits any parse problems/warnings on `out` per `R-EDIT-8` and `docs/format.md` §13 so read
/// commands still work on files with write-blocking problems.
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
    load_for_read_inner(found, cfg, from_archive, load_archive_for_deps, out)
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
    load_for_read_inner(found, cfg, from_archive, load_archive_for_deps, out)
}

fn load_for_read_inner(
    found: FoundTodo,
    cfg: &OpsConfig,
    from_archive: bool,
    load_archive_for_deps: bool,
    out: &mut Output,
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

/// Returns `true` if `task` is eligible to be handed out by `next` / `list --ready` (`R-READ-3`).
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
    if is_question(task) {
        return false;
    }
    doc.dependencies_satisfied(task, archive)
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
    /// Task type, if present.
    #[serde(rename = "type")]
    pub task_type: Option<String>,
    /// Task area, if present.
    pub area: Option<String>,
    /// Canonicalized or raw dependency strings from `- depends:`.
    pub depends: Vec<String>,
    /// Detailed dependency statuses (`R-DEP-2`).
    pub dependencies: Vec<DependencyStatusJson>,
    /// Tasks waiting on this task (`R-DEP-2`).
    pub dependents: Vec<DependentTaskJson>,
    /// Questions whose `about` field names this entry, in file order (`R-KIND-2`).
    pub questions: Vec<QuestionAboutJson>,
    /// Whether the task is ready for `next`.
    pub ready: bool,
    /// `- files:` value, if present.
    pub files: Option<String>,
    /// `- problem:` value, if present.
    pub problem: Option<String>,
    /// `- fix:` value, if present.
    pub fix: Option<String>,
    /// `- accept:` value, if present.
    pub accept: Option<String>,
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
    /// Task type, if present.
    #[serde(rename = "type")]
    pub task_type: Option<String>,
    /// Task area, if present.
    pub area: Option<String>,
    /// Dependency tokens from `- depends:`.
    pub depends: Vec<String>,
    /// Whether the task is ready (`next` could hand it out).
    pub ready: bool,
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
    let field = task.unknown_fields.iter().find(|f| f.key == "about")?;
    cfg.id_config.parse_user_id(&field.value)
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
        task_type: task.task_type.clone(),
        area: task.area.clone(),
        depends: task.depends.iter().map(|d| d.raw.clone()).collect(),
        dependencies,
        dependents,
        questions,
        ready: is_task_ready(primary, task, ready_archive, now, cfg),
        files: task.files.clone(),
        problem: task.problem.clone(),
        fix: task.fix.clone(),
        accept: task.accept.clone(),
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
    ListTaskJson {
        id: cfg.id_config.format_id(task.id),
        title: task.title.clone(),
        section: doc.task_section_name(task).map(str::to_string),
        status: task.status.as_str().to_string(),
        owner: task.owner.clone(),
        claimed: task.claimed.clone(),
        stale: cfg.is_claim_stale(task, now),
        priority: task.priority.clone(),
        task_type: task.task_type.clone(),
        area: task.area.clone(),
        depends: task.depends.iter().map(|d| d.raw.clone()).collect(),
        ready: is_task_ready(doc, task, archive, now, cfg),
        fields,
        line_start: task.block_range.start + 1,
        line_end: task.block_range.end,
    }
}
