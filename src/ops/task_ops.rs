//! One implementation of every single-task edit, shared by the single commands and `todo apply`.
//!
//! [`TaskOp`] names the operation with its already-validated parameters; [`TaskOp::build_edits`]
//! turns it into whole-line [`LineEdit`]s for one task. `todo claim`, `todo done`, ... and the
//! matching `todo apply` operations all call it, so a rule such as the forced-note audit line
//! exists once. Argument parsing, empty-text checks and output stay with the callers.

use chrono::{DateTime, FixedOffset};
use serde::Deserialize;

use crate::doc::{Document, LineEdit, Status, Task};
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::milestone;
use crate::ops::pipeline::{build_claim_edits_with_note, build_release_edits};
use crate::ops::rules::{
    OwnershipDecision, StatusCommand, TaskCommand, check_ownership, check_promotable,
    forced_note_text, plan_claim, plan_release, validate_transition,
};
use crate::ops::scope::check_claim_files;
use crate::ops::{OpsConfig, TaskEditBuilder};

/// Who performs an operation, when, and with which settings.
#[derive(Clone, Copy)]
pub struct OpEnv<'a> {
    /// Settings loaded from `todo.toml`.
    pub cfg: &'a OpsConfig,
    /// Agent performing the change.
    pub agent: &'a str,
    /// Whether `--force` was given.
    pub force: bool,
    /// Time used for notes and claim timestamps.
    pub now: DateTime<FixedOffset>,
    /// Agent session written on new claims (`R-CLAIM-7`), if known.
    pub session: Option<&'a str>,
}

/// One change to apply with `set` (applied in the order given).
#[derive(Debug, Clone, Copy)]
pub enum SetChange<'a> {
    /// Replace the title.
    Title(&'a str),
    /// Set a field.
    Field(&'a str, &'a str),
    /// Remove a field.
    Unset(&'a str),
}

/// A single-task edit with its parameters (text already resolved and checked by the caller).
#[derive(Debug, Clone, Copy)]
pub enum TaskOp<'a> {
    /// Claim the task (`R-WRITE-2`).
    Claim,
    /// Release the claim.
    Release,
    /// Mark done, optionally with a note and a `commit` field.
    Done {
        /// Optional note text.
        note: Option<&'a str>,
        /// Optional commit value for the `commit` field.
        commit: Option<&'a str>,
        /// Optional `by` value: the person behind the work (`R-WRITE-16`).
        by: Option<&'a str>,
    },
    /// Drop the task with a reason. A milestone with unfinished entries below it is refused
    /// (R-MILE-2); `drop --tree` drops those entries first.
    Drop {
        /// Reason text.
        reason: &'a str,
    },
    /// Drop a milestone as part of `drop --tree` (R-MILE-2): as [`TaskOp::Drop`], without the
    /// check for unfinished entries below it.
    DropTree {
        /// Reason text.
        reason: &'a str,
    },
    /// Drop an entry below `milestone` as part of `drop --tree`; the note names the milestone.
    DropWithin {
        /// Canonical ID of the milestone being dropped.
        milestone: &'a str,
        /// Reason text.
        reason: &'a str,
    },
    /// Block the task with a reason.
    Block {
        /// Reason text.
        reason: &'a str,
    },
    /// Unblock the task, optionally with a note.
    Unblock {
        /// Optional note text.
        note: Option<&'a str>,
    },
    /// Give a claimed task back to the planner with the problem (`R-STUCK-1`): blocked, no owner
    /// and no claim, one more `attempts`, the note `stuck — <problem>`.
    Stuck {
        /// The problem text, required and non-empty.
        reason: &'a str,
    },
    /// Reopen a done or dropped task, optionally with a note.
    Reopen {
        /// Optional note text.
        note: Option<&'a str>,
    },
    /// Answer a question task (marks it done). The note reads `answer — <text>` (format.md §9).
    Answer {
        /// Answer text.
        text: &'a str,
    },
    /// Turn an idea into a task (`R-KIND-1`): removes `type: idea`, sets the priority and, when
    /// given, the milestone, and adds the note `promoted from idea`. The milestone is an ID the
    /// caller has already checked.
    Promote {
        /// Configured priority, canonical spelling.
        priority: &'a str,
        /// Canonical ID of the parent milestone, if any.
        milestone: Option<&'a str>,
    },
    /// Append a note.
    Note {
        /// Note text.
        text: &'a str,
    },
    /// Change title and fields.
    Set {
        /// Changes in order.
        changes: &'a [SetChange<'a>],
    },
}

/// `verb`, or `verb — text` for a non-empty trimmed `text`.
fn note_with(verb: &str, text: Option<&str>) -> String {
    match text.map(str::trim).filter(|s| !s.is_empty()) {
        Some(t) => format!("{verb} \u{2014} {t}"),
        None => verb.to_string(),
    }
}

/// The `attempts` value after one more failure (`R-STUCK-1`): 1 when the task has no `attempts`
/// line yet, else the number written plus one. A value that is not a whole number is refused
/// (`BAD_VALUE`) rather than reset, so no failure count is lost silently.
fn attempts_after_failure(task: &Task, id_str: &str) -> Result<u64> {
    let Some(raw) = task.field_value("attempts") else {
        return Ok(1);
    };
    raw.trim()
        .parse::<u64>()
        .ok()
        .and_then(|n| n.checked_add(1))
        .ok_or_else(|| {
            TodoError::new(
                ErrorCode::BadValue,
                format!(
                    "attempts of {id_str} is not a whole number ({raw}); fix it by hand before giving the task back"
                ),
            )
            .with_task(id_str)
        })
}

impl TaskOp<'_> {
    /// Command name, used for forced notes and as the `apply` operation name.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Claim => "claim",
            Self::Release => "release",
            Self::Done { .. } => "done",
            Self::Drop { .. } | Self::DropTree { .. } | Self::DropWithin { .. } => "drop",
            Self::Block { .. } => "block",
            Self::Unblock { .. } => "unblock",
            Self::Stuck { .. } => "stuck",
            Self::Reopen { .. } => "reopen",
            Self::Answer { .. } => "answer",
            Self::Promote { .. } => "promote",
            Self::Note { .. } => "note",
            Self::Set { .. } => "set",
        }
    }

    /// Builds the line edits for `task` (empty when nothing changes).
    pub fn build_edits(
        &self,
        doc: &Document,
        task: &Task,
        env: &OpEnv<'_>,
    ) -> Result<Vec<LineEdit>> {
        let (cfg, agent, now) = (env.cfg, env.agent, env.now);
        match self {
            Self::Claim => {
                let plan = plan_claim(task, agent, env.force, now, cfg)?;
                let files_note = check_claim_files(doc, task, agent, env.force, now, cfg)?;
                let (edits, _) = build_claim_edits_with_note(
                    doc,
                    task,
                    &plan,
                    env.session,
                    now,
                    files_note.as_deref(),
                );
                return Ok(edits);
            }
            Self::Release => {
                let plan = plan_release(task, agent, env.force, cfg)?;
                return Ok(build_release_edits(doc, task, &plan, agent, now).0);
            }
            _ => {}
        }

        let id_str = cfg.id_config.format_id(task.id);
        let date = OpsConfig::format_note_date(now);
        let mut builder = TaskEditBuilder::new(doc, task);

        // R-MILE-2: a milestone is done or dropped only when nothing below it is unfinished.
        let mut forced_below: Option<usize> = None;
        if task.is_milestone() {
            let unfinished = doc.unfinished_below(task.id);
            if !unfinished.is_empty() {
                match self {
                    Self::Done { .. } if env.force => forced_below = Some(unfinished.len()),
                    Self::Done { .. } => {
                        return Err(milestone::refuse_unfinished(
                            cfg,
                            task,
                            &unfinished,
                            "finish them first, or pass --force to mark the milestone done anyway",
                        ));
                    }
                    Self::Drop { .. } => {
                        return Err(milestone::refuse_unfinished(
                            cfg,
                            task,
                            &unfinished,
                            &format!(
                                "use `todo drop {id_str} --tree --note <TEXT>` to drop them with it"
                            ),
                        ));
                    }
                    _ => {}
                }
            }
        }

        let ownership = match self {
            Self::Note { .. } => {
                check_ownership(task, TaskCommand::Note, agent, env.force, &id_str)?
            }
            Self::Set { .. } => check_ownership(task, TaskCommand::Set, agent, env.force, &id_str)?,
            Self::Promote { .. } => {
                check_promotable(task, &id_str)?;
                check_ownership(task, TaskCommand::Promote, agent, env.force, &id_str)?
            }
            _ => {
                let cmd = match self {
                    Self::Done { .. } | Self::Answer { .. } => StatusCommand::Done,
                    Self::Drop { .. } | Self::DropTree { .. } | Self::DropWithin { .. } => {
                        StatusCommand::Drop
                    }
                    Self::Block { .. } => StatusCommand::Block,
                    Self::Unblock { .. } => StatusCommand::Unblock,
                    Self::Stuck { .. } => StatusCommand::Stuck,
                    _ => StatusCommand::Reopen,
                };
                let (next, ownership) = validate_transition(task, cmd, agent, env.force, &id_str)?;
                builder.set_status(next);
                match self {
                    Self::Done { .. } | Self::Answer { .. } => {
                        builder.set_owner(Some(agent));
                        builder.remove_claimed();
                    }
                    Self::Drop { .. } | Self::DropTree { .. } | Self::DropWithin { .. } => {
                        builder.remove_claimed();
                    }
                    Self::Unblock { .. } if next == Status::Claimed => {
                        builder.set_claimed(&OpsConfig::format_claimed_timestamp(now));
                        builder.set_session(env.session);
                    }
                    Self::Unblock { .. } => builder.remove_claimed(),
                    Self::Stuck { .. } => {
                        // R-STUCK-1: the owner and the claim (with its session) go; the level stays.
                        builder.clear_owner_if_present();
                        builder.remove_claimed();
                        let attempts = attempts_after_failure(task, &id_str)?;
                        builder.set_field("attempts", &attempts.to_string());
                    }
                    Self::Reopen { .. } => {
                        builder.set_owner(None);
                        builder.remove_claimed();
                        builder.unset_field("by");
                    }
                    _ => {}
                }
                ownership
            }
        };

        if let Self::Done { commit, by, .. } = self {
            if let Some(c) = commit {
                builder.set_field("commit", c);
            }
            if let Some(b) = by {
                builder.set_field("by", b);
            }
        }
        if let Self::Promote {
            priority,
            milestone,
        } = self
        {
            builder.unset_field("type");
            builder.set_field("priority", priority);
            if let Some(m) = milestone {
                builder.set_field("milestone", m);
            }
        }
        if let Self::Set { changes } = self {
            milestone::check_set_changes(doc, cfg, task, changes)?;
            for change in *changes {
                match *change {
                    SetChange::Title(t) => builder.set_title(t),
                    SetChange::Field(k, v) if k.trim().eq_ignore_ascii_case("milestone") => {
                        builder.set_field(k, &milestone::canonical_parent(doc, cfg, v));
                    }
                    SetChange::Field("difficulty", v) => {
                        // Only configured difficulties are written (`R-WRITE-18`).
                        let canonical = cfg.canonical_difficulty(v)?;
                        builder.set_field("difficulty", &canonical);
                    }
                    SetChange::Field(k, v) => builder.set_field(k, v),
                    SetChange::Unset(k) => builder.unset_field(k),
                }
            }
        }

        if let OwnershipDecision::Forced { previous_owner } = ownership {
            builder.add_note(
                &date,
                agent,
                &forced_note_text(self.name(), &previous_owner),
            );
        }
        if let Some(count) = forced_below {
            builder.add_note(
                &date,
                agent,
                &format!("forced done ({count} entries below unfinished)"),
            );
        }
        let note = match *self {
            Self::Done { note, .. } => Some(note_with("done", note)),
            Self::Drop { reason } | Self::DropTree { reason } => {
                Some(note_with("dropped", Some(reason)))
            }
            Self::DropWithin { milestone, reason } => {
                Some(format!("dropped with {milestone} \u{2014} {reason}"))
            }
            Self::Block { reason } => Some(note_with("blocked", Some(reason))),
            Self::Unblock { note } => Some(note_with("unblocked", note)),
            Self::Stuck { reason } => Some(note_with("stuck", Some(reason))),
            Self::Reopen { note } => Some(note_with("reopened", note)),
            Self::Answer { text } => Some(note_with("answer", Some(text))),
            Self::Promote { .. } => Some("promoted from idea".to_string()),
            Self::Note { text } => Some(text.to_string()),
            _ => None,
        };
        if let Some(text) = note {
            builder.add_note(&date, agent, &text);
        }
        Ok(builder.build().0)
    }
}

/// Highest task number in `doc`, the archive and the ID state (`R-ID-2`); the next ID is one more.
#[must_use]
pub fn highest_task_number(doc: &Document, archive: Option<&Document>, state_highest: u64) -> u64 {
    let max_of = |d: &Document| d.tasks.iter().map(|t| t.id.number()).max().unwrap_or(0);
    max_of(doc)
        .max(archive.map_or(0, max_of))
        .max(state_highest)
}

/// The steps of a task given as text: each line is one step, and blank lines are skipped. A number
/// such as `1.` or `2)` at the start of a line is removed, because todo numbers the steps itself
/// (`R-DETAIL-1`). A dot not followed by a space or the end of the line stays (`2.5 GB`).
#[must_use]
pub fn steps_from_text(text: &str) -> Vec<String> {
    text.split(['\r', '\n'])
        .map(|line| strip_step_number(line.trim()).trim().to_string())
        .filter(|step| !step.is_empty())
        .collect()
}

/// `line` without a leading `1.` or `2)` and the spaces after it (see [`steps_from_text`]).
fn strip_step_number(line: &str) -> &str {
    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return line;
    }
    let after = &line[digits..];
    let Some(rest) = after.strip_prefix(['.', ')']) else {
        return line;
    };
    if rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t') {
        rest
    } else {
        line
    }
}

/// The `steps` value of a JSON operation: a list of strings, or one string with line breaks
/// (`R-DETAIL-1`).
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum StepsInput {
    /// One string; each of its lines is a step.
    One(String),
    /// A list; each entry is a step, or several lines of them.
    Many(Vec<String>),
}

impl StepsInput {
    /// The steps as text with one step per line, for [`steps_from_text`].
    #[must_use]
    pub fn text(&self) -> String {
        match self {
            Self::One(text) => text.clone(),
            Self::Many(items) => items.join("\n"),
        }
    }
}

/// The values of a new task, already validated and trimmed (`todo add`, `add --from`, `apply`).
#[derive(Debug, Default)]
pub struct NewTask<'a> {
    /// Title text.
    pub title: &'a str,
    /// `priority` value.
    pub priority: Option<&'a str>,
    /// `difficulty` value (one of the configured difficulties; `R-WRITE-18`).
    pub difficulty: Option<&'a str>,
    /// `type` value.
    pub task_type: Option<&'a str>,
    /// `area` value.
    pub area: Option<&'a str>,
    /// `milestone` value: the canonical ID of the parent milestone (R-MILE-1).
    pub milestone: Option<&'a str>,
    /// `goal` value, for a milestone (R-MILE-1).
    pub goal: Option<&'a str>,
    /// `why` value: what the task is for.
    pub why: Option<&'a str>,
    /// `depends` value (comma-separated IDs).
    pub depends: Option<&'a str>,
    /// `files` value.
    pub files: Option<&'a str>,
    /// `read` value: the paths to look at but not change (`R-DETAIL-1`).
    pub read: Option<&'a str>,
    /// `problem` value.
    pub problem: Option<&'a str>,
    /// `fix` value.
    pub fix: Option<&'a str>,
    /// The steps, one per entry, as [`steps_from_text`] splits them (`R-DETAIL-1`).
    pub steps: &'a [String],
    /// `accept` value.
    pub accept: Option<&'a str>,
    /// `verify` value: the command that checks the task (`R-DETAIL-1`).
    pub verify: Option<&'a str>,
    /// Further `key: value` fields; keys that have their own member above are skipped.
    pub extra: &'a [(String, String)],
}

impl NewTask<'_> {
    /// Canonical lines of the new task block (`docs/format.md` §10), starting with a blank line.
    #[must_use]
    pub fn lines(&self, id_str: &str, date: &str, agent: &str) -> Vec<String> {
        use crate::doc::{CANONICAL_HEADING_SEPARATOR, CANONICAL_NOBODY_OWNER};
        let mut lines = vec![
            String::new(),
            format!("### {id_str}{CANONICAL_HEADING_SEPARATOR}{}", self.title),
            "- status: open".to_string(),
            format!("- owner: {CANONICAL_NOBODY_OWNER}"),
        ];
        let combined: Vec<String> = [
            ("priority", self.priority),
            ("difficulty", self.difficulty),
            ("type", self.task_type),
            ("area", self.area),
        ]
        .into_iter()
        .filter_map(|(k, v)| v.map(|v| format!("{k}: {v}")))
        .collect();
        if !combined.is_empty() {
            lines.push(format!("- {}", combined.join(" \u{00b7} ")));
        }
        for (key, value) in [
            ("milestone", self.milestone),
            ("goal", self.goal),
            ("why", self.why),
            ("depends", self.depends),
            ("files", self.files),
            ("read", self.read),
            ("problem", self.problem),
            ("fix", self.fix),
        ] {
            if let Some(v) = value {
                lines.push(format!("- {key}: {v}"));
            }
        }
        if !self.steps.is_empty() {
            lines.push("- steps:".to_string());
            lines.extend(
                self.steps
                    .iter()
                    .enumerate()
                    .map(|(index, text)| format!("  {}. {text}", index + 1)),
            );
        }
        for (key, value) in [("accept", self.accept), ("verify", self.verify)] {
            if let Some(v) = value {
                lines.push(format!("- {key}: {v}"));
            }
        }
        let known = [
            "priority",
            "difficulty",
            "type",
            "area",
            "milestone",
            "goal",
            "why",
            "depends",
            "files",
            "read",
            "problem",
            "fix",
            "steps",
            "accept",
            "verify",
        ];
        for (k, v) in self.extra {
            if !known.contains(&k.as_str()) {
                lines.push(format!("- {k}: {v}"));
            }
        }
        lines.push(format!("- note ({date}, {agent}): added"));
        lines
    }
}
