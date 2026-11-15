//! The agent rules text and agent setup commands (T-050, later T-051–T-054).

/// The canonical agent rules text required by R-AGENT-1.
///
/// Reused by `todo help agents`, `todo init`, `todo agents install`, and `todo skill install`.
pub const RULES_TEXT: &str = include_str!("rules.md");

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
