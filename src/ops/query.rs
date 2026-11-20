//! Task filtering and `next` task selection (`R-READ-1`, `R-READ-3`).

use std::collections::BTreeSet;

use chrono::{DateTime, FixedOffset};

use crate::doc::{
    Document, EntryKind, Section, Status, Task, TaskId, agent_names_equal, is_nobody_owner,
    strip_section_number,
};
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::config::OpsConfig;
use crate::ops::read::is_task_ready;
use crate::ops::scope::files_busy;

/// Parsed `--status` filter for `todo list` (`R-READ-1`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusFilter {
    /// Include tasks whose status is in the given list.
    Only(Vec<Status>),
    /// Include tasks of every status (`--status all`).
    All,
}

impl StatusFilter {
    /// Parses `--status` input (comma-separated statuses or `all`).
    pub fn parse(raw: &str) -> Result<Self> {
        let trimmed = raw.trim();
        if trimmed.eq_ignore_ascii_case("all") {
            return Ok(Self::All);
        }
        let mut statuses = Vec::new();
        for part in trimmed.split(',') {
            let token = part.trim().to_ascii_lowercase();
            if token.is_empty() {
                continue;
            }
            if token == "all" {
                return Ok(Self::All);
            }
            let Some(st) = Status::parse(&token) else {
                return Err(TodoError::new(
                    ErrorCode::BadValue,
                    format!(
                        "unknown status '{token}': expected open, claimed, blocked, done, dropped, or all"
                    ),
                ));
            };
            if !statuses.contains(&st) {
                statuses.push(st);
            }
        }
        if statuses.is_empty() {
            return Err(TodoError::new(
                ErrorCode::BadValue,
                "empty --status filter: expected open, claimed, blocked, done, dropped, or all",
            ));
        }
        Ok(Self::Only(statuses))
    }

    /// Default filter for `TODO.md` (`R-READ-1`): hides `done` and `dropped`.
    #[must_use]
    pub fn default_active() -> Self {
        Self::Only(vec![Status::Open, Status::Claimed, Status::Blocked])
    }

    /// Returns `true` if `status` passes this filter.
    #[must_use]
    pub fn matches(&self, status: Status) -> bool {
        match self {
            Self::All => true,
            Self::Only(list) => list.contains(&status),
        }
    }
}

/// A parsed `--where key=value` predicate (`R-READ-1`, `R-READ-3`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhereClause {
    /// Field name (trimmed).
    pub key: String,
    /// Expected field value (trimmed).
    pub value: String,
}

impl WhereClause {
    /// Parses a `key=value` string.
    pub fn parse(raw: &str) -> Result<Self> {
        let Some((k, v)) = raw.split_once('=') else {
            return Err(TodoError::new(
                ErrorCode::Usage,
                format!("invalid --where clause '{raw}': expected key=value"),
            ));
        };
        let key = k.trim();
        if key.is_empty() {
            return Err(TodoError::new(
                ErrorCode::Usage,
                format!("invalid --where clause '{raw}': empty field key"),
            ));
        }
        Ok(Self {
            key: key.to_string(),
            value: v.trim().to_string(),
        })
    }

    /// Returns `true` if `task` satisfies this predicate (case-insensitive value comparison).
    #[must_use]
    pub fn matches(&self, task: &Task) -> bool {
        if self.key.eq_ignore_ascii_case("owner") && is_nobody_owner(&self.value) {
            return task.owner.is_none();
        }
        task.field_value(&self.key)
            .is_some_and(|val| val.eq_ignore_ascii_case(&self.value))
    }
}

/// Whether `task` has a difficulty that is one of `list` (ignoring case; `R-WRITE-18`).
fn has_difficulty_in(task: &Task, list: &[String]) -> bool {
    task.difficulty
        .as_deref()
        .is_some_and(|d| list.iter().any(|w| w.eq_ignore_ascii_case(d)))
}

/// Filters for `todo list` (`R-READ-1`).
#[derive(Debug, Clone, Default)]
pub struct ListFilter<'a> {
    /// Status filter (`None` uses default: active tasks for `TODO.md`, all for `--archive`).
    pub status: Option<StatusFilter>,
    /// Match priority ignoring case.
    pub priority: Option<&'a str>,
    /// Match section name (ignoring case and leading section numbers).
    pub section: Option<&'a str>,
    /// Match `type` ignoring case.
    pub task_type: Option<&'a str>,
    /// Match `area` ignoring case.
    pub area: Option<&'a str>,
    /// Match any of these `difficulty` values (`--difficulty`; configured spellings).
    pub difficulties: Option<&'a [String]>,
    /// Match `owner` ignoring case (or nobody forms).
    pub owner: Option<&'a str>,
    /// Field predicates (`--where key=value`).
    pub where_clauses: &'a [WhereClause],
    /// Only tasks that `next` could hand out (`--ready`).
    pub ready_only: bool,
    /// Only tasks claimed by `mine_agent` (`--mine`).
    pub mine_agent: Option<&'a str>,
    /// Only stale claims (`--stale`).
    pub stale_only: bool,
    /// Whether listing `TODO-archive.md`.
    pub archive_mode: bool,
    /// Match any of these kinds (`--kind`, `R-KIND-3`).
    pub kinds: Option<&'a [EntryKind]>,
    /// Only entries below this milestone, at any depth (`--milestone`, `R-MILE-3`).
    pub milestone: Option<TaskId>,
}

/// Filters `doc.tasks` according to `filter` and returns matching task references in file order.
#[must_use]
pub fn filter_tasks<'a>(
    doc: &'a Document,
    archive: Option<&Document>,
    filter: &ListFilter<'_>,
    now: DateTime<FixedOffset>,
    cfg: &OpsConfig,
) -> Vec<&'a Task> {
    let default_status = if filter.archive_mode {
        StatusFilter::All
    } else {
        StatusFilter::default_active()
    };
    let status_filter = filter.status.as_ref().unwrap_or(&default_status);
    let section_matches = filter
        .section
        .map(|query| matching_section_indices(&doc.sections, query));

    doc.tasks
        .iter()
        .filter(|task| {
            if !status_filter.matches(task.status) {
                return false;
            }
            if let Some(prio) = filter.priority {
                let p = prio.trim();
                if !task
                    .priority
                    .as_deref()
                    .is_some_and(|tp| tp.eq_ignore_ascii_case(p))
                {
                    return false;
                }
            }
            if let Some(ref allowed_sections) = section_matches {
                match task.section {
                    Some(idx) if allowed_sections.contains(&idx) => {}
                    _ => return false,
                }
            }
            if let Some(ty) = filter.task_type {
                let t = ty.trim();
                if !task
                    .task_type
                    .as_deref()
                    .is_some_and(|tt| tt.eq_ignore_ascii_case(t))
                {
                    return false;
                }
            }
            if let Some(area) = filter.area {
                let a = area.trim();
                if !task
                    .area
                    .as_deref()
                    .is_some_and(|ta| ta.eq_ignore_ascii_case(a))
                {
                    return false;
                }
            }
            if let Some(list) = filter.difficulties
                && !has_difficulty_in(task, list)
            {
                return false;
            }
            if let Some(owner_q) = filter.owner {
                if is_nobody_owner(owner_q) {
                    if task.owner.is_some() {
                        return false;
                    }
                } else if !task
                    .owner
                    .as_deref()
                    .is_some_and(|to| agent_names_equal(to, owner_q.trim()))
                {
                    return false;
                }
            }
            if let Some(kinds) = filter.kinds
                && !kinds.contains(&task.kind())
            {
                return false;
            }
            if let Some(milestone) = filter.milestone
                && !doc
                    .milestone_ancestors(task)
                    .iter()
                    .any(|a| a.id == milestone)
            {
                return false;
            }
            if !filter.where_clauses.iter().all(|w| w.matches(task)) {
                return false;
            }
            if filter.ready_only && !is_task_ready(doc, task, archive, now, cfg) {
                return false;
            }
            if let Some(mine) = filter.mine_agent
                && (task.status != Status::Claimed
                    || !task
                        .owner
                        .as_deref()
                        .is_some_and(|to| agent_names_equal(to, mine)))
            {
                return false;
            }
            !filter.stale_only || cfg.is_claim_stale(task, now)
        })
        .collect()
}

/// Filters for `todo next` (`R-READ-3`).
#[derive(Debug, Clone, Default)]
pub struct NextFilter<'a> {
    /// Match section name.
    pub section: Option<&'a str>,
    /// Match `type`.
    pub task_type: Option<&'a str>,
    /// Match `area`.
    pub area: Option<&'a str>,
    /// Match any of these `difficulty` values (`--difficulty`; configured spellings).
    pub difficulties: Option<&'a [String]>,
    /// Match this `difficulty` or easier, in the configured order (`--max-difficulty`).
    pub max_difficulty: Option<&'a str>,
    /// Field predicates (`--where key=value`).
    pub where_clauses: &'a [WhereClause],
    /// Current agent name, if provided (`--agent` / `TODO_AGENT`).
    pub agent: Option<&'a str>,
    /// Skip returning `agent`'s existing claimed task first (`--new`).
    pub ignore_own_claim: bool,
    /// Only tasks below this milestone, at any depth (`--milestone`, `R-MILE-6`).
    pub milestone: Option<TaskId>,
}

/// The order `next` hands out ready tasks in (`R-READ-3`, `R-MILE-6`): priority first; then tasks
/// in a milestone where work has started (`started` from [`Document::started_milestones`]); then
/// file order. `summary` sorts its ready list with the same key.
#[must_use]
pub fn ready_sort_key(
    doc: &Document,
    task: &Task,
    started: &BTreeSet<TaskId>,
    cfg: &OpsConfig,
) -> (usize, bool, usize) {
    (
        cfg.priority_rank(task.priority.as_deref()),
        !doc.above_started_milestone(task, started),
        task.heading_line,
    )
}

/// Result of selecting the next task (`R-READ-3`).
#[derive(Debug, Clone, Copy)]
pub struct SelectedNext<'a> {
    /// The selected task.
    pub task: &'a Task,
    /// `true` if `task` is already claimed by the requesting agent.
    pub already_theirs: bool,
}

/// Picks the next task according to `R-READ-3`.
///
/// 1. If `filter.agent` is `Some(agent)` and `!filter.ignore_own_claim`, returns the highest-priority
///    (then earliest) task already claimed by `agent` that matches the attribute filters.
/// 2. Otherwise returns the highest-priority (then earliest) ready task (`open` or stale `claimed`,
///    dependencies satisfied, matching filters) whose files no other agent holds (`R-SCOPE-1`).
///
/// Only entries of kind task are returned, never a question, an idea or a milestone (`R-KIND-3`).
#[must_use]
pub fn select_next_task<'a>(
    doc: &'a Document,
    archive: Option<&Document>,
    filter: &NextFilter<'_>,
    now: DateTime<FixedOffset>,
    cfg: &OpsConfig,
) -> Option<SelectedNext<'a>> {
    let section_matches = filter
        .section
        .map(|query| matching_section_indices(&doc.sections, query));

    let matches_common_filters = |task: &Task| -> bool {
        if task.kind() != EntryKind::Task {
            return false;
        }
        if let Some(ref allowed_sections) = section_matches {
            match task.section {
                Some(idx) if allowed_sections.contains(&idx) => {}
                _ => return false,
            }
        }
        if let Some(ty) = filter.task_type
            && !task
                .task_type
                .as_deref()
                .is_some_and(|tt| tt.eq_ignore_ascii_case(ty.trim()))
        {
            return false;
        }
        if let Some(area) = filter.area
            && !task
                .area
                .as_deref()
                .is_some_and(|ta| ta.eq_ignore_ascii_case(area.trim()))
        {
            return false;
        }
        if let Some(list) = filter.difficulties
            && !has_difficulty_in(task, list)
        {
            return false;
        }
        if let Some(max) = filter.max_difficulty {
            let within = match (
                cfg.difficulty_rank(task.difficulty.as_deref()),
                cfg.difficulty_rank(Some(max)),
            ) {
                (Some(task_rank), Some(max_rank)) => task_rank <= max_rank,
                _ => false,
            };
            if !within {
                return false;
            }
        }
        if let Some(milestone) = filter.milestone
            && !doc
                .milestone_ancestors(task)
                .iter()
                .any(|m| m.id == milestone)
        {
            return false;
        }
        filter.where_clauses.iter().all(|w| w.matches(task))
    };

    if let Some(agent) = filter.agent
        && !filter.ignore_own_claim
    {
        let own = doc
            .tasks
            .iter()
            .filter(|t| {
                t.status == Status::Claimed
                    && !t.is_milestone()
                    && t.owner
                        .as_deref()
                        .is_some_and(|o| agent_names_equal(o, agent))
                    && matches_common_filters(t)
            })
            .min_by_key(|t| (cfg.priority_rank(t.priority.as_deref()), t.heading_line));
        if let Some(task) = own {
            return Some(SelectedNext {
                task,
                already_theirs: true,
            });
        }
    }

    let started = doc.started_milestones();
    // A ready task whose files another agent holds is held back (R-READ-3, R-SCOPE-1).
    let candidate = doc
        .tasks
        .iter()
        .filter(|t| {
            is_task_ready(doc, t, archive, now, cfg)
                && matches_common_filters(t)
                && files_busy(doc, t, filter.agent, now, cfg).is_empty()
        })
        .min_by_key(|t| ready_sort_key(doc, t, &started, cfg))?;

    Some(SelectedNext {
        task: candidate,
        already_theirs: false,
    })
}

/// Returns indices of sections matching `query` (case-insensitive, ignoring leading numbers).
///
/// Exact matches on full or normalized section names take precedence over substring matches.
#[must_use]
pub fn matching_section_indices(sections: &[Section], query: &str) -> Vec<usize> {
    let q_norm = strip_section_number(query).to_lowercase();
    let q_full = query.trim().to_lowercase();
    if q_norm.is_empty() && q_full.is_empty() {
        return Vec::new();
    }

    let exact: Vec<usize> = sections
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            s.name.trim().to_lowercase() == q_full || s.normalized_name().to_lowercase() == q_norm
        })
        .map(|(idx, _)| idx)
        .collect();
    if !exact.is_empty() {
        return exact;
    }

    sections
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            (!q_norm.is_empty() && s.normalized_name().to_lowercase().contains(&q_norm))
                || (!q_full.is_empty() && s.name.to_lowercase().contains(&q_full))
        })
        .map(|(idx, _)| idx)
        .collect()
}

/// Resolves a task argument that may be either an ID (`R-ID-3`) or a case-insensitive substring
/// of an active (non-done, non-dropped) task's title (`R-ID-4`).
pub fn resolve_task_in_doc<'a>(doc: &'a Document, input: &str) -> Result<&'a Task> {
    if let Some(task_id) = doc.id_config.parse_user_id(input) {
        let id_str = doc.id_config.format_id(task_id);
        return doc.task_by_id(task_id).ok_or_else(|| {
            TodoError::new(ErrorCode::NotFound, format!("task {id_str} not found"))
                .with_task(id_str)
        });
    }
    resolve_task_by_title(doc, input)
}

/// Resolves a non-ID task argument by case-insensitive title substring among tasks that are not
/// `done` or `dropped` (`R-ID-4`).
pub fn resolve_task_by_title<'a>(doc: &'a Document, input: &str) -> Result<&'a Task> {
    let needle = input.trim().to_lowercase();
    if needle.is_empty() {
        return Err(TodoError::new(
            ErrorCode::NotFound,
            "no active task matches empty title query",
        ));
    }

    let matches: Vec<&Task> = doc
        .tasks
        .iter()
        .filter(|t| {
            !matches!(t.status, Status::Done | Status::Dropped)
                && t.title.to_lowercase().contains(&needle)
        })
        .collect();

    match matches.as_slice() {
        [only] => Ok(only),
        [] => Err(TodoError::new(
            ErrorCode::NotFound,
            format!("no active task matches title {input:?}"),
        )),
        many => {
            let candidates: Vec<String> =
                many.iter().map(|t| doc.id_config.format_id(t.id)).collect();
            let listed: Vec<String> = many
                .iter()
                .map(|t| format!("{} · {}", doc.id_config.format_id(t.id), t.title))
                .collect();
            Err(TodoError::new(
                ErrorCode::Ambiguous,
                format!(
                    "title {input:?} matches {} tasks: {}",
                    many.len(),
                    listed.join("; ")
                ),
            )
            .with_candidates(candidates))
        }
    }
}
