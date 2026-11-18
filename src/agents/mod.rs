//! The agent rules text and agent setup commands (T-050, T-051, T-052).

pub mod hooks_file;
pub mod project;

/// Opening marker for a tool-managed agent rules block (`docs/format.md` §12).
pub const BEGIN_MARKER: &str = "<!-- todo:begin -->";

/// Closing marker for a tool-managed agent rules block (`docs/format.md` §12).
pub const END_MARKER: &str = "<!-- todo:end -->";

/// The canonical agent rules text required by R-AGENT-1.
///
/// Reused by `todo help agents`, `todo init`, `todo agents install`, and `todo skill install`.
pub const RULES_TEXT: &str = include_str!("rules.md");

/// Status of a managed agent-rules block in a Markdown file (`R-AGENT-3`, `docs/format.md` §12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedBlockStatus {
    /// No valid `<!-- todo:begin -->` ... `<!-- todo:end -->` pair was found.
    Missing,
    /// The managed block exists and matches [`RULES_TEXT`].
    Current {
        /// 1-based line number of `<!-- todo:begin -->`.
        begin_line: usize,
        /// 1-based line number of `<!-- todo:end -->`.
        end_line: usize,
    },
    /// The managed block exists, but its contents differ from [`RULES_TEXT`].
    Outdated {
        /// 1-based line number of `<!-- todo:begin -->`.
        begin_line: usize,
        /// 1-based line number of `<!-- todo:end -->`.
        end_line: usize,
    },
}

/// Checks whether `text` contains a managed rules block and whether it matches [`RULES_TEXT`].
#[must_use]
pub fn check_managed_block_in_text(text: &str) -> ManagedBlockStatus {
    let lines: Vec<&str> = text.lines().collect();
    let mut begin_idx: Option<usize> = None;
    let mut end_idx: Option<usize> = None;
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if begin_idx.is_none() && trimmed == BEGIN_MARKER {
            begin_idx = Some(idx);
        } else if begin_idx.is_some() && trimmed == END_MARKER {
            end_idx = Some(idx);
            break;
        }
    }
    let (Some(b), Some(e)) = (begin_idx, end_idx) else {
        return ManagedBlockStatus::Missing;
    };

    let existing_body = lines[b + 1..e].join("\n");
    if existing_body.trim() == rules_text_lf() {
        ManagedBlockStatus::Current {
            begin_line: b + 1,
            end_line: e + 1,
        }
    } else {
        ManagedBlockStatus::Outdated {
            begin_line: b + 1,
            end_line: e + 1,
        }
    }
}

/// Renders the canonical managed block (`BEGIN_MARKER`, [`RULES_TEXT`], `END_MARKER`) with `eol`.
#[must_use]
pub fn render_managed_block(eol: &str) -> String {
    let mut out = String::new();
    out.push_str(BEGIN_MARKER);
    out.push_str(eol);
    for line in RULES_TEXT.trim().lines() {
        out.push_str(line);
        out.push_str(eol);
    }
    out.push_str(END_MARKER);
    out
}

/// Renders the Claude Code `.claude/skills/todo/SKILL.md` file from [`RULES_TEXT`] (`R-AGENT-5`).
#[must_use]
pub fn render_claude_skill() -> String {
    format!(
        "---\nname: todo\ndescription: Share a TODO.md task list safely across agents and people using the `todo` CLI.\n---\n\n{}\n",
        rules_text_lf()
    )
}

/// [`RULES_TEXT`] trimmed, with LF line endings. The text is embedded from `rules.md` at build
/// time, so a checkout with CRLF endings (git `autocrlf` on Windows) would otherwise make a
/// freshly installed block compare as outdated.
fn rules_text_lf() -> String {
    RULES_TEXT.trim().lines().collect::<Vec<_>>().join("\n")
}

/// Inserts or updates the managed agent-rules block in `existing_bytes` (`None` for a new file),
/// preserving all bytes outside `<!-- todo:begin -->` ... `<!-- todo:end -->` (`R-AGENT-3`).
pub fn upsert_managed_block(
    existing_bytes: Option<&[u8]>,
    label: &str,
) -> crate::error::Result<Vec<u8>> {
    use crate::doc::lines::{LineDoc, LineEdit};
    use crate::error::{ErrorCode, TodoError};

    let canonical_lines: Vec<String> = RULES_TEXT.trim().lines().map(str::to_string).collect();

    let Some(bytes) = existing_bytes else {
        let mut s = render_managed_block("\n");
        s.push('\n');
        return Ok(s.into_bytes());
    };

    let line_doc = LineDoc::from_bytes(bytes).map_err(|e| {
        TodoError::new(ErrorCode::UnsafeFile, e.to_string())
            .with_file(label.to_string())
            .with_line(e.line)
    })?;

    let mut begin_idx: Option<usize> = None;
    let mut end_idx: Option<usize> = None;
    for (idx, line) in line_doc.lines.iter().enumerate() {
        let trimmed = line.text.trim();
        if begin_idx.is_none() && trimmed == BEGIN_MARKER {
            begin_idx = Some(idx);
        } else if begin_idx.is_some() && trimmed == END_MARKER {
            end_idx = Some(idx);
            break;
        }
    }

    let edit = if let (Some(b), Some(e)) = (begin_idx, end_idx) {
        let existing_inner: Vec<&str> = line_doc.lines[b + 1..e]
            .iter()
            .map(|l| l.text.as_str())
            .collect();
        let matches = existing_inner.len() == canonical_lines.len()
            && existing_inner
                .iter()
                .zip(canonical_lines.iter())
                .all(|(a, b)| *a == b.as_str());
        if matches {
            return Ok(bytes.to_vec());
        }
        LineEdit::replace_range((b + 1)..e, canonical_lines)
    } else {
        let mut to_insert = Vec::new();
        if !line_doc.lines.is_empty()
            && !line_doc
                .lines
                .last()
                .is_some_and(|l| l.text.trim().is_empty())
        {
            to_insert.push(String::new());
        }
        to_insert.push(BEGIN_MARKER.to_string());
        to_insert.extend(canonical_lines);
        to_insert.push(END_MARKER.to_string());
        LineEdit::insert(line_doc.lines.len(), to_insert)
    };

    let applied = line_doc.apply_edits(&[edit]).map_err(|e| {
        TodoError::new(ErrorCode::WriteFailed, e.to_string()).with_file(label.to_string())
    })?;
    Ok(applied.bytes)
}

#[cfg(test)]
mod tests {
    use super::RULES_TEXT;

    #[test]
    fn rules_text_covers_all_r_agent_1_points() {
        for needle in [
            "Never edit `TODO.md`",
            "--agent",
            "TODO_AGENT",
            "summary",
            "next",
            "next --claim",
            "Exit code `1`",
            "Exit code `3`",
            "note",
            "done",
            "--note",
            "--commit",
            "add",
            "ask",
            "T-012:",
        ] {
            assert!(RULES_TEXT.contains(needle), "RULES_TEXT missing `{needle}`");
        }
    }
}
