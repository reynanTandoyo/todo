//! Centralized status transition table (`R-WRITE-9`) and ownership rules (`R-CLAIM-3`, `R-CLAIM-4`).

use chrono::{DateTime, FixedOffset};
use serde_json::Value;

use crate::doc::{EntryKind, Status, Task, agent_names_equal};
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::config::OpsConfig;

/// Commands that transition task status according to the `R-WRITE-9` table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusCommand {
    /// `todo claim`
    Claim,
    /// `todo release`
    Release,
    /// `todo block`
    Block,
    /// `todo unblock`
    Unblock,
    /// `todo done`
    Done,
    /// `todo drop`
    Drop,
    /// `todo reopen`
    Reopen,
    /// `todo stuck` (`R-STUCK-1`): a claimed task is given back to the planner.
    Stuck,
}

impl StatusCommand {
    /// Returns the CLI command name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Claim => "claim",
            Self::Release => "release",
            Self::Block => "block",
            Self::Unblock => "unblock",
            Self::Done => "done",
            Self::Drop => "drop",
            Self::Reopen => "reopen",
            Self::Stuck => "stuck",
        }
    }
}

/// Any task-mutating command subject to `R-CLAIM-3` ownership checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskCommand {
    /// A status-transition command from `R-WRITE-9`.
    Status(StatusCommand),
    /// `todo note` (always allowed on any task in any status by any agent, `R-WRITE-7`).
    Note,
    /// `todo set` (`R-WRITE-8`).
    Set,
    /// `todo move` (`R-WRITE-10`).
    Move,
    /// `todo answer` (`R-WRITE-11`).
    Answer,
    /// `todo promote` (`R-KIND-1`).
    Promote,
}

impl TaskCommand {
    /// Returns the CLI command name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Status(s) => s.as_str(),
            Self::Note => "note",
            Self::Set => "set",
            Self::Move => "move",
            Self::Answer => "answer",
            Self::Promote => "promote",
        }
    }
}

/// Refuses `promote` of an entry that is not an idea (`R-KIND-1`): `BAD_TRANSITION`, exit 1.
pub fn check_promotable(task: &Task, id_str: &str) -> Result<()> {
    if task.kind() == EntryKind::Idea {
        return Ok(());
    }
    Err(TodoError::new(
        ErrorCode::BadTransition,
        format!("task {id_str} is not an idea; promote works only on entries with `type: idea`"),
    )
    .with_task(id_str))
}

/// Returns the status commands allowed from `status` without `--force` (`R-WRITE-9`).
#[must_use]
pub const fn allowed_commands_from(status: Status) -> &'static [StatusCommand] {
    match status {
        Status::Open => &[
            StatusCommand::Claim,
            StatusCommand::Block,
            StatusCommand::Done,
            StatusCommand::Drop,
        ],
        Status::Claimed => &[
            StatusCommand::Claim,
            StatusCommand::Release,
            StatusCommand::Block,
            StatusCommand::Done,
            StatusCommand::Drop,
            StatusCommand::Stuck,
        ],
        Status::Blocked => &[
            StatusCommand::Release,
            StatusCommand::Unblock,
            StatusCommand::Drop,
        ],
        Status::Done | Status::Dropped => &[StatusCommand::Reopen],
    }
}

/// Builds a `BAD_TRANSITION` error (exit 1) naming the current status and allowed commands (`R-WRITE-9`).
#[must_use]
pub fn bad_transition_error(id_str: &str, status: Status, cmd: StatusCommand) -> TodoError {
    let allowed = allowed_commands_from(status);
    let allowed_names: Vec<&'static str> = allowed.iter().map(|c| c.as_str()).collect();
    let allowed_joined = allowed_names.join(", ");

    let message = if status == Status::Blocked
        && matches!(cmd, StatusCommand::Claim | StatusCommand::Done)
    {
        format!(
            "cannot {} task {id_str} while status is {} without --force; allowed from {}: {allowed_joined} (or claim, done with --force)",
            cmd.as_str(),
            status.as_str(),
            status.as_str(),
        )
    } else {
        format!(
            "cannot {} task {id_str} while status is {}; allowed from {}: {allowed_joined}",
            cmd.as_str(),
            status.as_str(),
            status.as_str(),
        )
    };

    TodoError::new(ErrorCode::BadTransition, message)
        .with_task(id_str)
        .with_detail("status", Value::String(status.as_str().to_string()))
        .with_detail("command", Value::String(cmd.as_str().to_string()))
        .with_detail(
            "allowed",
            Value::Array(
                allowed_names
                    .into_iter()
                    .map(|s| Value::String(s.to_string()))
                    .collect(),
            ),
        )
}

/// Result of checking `R-CLAIM-3` ownership on a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipDecision {
    /// Operation is permitted without a forced-override note.
    Allowed,
    /// Operation overrode another agent's claim/ownership via `--force`; caller should record a note.
    Forced {
        /// Previous owner whose claim/ownership was overridden.
        previous_owner: String,
    },
}

/// Formats the canonical note text for a `--force` override (`docs/format.md` §9).
#[must_use]
pub fn forced_note_text(command: &str, previous_owner: &str) -> String {
    format!("forced {command} (owner was {previous_owner})")
}

/// Checks `R-CLAIM-3` ownership rules for `task`.
///
/// - `TaskCommand::Note` is always allowed for any agent on any status (`R-WRITE-7`).
/// - On `claimed` or `blocked` tasks owned by another agent, `--force` is required; otherwise
///   returns `NOT_OWNER` (exit 1).
pub fn check_ownership(
    task: &Task,
    cmd: TaskCommand,
    agent: &str,
    force: bool,
    id_str: &str,
) -> Result<OwnershipDecision> {
    if cmd == TaskCommand::Note {
        return Ok(OwnershipDecision::Allowed);
    }

    if matches!(task.status, Status::Claimed | Status::Blocked)
        && let Some(ref owner) = task.owner
        && !agent_names_equal(owner, agent)
    {
        if force {
            return Ok(OwnershipDecision::Forced {
                previous_owner: owner.clone(),
            });
        }
        return Err(TodoError::new(
            ErrorCode::NotOwner,
            format!(
                "task {id_str} is owned by {owner}; pass --force to {}",
                cmd.as_str()
            ),
        )
        .with_task(id_str)
        .with_owner(owner.clone()));
    }

    Ok(OwnershipDecision::Allowed)
}

/// Validates a general status transition (`block`, `unblock`, `done`, `drop`, `reopen`, `stuck`, or
/// `claim`/`release`) against `R-WRITE-9` and `R-CLAIM-3`.
///
/// Returns `(next_status, ownership_decision)`. For `claim` and `release`, prefer
/// [`plan_claim`] and [`plan_release`] which also handle stale takeovers and notes.
pub fn validate_transition(
    task: &Task,
    cmd: StatusCommand,
    agent: &str,
    force: bool,
    id_str: &str,
) -> Result<(Status, OwnershipDecision)> {
    let next_status = match (task.status, cmd) {
        (Status::Open, StatusCommand::Claim) => Status::Claimed,
        (Status::Open, StatusCommand::Block) => Status::Blocked,
        (Status::Open, StatusCommand::Done) => Status::Done,
        (Status::Open, StatusCommand::Drop) => Status::Dropped,

        (Status::Claimed, StatusCommand::Claim) => Status::Claimed,
        (Status::Claimed, StatusCommand::Release) => Status::Open,
        (Status::Claimed, StatusCommand::Block) => Status::Blocked,
        (Status::Claimed, StatusCommand::Done) => Status::Done,
        (Status::Claimed, StatusCommand::Drop) => Status::Dropped,
        (Status::Claimed, StatusCommand::Stuck) => Status::Blocked,

        (Status::Blocked, StatusCommand::Claim) if force => Status::Claimed,
        (Status::Blocked, StatusCommand::Release) => Status::Blocked,
        (Status::Blocked, StatusCommand::Unblock) => {
            if task.owner.is_some() {
                Status::Claimed
            } else {
                Status::Open
            }
        }
        (Status::Blocked, StatusCommand::Done) if force => Status::Done,
        (Status::Blocked, StatusCommand::Drop) => Status::Dropped,

        (Status::Done | Status::Dropped, StatusCommand::Reopen) => Status::Open,

        _ => return Err(bad_transition_error(id_str, task.status, cmd)),
    };

    let ownership = check_ownership(task, TaskCommand::Status(cmd), agent, force, id_str)?;
    Ok((next_status, ownership))
}

/// Planned line-level effects of `todo claim` (`R-WRITE-2`, `R-CLAIM-1..4`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimPlan {
    /// New status (always `Status::Claimed`).
    pub new_status: Status,
    /// Owner to write (preserving existing owner casing if already owned by `agent`).
    pub new_owner: String,
    /// Canonical `claimed` timestamp (`YYYY-MM-DDTHH:MM±HH:MM`).
    pub claimed_timestamp: String,
    /// Optional automatic note (stale takeover or `--force`).
    pub auto_note: Option<String>,
}

/// Computes the [`ClaimPlan`] for `todo claim` on `task` (`R-WRITE-2`, `R-WRITE-9`, `R-CLAIM-1..4`).
pub fn plan_claim(
    task: &Task,
    agent: &str,
    force: bool,
    now: DateTime<FixedOffset>,
    cfg: &OpsConfig,
) -> Result<ClaimPlan> {
    let id_str = cfg.id_config.format_id(task.id);
    if task.is_milestone() {
        // R-MILE-2: a milestone is never claimed; its tasks are.
        return Err(TodoError::new(
            ErrorCode::BadTransition,
            format!(
                "cannot claim milestone {id_str}: a milestone is never claimed; claim one of the tasks below it"
            ),
        )
        .with_task(id_str));
    }
    let claimed_timestamp = OpsConfig::format_claimed_timestamp(now);

    match task.status {
        Status::Open => Ok(ClaimPlan {
            new_status: Status::Claimed,
            new_owner: agent.to_string(),
            claimed_timestamp,
            auto_note: None,
        }),
        Status::Claimed => match task.owner.as_deref() {
            None => Ok(ClaimPlan {
                new_status: Status::Claimed,
                new_owner: agent.to_string(),
                claimed_timestamp,
                auto_note: None,
            }),
            Some(owner) if agent_names_equal(owner, agent) => Ok(ClaimPlan {
                new_status: Status::Claimed,
                new_owner: owner.to_string(),
                claimed_timestamp,
                auto_note: None,
            }),
            Some(other) => {
                if cfg.is_claim_stale(task, now) {
                    let note = match cfg.claim_age(task, now) {
                        Some(age) => {
                            let hours = age.num_hours().max(0);
                            format!("took over from {other} (claim was {hours} h old)")
                        }
                        None => {
                            format!("took over from {other} (claim had no valid timestamp)")
                        }
                    };
                    Ok(ClaimPlan {
                        new_status: Status::Claimed,
                        new_owner: agent.to_string(),
                        claimed_timestamp,
                        auto_note: Some(note),
                    })
                } else if force {
                    Ok(ClaimPlan {
                        new_status: Status::Claimed,
                        new_owner: agent.to_string(),
                        claimed_timestamp,
                        auto_note: Some(forced_note_text("claim", other)),
                    })
                } else {
                    Err(already_claimed_error(&id_str, other, task, now, cfg))
                }
            }
        },
        Status::Blocked => {
            if !force {
                return Err(bad_transition_error(
                    &id_str,
                    task.status,
                    StatusCommand::Claim,
                ));
            }
            let prev = task.owner.as_deref().unwrap_or("—");
            Ok(ClaimPlan {
                new_status: Status::Claimed,
                new_owner: agent.to_string(),
                claimed_timestamp,
                auto_note: Some(forced_note_text("claim", prev)),
            })
        }
        Status::Done | Status::Dropped => Err(bad_transition_error(
            &id_str,
            task.status,
            StatusCommand::Claim,
        )),
    }
}

/// Planned line-level effects of `todo release` (`R-WRITE-3`, `R-WRITE-9`, `R-CLAIM-3`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleasePlan {
    /// New status (`Open` for claimed tasks, `Blocked` for blocked tasks).
    pub new_status: Status,
    /// Optional automatic note when `--force` overrode another agent's ownership.
    pub auto_note: Option<String>,
}

/// Computes the [`ReleasePlan`] for `todo release` on `task` (`R-WRITE-3`, `R-WRITE-9`, `R-CLAIM-3`).
pub fn plan_release(task: &Task, agent: &str, force: bool, cfg: &OpsConfig) -> Result<ReleasePlan> {
    let id_str = cfg.id_config.format_id(task.id);
    let (new_status, ownership) =
        validate_transition(task, StatusCommand::Release, agent, force, &id_str)?;

    let auto_note = match ownership {
        OwnershipDecision::Allowed => None,
        OwnershipDecision::Forced { previous_owner } => {
            Some(forced_note_text("release", &previous_owner))
        }
    };

    Ok(ReleasePlan {
        new_status,
        auto_note,
    })
}

fn already_claimed_error(
    id_str: &str,
    owner: &str,
    task: &Task,
    now: DateTime<FixedOffset>,
    cfg: &OpsConfig,
) -> TodoError {
    let mut err = match cfg.claim_age(task, now) {
        Some(age) => {
            let total_mins = age.num_minutes().max(0);
            let hours = total_mins / 60;
            let mins = total_mins % 60;
            TodoError::new(
                ErrorCode::AlreadyClaimed,
                format!(
                    "task {id_str} is already claimed by {owner} ({hours}h {mins}m old); pass --force to override"
                ),
            )
            .with_detail("age_hours", Value::Number(hours.into()))
            .with_detail("age_minutes", Value::Number(total_mins.into()))
        }
        None => TodoError::new(
            ErrorCode::AlreadyClaimed,
            format!("task {id_str} is already claimed by {owner}; pass --force to override"),
        ),
    };
    err = err.with_task(id_str).with_owner(owner);
    if let Some(ref claimed) = task.claimed {
        err = err.with_detail("claimed_at", Value::String(claimed.clone()));
    }
    err
}
