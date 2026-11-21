//! One entry of the v1.0 page, decided once for every part that shows entries (T-211; R-REPORT-1
//! "An entry" and "Details", section 5.23).
//!
//! [`entry_view`] decides an entry's state in words, its date, its second line and its details from
//! the parsed task lists and the clock. Every part of the page (the tree, Needs you, Last 7 days)
//! shows entries through it, so one entry reads the same everywhere. It only reads: nothing here
//! changes a document.

use chrono::{DateTime, FixedOffset, NaiveDate};

use crate::doc::{Document, EntryKind, NO_REASON, Note, Status, Task, TaskId};
use crate::ops::OpsConfig;

/// The em dash that separates a note's words from its text (`docs/format.md` §9).
const DASH: char = '\u{2014}';

/// Shown in place of the owner of a claim that names nobody.
const NO_OWNER: &str = "no owner";

/// The five states of the page, in words for people (section 5.23).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// `open`.
    ToDo,
    /// `claimed`.
    InProgress,
    /// `blocked`, or an open task that waits for an unanswered question.
    Waiting,
    /// `done`.
    Done,
    /// `dropped`.
    Cancelled,
}

impl State {
    /// The state in words, as the page writes it.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::ToDo => "to do",
            Self::InProgress => "in progress",
            Self::Waiting => "waiting",
            Self::Done => "done",
            Self::Cancelled => "cancelled",
        }
    }

    /// The mark in front of an entry and in the legend: ○ ● ◐ ✓ ✕.
    #[must_use]
    pub const fn mark(self) -> char {
        match self {
            Self::ToDo => '○',
            Self::InProgress => '●',
            Self::Waiting => '◐',
            Self::Done => '✓',
            Self::Cancelled => '✕',
        }
    }

    /// The CSS class of an entry in the mockup (`docs/report-mockup.html`).
    #[must_use]
    pub const fn class(self) -> &'static str {
        match self {
            Self::ToDo => "todo",
            Self::InProgress => "prog",
            Self::Waiting => "wait",
            Self::Done => "done",
            Self::Cancelled => "drop",
        }
    }
}

/// What an entry's view is decided from: `TODO.md`, the archive when it was read, the configuration
/// and the clock of the build.
#[derive(Debug, Clone, Copy)]
pub struct Ctx<'a> {
    /// `TODO.md`.
    pub doc: &'a Document,
    /// `TODO-archive.md`, when it was read.
    pub archive: Option<&'a Document>,
    pub cfg: &'a OpsConfig,
    /// The clock of the build.
    pub now: DateTime<FixedOffset>,
}

impl Ctx<'_> {
    /// The task with `id`: the one in `TODO.md`, else the one in the archive.
    #[must_use]
    pub fn find(&self, id: TaskId) -> Option<&Task> {
        self.doc
            .task_by_id(id)
            .or_else(|| self.archive.and_then(|archive| archive.task_by_id(id)))
    }
}

/// The second line of an entry, for the states that have one (R-REPORT-1, "An entry").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecondLine {
    /// "In progress · <owner> · <since>", followed by "· no activity" when the claim is stale.
    InProgress {
        owner: String,
        /// `None` when the claim has no valid time.
        since: Option<Since>,
        stale: bool,
    },
    /// "Waiting · <reason>".
    Waiting(WaitReason),
    /// "Cancelled · <reason of its latest drop note>".
    Cancelled(String),
}

/// How long a claim has been held, in calendar days of the clock's offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Since {
    /// Claimed today: "since today".
    Today,
    /// Claimed yesterday: "since yesterday".
    Yesterday,
    /// Claimed two or more days ago: "for N days".
    Days(i64),
}

/// Why an entry is waiting (R-REPORT-1, R-REPORT-4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaitReason {
    /// A task a worker gave back (R-STUCK-1), with its problem.
    GivenBack(String),
    /// An open task that waits for an unanswered question, with the question's title.
    Question(String),
    /// A blocked task, with the reason of its latest block note, or [`NO_REASON`].
    Blocked(String),
}

/// One entry of the page as the page shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryView {
    /// The canonical ID, formatted (`T-007`).
    pub id: String,
    pub title: String,
    pub state: State,
    /// The date a done entry was finished or a cancelled one dropped: the date of its latest `done`
    /// or `dropped` note. `None` for the other states, or when no such note exists.
    pub date: Option<NaiveDate>,
    /// The second line, for the states that have one.
    pub second: Option<SecondLine>,
    /// `summary`, written for people (R-WRITE-19); the page shows it before `why`.
    pub summary: Option<String>,
    /// `why`: what the task is for.
    pub why: Option<String>,
    /// For a done entry: the text of its latest `done — <text>` note.
    pub result: Option<String>,
    /// The closed part "Technical details" (R-REPORT-1).
    pub details: Details,
}

/// The technical details of an entry. Empty fields are `None` or empty, and the page leaves them out.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Details {
    /// The level (`difficulty`).
    pub level: Option<String>,
    pub priority: Option<String>,
    /// The task type (`type`).
    pub task_type: Option<String>,
    pub area: Option<String>,
    pub files: Option<String>,
    /// The check (`verify`).
    pub verify: Option<String>,
    /// The date the check last passed (`verified`).
    pub verified: Option<String>,
    /// The commit as written (`commit`).
    pub commit: Option<String>,
    /// The person who finished the task (`by`), else the last owner.
    pub done_by: Option<String>,
    pub depends: Vec<Dependency>,
    /// How often the task failed (`attempts`), as written.
    pub attempts: Option<String>,
    /// The dated notes, in file order.
    pub notes: Vec<NoteLine>,
}

/// One entry of `depends`: its canonical ID, and its title when a task list holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    pub id: String,
    pub title: Option<String>,
}

/// A dated note as the page shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteLine {
    pub date: NaiveDate,
    pub agent: String,
    pub text: String,
}

/// The view of `task`, decided with the task lists and the clock of `ctx` (R-REPORT-1).
#[must_use]
pub fn entry_view(task: &Task, ctx: &Ctx<'_>) -> EntryView {
    let question = match task.status {
        Status::Open => unanswered_question(task, ctx),
        Status::Claimed | Status::Blocked | Status::Done | Status::Dropped => None,
    };
    let state = match task.status {
        Status::Open if question.is_some() => State::Waiting,
        Status::Open => State::ToDo,
        Status::Claimed => State::InProgress,
        Status::Blocked => State::Waiting,
        Status::Done => State::Done,
        Status::Dropped => State::Cancelled,
    };

    let date = match state {
        State::Done => latest_note_date(task, "done"),
        State::Cancelled => latest_note_date(task, "dropped"),
        State::ToDo | State::InProgress | State::Waiting => None,
    };
    let second = match state {
        State::InProgress => Some(SecondLine::InProgress {
            owner: task.owner.clone().unwrap_or_else(|| NO_OWNER.to_string()),
            since: since(task, ctx),
            stale: ctx.cfg.is_claim_stale(task, ctx.now),
        }),
        State::Waiting => Some(SecondLine::Waiting(wait_reason(task, question))),
        State::Cancelled => Some(SecondLine::Cancelled(
            task.dropped_reason().unwrap_or(NO_REASON).to_string(),
        )),
        State::ToDo | State::Done => None,
    };

    EntryView {
        id: ctx.cfg.id_config.format_id(task.id),
        title: task.title.clone(),
        state,
        date,
        second,
        summary: text_of(task.summary.as_deref()),
        why: text_of(task.why.as_deref()),
        result: match state {
            State::Done => text_of(task.latest_reason("done")),
            State::ToDo | State::InProgress | State::Waiting | State::Cancelled => None,
        },
        details: details(task, ctx),
    }
}

/// The title of the first question that `task` depends on and that no answer has closed. A question
/// is answered when it is done or dropped (R-DEP-1, R-KIND-3). `None` when there is none.
fn unanswered_question(task: &Task, ctx: &Ctx<'_>) -> Option<String> {
    task.depends.iter().find_map(|dep| {
        let question = ctx.find(dep.id?)?;
        (question.kind() == EntryKind::Question && !question.status.is_finished())
            .then(|| question.title.clone())
    })
}

/// The reason an entry waits: its question, else its problem when given back, else its block reason.
fn wait_reason(task: &Task, question: Option<String>) -> WaitReason {
    if let Some(title) = question {
        WaitReason::Question(title)
    } else if task.is_given_back() {
        WaitReason::GivenBack(block_reason(task))
    } else {
        WaitReason::Blocked(block_reason(task))
    }
}

/// The reason of the latest block or stuck note of `task`, or [`NO_REASON`].
fn block_reason(task: &Task) -> String {
    task.blocked_reason().unwrap_or(NO_REASON).to_string()
}

/// How long the claim of `task` has been held, counted in calendar days of the clock's offset.
/// `None` when the claim has no valid time (R-CLAIM-4).
fn since(task: &Task, ctx: &Ctx<'_>) -> Option<Since> {
    // The age needs a valid claim time; the days are then counted on the calendar.
    ctx.cfg.claim_age(task, ctx.now)?;
    let claimed = task.parse_claimed_at()?;
    let claimed_day = claimed.with_timezone(ctx.now.offset()).date_naive();
    let days = ctx
        .now
        .date_naive()
        .signed_duration_since(claimed_day)
        .num_days();
    Some(match days {
        ..=0 => Since::Today,
        1 => Since::Yesterday,
        n => Since::Days(n),
    })
}

/// The date of the latest note of `task` whose text starts with `word`.
fn latest_note_date(task: &Task, word: &str) -> Option<NaiveDate> {
    task.notes
        .iter()
        .rev()
        .find(|note| starts_with_word(&note.text, word))
        .map(|note| note.date)
}

/// Whether a note's text starts with `word`: its first word, before any dash, is `word`. So `done`,
/// `done — <note>` and `dropped with T-001 — <reason>` start with `done` or `dropped` as they should.
fn starts_with_word(text: &str, word: &str) -> bool {
    let before_dash = text.split(DASH).next().unwrap_or_default();
    before_dash.split_whitespace().next() == Some(word)
}

fn details(task: &Task, ctx: &Ctx<'_>) -> Details {
    Details {
        level: text_of(task.difficulty.as_deref()),
        priority: text_of(task.priority.as_deref()),
        task_type: text_of(task.task_type.as_deref()),
        area: text_of(task.area.as_deref()),
        files: text_of(task.files.as_deref()),
        verify: text_of(task.verify.as_deref()),
        verified: text_of(task.verified.as_deref()),
        commit: text_of(task.commit_raw.as_deref()),
        done_by: text_of(task.field_value("by")).or_else(|| text_of(task.owner.as_deref())),
        depends: dependencies(task, ctx),
        attempts: text_of(task.attempts.as_deref()),
        notes: task.notes.iter().map(note_line).collect(),
    }
}

fn dependencies(task: &Task, ctx: &Ctx<'_>) -> Vec<Dependency> {
    task.depends
        .iter()
        .map(|dep| match dep.id {
            Some(id) => Dependency {
                id: ctx.cfg.id_config.format_id(id),
                title: ctx.find(id).map(|found| found.title.clone()),
            },
            None => Dependency {
                id: dep.raw.clone(),
                title: None,
            },
        })
        .collect()
}

fn note_line(note: &Note) -> NoteLine {
    NoteLine {
        date: note.date,
        agent: note.agent.clone(),
        text: note.text.clone(),
    }
}

/// The value without its surrounding spaces, or `None` when it is empty.
fn text_of(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The clock of every test: 21 Nov 2026, 15:00, in the +01:00 offset.
    const NOW: &str = "2026-11-21T15:00:00+01:00";

    fn now() -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(NOW).unwrap()
    }

    fn day(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
    }

    /// The lines of a fixture as one text, with a final newline.
    fn text(lines: &[&str]) -> String {
        let mut text = lines.join("\n");
        text.push('\n');
        text
    }

    /// The view of the entry at `index` in `text`, read at [`NOW`] with no archive.
    fn view_at(text: &str, index: usize) -> EntryView {
        let doc = Document::parse_bytes(text.as_bytes()).unwrap();
        let cfg = OpsConfig::default();
        let ctx = Ctx {
            doc: &doc,
            archive: None,
            cfg: &cfg,
            now: now(),
        };
        entry_view(&doc.tasks[index], &ctx)
    }

    /// The second line of a single claimed task that was claimed at `when`.
    fn claimed_second(when: Option<&str>) -> Option<SecondLine> {
        let mut lines = vec![
            "### T-001 · Update the build instructions".to_string(),
            "- status: claimed".to_string(),
            "- owner: worker-1".to_string(),
        ];
        if let Some(when) = when {
            lines.push(format!("- claimed: {when}"));
        }
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        view_at(&text(&refs), 0).second
    }

    #[test]
    fn each_state_has_its_word_mark_and_class() {
        let all = [
            (State::ToDo, "to do", '○', "todo"),
            (State::InProgress, "in progress", '●', "prog"),
            (State::Waiting, "waiting", '◐', "wait"),
            (State::Done, "done", '✓', "done"),
            (State::Cancelled, "cancelled", '✕', "drop"),
        ];
        for (state, word, mark, class) in all {
            assert_eq!(state.word(), word);
            assert_eq!(state.mark(), mark);
            assert_eq!(state.class(), class);
        }
    }

    #[test]
    fn an_open_task_is_to_do_with_no_second_line() {
        let view = view_at(&text(&["### T-001 · Light the scene", "- status: open"]), 0);
        assert_eq!(view.id, "T-001");
        assert_eq!(view.title, "Light the scene");
        assert_eq!(view.state, State::ToDo);
        assert_eq!(view.date, None);
        assert_eq!(view.second, None);
        assert_eq!(view.result, None);
    }

    #[test]
    fn a_done_entry_has_the_date_and_result_of_its_latest_done_notes() {
        let view = view_at(
            &text(&[
                "### T-001 · Shipped part",
                "- status: done",
                "- note (2026-11-19, bob): added",
                "- note (2026-11-20, bob): done — first try",
                "- note (2026-11-21, bob): done",
            ]),
            0,
        );
        assert_eq!(view.state, State::Done);
        // The latest `done` note gives the date, even a bare one; the result is the latest note
        // with a text after the dash.
        assert_eq!(view.date, Some(day(21)));
        assert_eq!(view.result.as_deref(), Some("first try"));
        assert_eq!(view.second, None);
    }

    #[test]
    fn a_cancelled_entry_has_its_drop_date_and_reason() {
        let view = view_at(
            &text(&[
                "### T-001 · Cache compiled shaders",
                "- status: dropped",
                "- note (2026-11-18, bob): added",
                "- note (2026-11-20, bob): dropped — start-up is already fast enough",
            ]),
            0,
        );
        assert_eq!(view.state, State::Cancelled);
        assert_eq!(view.date, Some(day(20)));
        assert_eq!(
            view.second,
            Some(SecondLine::Cancelled(
                "start-up is already fast enough".to_string()
            ))
        );
        assert_eq!(view.result, None);
    }

    #[test]
    fn a_cancelled_entry_without_a_reason_says_so() {
        let view = view_at(
            &text(&[
                "### T-001 · Dropped quietly",
                "- status: dropped",
                "- note (2026-11-20, bob): dropped",
            ]),
            0,
        );
        assert_eq!(view.date, Some(day(20)));
        assert_eq!(
            view.second,
            Some(SecondLine::Cancelled(NO_REASON.to_string()))
        );
    }

    #[test]
    fn a_dropped_note_that_names_a_milestone_is_still_its_drop_date() {
        let view = view_at(
            &text(&[
                "### T-002 · Below a milestone",
                "- status: dropped",
                "- note (2026-11-20, bob): dropped with T-001 — the scene no longer needs it",
            ]),
            0,
        );
        assert_eq!(view.date, Some(day(20)));
        assert_eq!(
            view.second,
            Some(SecondLine::Cancelled(
                "the scene no longer needs it".to_string()
            ))
        );
    }

    #[test]
    fn a_blocked_task_is_waiting_for_its_reason() {
        let view = view_at(
            &text(&[
                "### T-001 · Draw a triangle",
                "- status: blocked",
                "- note (2026-11-20, bob): blocked — waiting for the review",
            ]),
            0,
        );
        assert_eq!(view.state, State::Waiting);
        assert_eq!(
            view.second,
            Some(SecondLine::Waiting(WaitReason::Blocked(
                "waiting for the review".to_string()
            )))
        );
    }

    #[test]
    fn a_blocked_task_with_no_reason_says_so() {
        let view = view_at(&text(&["### T-001 · Blocked", "- status: blocked"]), 0);
        assert_eq!(
            view.second,
            Some(SecondLine::Waiting(WaitReason::Blocked(
                NO_REASON.to_string()
            )))
        );
    }

    #[test]
    fn a_task_given_back_by_a_worker_says_so_with_its_problem() {
        let view = view_at(
            &text(&[
                "### T-001 · Draw a textured triangle",
                "- status: blocked",
                "- note (2026-11-20, worker-2): stuck — the schema file is missing",
            ]),
            0,
        );
        assert_eq!(view.state, State::Waiting);
        assert_eq!(
            view.second,
            Some(SecondLine::Waiting(WaitReason::GivenBack(
                "the schema file is missing".to_string()
            )))
        );
    }

    #[test]
    fn an_open_task_waits_for_an_unanswered_question_it_depends_on() {
        let question =
            "### T-002 · Vulkan only, or also DirectX 12?\n- type: question\n- status: open\n";
        let fixture =
            format!("### T-001 · Link the stages\n- status: open\n- depends: T-002\n{question}");
        let view = view_at(&fixture, 0);
        assert_eq!(view.state, State::Waiting);
        assert_eq!(
            view.second,
            Some(SecondLine::Waiting(WaitReason::Question(
                "Vulkan only, or also DirectX 12?".to_string()
            )))
        );
    }

    #[test]
    fn a_question_that_is_done_or_dropped_no_longer_makes_the_task_wait() {
        for answered in ["done", "dropped"] {
            let fixture = format!(
                "### T-001 · Link the stages\n- status: open\n- depends: T-002\n\
                 ### T-002 · Vulkan only?\n- type: question\n- status: {answered}\n"
            );
            let view = view_at(&fixture, 0);
            assert_eq!(view.state, State::ToDo, "answered by {answered}");
            assert_eq!(view.second, None);
        }
    }

    #[test]
    fn only_an_open_task_waits_for_a_question_and_a_claimed_one_does_not() {
        let fixture = "### T-001 · Link the stages\n- status: claimed\n- owner: worker-3\n\
            - claimed: 2026-11-21T09:00+01:00\n- depends: T-002\n\
            ### T-002 · Vulkan only?\n- type: question\n- status: open\n";
        let view = view_at(fixture, 0);
        assert_eq!(view.state, State::InProgress);
        assert!(matches!(view.second, Some(SecondLine::InProgress { .. })));
    }

    #[test]
    fn a_claim_made_today_is_since_today() {
        assert_eq!(
            claimed_second(Some("2026-11-21T09:00+01:00")),
            Some(SecondLine::InProgress {
                owner: "worker-1".to_string(),
                since: Some(Since::Today),
                stale: false,
            })
        );
    }

    #[test]
    fn a_claim_made_just_after_midnight_is_since_today_not_yesterday() {
        // Fourteen and a half hours old: today by the calendar, and not stale.
        assert_eq!(
            claimed_second(Some("2026-11-21T00:30+01:00")),
            Some(SecondLine::InProgress {
                owner: "worker-1".to_string(),
                since: Some(Since::Today),
                stale: false,
            })
        );
    }

    #[test]
    fn a_claim_made_late_yesterday_is_since_yesterday_and_fresh() {
        // Fifteen and a half hours old, but yesterday by the calendar.
        assert_eq!(
            claimed_second(Some("2026-11-20T23:30+01:00")),
            Some(SecondLine::InProgress {
                owner: "worker-1".to_string(),
                since: Some(Since::Yesterday),
                stale: false,
            })
        );
    }

    #[test]
    fn a_claim_older_than_the_ttl_is_stale_and_says_since_when() {
        // Thirty-one hours old: past the default 24 hours of `claim_ttl_hours`.
        assert_eq!(
            claimed_second(Some("2026-11-20T08:00+01:00")),
            Some(SecondLine::InProgress {
                owner: "worker-1".to_string(),
                since: Some(Since::Yesterday),
                stale: true,
            })
        );
        assert_eq!(
            claimed_second(Some("2026-11-18T10:00+01:00")),
            Some(SecondLine::InProgress {
                owner: "worker-1".to_string(),
                since: Some(Since::Days(3)),
                stale: true,
            })
        );
    }

    #[test]
    fn a_claim_with_no_valid_time_is_stale_and_has_no_since() {
        assert_eq!(
            claimed_second(None),
            Some(SecondLine::InProgress {
                owner: "worker-1".to_string(),
                since: None,
                stale: true,
            })
        );
    }

    #[test]
    fn summary_and_why_are_read_and_empty_values_are_left_out() {
        let view = view_at(
            &text(&[
                "### T-001 · Build",
                "- status: open",
                "- why: the page needs it",
                "- summary: The page shows the work.",
            ]),
            0,
        );
        assert_eq!(view.why.as_deref(), Some("the page needs it"));
        assert_eq!(view.summary.as_deref(), Some("The page shows the work."));

        let why_only = view_at(
            &text(&[
                "### T-001 · Build",
                "- status: open",
                "- why: the page needs it",
            ]),
            0,
        );
        assert_eq!(why_only.summary, None);
        assert_eq!(text_of(Some("   ")), None);
        assert_eq!(text_of(None), None);
    }

    #[test]
    fn details_hold_the_technical_fields_and_the_notes() {
        let view = view_at(
            &text(&[
                "### T-001 · Parse shader source",
                "- status: done",
                "- priority: P1",
                "- type: task",
                "- area: report",
                "- difficulty: 2",
                "- files: src/report/page/entry.rs",
                "- verify: cargo test --lib report::page",
                "- verified: 2026-11-21",
                "- commit: 75b1603",
                "- attempts: 1",
                "- by: Mara Lindqvist",
                "- owner: bob",
                "- note (2026-11-20, bob): added",
                "- note (2026-11-21, bob): done — parses both examples",
            ]),
            0,
        );
        let details = &view.details;
        assert_eq!(details.level.as_deref(), Some("2"));
        assert_eq!(details.priority.as_deref(), Some("P1"));
        assert_eq!(details.task_type.as_deref(), Some("task"));
        assert_eq!(details.area.as_deref(), Some("report"));
        assert_eq!(details.files.as_deref(), Some("src/report/page/entry.rs"));
        assert_eq!(
            details.verify.as_deref(),
            Some("cargo test --lib report::page")
        );
        assert_eq!(details.verified.as_deref(), Some("2026-11-21"));
        assert_eq!(details.commit.as_deref(), Some("75b1603"));
        assert_eq!(details.attempts.as_deref(), Some("1"));
        // `by` names the person who finished the task, and wins over the owner.
        assert_eq!(details.done_by.as_deref(), Some("Mara Lindqvist"));
        assert_eq!(
            details.notes,
            vec![
                NoteLine {
                    date: day(20),
                    agent: "bob".to_string(),
                    text: "added".to_string(),
                },
                NoteLine {
                    date: day(21),
                    agent: "bob".to_string(),
                    text: "done — parses both examples".to_string(),
                },
            ]
        );
        assert_eq!(view.result.as_deref(), Some("parses both examples"));
    }

    #[test]
    fn done_by_is_the_owner_when_there_is_no_by_field() {
        let view = view_at(
            &text(&["### T-001 · Parsed", "- status: done", "- owner: bob"]),
            0,
        );
        assert_eq!(view.details.done_by.as_deref(), Some("bob"));
        assert!(view.details.depends.is_empty());
        assert!(view.details.notes.is_empty());
    }

    #[test]
    fn depends_show_the_title_of_each_task_they_name() {
        let view = view_at(
            &text(&[
                "### T-001 · Link the program",
                "- status: open",
                "- depends: T-002, T-009",
                "### T-002 · Shipped part",
                "- status: done",
            ]),
            0,
        );
        assert_eq!(
            view.details.depends,
            vec![
                Dependency {
                    id: "T-002".to_string(),
                    title: Some("Shipped part".to_string()),
                },
                Dependency {
                    id: "T-009".to_string(),
                    title: None,
                },
            ]
        );
    }

    #[test]
    fn a_dependency_title_is_also_read_from_the_archive() {
        let doc = Document::parse_bytes(
            text(&["### T-001 · Build", "- status: open", "- depends: T-005"]).as_bytes(),
        )
        .unwrap();
        let archive =
            Document::parse_bytes(text(&["### T-005 · Older part", "- status: done"]).as_bytes())
                .unwrap();
        let cfg = OpsConfig::default();
        let ctx = Ctx {
            doc: &doc,
            archive: Some(&archive),
            cfg: &cfg,
            now: now(),
        };
        let view = entry_view(&doc.tasks[0], &ctx);
        assert_eq!(
            view.details.depends,
            vec![Dependency {
                id: "T-005".to_string(),
                title: Some("Older part".to_string()),
            }]
        );
    }

    #[test]
    fn starts_with_word_reads_the_first_word_before_any_dash() {
        assert!(starts_with_word("done", "done"));
        assert!(starts_with_word("done — shipped", "done"));
        assert!(starts_with_word("dropped with T-001 — no", "dropped"));
        assert!(!starts_with_word("donezo — shipped", "done"));
        assert!(!starts_with_word("unblocked", "blocked"));
    }
}
