//! The kind of an entry, taken from its `type` (`docs/format.md` §16, `R-KIND-3`).
//!
//! Milestones and tasks are work; questions and ideas are not. Commands that hand out work
//! (`next`, the readiness check) accept only [`EntryKind::Task`].

use super::Task;

/// What a task block is: work, a milestone, a question or an idea (`docs/format.md` §16).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    /// An ordinary task: any `type` other than the three below, or none.
    Task,
    /// `type: milestone`: a piece of work with a goal, divided into tasks; never handed out.
    Milestone,
    /// `type: question`: a question for a person to answer.
    Question,
    /// `type: idea`: an idea that is not work until `promote` turns it into a task.
    Idea,
}

impl EntryKind {
    /// Classifies a `type` value. The three reserved names match ignoring case.
    #[must_use]
    pub fn from_type(task_type: Option<&str>) -> Self {
        match task_type.map(str::trim) {
            Some(t) if t.eq_ignore_ascii_case("milestone") => Self::Milestone,
            Some(t) if t.eq_ignore_ascii_case("question") => Self::Question,
            Some(t) if t.eq_ignore_ascii_case("idea") => Self::Idea,
            _ => Self::Task,
        }
    }
}

impl Task {
    /// The kind of this entry (`R-KIND-3`).
    #[must_use]
    pub fn kind(&self) -> EntryKind {
        EntryKind::from_type(self.task_type.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::EntryKind;

    #[test]
    fn reserved_types_set_the_kind_ignoring_case() {
        assert_eq!(EntryKind::from_type(None), EntryKind::Task);
        assert_eq!(EntryKind::from_type(Some("feature")), EntryKind::Task);
        assert_eq!(
            EntryKind::from_type(Some("milestone")),
            EntryKind::Milestone
        );
        assert_eq!(EntryKind::from_type(Some("Question")), EntryKind::Question);
        assert_eq!(EntryKind::from_type(Some(" IDEA ")), EntryKind::Idea);
    }
}
