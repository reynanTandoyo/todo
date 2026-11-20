//! The planner's list (T-149; R-LEVEL-2): what the planner still has to give back, split, detail,
//! level or check before every task can go to a worker.
//!
//! `todo plan` and the planner's line of `todo summary` both read this module, so each task is put
//! in its group here and nowhere else. The rules are the ones the other commands use: the target
//! level ([`OpsConfig::is_above_target`], [`OpsConfig::is_at_or_below_target`]), the detail rule
//! ([`detail_gaps`]), the given-back rule ([`Task::is_given_back`]), the missing level ([`lacks_level`])
//! and the unverified done task ([`awaits_verified`]).
//!
//! Each task is listed in the first group it fits, in this order, so the groups do not overlap.

use crate::doc::{Document, EntryKind, Status, Task, TaskId};
use crate::ops::config::OpsConfig;
use crate::ops::read::lacks_level;
use crate::ops::verify::awaits_verified;
use crate::ops::waves::detail_gaps;

/// One group of the planner's list (`R-LEVEL-2`), in the order `todo plan` shows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    /// Given back to the planner (`R-STUCK-1`); listed first, with its problem.
    GivenBack,
    /// Above the target level (`R-LEVEL-2`, group 2); listed hardest first.
    ToSplit,
    /// At or below the target level, not ready for lack of detail (group 3).
    ToDetail,
    /// Without a difficulty (group 4).
    WithoutLevel,
    /// Kept above the target level on purpose with `final` (group 5); counted apart.
    KeptOnPurpose,
    /// Done, with a `verify` command and no `verified` line (group 6); counted apart.
    Unverified,
}

impl Group {
    /// Whether the group is counted apart: it does not change the exit code of `todo plan` (R-LEVEL-2).
    #[must_use]
    pub const fn is_counted_apart(self) -> bool {
        matches!(self, Self::KeptOnPurpose | Self::Unverified)
    }
}

/// The group `task` is in, or `None` when the planner has nothing to do with it. Milestones, ideas,
/// questions, claimed and dropped entries are never in a group, and neither are done tasks that
/// need no check (R-LEVEL-2).
#[must_use]
pub fn group_of(
    doc: &Document,
    archive: Option<&Document>,
    task: &Task,
    cfg: &OpsConfig,
) -> Option<Group> {
    if task.kind() != EntryKind::Task {
        return None;
    }
    if task.status == Status::Done {
        // `check` does not list these while `verify = false`, since no check can pass then.
        return (cfg.verify_enabled && awaits_verified(task)).then_some(Group::Unverified);
    }
    if !matches!(task.status, Status::Open | Status::Blocked) {
        return None;
    }
    if task.is_given_back() {
        return Some(Group::GivenBack);
    }
    if lacks_level(task, cfg) {
        return Some(Group::WithoutLevel);
    }
    if cfg.is_above_target(task.difficulty.as_deref()) {
        return Some(if task.final_reason.is_some() {
            Group::KeptOnPurpose
        } else {
            Group::ToSplit
        });
    }
    if cfg.is_at_or_below_target(task.difficulty.as_deref())
        && !detail_gaps(task, doc, archive, cfg).is_empty()
    {
        return Some(Group::ToDetail);
    }
    None
}

/// The tasks of the planner's list, by group, each group in file order except `to_split`, which
/// lists the hardest level first (`R-LEVEL-2`).
#[derive(Debug, Default)]
pub struct Plan<'a> {
    /// Group 1: given back to the planner.
    pub given_back: Vec<&'a Task>,
    /// Group 2: above the target level.
    pub to_split: Vec<&'a Task>,
    /// Group 3: at or below the target level, without its detail.
    pub to_detail: Vec<&'a Task>,
    /// Group 4: without a usable difficulty: none, or one that is not configured.
    pub without_level: Vec<&'a Task>,
    /// Group 5: kept above the target level on purpose.
    pub kept_on_purpose: Vec<&'a Task>,
    /// Group 6: done tasks whose `verify` command has not passed.
    pub unverified: Vec<&'a Task>,
}

impl<'a> Plan<'a> {
    /// Whether groups 1 to 4 are empty, which is when `todo plan` exits 0 (R-LEVEL-2).
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.given_back.is_empty()
            && self.to_split.is_empty()
            && self.to_detail.is_empty()
            && self.without_level.is_empty()
    }
}

/// Builds the planner's list of `doc`, reading `archive` for the questions that hold a task back.
///
/// With `milestone`, only the entries below it, at any depth, are listed; the groups are otherwise
/// the same.
#[must_use]
pub fn build_plan<'a>(
    doc: &'a Document,
    archive: Option<&Document>,
    cfg: &OpsConfig,
    milestone: Option<TaskId>,
) -> Plan<'a> {
    let mut plan = Plan::default();
    for task in &doc.tasks {
        if let Some(m) = milestone
            && !doc.milestone_ancestors(task).iter().any(|a| a.id == m)
        {
            continue;
        }
        let Some(group) = group_of(doc, archive, task, cfg) else {
            continue;
        };
        let list = match group {
            Group::GivenBack => &mut plan.given_back,
            Group::ToSplit => &mut plan.to_split,
            Group::ToDetail => &mut plan.to_detail,
            Group::WithoutLevel => &mut plan.without_level,
            Group::KeptOnPurpose => &mut plan.kept_on_purpose,
            Group::Unverified => &mut plan.unverified,
        };
        list.push(task);
    }
    // The hardest level first; the sort is stable, so equal levels keep their file order.
    plan.to_split
        .sort_by_key(|t| std::cmp::Reverse(cfg.difficulty_rank(t.difficulty.as_deref())));
    plan
}
