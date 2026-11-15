//! One implementation of every single-task edit, shared by the single commands and `todo apply`.
//!
//! [`TaskOp`] names the operation with its already-validated parameters; [`TaskOp::build_edits`]
//! turns it into whole-line [`LineEdit`]s for one task. `todo claim`, `todo done`, ... and the
//! matching `todo apply` operations all call it, so a rule such as the forced-note audit line
//! exists once. Argument parsing, empty-text checks and output stay with the callers.

use chrono::{DateTime, FixedOffset};

use crate::doc::{Document, LineEdit, Status, Task};
use crate::error::Result;
use crate::ops::pipeline::{build_claim_edits, build_release_edits};
use crate::ops::rules::{
    OwnershipDecision, StatusCommand, TaskCommand, check_ownership, forced_note_text, plan_claim,
    plan_release, validate_transition,
};
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
    },
    /// Drop the task with a reason.
    Drop {
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

impl TaskOp<'_> {
    /// Command name, used for forced notes and as the `apply` operation name.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Claim => "claim",
            Self::Release => "release",
            Self::Done { .. } => "done",
            Self::Drop { .. } => "drop",
            Self::Block { .. } => "block",
            Self::Unblock { .. } => "unblock",
            Self::Reopen { .. } => "reopen",
            Self::Answer { .. } => "answer",
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
                return Ok(build_claim_edits(doc, task, &plan, now).0);
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

        let ownership = match self {
            Self::Note { .. } => {
                check_ownership(task, TaskCommand::Note, agent, env.force, &id_str)?
            }
            Self::Set { .. } => check_ownership(task, TaskCommand::Set, agent, env.force, &id_str)?,
            _ => {
                let cmd = match self {
                    Self::Done { .. } | Self::Answer { .. } => StatusCommand::Done,
                    Self::Drop { .. } => StatusCommand::Drop,
                    Self::Block { .. } => StatusCommand::Block,
                    Self::Unblock { .. } => StatusCommand::Unblock,
                    _ => StatusCommand::Reopen,
                };
                let (next, ownership) = validate_transition(task, cmd, agent, env.force, &id_str)?;
                builder.set_status(next);
                match self {
                    Self::Done { .. } | Self::Answer { .. } => {
                        builder.set_owner(Some(agent));
                        builder.remove_claimed();
                    }
                    Self::Drop { .. } => builder.remove_claimed(),
                    Self::Unblock { .. } if next == Status::Claimed => {
                        builder.set_claimed(&OpsConfig::format_claimed_timestamp(now));
                    }
                    Self::Unblock { .. } => builder.remove_claimed(),
                    Self::Reopen { .. } => {
                        builder.set_owner(None);
                        builder.remove_claimed();
                    }
                    _ => {}
                }
                ownership
            }
        };

        if let Self::Done {
            commit: Some(c), ..
        } = self
        {
            builder.set_field("commit", c);
        }
        if let Self::Set { changes } = self {
            for change in *changes {
                match *change {
                    SetChange::Title(t) => builder.set_title(t),
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
        let note = match *self {
            Self::Done { note, .. } => Some(note_with("done", note)),
            Self::Drop { reason } => Some(note_with("dropped", Some(reason))),
            Self::Block { reason } => Some(note_with("blocked", Some(reason))),
            Self::Unblock { note } => Some(note_with("unblocked", note)),
            Self::Reopen { note } => Some(note_with("reopened", note)),
            Self::Answer { text } => Some(note_with("answer", Some(text))),
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

/// The values of a new task, already validated and trimmed (`todo add`, `add --from`, `apply`).
#[derive(Debug, Default)]
pub struct NewTask<'a> {
    /// Title text.
    pub title: &'a str,
    /// `priority` value.
    pub priority: Option<&'a str>,
    /// `type` value.
    pub task_type: Option<&'a str>,
    /// `area` value.
    pub area: Option<&'a str>,
    /// `depends` value (comma-separated IDs).
    pub depends: Option<&'a str>,
    /// `files` value.
    pub files: Option<&'a str>,
    /// `problem` value.
    pub problem: Option<&'a str>,
    /// `fix` value.
    pub fix: Option<&'a str>,
    /// `accept` value.
    pub accept: Option<&'a str>,
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
            ("depends", self.depends),
            ("files", self.files),
            ("problem", self.problem),
            ("fix", self.fix),
            ("accept", self.accept),
        ] {
            if let Some(v) = value {
                lines.push(format!("- {key}: {v}"));
            }
        }
        let known = [
            "priority", "type", "area", "depends", "files", "problem", "fix", "accept",
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
