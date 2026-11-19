//! The reason a task is blocked or dropped, read from its notes (T-140; `docs/format.md` §9,
//! R-REPORT-1, R-READ-5).
//!
//! `block` and `drop` write a note such as `blocked — <reason>` or `dropped — <reason>`. The reason
//! is the text after the em dash of the latest note that names the state. Every command that shows
//! a reason (`summary`, the HTML report) calls [`Task::blocked_reason`] or [`Task::dropped_reason`],
//! so they cannot disagree.

use crate::doc::parser::Task;

/// The em dash that separates a reason from the words before it.
const DASH: &str = "\u{2014}";

/// Shown in place of a reason that no note gives.
pub const NO_REASON: &str = "no reason recorded";

impl Task {
    /// The reason the task is blocked: the reason of the latest `blocked — <reason>` note that
    /// gives one. A later note that is not a block note does not count, and neither does a
    /// `blocked` note with no reason after the dash. `None` when no note gives a reason.
    #[must_use]
    pub fn blocked_reason(&self) -> Option<&str> {
        self.latest_reason("blocked")
    }

    /// The reason the task was dropped: the reason of the latest `dropped — <reason>` note, or of
    /// the latest `dropped with <milestone> — <reason>` note (`drop --tree`). `None` when no note
    /// gives a reason.
    #[must_use]
    pub fn dropped_reason(&self) -> Option<&str> {
        self.latest_reason("dropped")
    }

    /// The reason in the latest note whose state word is `keyword` (`blocked`, `dropped`), newest
    /// first. The state word is the first word of the note's text before the dash, possibly
    /// followed by more words (`dropped with T-001`), so `unblocked` and `blockedish` do not match.
    #[must_use]
    pub fn latest_reason(&self, keyword: &str) -> Option<&str> {
        self.notes
            .iter()
            .rev()
            .find_map(|note| reason_in_text(&note.text, keyword))
    }
}

/// The reason in the text of one note (the words after `- note (date, agent): `), when its state
/// word is `keyword` and it gives a non-empty reason. The state word is the first word of the text
/// before the dash.
pub fn reason_in_text<'a>(text: &'a str, keyword: &str) -> Option<&'a str> {
    let (words, reason) = text.trim().split_once(DASH)?;
    let words = words.trim();
    let names_state = words == keyword
        || words
            .strip_prefix(keyword)
            .is_some_and(|rest| rest.starts_with(' '));
    let reason = reason.trim();
    (names_state && !reason.is_empty()).then_some(reason)
}

#[cfg(test)]
mod tests {
    use crate::doc::parser::Document;

    /// The first task of a document with `notes` as its note lines.
    fn task_with_notes(notes: &[&str]) -> Document {
        let mut text = String::from("### T-001 · A task\n- status: blocked\n");
        for note in notes {
            text.push_str(&format!("- note (2026-11-18, bob): {note}\n"));
        }
        Document::parse_bytes(text.as_bytes()).unwrap()
    }

    #[test]
    fn the_latest_block_reason_wins_over_earlier_ones_and_later_unrelated_notes() {
        let doc = task_with_notes(&[
            "blocked — waiting on the API key",
            "unblocked — the key arrived",
            "blocked — the vendor driver crashes",
            "checked the logs",
        ]);
        assert_eq!(
            doc.tasks[0].blocked_reason(),
            Some("the vendor driver crashes")
        );
    }

    #[test]
    fn a_bare_or_empty_block_note_gives_the_reason_of_the_one_before_it() {
        let doc = task_with_notes(&["blocked — waiting for the review", "blocked", "blocked — "]);
        assert_eq!(
            doc.tasks[0].blocked_reason(),
            Some("waiting for the review")
        );
    }

    #[test]
    fn no_block_note_gives_no_reason() {
        let none = task_with_notes(&["added", "unblocked — the key arrived"]);
        assert_eq!(none.tasks[0].blocked_reason(), None);
        let no_notes = task_with_notes(&[]);
        assert_eq!(no_notes.tasks[0].blocked_reason(), None);
    }

    #[test]
    fn the_state_word_must_match_exactly() {
        let doc = task_with_notes(&["blockedish — not a block", "unblocked — not a block"]);
        assert_eq!(doc.tasks[0].blocked_reason(), None);
    }

    #[test]
    fn a_dropped_reason_may_name_a_milestone() {
        let doc = Document::parse_bytes(
            "### T-002 · Dropped below a milestone\n\
- status: dropped\n\
- note (2026-11-18, bob): dropped with T-001 — the scene no longer needs it\n"
                .as_bytes(),
        )
        .unwrap();
        assert_eq!(
            doc.tasks[0].dropped_reason(),
            Some("the scene no longer needs it")
        );
        assert_eq!(doc.tasks[0].blocked_reason(), None);
    }

    #[test]
    fn a_reason_may_contain_a_dash() {
        let doc = task_with_notes(&["blocked — waiting — for the second review"]);
        assert_eq!(
            doc.tasks[0].blocked_reason(),
            Some("waiting — for the second review")
        );
    }
}
