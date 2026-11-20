//! Format migrations (`docs/format.md` §18, R-MIG-1).
//!
//! A file in an older format is brought up to the current one by an ordered list of steps. Each
//! step goes from one format version to the next and changes whole lines only (R-EDIT-1); the
//! bytes outside those lines stay the same. Format 1 to 2 changes the preamble only; format 2 to 3
//! also rewrites the difficulty of open tasks (R-LEVEL-1), so task lines may change. The write
//! pipeline in `ops::migrate` applies the steps under the write lock and in the same write as the
//! command's own change.

use std::fmt;

use crate::doc::lines::{EditError, LineEdit};
use crate::doc::parser::{Document, FieldLocation, Status};
use crate::doc::stamp::tool_line_text;
use crate::version::RUNNING_VERSION;

/// The text of a format line for `version`, such as `<!-- todo-format: 2 -->` (`docs/format.md` §4).
#[must_use]
pub fn format_line_text(version: u32) -> String {
    format!("<!-- todo-format: {version} -->")
}

/// What a migration step needs to know from `todo.toml`, which the document does not hold.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MigrationSettings {
    /// `true` when `todo.toml` sets `difficulties` of its own. The format 2 to 3 step then keeps
    /// every difficulty value as it is (`docs/format.md` §18).
    pub difficulties_set: bool,
}

/// One migration step from format version `from` to `from + 1` (`docs/format.md` §18).
#[derive(Debug, Clone, Copy)]
pub struct MigrationStep {
    /// The format version the step starts from.
    pub from: u32,
    /// The format version the step leads to; always `from + 1`.
    pub to: u32,
    /// The whole-line edits that raise a document from `from` to `to`, computed from the document
    /// as it is before the step and from the settings. Only whole lines may change.
    pub edits: fn(&Document, &MigrationSettings) -> Vec<LineEdit>,
}

/// The steps that bring a file to the current format, in the order they apply (R-MIG-1).
pub const STEPS: &[MigrationStep] = &[FORMAT_1_TO_2, FORMAT_2_TO_3];

/// Format 1 to 2 (`docs/format.md` §18): the format line is raised to 2, or added as the first line
/// when it is missing, and the tool line is added directly after it when there is none. Task lines
/// stay as they are.
pub const FORMAT_1_TO_2: MigrationStep = MigrationStep {
    from: 1,
    to: 2,
    edits: format_1_to_2_edits,
};

fn format_1_to_2_edits(doc: &Document, _settings: &MigrationSettings) -> Vec<LineEdit> {
    let add_stamp = doc.tool_stamp.is_none();
    let stamp = tool_line_text(RUNNING_VERSION);
    match doc.format_line {
        Some(index) => {
            let mut edits = vec![LineEdit::replace_line(index, format_line_text(2))];
            if add_stamp {
                edits.push(LineEdit::insert(index + 1, vec![stamp]));
            }
            edits
        }
        None => {
            // The format line goes first; a byte-order mark stays in front of it (`LineDoc`).
            let mut lines = vec![format_line_text(2)];
            if add_stamp {
                lines.push(stamp);
            }
            vec![LineEdit::insert(0, lines)]
        }
    }
}

/// Format 2 to 3 (`docs/format.md` §18, R-LEVEL-1): the format line is raised to 3. When `todo.toml`
/// sets no `difficulties` of its own, the difficulty of every entry that is not done or dropped is
/// rewritten from the old names to the levels: `easy` to `2`, `medium` to `3` and `hard` to `4`.
/// Only the value changes; the rest of its line stays. Open tasks without a difficulty are left for
/// the planner's list (R-LEVEL-2).
pub const FORMAT_2_TO_3: MigrationStep = MigrationStep {
    from: 2,
    to: 3,
    edits: format_2_to_3_edits,
};

/// The old difficulty names and the levels they become in format 3.
const OLD_DIFFICULTIES: [(&str, &str); 3] = [("easy", "2"), ("medium", "3"), ("hard", "4")];

fn format_2_to_3_edits(doc: &Document, settings: &MigrationSettings) -> Vec<LineEdit> {
    let mut edits: Vec<LineEdit> = doc
        .format_line
        .map(|index| LineEdit::replace_line(index, format_line_text(3)))
        .into_iter()
        .collect();
    if settings.difficulties_set {
        return edits;
    }
    for task in &doc.tasks {
        if matches!(task.status, Status::Done | Status::Dropped) {
            continue;
        }
        let Some(loc) = task.difficulty_loc else {
            continue;
        };
        let Some(line) = doc.line_doc.line(loc.line()) else {
            continue;
        };
        if let Some(text) = level_line(&line.text, loc) {
            edits.push(LineEdit::replace_line(loc.line(), text));
        }
    }
    edits
}

/// The level that an old difficulty name becomes, compared ignoring case; `None` for any other
/// value, which the migration keeps as it is.
fn level_for(value: &str) -> Option<&'static str> {
    OLD_DIFFICULTIES
        .iter()
        .find(|(old, _)| old.eq_ignore_ascii_case(value))
        .map(|(_, level)| *level)
}

/// The text of a line that holds `difficulty` at `loc` with its old name replaced by the level, or
/// `None` when that value is not an old name. Only the value and the spaces around it change; on a
/// combined priority line every other part is kept as written (`docs/format.md` §8).
fn level_line(text: &str, loc: FieldLocation) -> Option<String> {
    match loc {
        FieldLocation::OwnLine(_) => {
            let after = text.strip_prefix("- difficulty:")?;
            let level = level_for(after.trim())?;
            Some(format!("- difficulty:{}", with_value(after, level)))
        }
        FieldLocation::CombinedPriority(_) => {
            let mut changed = false;
            let parts: Vec<String> = text
                .split(" · ")
                .enumerate()
                .map(|(index, part)| {
                    // The first part is the priority, which is never a difficulty.
                    if index == 0 {
                        return part.to_string();
                    }
                    match part.split_once(':') {
                        Some((key, value)) if key.trim() == "difficulty" => {
                            match level_for(value.trim()) {
                                Some(level) => {
                                    changed = true;
                                    format!("{key}:{}", with_value(value, level))
                                }
                                None => part.to_string(),
                            }
                        }
                        _ => part.to_string(),
                    }
                })
                .collect();
            changed.then(|| parts.join(" · "))
        }
    }
}

/// `raw` with its trimmed value replaced by `value`, keeping the spaces before and after it.
fn with_value(raw: &str, value: &str) -> String {
    let lead = raw.len() - raw.trim_start().len();
    let trail = raw.len() - raw.trim_end().len();
    format!("{}{value}{}", &raw[..lead], &raw[raw.len() - trail..])
}

/// A test-only step from format 2 to 3, compiled only with the `test-hooks` feature. It raises the
/// format line to 3 and nothing else, so a test can show that two steps run in order (R-MIG-1).
#[cfg(feature = "test-hooks")]
pub const TEST_STEP_2_TO_3: MigrationStep = MigrationStep {
    from: 2,
    to: 3,
    edits: test_step_2_to_3_edits,
};

#[cfg(feature = "test-hooks")]
fn test_step_2_to_3_edits(doc: &Document, _settings: &MigrationSettings) -> Vec<LineEdit> {
    doc.format_line
        .map(|index| LineEdit::replace_line(index, format_line_text(3)))
        .into_iter()
        .collect()
}

/// A document after migration: the migrated text, parsed again, and the versions it moved between.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Migrated {
    /// The migrated document. It is parsed but not judged: a caller refuses it when it has
    /// write-blocking problems (`docs/format.md` §13).
    pub doc: Document,
    /// The format version before the first step.
    pub from: u32,
    /// The format version after the last step.
    pub to: u32,
}

/// Why a migration could not be applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationError {
    /// No step starts at `version`, so the list of steps has a gap.
    NoStep { version: u32 },
    /// A step does not lead from `from` to `from + 1`.
    NotOneVersion { from: u32, to: u32 },
    /// The edits of a step could not be applied, or the bytes outside them changed (R-EDIT-7).
    Edit(EditError),
}

impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoStep { version } => {
                write!(f, "no migration step starts at format version {version}")
            }
            Self::NotOneVersion { from, to } => write!(
                f,
                "migration step from format version {from} leads to {to}, not {}",
                from + 1
            ),
            Self::Edit(err) => write!(f, "migration step failed: {err}"),
        }
    }
}

impl std::error::Error for MigrationError {}

/// Applies the steps from the format version of `doc` up to `target`, one at a time, in order
/// (R-MIG-1), with the `settings` from `todo.toml`. Returns `None` when `doc` is already at `target`
/// or newer.
///
/// Each step is checked before the next one runs: every byte outside its edited lines must stay the
/// same (R-EDIT-7).
pub fn migrate_document(
    doc: &Document,
    steps: &[MigrationStep],
    target: u32,
    settings: &MigrationSettings,
) -> Result<Option<Migrated>, MigrationError> {
    let from = doc.effective_format_version();
    if from >= target {
        return Ok(None);
    }
    let mut version = from;
    let mut migrated: Option<Document> = None;
    while version < target {
        let step = steps
            .iter()
            .find(|s| s.from == version)
            .ok_or(MigrationError::NoStep { version })?;
        if step.to != step.from + 1 {
            return Err(MigrationError::NotOneVersion {
                from: step.from,
                to: step.to,
            });
        }
        let next = apply_step(step, migrated.as_ref().unwrap_or(doc), settings)?;
        migrated = Some(next);
        version = step.to;
    }
    Ok(migrated.map(|doc| Migrated {
        doc,
        from,
        to: version,
    }))
}

fn apply_step(
    step: &MigrationStep,
    doc: &Document,
    settings: &MigrationSettings,
) -> Result<Document, MigrationError> {
    let edits = (step.edits)(doc, settings);
    let applied = doc
        .line_doc
        .apply_and_verify_bytes(&edits)
        .map_err(MigrationError::Edit)?;
    Ok(Document::from_line_doc(applied.doc, doc.id_config.clone()))
}
