//! The agent rules text and agent setup commands (T-050, T-051, T-052).

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
    let canonical_body = RULES_TEXT.trim();
    if existing_body.trim() == canonical_body {
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
