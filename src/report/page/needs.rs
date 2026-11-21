//! The part *Needs you (n)* of the v1.0 page (T-199; R-REPORT-4, section 5.23).
//!
//! [`build_needs`] lists what waits for a person, in the order of R-REPORT-4: every open question,
//! then every task a worker gave back, then every claim that has gone stale. Each row names its
//! entry by title. The part only reads: nothing here changes a document.

use chrono::NaiveDate;

use super::entry::Ctx;
use crate::doc::reason::reason_in_text;
use crate::doc::{Document, EntryKind, Task};
use crate::ops::read::about_id;

/// The em dash that separates a note's words from its text (`docs/format.md` §9).
const DASH: char = '\u{2014}';

/// Shown in place of the owner of a claim that names nobody.
const NO_OWNER: &str = "no owner";

/// What the part says when nothing waits for a person.
pub const EMPTY: &str = "Nothing needs you.";

/// One row of the part *Needs you* (R-REPORT-4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Need {
    /// An open question: the tag "Question", its title, the date it was asked, the entry it is
    /// about and the tasks that wait for its answer.
    Question {
        title: String,
        /// The date of its `added` note, else of its first note. `None` when it has no dated note.
        asked: Option<NaiveDate>,
        /// The title of the entry its `about` field names. `None` when it names none, or an entry
        /// that no task list holds.
        about: Option<String>,
        /// The titles of the unfinished tasks whose `depends` name the question, in file order.
        waiting: Vec<String>,
    },
    /// A task a worker gave back (R-STUCK-1): the tag "Given back", its title and its problem.
    GivenBack {
        title: String,
        /// The date of its latest `stuck — …` note.
        date: NaiveDate,
        /// The text of that note.
        problem: String,
    },
    /// A claim that has gone stale (`claim_ttl_hours`, R-CLAIM-4): the tag "No activity", its title,
    /// its owner and the days the claim has been held.
    NoActivity {
        title: String,
        owner: String,
        /// Whole days since the claim was made. `None` when the claim has no valid time.
        days: Option<i64>,
    },
}

/// The rows of *Needs you* for `doc`, read with the configuration and clock of `ctx` (R-REPORT-4).
///
/// Every open question comes first (a question that is not done or dropped), then every task that
/// is given back (R-STUCK-1), then every task whose claim is stale. Each kind is in file order.
#[must_use]
pub fn build_needs(doc: &Document, ctx: &Ctx<'_>) -> Vec<Need> {
    let questions = doc
        .tasks
        .iter()
        .filter(|task| task.kind() == EntryKind::Question && !task.status.is_finished())
        .map(|question| question_need(doc, ctx, question));
    let given_back = doc.tasks.iter().filter_map(given_back_need);
    let stale = doc
        .tasks
        .iter()
        .filter(|task| ctx.cfg.is_claim_stale(task, ctx.now))
        .map(|task| no_activity_need(task, ctx));
    questions.chain(given_back).chain(stale).collect()
}

/// The heading of the part: "Needs you (n)", with the number of rows.
#[must_use]
pub fn heading(needs: &[Need]) -> String {
    format!("Needs you ({})", needs.len())
}

/// The sentence that says which tasks wait for a question: 'the task “A” waits for this answer', or
/// 'the tasks “A” and “B” wait for this answer', with three or more as “A”, “B” and “C”. `None` when
/// no task waits.
#[must_use]
pub fn waiting_text(titles: &[String]) -> Option<String> {
    let quoted: Vec<String> = titles.iter().map(|title| format!("“{title}”")).collect();
    match quoted.as_slice() {
        [] => None,
        [one] => Some(format!("the task {one} waits for this answer")),
        [init @ .., last] => Some(format!(
            "the tasks {} and {last} wait for this answer",
            init.join(", ")
        )),
    }
}

fn question_need(doc: &Document, ctx: &Ctx<'_>, question: &Task) -> Need {
    Need::Question {
        title: question.title.clone(),
        asked: asked_on(question),
        about: about_title(question, ctx),
        waiting: waiting_titles(doc, question),
    }
}

/// The date of the `added` note of `question`, else of its first note.
fn asked_on(question: &Task) -> Option<NaiveDate> {
    question
        .notes
        .iter()
        .find(|note| is_added(&note.text))
        .or_else(|| question.notes.first())
        .map(|note| note.date)
}

/// Whether a note's text is the `added` note: its words before any dash are `added`.
fn is_added(text: &str) -> bool {
    text.split(DASH)
        .next()
        .is_some_and(|words| words.trim() == "added")
}

/// The title of the entry that `question` names in its `about` field.
fn about_title(question: &Task, ctx: &Ctx<'_>) -> Option<String> {
    let target = about_id(question, ctx.cfg)?;
    ctx.find(target).map(|entry| entry.title.clone())
}

/// The titles of the unfinished tasks of `doc` whose `depends` name `question`, in file order.
fn waiting_titles(doc: &Document, question: &Task) -> Vec<String> {
    doc.tasks
        .iter()
        .filter(|task| !task.status.is_finished() && task.dependency_ids().contains(&question.id))
        .map(|task| task.title.clone())
        .collect()
}

/// The row of a task given back: its latest `stuck — …` note gives the date and the problem.
fn given_back_need(task: &Task) -> Option<Need> {
    if !task.is_given_back() {
        return None;
    }
    let (date, problem) = task.notes.iter().rev().find_map(|note| {
        reason_in_text(&note.text, "stuck").map(|problem| (note.date, problem.to_string()))
    })?;
    Some(Need::GivenBack {
        title: task.title.clone(),
        date,
        problem,
    })
}

fn no_activity_need(task: &Task, ctx: &Ctx<'_>) -> Need {
    Need::NoActivity {
        title: task.title.clone(),
        owner: task.owner.clone().unwrap_or_else(|| NO_OWNER.to_string()),
        days: ctx.cfg.claim_age(task, ctx.now).map(|age| age.num_days()),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, FixedOffset};

    use super::*;
    use crate::ops::OpsConfig;

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

    /// The rows of *Needs you* for a fixture, read at [`NOW`] with no archive.
    fn needs_of(text: &str) -> Vec<Need> {
        let doc = Document::parse_bytes(text.as_bytes()).unwrap();
        let cfg = OpsConfig::default();
        let ctx = Ctx {
            doc: &doc,
            archive: None,
            cfg: &cfg,
            now: now(),
        };
        build_needs(&doc, &ctx)
    }

    #[test]
    fn an_open_question_has_its_date_its_about_entry_and_the_tasks_that_wait() {
        let needs = needs_of(&text(&[
            "### T-001 · Link the stages into one program",
            "- status: open",
            "- depends: T-002",
            "### T-002 · Should compiled shaders target Vulkan only, or also DirectX 12?",
            "- type: question",
            "- status: open",
            "- about: T-003",
            "- note (2026-11-19, bob): added",
            "### T-003 · GPU compiler",
            "- status: open",
            "### T-004 · Draw the lights",
            "- status: open",
            "- depends: T-002",
            "### T-005 · Waits for a question that is done",
            "- status: done",
            "- depends: T-002",
        ]));
        assert_eq!(
            needs,
            vec![Need::Question {
                title: "Should compiled shaders target Vulkan only, or also DirectX 12?"
                    .to_string(),
                asked: Some(day(19)),
                about: Some("GPU compiler".to_string()),
                waiting: vec![
                    "Link the stages into one program".to_string(),
                    "Draw the lights".to_string(),
                ],
            }]
        );
    }

    #[test]
    fn a_question_is_dated_by_its_added_note_not_by_its_first_note() {
        let needs = needs_of(&text(&[
            "### T-002 · Which colour space?",
            "- type: question",
            "- status: open",
            "- note (2026-11-18, bob): reworded",
            "- note (2026-11-20, bob): added",
        ]));
        assert_eq!(
            needs,
            vec![Need::Question {
                title: "Which colour space?".to_string(),
                asked: Some(day(20)),
                about: None,
                waiting: Vec::new(),
            }]
        );
    }

    #[test]
    fn a_question_with_no_added_note_is_dated_by_its_first_note() {
        let needs = needs_of(&text(&[
            "### T-002 · Which colour space?",
            "- type: question",
            "- status: open",
            "- note (2026-11-18, bob): reworded",
            "- note (2026-11-20, bob): reworded again",
        ]));
        assert_eq!(
            needs,
            vec![Need::Question {
                title: "Which colour space?".to_string(),
                asked: Some(day(18)),
                about: None,
                waiting: Vec::new(),
            }]
        );
    }

    #[test]
    fn a_question_without_any_note_has_no_date_and_still_shows() {
        let needs = needs_of(&text(&[
            "### T-002 · Which colour space?",
            "- type: question",
            "- status: open",
            "- about: T-099",
        ]));
        assert_eq!(
            needs,
            vec![Need::Question {
                title: "Which colour space?".to_string(),
                asked: None,
                about: None,
                waiting: Vec::new(),
            }]
        );
    }

    #[test]
    fn an_answered_question_is_not_listed() {
        let needs = needs_of(&text(&[
            "### T-001 · Answered",
            "- type: question",
            "- status: done",
            "### T-002 · Dropped",
            "- type: question",
            "- status: dropped",
        ]));
        assert!(needs.is_empty());
    }

    #[test]
    fn a_task_given_back_has_the_date_and_problem_of_its_latest_stuck_note() {
        let needs = needs_of(&text(&[
            "### T-004 · Draw a textured triangle",
            "- status: blocked",
            "- note (2026-11-19, worker-2): stuck — the schema file is missing",
            "- note (2026-11-20, worker-2): stuck — the test image differs by 2 pixels",
            "### T-005 · Waits for the review",
            "- status: blocked",
            "- note (2026-11-20, worker-3): blocked — waiting for the review",
        ]));
        assert_eq!(
            needs,
            vec![Need::GivenBack {
                title: "Draw a textured triangle".to_string(),
                date: day(20),
                problem: "the test image differs by 2 pixels".to_string(),
            }]
        );
    }

    #[test]
    fn a_stale_claim_is_listed_with_its_owner_and_whole_days() {
        let needs = needs_of(&text(&[
            "### T-006 · Update the build instructions",
            "- status: claimed",
            "- owner: worker-1",
            "- claimed: 2026-11-17T10:00+01:00",
            "### T-007 · Fresh claim",
            "- status: claimed",
            "- owner: worker-2",
            "- claimed: 2026-11-21T09:00+01:00",
        ]));
        assert_eq!(
            needs,
            vec![Need::NoActivity {
                title: "Update the build instructions".to_string(),
                owner: "worker-1".to_string(),
                days: Some(4),
            }]
        );
    }

    #[test]
    fn a_stale_claim_with_no_valid_time_has_no_days_and_no_owner_says_so() {
        let needs = needs_of(&text(&[
            "### T-006 · Claimed without a time",
            "- status: claimed",
            "- owner: worker-1",
        ]));
        assert_eq!(
            needs,
            vec![Need::NoActivity {
                title: "Claimed without a time".to_string(),
                owner: "worker-1".to_string(),
                days: None,
            }]
        );

        let no_owner = needs_of(&text(&[
            "### T-008 · Claimed by nobody",
            "- status: claimed",
            "- claimed: 2026-11-17T10:00+01:00",
        ]));
        assert_eq!(
            no_owner,
            vec![Need::NoActivity {
                title: "Claimed by nobody".to_string(),
                owner: NO_OWNER.to_string(),
                days: Some(4),
            }]
        );
    }

    #[test]
    fn the_kinds_come_in_the_order_of_r_report_4_and_file_order_holds_inside_each() {
        let needs = needs_of(&text(&[
            "### T-001 · Stale first in the file",
            "- status: claimed",
            "- owner: worker-1",
            "- claimed: 2026-11-10T10:00+01:00",
            "### T-002 · First given back",
            "- status: blocked",
            "- note (2026-11-19, worker-2): stuck — first problem",
            "### T-003 · First question",
            "- type: question",
            "- status: open",
            "- note (2026-11-18, bob): added",
            "### T-004 · Second given back",
            "- status: blocked",
            "- note (2026-11-20, worker-3): stuck — second problem",
            "### T-005 · Second question",
            "- type: question",
            "- status: open",
            "- note (2026-11-17, bob): added",
        ]));
        let titles: Vec<&str> = needs
            .iter()
            .map(|need| match need {
                Need::Question { title, .. }
                | Need::GivenBack { title, .. }
                | Need::NoActivity { title, .. } => title.as_str(),
            })
            .collect();
        assert_eq!(
            titles,
            vec![
                "First question",
                "Second question",
                "First given back",
                "Second given back",
                "Stale first in the file",
            ]
        );
        assert!(matches!(needs[0], Need::Question { .. }));
        assert!(matches!(needs[2], Need::GivenBack { .. }));
        assert!(matches!(needs[4], Need::NoActivity { .. }));
    }

    #[test]
    fn a_reclaimed_task_is_no_longer_given_back() {
        let needs = needs_of(&text(&[
            "### T-004 · Draw a textured triangle",
            "- status: claimed",
            "- owner: worker-2",
            "- claimed: 2026-11-21T09:00+01:00",
            "- note (2026-11-20, worker-2): stuck — the test image differs by 2 pixels",
        ]));
        assert!(needs.is_empty());
    }

    #[test]
    fn an_empty_list_reads_nothing_needs_you() {
        let needs = needs_of("");
        assert!(needs.is_empty());
        assert_eq!(heading(&needs), "Needs you (0)");
        assert_eq!(EMPTY, "Nothing needs you.");
    }

    #[test]
    fn the_heading_gives_the_number_of_rows() {
        let needs = needs_of(&text(&[
            "### T-002 · First question",
            "- type: question",
            "- status: open",
            "### T-004 · Given back",
            "- status: blocked",
            "- note (2026-11-20, worker-2): stuck — a problem",
        ]));
        assert_eq!(heading(&needs), "Needs you (2)");
    }

    #[test]
    fn waiting_text_names_one_two_or_three_or_more_tasks() {
        assert_eq!(waiting_text(&[]), None);
        assert_eq!(
            waiting_text(&["A".to_string()]).as_deref(),
            Some("the task “A” waits for this answer")
        );
        assert_eq!(
            waiting_text(&["A".to_string(), "B".to_string()]).as_deref(),
            Some("the tasks “A” and “B” wait for this answer")
        );
        assert_eq!(
            waiting_text(&["A".to_string(), "B".to_string(), "C".to_string()]).as_deref(),
            Some("the tasks “A”, “B” and “C” wait for this answer")
        );
    }
}
