//! The closed part "Show chain" of a task's details (T-242; R-REPORT-7; T-228).
//!
//! [`push_chain`] writes the chain of one task from [`super::chain::Chain`] in the markup of
//! `docs/report-mockup.html`: a `details.chain` element, closed, with the groups "Before this task"
//! and "After this task", one `p.chain-line` per path and a count of the paths not shown. A step is
//! its state mark (or a milestone's "M"), its ID and its title, with its state in words for screen
//! readers and print. Every text from the file is escaped with [`escape`]; titles also go through
//! [`inline_code`], as the titles of the rest of the page do. The part needs no script.

use std::fmt::Write as _;

use super::chain::{Chain, MAX_COUNTED, Path, PathEnd, Step, StepState};
use super::text::inline_code;
use super::tree::MilestoneState;
use crate::report::html::escape;

/// The separator between two steps of a line: an arrow with a space on each side.
const ARROW: &str = " \u{2192} ";

/// The ellipsis of a cut path and of a loop back ("… back to T-005").
const ELLIPSIS: &str = "\u{2026}";

/// Which group a path belongs to: the paths through `depends` before the task, or the paths
/// through the entries that wait for it after the task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Group {
    Before,
    After,
}

/// Writes the closed part "Show chain" of a task (R-REPORT-7): the group of paths before the task
/// and the group of paths after it. A group with no path is left out, and so is its count of
/// paths not shown when there is none.
pub fn push_chain(h: &mut String, chain: &Chain) {
    h.push_str("<details class=\"chain\"><summary>Show chain</summary>\n");
    if !chain.before.is_empty() {
        h.push_str("<p class=\"lbl\">Before this task</p>\n");
        for path in &chain.before {
            push_line(h, path, Group::Before);
        }
        push_more(h, chain.more_before);
    }
    if !chain.after.is_empty() {
        h.push_str("<p class=\"lbl\">After this task</p>\n");
        for path in &chain.after {
            push_line(h, path, Group::After);
        }
        push_more(h, chain.more_after);
    }
    h.push_str("</details>\n");
}

/// Writes one path as a `p.chain-line`. A line before the task reads from its far end to the task,
/// and a line after the task from the task to its far end. A cut path is marked with "…" at the far
/// end of its line, and a loop with "… back to" at the end of its steps.
fn push_line(h: &mut String, path: &Path, group: Group) {
    let (loop_id, steps) = shown_steps(path, group);
    let mut parts: Vec<String> = Vec::new();
    match group {
        Group::Before => {
            if path.cut {
                parts.push(ELLIPSIS.to_string());
            }
            parts.extend(steps.iter().map(step_html));
            if let Some(id) = loop_id {
                parts.push(back_to(id));
            }
            parts.push("<b>this task</b>".to_string());
        }
        Group::After => {
            parts.push("<b>this task</b>".to_string());
            parts.extend(steps.iter().map(step_html));
            if let Some(id) = loop_id {
                parts.push(back_to(id));
            }
            if path.cut {
                parts.push(ELLIPSIS.to_string());
            }
        }
    }
    let _ = writeln!(h, "<p class=\"chain-line\">{}</p>", parts.join(ARROW));
}

/// The steps of a path that are written as steps, and the ID of its loop when it ends in one.
///
/// A loop path repeats the ID it comes back to as its last step in walk order: the first step of a
/// line before the task, the last of a line after it. That step is not written as a step; the
/// "… back to" text names it instead. Any other step stays.
fn shown_steps(path: &Path, group: Group) -> (Option<&str>, &[Step]) {
    let PathEnd::Loop(id) = &path.end else {
        return (None, &path.steps);
    };
    let steps = match group {
        Group::Before => match path.steps.split_first() {
            Some((first, rest)) if first.id == *id => rest,
            _ => &path.steps,
        },
        Group::After => match path.steps.split_last() {
            Some((last, rest)) if last.id == *id => rest,
            _ => &path.steps,
        },
    };
    (Some(id.as_str()), steps)
}

/// The text "… back to T-005", with the ID as a `span.cid`.
fn back_to(id: &str) -> String {
    format!(
        "{ELLIPSIS} back to <span class=\"cid\">{}</span>",
        escape(id)
    )
}

/// One step of a line: a task, a question or an idea with its mark, a milestone with its "M", or an
/// ID that names no entry. An archived step is marked as such.
fn step_html(step: &Step) -> String {
    let mut out = match &step.state {
        StepState::Task(state) => format!(
            "<span class=\"mark {}\">{}</span> <span class=\"cid\">{}</span> {} <span class=\"sr\">({})</span>",
            state.class(),
            state.mark(),
            escape(&step.id),
            inline_code(&step.title),
            state.word(),
        ),
        StepState::Milestone(state) => format!(
            "<span class=\"ms-mark\" role=\"img\" aria-label=\"Milestone\">M</span> <span class=\"cid\">{}</span> {} <span class=\"muted\">({})</span>",
            escape(&step.id),
            inline_code(&step.title),
            milestone_words(*state),
        ),
        StepState::NotFound => format!(
            "<span class=\"cid\">{}</span> <span class=\"muted\">(not found)</span>",
            escape(&step.id),
        ),
    };
    if step.archived {
        out.push_str(" <span class=\"muted\">(archived)</span>");
    }
    out
}

/// A milestone's state in words, as the tree shows it (without its date).
const fn milestone_words(state: MilestoneState) -> &'static str {
    match state {
        MilestoneState::NotStarted => "not started",
        MilestoneState::InProgress => "in progress",
        MilestoneState::Finished(_) => "finished",
        MilestoneState::Cancelled(_) => "cancelled",
    }
}

/// The count of paths not shown, after a group's lines: "and 1 more path", "and 3 more paths", or
/// "and more than 100 more paths" from 101 on. Nothing for none.
fn push_more(h: &mut String, more: usize) {
    let text = match more {
        0 => return,
        1 => "and 1 more path".to_string(),
        n if n < MAX_COUNTED => format!("and {n} more paths"),
        _ => format!("and more than {} more paths", MAX_COUNTED - 1),
    };
    let _ = writeln!(h, "<p class=\"muted\">{text}</p>");
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;

    use super::*;
    use crate::doc::Document;
    use crate::ops::OpsConfig;
    use crate::report::page::chain::ChainIndex;
    use crate::report::page::entry::{Ctx, State};

    fn task(id: &str, title: &str, state: State) -> Step {
        Step {
            id: id.to_string(),
            title: title.to_string(),
            state: StepState::Task(state),
            archived: false,
        }
    }

    fn path(steps: Vec<Step>) -> Path {
        Path {
            steps,
            cut: false,
            end: PathEnd::Open,
        }
    }

    fn chain(before: Vec<Path>, after: Vec<Path>) -> Chain {
        Chain {
            before,
            more_before: 0,
            after,
            more_after: 0,
        }
    }

    fn html_of(chain: &Chain) -> String {
        let mut h = String::new();
        push_chain(&mut h, chain);
        h
    }

    #[test]
    fn the_closed_part_has_both_groups_in_order() {
        let html = html_of(&chain(
            vec![path(vec![
                task("T-001", "Define the grammar", State::Done),
                task("T-002", "Split the text", State::Done),
            ])],
            vec![path(vec![
                task("T-003", "Emit the vertex stage", State::InProgress),
                task("T-004", "Link the stages", State::Waiting),
            ])],
        ));
        assert_eq!(
            html,
            concat!(
                "<details class=\"chain\"><summary>Show chain</summary>\n",
                "<p class=\"lbl\">Before this task</p>\n",
                "<p class=\"chain-line\"><span class=\"mark done\">\u{2713}</span> <span class=\"cid\">T-001</span> Define the grammar <span class=\"sr\">(done)</span> \u{2192} <span class=\"mark done\">\u{2713}</span> <span class=\"cid\">T-002</span> Split the text <span class=\"sr\">(done)</span> \u{2192} <b>this task</b></p>\n",
                "<p class=\"lbl\">After this task</p>\n",
                "<p class=\"chain-line\"><b>this task</b> \u{2192} <span class=\"mark prog\">\u{25CF}</span> <span class=\"cid\">T-003</span> Emit the vertex stage <span class=\"sr\">(in progress)</span> \u{2192} <span class=\"mark wait\">\u{25D0}</span> <span class=\"cid\">T-004</span> Link the stages <span class=\"sr\">(waiting)</span></p>\n",
                "</details>\n",
            )
        );
    }

    #[test]
    fn a_group_without_paths_is_left_out() {
        let html = html_of(&chain(
            Vec::new(),
            vec![path(vec![task("T-002", "Trim the wick", State::ToDo)])],
        ));
        assert!(!html.contains("Before this task"));
        assert!(html.contains("<p class=\"lbl\">After this task</p>"));
    }

    #[test]
    fn a_milestone_step_has_the_milestone_marker_and_its_state_in_words() {
        let milestone = |state| Step {
            id: "T-150".to_string(),
            title: "Develop the renderer".to_string(),
            state: StepState::Milestone(state),
            archived: false,
        };
        let html = html_of(&chain(
            vec![path(vec![milestone(MilestoneState::InProgress)])],
            vec![path(vec![milestone(MilestoneState::Finished(None))])],
        ));
        assert!(html.contains(
            "<span class=\"ms-mark\" role=\"img\" aria-label=\"Milestone\">M</span> <span class=\"cid\">T-150</span> Develop the renderer <span class=\"muted\">(in progress)</span>"
        ));
        assert!(html.contains(
            "<span class=\"cid\">T-150</span> Develop the renderer <span class=\"muted\">(finished)</span>"
        ));
        assert!(!html.contains("class=\"mark"));
    }

    #[test]
    fn every_milestone_state_has_its_words() {
        assert_eq!(milestone_words(MilestoneState::NotStarted), "not started");
        assert_eq!(milestone_words(MilestoneState::InProgress), "in progress");
        assert_eq!(milestone_words(MilestoneState::Finished(None)), "finished");
        assert_eq!(
            milestone_words(MilestoneState::Cancelled(None)),
            "cancelled"
        );
    }

    #[test]
    fn a_not_found_step_is_an_id_and_not_found_in_muted_text() {
        let missing = Step {
            id: "T-999".to_string(),
            title: String::new(),
            state: StepState::NotFound,
            archived: false,
        };
        let html = html_of(&chain(vec![path(vec![missing])], Vec::new()));
        assert!(html.contains(
            "<span class=\"cid\">T-999</span> <span class=\"muted\">(not found)</span> \u{2192} <b>this task</b>"
        ));
    }

    #[test]
    fn an_archived_step_adds_archived_after_its_state() {
        let mut archived = task("T-010", "Older part", State::Done);
        archived.archived = true;
        let html = html_of(&chain(vec![path(vec![archived])], Vec::new()));
        assert!(html.contains(
            "<span class=\"sr\">(done)</span> <span class=\"muted\">(archived)</span> \u{2192} <b>this task</b>"
        ));
    }

    #[test]
    fn a_cut_before_line_starts_with_an_ellipsis() {
        let cut = Path {
            steps: vec![task("T-001", "Far away", State::Done)],
            cut: true,
            end: PathEnd::Open,
        };
        let html = html_of(&chain(vec![cut], Vec::new()));
        assert!(html.contains(
            "<p class=\"chain-line\">\u{2026} \u{2192} <span class=\"mark done\">\u{2713}</span> <span class=\"cid\">T-001</span> Far away"
        ));
    }

    #[test]
    fn a_cut_after_line_ends_with_an_ellipsis_after_the_task() {
        // The steps nearest the task are kept, so the cut is at the far end, after the task.
        let cut = Path {
            steps: vec![task("T-003", "Near", State::ToDo)],
            cut: true,
            end: PathEnd::Open,
        };
        let html = html_of(&chain(Vec::new(), vec![cut]));
        assert!(html.contains(
            "<p class=\"chain-line\"><b>this task</b> \u{2192} <span class=\"mark todo\">\u{25CB}</span> <span class=\"cid\">T-003</span> Near <span class=\"sr\">(to do)</span> \u{2192} \u{2026}</p>"
        ));
    }

    #[test]
    fn a_loop_before_the_task_names_the_id_it_comes_back_to() {
        // T-001 waits for T-002, which waits for T-001: the walk repeats T-001 as its first step.
        let looped = Path {
            steps: vec![
                task("T-001", "Light the lamp", State::ToDo),
                task("T-002", "Trim the wick", State::ToDo),
            ],
            cut: false,
            end: PathEnd::Loop("T-001".to_string()),
        };
        let html = html_of(&chain(vec![looped], Vec::new()));
        assert!(html.contains(
            "<p class=\"chain-line\"><span class=\"mark todo\">\u{25CB}</span> <span class=\"cid\">T-002</span> Trim the wick <span class=\"sr\">(to do)</span> \u{2192} \u{2026} back to <span class=\"cid\">T-001</span> \u{2192} <b>this task</b></p>"
        ));
    }

    #[test]
    fn a_loop_after_the_task_ends_with_the_id_it_comes_back_to() {
        let looped = Path {
            steps: vec![
                task("T-002", "Trim the wick", State::ToDo),
                task("T-001", "Light the lamp", State::ToDo),
            ],
            cut: false,
            end: PathEnd::Loop("T-001".to_string()),
        };
        let html = html_of(&chain(Vec::new(), vec![looped]));
        assert!(html.contains(
            "<p class=\"chain-line\"><b>this task</b> \u{2192} <span class=\"mark todo\">\u{25CB}</span> <span class=\"cid\">T-002</span> Trim the wick <span class=\"sr\">(to do)</span> \u{2192} \u{2026} back to <span class=\"cid\">T-001</span></p>"
        ));
    }

    #[test]
    fn a_loop_from_the_index_is_written_without_its_repeated_step() {
        // The real walk of `chain_of`: T-001 and T-002 wait for each other.
        let text = "### T-001 · Light the lamp\n- status: open\n- depends: T-002\n### T-002 · Trim the wick\n- status: open\n- depends: T-001\n";
        let doc = Document::parse_bytes(text.as_bytes()).unwrap();
        let cfg = OpsConfig::default();
        let now = DateTime::parse_from_rfc3339("2026-11-21T15:00:00+01:00").unwrap();
        let ctx = Ctx {
            doc: &doc,
            archive: None,
            cfg: &cfg,
            now,
        };
        let index = ChainIndex::new(&doc, None, &ctx);
        let html = html_of(&index.chain_of("T-001").unwrap());
        assert!(html.contains(
            "<p class=\"chain-line\"><span class=\"mark todo\">\u{25CB}</span> <span class=\"cid\">T-002</span> Trim the wick <span class=\"sr\">(to do)</span> \u{2192} \u{2026} back to <span class=\"cid\">T-001</span> \u{2192} <b>this task</b></p>"
        ));
        assert!(html.contains(
            "<p class=\"chain-line\"><b>this task</b> \u{2192} <span class=\"mark todo\">\u{25CB}</span> <span class=\"cid\">T-002</span> Trim the wick <span class=\"sr\">(to do)</span> \u{2192} \u{2026} back to <span class=\"cid\">T-001</span></p>"
        ));
    }

    #[test]
    fn the_more_text_says_one_path_five_paths_or_more_than_a_hundred() {
        let line = || path(vec![task("T-002", "Trim the wick", State::ToDo)]);
        let with_more = |more: usize| {
            let mut c = chain(vec![line()], vec![line()]);
            c.more_before = more;
            c.more_after = more;
            html_of(&c)
        };
        let one = with_more(1);
        assert_eq!(one.matches("and 1 more path</p>").count(), 2);
        assert!(!one.contains("and 1 more paths"));

        let five = with_more(5);
        assert_eq!(five.matches("and 5 more paths</p>").count(), 2);

        let hundred = with_more(100);
        assert!(hundred.contains("and 100 more paths</p>"));

        let many = with_more(101);
        assert_eq!(many.matches("and more than 100 more paths</p>").count(), 2);
        assert!(!many.contains("and 101 more paths"));
    }

    #[test]
    fn no_more_text_without_a_count() {
        let html = html_of(&chain(
            vec![path(vec![task("T-002", "Trim", State::ToDo)])],
            Vec::new(),
        ));
        assert!(!html.contains("more path"));
    }

    #[test]
    fn a_title_with_script_is_escaped() {
        let hostile = task(
            "T-001",
            "<script>alert(1)</script> & \"quoted\"",
            State::ToDo,
        );
        let html = html_of(&chain(vec![path(vec![hostile])], Vec::new()));
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt; &amp; &quot;quoted&quot;"));
    }

    #[test]
    fn a_title_with_backticks_shows_code() {
        let coded = task("T-001", "Run `todo add` first", State::ToDo);
        let html = html_of(&chain(vec![path(vec![coded])], Vec::new()));
        assert!(html.contains("Run <code>todo add</code> first"));
    }
}
