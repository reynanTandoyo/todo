//! The part *Last 7 days* of the v1.0 page (T-200; R-REPORT-5, section 5.23).
//!
//! [`build_recent`] lists what changed in the seven days that end on the day of the build: tasks
//! done and cancelled, milestones finished, and entries added. Events come from the note dates of
//! `TODO.md` and of the archive. [`row_text`] gives the words of one row. The part only reads:
//! nothing here changes a document.

use std::cmp::Reverse;
use std::collections::BTreeMap;

use chrono::NaiveDate;

use super::entry::Ctx;
use super::tree::{MilestoneView, Tree, build_tree};
use crate::doc::{Document, EntryKind, NO_REASON, Note, Task, TaskId};

/// The em dash that separates a row's name from its words (`docs/format.md` §9).
const DASH: char = '\u{2014}';

/// What the part says when nothing changed in the seven days.
pub const EMPTY: &str = "Nothing changed in the last 7 days.";

/// How many days the part covers: the day of the build and the six days before it.
const DAYS: i64 = 7;

/// How many rows are shown before the closed part "Earlier this week".
const SHOWN: usize = 15;

/// One event of the part (R-REPORT-5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A milestone finished (its latest `done` note), with the tasks below it that are done of
    /// those counted (cancelled tasks left out).
    MilestoneFinished {
        title: String,
        done: usize,
        total: usize,
    },
    /// A task done (its latest `done` note).
    Done { title: String },
    /// A task cancelled (its latest `dropped` note), with its reason.
    Cancelled { title: String, reason: String },
    /// Entries added on one day below one milestone, or below none (`milestone` is `None`, Other
    /// work). `title` is the entry's title when `count` is 1, and empty otherwise.
    Added {
        count: usize,
        milestone: Option<String>,
        title: String,
    },
}

/// The rows of the part (R-REPORT-5): the first 15 in `rows`, the rest in `earlier`. Each row has
/// its date; the newest day comes first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recent {
    /// The rows shown.
    pub rows: Vec<(NaiveDate, Event)>,
    /// The rows behind the closed part "Earlier this week".
    pub earlier: Vec<(NaiveDate, Event)>,
}

/// The kinds of event, in the order they show within one day (R-REPORT-5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    MilestoneFinished,
    Done,
    Cancelled,
    Added,
}

/// An event with its date and its kind, which fixes its place within the day.
struct Dated {
    date: NaiveDate,
    kind: Kind,
    event: Event,
}

/// One `added` note of a task or milestone in the seven days, before the entries of one day below
/// one milestone are grouped into one row.
struct AddedEntry {
    date: NaiveDate,
    /// The milestone above the entry: its ID and title. `None` for Other work.
    parent: Option<(TaskId, String)>,
    title: String,
    /// Whether the entry is a task. Tasks of one day below one milestone are one row; milestones
    /// are never grouped, so a row that says "tasks" holds tasks only.
    is_task: bool,
}

/// One row of added entries under construction.
struct Group {
    date: NaiveDate,
    parent: Option<TaskId>,
    milestone: Option<String>,
    is_task: bool,
    count: usize,
    title: String,
}

/// The rows of the part for `doc` and its `archive`, read with the clock of `ctx` (R-REPORT-5).
///
/// The seven days are the day of `ctx.now` and the six days before it. Events from `doc` come
/// before those from `archive`, so within one day and kind the file order holds across both.
#[must_use]
pub fn build_recent(doc: &Document, archive: Option<&Document>, ctx: &Ctx<'_>) -> Recent {
    let today = ctx.now.date_naive();
    let mut dated = Vec::new();
    let mut added = Vec::new();
    collect_events(doc, ctx, today, &mut dated, &mut added);
    if let Some(archive) = archive {
        collect_events(archive, ctx, today, &mut dated, &mut added);
    }
    dated.extend(group_added(added));
    // A stable sort keeps the file order of the events of one kind on one day.
    dated.sort_by_key(|entry| (Reverse(entry.date), entry.kind));

    let mut rows: Vec<(NaiveDate, Event)> = dated
        .into_iter()
        .map(|entry| (entry.date, entry.event))
        .collect();
    let earlier = rows.split_off(rows.len().min(SHOWN));
    Recent { rows, earlier }
}

/// The words of one row (R-REPORT-5).
#[must_use]
pub fn row_text(event: &Event) -> String {
    match event {
        Event::MilestoneFinished {
            title,
            done: 1,
            total: 1,
        } => format!("{title} finished {DASH} its 1 task done"),
        Event::MilestoneFinished { title, done, total } if done == total => {
            format!("{title} finished {DASH} all {total} tasks done")
        }
        Event::MilestoneFinished {
            title,
            done,
            total: 1,
        } => format!("{title} finished {DASH} {done} of 1 task done"),
        Event::MilestoneFinished { title, done, total } => {
            format!("{title} finished {DASH} {done} of {total} tasks done")
        }
        Event::Done { title } => title.clone(),
        Event::Cancelled { title, reason } => format!("{title} {DASH} cancelled: {reason}"),
        Event::Added {
            count: 1, title, ..
        } => format!("Added: {title}"),
        Event::Added {
            count, milestone, ..
        } => {
            let place = milestone.as_deref().unwrap_or("Other work");
            format!("{count} tasks added to {place}")
        }
    }
}

/// Adds the events of `doc`: milestones finished, tasks done and cancelled, and the `added` notes
/// of tasks and milestones in the seven days, each to its list, in file order.
fn collect_events(
    doc: &Document,
    ctx: &Ctx<'_>,
    today: NaiveDate,
    dated: &mut Vec<Dated>,
    added: &mut Vec<AddedEntry>,
) {
    let counts = milestone_counts(&build_tree(doc, ctx));
    for task in &doc.tasks {
        match task.kind() {
            EntryKind::Task => {
                push_task_events(task, today, dated);
                push_added(doc, task, today, added);
            }
            EntryKind::Milestone => {
                push_milestone_event(task, ctx, &counts, today, dated);
                push_added(doc, task, today, added);
            }
            EntryKind::Question | EntryKind::Idea => {}
        }
    }
}

/// The done and cancelled events of `task`, each from its latest note of that word.
fn push_task_events(task: &Task, today: NaiveDate, dated: &mut Vec<Dated>) {
    if let Some(date) = day_in_window(latest_note(task, "done"), today) {
        dated.push(Dated {
            date,
            kind: Kind::Done,
            event: Event::Done {
                title: task.title.clone(),
            },
        });
    }
    if let Some(date) = day_in_window(latest_note(task, "dropped"), today) {
        dated.push(Dated {
            date,
            kind: Kind::Cancelled,
            event: Event::Cancelled {
                title: task.title.clone(),
                reason: task.dropped_reason().unwrap_or(NO_REASON).to_string(),
            },
        });
    }
}

/// The finished event of `milestone`, from its latest `done` note, with the counts the tree gives.
fn push_milestone_event(
    milestone: &Task,
    ctx: &Ctx<'_>,
    counts: &BTreeMap<String, (usize, usize)>,
    today: NaiveDate,
    dated: &mut Vec<Dated>,
) {
    let Some(date) = day_in_window(latest_note(milestone, "done"), today) else {
        return;
    };
    let id = ctx.cfg.id_config.format_id(milestone.id);
    let Some(&(done, total)) = counts.get(&id) else {
        return;
    };
    dated.push(Dated {
        date,
        kind: Kind::MilestoneFinished,
        event: Event::MilestoneFinished {
            title: milestone.title.clone(),
            done,
            total,
        },
    });
}

/// Adds one [`AddedEntry`] for each `added` note of `task` in the seven days, with the milestone
/// above `task` as its parent.
fn push_added(doc: &Document, task: &Task, today: NaiveDate, added: &mut Vec<AddedEntry>) {
    let parent = doc
        .milestone_ancestors(task)
        .first()
        .map(|milestone| (milestone.id, milestone.title.clone()));
    let is_task = task.kind() == EntryKind::Task;
    for note in task
        .notes
        .iter()
        .filter(|note| starts_with_word(&note.text, "added") && in_window(note.date, today))
    {
        added.push(AddedEntry {
            date: note.date,
            parent: parent.clone(),
            title: task.title.clone(),
            is_task,
        });
    }
}

/// Groups the added entries of one day below one milestone into one row each, in the order of
/// their first entry. A milestone is always a row of its own.
fn group_added(entries: Vec<AddedEntry>) -> Vec<Dated> {
    let mut groups: Vec<Group> = Vec::new();
    for entry in entries {
        let parent_id = entry.parent.as_ref().map(|(id, _)| *id);
        let existing = if entry.is_task {
            groups.iter_mut().find(|group| {
                group.is_task && group.date == entry.date && group.parent == parent_id
            })
        } else {
            None
        };
        match existing {
            Some(group) => group.count += 1,
            None => groups.push(Group {
                date: entry.date,
                parent: parent_id,
                milestone: entry.parent.map(|(_, title)| title),
                is_task: entry.is_task,
                count: 1,
                title: entry.title,
            }),
        }
    }
    groups
        .into_iter()
        .map(|group| Dated {
            date: group.date,
            kind: Kind::Added,
            event: Event::Added {
                count: group.count,
                milestone: group.milestone,
                title: if group.count == 1 {
                    group.title
                } else {
                    String::new()
                },
            },
        })
        .collect()
}

/// The "N of M" of every milestone of `tree` by its ID, at any depth.
fn milestone_counts(tree: &Tree) -> BTreeMap<String, (usize, usize)> {
    let mut counts = BTreeMap::new();
    let mut pending: Vec<&MilestoneView> = tree.milestones.iter().collect();
    while let Some(milestone) = pending.pop() {
        counts.insert(milestone.id.clone(), (milestone.done, milestone.total));
        pending.extend(milestone.milestones.iter());
    }
    counts
}

/// The latest note of `task` whose text starts with `word`.
fn latest_note<'a>(task: &'a Task, word: &str) -> Option<&'a Note> {
    task.notes
        .iter()
        .rev()
        .find(|note| starts_with_word(&note.text, word))
}

/// The date of `note` when it falls in the seven days.
fn day_in_window(note: Option<&Note>, today: NaiveDate) -> Option<NaiveDate> {
    note.map(|note| note.date)
        .filter(|date| in_window(*date, today))
}

/// Whether `date` is the day of `today` or one of the six days before it.
fn in_window(date: NaiveDate, today: NaiveDate) -> bool {
    (0..DAYS).contains(&today.signed_duration_since(date).num_days())
}

/// Whether a note's text starts with `word`: its first word, before any dash, is `word`. The same
/// reading as the entry views use (`entry.rs`).
fn starts_with_word(text: &str, word: &str) -> bool {
    let before_dash = text.split(DASH).next().unwrap_or_default();
    before_dash.split_whitespace().next() == Some(word)
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, FixedOffset};

    use super::*;
    use crate::ops::OpsConfig;

    /// The clock of every test: 21 Nov 2026, 15:00, in the +01:00 offset.
    const NOW: &str = "2026-11-21T15:00:00+01:00";

    fn day(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
    }

    /// The lines of a fixture as one text, with a final newline.
    fn text(lines: &[&str]) -> String {
        let mut text = lines.join("\n");
        text.push('\n');
        text
    }

    /// The part for `doc` and `archive`, read at [`NOW`].
    fn recent_of(doc: &str, archive: Option<&str>) -> Recent {
        let doc = Document::parse_bytes(doc.as_bytes()).unwrap();
        let archive = archive.map(|text| Document::parse_bytes(text.as_bytes()).unwrap());
        let cfg = OpsConfig::default();
        let now = DateTime::<FixedOffset>::parse_from_rfc3339(NOW).unwrap();
        let ctx = Ctx {
            doc: &doc,
            archive: archive.as_ref(),
            cfg: &cfg,
            now,
        };
        build_recent(&doc, archive.as_ref(), &ctx)
    }

    /// The words of every shown row, in order.
    fn shown(recent: &Recent) -> Vec<String> {
        recent
            .rows
            .iter()
            .map(|(_, event)| row_text(event))
            .collect()
    }

    /// The dates of every shown row, in order.
    fn dates(recent: &Recent) -> Vec<NaiveDate> {
        recent.rows.iter().map(|(date, _)| *date).collect()
    }

    #[test]
    fn each_kind_of_event_is_listed_newest_day_first_and_in_kind_order_within_a_day() {
        let fixture = text(&[
            "### T-001 · Shader C",
            "- status: done",
            "- type: milestone",
            "- note (2026-11-18, bob): added",
            "- note (2026-11-20, bob): done — all in",
            "### T-002 · Parse source",
            "- status: done",
            "- milestone: T-001",
            "- note (2026-11-20, bob): added",
            "- note (2026-11-20, bob): done — parses both examples",
            "### T-003 · Cache shaders",
            "- status: dropped",
            "- note (2026-11-18, bob): added",
            "- note (2026-11-20, bob): dropped — start-up is already fast enough",
            "### T-004 · Trim the wick",
            "- status: open",
            "- note (2026-11-20, bob): added",
            "### T-005 · Clean the glass",
            "- status: done",
            "- milestone: T-001",
            "- note (2026-11-18, bob): added",
            "- note (2026-11-19, bob): done",
            "### T-006 · Paint the door",
            "- status: dropped",
            "- milestone: T-001",
            "- note (2026-11-18, bob): added",
            "- note (2026-11-19, bob): dropped — not needed",
        ]);
        let recent = recent_of(&fixture, None);
        assert_eq!(
            shown(&recent),
            [
                "Shader C finished — all 2 tasks done",
                "Parse source",
                "Cache shaders — cancelled: start-up is already fast enough",
                "Added: Parse source",
                "Added: Trim the wick",
                "Clean the glass",
                "Paint the door — cancelled: not needed",
                "Added: Shader C",
                "Added: Cache shaders",
                "2 tasks added to Shader C",
            ]
        );
        assert_eq!(
            dates(&recent),
            [
                day(20),
                day(20),
                day(20),
                day(20),
                day(20),
                day(19),
                day(19),
                day(18),
                day(18),
                day(18),
            ]
        );
        assert!(recent.earlier.is_empty());
    }

    #[test]
    fn entries_added_on_one_day_below_one_milestone_are_one_row_with_the_count() {
        let fixture = text(&[
            "### T-001 · Scene loading",
            "- status: open",
            "- type: milestone",
            "### T-002 · Scene A",
            "- status: open",
            "- milestone: T-001",
            "- note (2026-11-20, bob): added",
            "### T-003 · Scene B",
            "- status: open",
            "- milestone: T-001",
            "- note (2026-11-20, bob): added",
            "### T-004 · Scene C",
            "- status: open",
            "- milestone: T-001",
            "- note (2026-11-20, bob): added",
            "### T-005 · Shade",
            "- status: open",
            "- type: milestone",
            "- note (2026-11-20, bob): added",
            "### T-006 · Lamp",
            "- status: open",
            "- milestone: T-005",
            "- note (2026-11-20, bob): added",
            "### T-007 · Loose one",
            "- status: open",
            "- note (2026-11-20, bob): added",
            "### T-008 · Loose two",
            "- status: open",
            "- note (2026-11-20, bob): added",
        ]);
        assert_eq!(
            shown(&recent_of(&fixture, None)),
            [
                "3 tasks added to Scene loading",
                "Added: Shade",
                "Added: Lamp",
                "2 tasks added to Other work",
            ]
        );
    }

    #[test]
    fn the_window_is_the_day_of_the_build_and_six_days_before_it() {
        let fixture = text(&[
            "### T-001 · Lamp on the 15th",
            "- status: done",
            "- note (2026-11-15, bob): done — the oldest day shown",
            "### T-002 · Ladder on the 14th",
            "- status: dropped",
            "- note (2026-11-14, bob): dropped — too early",
            "### T-003 · Note for tomorrow",
            "- status: done",
            "- note (2026-11-22, bob): done — not yet",
            "### T-004 · Today",
            "- status: done",
            "- note (2026-11-21, bob): done — today",
        ]);
        let recent = recent_of(&fixture, None);
        assert_eq!(shown(&recent), ["Today", "Lamp on the 15th"]);
        assert_eq!(dates(&recent), [day(21), day(15)]);
        assert!(recent.earlier.is_empty());
    }

    #[test]
    fn archive_events_come_after_todo_events_of_the_same_day_and_kind() {
        let doc = text(&[
            "### T-001 · First",
            "- status: done",
            "- note (2026-11-20, bob): done — here",
        ]);
        let archive = text(&[
            "### T-010 · Archived shipped",
            "- status: done",
            "- note (2026-11-20, bob): done — there",
            "### T-011 · Archived part",
            "- status: done",
            "- note (2026-11-20, bob): added",
        ]);
        assert_eq!(
            shown(&recent_of(&doc, Some(&archive))),
            ["First", "Archived shipped", "Added: Archived part"]
        );
    }

    #[test]
    fn more_than_fifteen_rows_split_into_rows_and_earlier() {
        let mut lines = Vec::new();
        for i in 1..=17 {
            lines.push(format!("### T-{i:03} · Task {i}"));
            lines.push("- status: done".to_string());
            lines.push("- note (2026-11-20, bob): done".to_string());
        }
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let recent = recent_of(&text(&refs), None);
        let expected_rows: Vec<String> = (1..=15).map(|i| format!("Task {i}")).collect();
        assert_eq!(shown(&recent), expected_rows);
        let earlier: Vec<String> = recent
            .earlier
            .iter()
            .map(|(_, event)| row_text(event))
            .collect();
        assert_eq!(earlier, ["Task 16", "Task 17"]);
    }

    #[test]
    fn a_week_with_no_event_has_no_rows() {
        let fixture = text(&[
            "### T-001 · Old",
            "- status: done",
            "- note (2026-11-01, bob): done",
        ]);
        let recent = recent_of(&fixture, None);
        assert!(recent.rows.is_empty());
        assert!(recent.earlier.is_empty());
        assert_eq!(
            recent_of("# Nothing yet\n", None),
            Recent {
                rows: Vec::new(),
                earlier: Vec::new(),
            }
        );
        assert_eq!(EMPTY, "Nothing changed in the last 7 days.");
    }

    #[test]
    fn a_finished_milestone_counts_its_tasks_at_any_depth_and_leaves_cancelled_out() {
        let fixture = text(&[
            "### T-001 · Renderer",
            "- status: done",
            "- type: milestone",
            "- note (2026-11-20, bob): done — all in",
            "### T-002 · Inner shaders",
            "- status: open",
            "- type: milestone",
            "- milestone: T-001",
            "### T-003 · Compile",
            "- status: done",
            "- milestone: T-002",
            "- note (2026-11-19, bob): done",
            "### T-004 · Link",
            "- status: open",
            "- milestone: T-002",
            "### T-005 · Old plan",
            "- status: dropped",
            "- milestone: T-001",
            "- note (2026-11-18, bob): dropped with T-001 — no longer needed",
        ]);
        let recent = recent_of(&fixture, None);
        assert_eq!(
            shown(&recent),
            [
                "Renderer finished — 1 of 2 tasks done",
                "Compile",
                "Old plan — cancelled: no longer needed",
            ]
        );
        assert_eq!(dates(&recent), [day(20), day(19), day(18)]);
    }

    #[test]
    fn questions_and_ideas_give_no_events() {
        let fixture = text(&[
            "### T-001 · Vulkan only?",
            "- type: question",
            "- status: done",
            "- note (2026-11-20, bob): added",
            "- note (2026-11-20, bob): done — Vulkan only",
            "### T-002 · Maybe a lamp",
            "- type: idea",
            "- status: open",
            "- note (2026-11-20, bob): added",
        ]);
        assert!(recent_of(&fixture, None).rows.is_empty());
    }

    #[test]
    fn a_count_of_one_is_never_written_as_tasks() {
        for done in 0..=3 {
            for total in done..=3 {
                let text = row_text(&Event::MilestoneFinished {
                    title: "Shader C".to_string(),
                    done,
                    total,
                });
                assert!(!text.contains("1 tasks"), "{text}");
            }
        }
        let finished = |done, total| {
            row_text(&Event::MilestoneFinished {
                title: "Shader C".to_string(),
                done,
                total,
            })
        };
        assert_eq!(finished(1, 1), "Shader C finished — its 1 task done");
        assert_eq!(finished(0, 1), "Shader C finished — 0 of 1 task done");
        assert_eq!(finished(1, 2), "Shader C finished — 1 of 2 tasks done");
        assert_eq!(finished(2, 2), "Shader C finished — all 2 tasks done");
    }

    #[test]
    fn a_milestone_with_one_task_reads_its_one_task_in_the_rows() {
        let fixture = text(&[
            "### T-001 · Shader",
            "- status: done",
            "- type: milestone",
            "- note (2026-11-20, bob): done — all in",
            "### T-002 · Parse source",
            "- status: done",
            "- milestone: T-001",
            "- note (2026-11-20, bob): done",
            "### T-003 · Lamp",
            "- status: done",
            "- type: milestone",
            "- note (2026-11-20, bob): done",
            "### T-004 · Wick",
            "- status: open",
            "- milestone: T-003",
        ]);
        assert_eq!(
            shown(&recent_of(&fixture, None)),
            [
                "Shader finished — its 1 task done",
                "Lamp finished — 0 of 1 task done",
                "Parse source",
            ]
        );
    }

    #[test]
    fn row_text_gives_the_words_of_each_event() {
        let finished_all = Event::MilestoneFinished {
            title: "Shader C".to_string(),
            done: 5,
            total: 5,
        };
        let finished_part = Event::MilestoneFinished {
            title: "Shader C".to_string(),
            done: 3,
            total: 5,
        };
        assert_eq!(
            row_text(&finished_all),
            "Shader C finished — all 5 tasks done"
        );
        assert_eq!(
            row_text(&finished_part),
            "Shader C finished — 3 of 5 tasks done"
        );
        assert_eq!(
            row_text(&Event::Done {
                title: "Parse source".to_string(),
            }),
            "Parse source"
        );
        assert_eq!(
            row_text(&Event::Cancelled {
                title: "Cache shaders".to_string(),
                reason: "start-up is already fast enough".to_string(),
            }),
            "Cache shaders — cancelled: start-up is already fast enough"
        );
        assert_eq!(
            row_text(&Event::Added {
                count: 1,
                milestone: Some("Scene loading".to_string()),
                title: "Scene A".to_string(),
            }),
            "Added: Scene A"
        );
        assert_eq!(
            row_text(&Event::Added {
                count: 5,
                milestone: Some("Scene loading".to_string()),
                title: String::new(),
            }),
            "5 tasks added to Scene loading"
        );
        assert_eq!(
            row_text(&Event::Added {
                count: 2,
                milestone: None,
                title: String::new(),
            }),
            "2 tasks added to Other work"
        );
    }
}
