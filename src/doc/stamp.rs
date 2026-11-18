//! The tool version stamp `<!-- todo-tool: X -->` (`docs/format.md` §4, R-VER-1): the edit that a
//! write adds or raises, and the warning for a file that a newer todo wrote.

use crate::doc::id::IdConfig;
use crate::doc::lines::{EditError, LineEdit};
use crate::doc::parser::{Document, ProblemKind};
use crate::doc::verify::{AppliedDocEdits, TaskChangeIntent};
use crate::version::{RUNNING_VERSION, ToolVersion};

/// The stamp line that records `version`, such as `<!-- todo-tool: 0.6.0 -->`.
#[must_use]
pub fn tool_line_text(version: ToolVersion) -> String {
    format!("<!-- todo-tool: {version} -->")
}

impl Document {
    /// The edit that makes the stamp at least `running` in a write (R-VER-1).
    ///
    /// A missing stamp is added directly after the format line, or as the first line when the file
    /// has no format line. A stamp below `running` is replaced. `None` when the stamp is `running`
    /// or higher (a newer stamp is never lowered), or when its value is not a version: such a line
    /// is kept as it is and reported by [`ProblemKind::InvalidToolVersion`].
    #[must_use]
    pub fn tool_stamp_edit(&self, running: ToolVersion) -> Option<LineEdit> {
        match &self.tool_stamp {
            Some(stamp) => match stamp.version {
                Some(version) if version < running => {
                    Some(LineEdit::replace_line(stamp.line, tool_line_text(running)))
                }
                _ => None,
            },
            None => {
                let at = self.format_line.map_or(0, |line| line + 1);
                Some(LineEdit::insert(at, vec![tool_line_text(running)]))
            }
        }
    }

    /// Adds or raises the stamp in a write of this document, in the same bytes (R-VER-1).
    ///
    /// Returns `None` when the stamp needs no change, and also when the document has a problem
    /// that blocks all writes: the tool never stamps a file it may not write.
    pub fn with_tool_stamp(
        &self,
        running: ToolVersion,
    ) -> Result<Option<AppliedDocEdits>, EditError> {
        if self.blocks_all_writes() {
            return Ok(None);
        }
        let Some(edit) = self.tool_stamp_edit(running) else {
            return Ok(None);
        };
        self.apply_and_verify(std::slice::from_ref(&edit), &TaskChangeIntent::default())
            .map(Some)
    }

    /// The warning that a file written by a newer todo gets, prefixed with `label:line:`
    /// (R-VER-1). `None` when the stamp is not newer than the running todo.
    #[must_use]
    pub fn newer_tool_warning(&self, label: &str) -> Option<String> {
        self.problems
            .iter()
            .find(|p| matches!(p.kind, ProblemKind::NewerToolVersion { .. }))
            .map(|p| p.format_with_path(label))
    }
}

/// Returns `bytes` with the stamp added or raised to the running version (R-VER-1), for a whole
/// file that a command writes without the task pipeline (`restore`, `import`).
///
/// Bytes that are not UTF-8 are returned unchanged, as are files the tool may not write.
pub fn stamp_bytes(bytes: &[u8], id_config: IdConfig) -> Result<Vec<u8>, EditError> {
    let Ok(doc) = Document::parse_bytes_with_config(bytes, id_config) else {
        return Ok(bytes.to_vec());
    };
    match doc.with_tool_stamp(RUNNING_VERSION)? {
        Some(applied) => Ok(applied.bytes),
        None => Ok(bytes.to_vec()),
    }
}
