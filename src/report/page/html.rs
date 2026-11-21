//! The v1.0 page as HTML (T-214; R-REPORT-1, section 5.23): the header, the overall line, Needs you,
//! Last 7 days, All work and More, in the markup and order of `docs/report-mockup.html`.
//!
//! [`render`] writes the page. The entries and the tree come from [`super::tree_html`], and the
//! planner's part from [`crate::report::html`], which writes the planning state and the counts per
//! area as the v0.8 page did. The stylesheet is [`STYLE`] and the one small script is [`SCRIPT`]
//! (T-203); the view buttons are hidden until the script adds the class `js`. Every text from the
//! file is escaped.

use std::fmt::Write as _;

use chrono::NaiveDate;

use super::PageData;
use super::entry::State;
use super::needs::{self, Need};
use super::recent::{self, Event};
use super::tree::{Counts, Earlier};
use super::tree_html::{date_short, push_tree};
use crate::report::html::{SINGLE, escape, push_areas, push_planning};

/// The stylesheet, copied into the page (`style.css`: light and dark mode, colour-blind colours,
/// narrow windows, print).
pub const STYLE: &str = include_str!("style.css");

/// The one small script, copied into the page inline (`script.js`, R-REPORT-6): the mode and colour
/// buttons, the view filter, and Expand all and Collapse all. It names no address and loads nothing.
pub const SCRIPT: &str = include_str!("script.js");

/// The mark of a done event in Last 7 days (✓).
const MARK_DONE: char = '\u{2713}';

/// The mark of a cancelled event in Last 7 days (✕).
const MARK_CANCELLED: char = '\u{2715}';

/// The mark of an event that added entries (＋), as the mockup draws it.
const MARK_ADDED: char = '\u{FF0B}';

/// Writes the whole page (R-REPORT-1): one document with the stylesheet and the script inline.
#[must_use]
pub fn render(page: &PageData) -> String {
    let mut h = String::with_capacity(64 * 1024);
    let _ = write!(
        h,
        "<!doctype html>\n<html lang=\"en\">\n<head>\n\
<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
<title>{} \u{2014} status</title>\n\
<style>\n{STYLE}</style>\n\
</head>\n<body>\n<div class=\"page\">\n",
        escape(&page.title)
    );
    push_header(&mut h, page);
    push_overall(&mut h, &page.counts);
    push_needs(&mut h, page);
    push_recent(&mut h, page);
    push_all_work(&mut h, page);
    push_more(&mut h, page);
    h.push_str("</div>\n<script>\n");
    h.push_str(SCRIPT);
    h.push_str("</script>\n</body>\n</html>\n");
    h
}

/// The title, the description when it is set, the time of the status, and the two buttons that
/// change the mode and the colours (R-REPORT-6).
fn push_header(h: &mut String, page: &PageData) {
    h.push_str("<header class=\"head\"><div class=\"grow\">\n");
    let _ = writeln!(h, "<h1>{}</h1>", escape(&page.title));
    if let Some(description) = &page.description {
        let _ = writeln!(h, "<p class=\"desc\">{}</p>", escape(description));
    }
    let _ = writeln!(
        h,
        "<p class=\"asof muted\">Status as of {} \u{00B7} from TODO.md</p>",
        escape(&page.as_of)
    );
    h.push_str(
        "</div>\n<div class=\"controls\">\n\
<button id=\"theme\" type=\"button\">Dark mode</button>\n\
<button id=\"cb\" type=\"button\" aria-pressed=\"false\" title=\"Colours that stay apart with red-green colour blindness\">Colour-blind colours</button>\n\
</div>\n</header>\n",
    );
}

/// The overall line (R-REPORT-1): "N of M tasks done", one bar in five parts, and the legend.
fn push_overall(h: &mut String, counts: &Counts) {
    let total = counts.total_counted();
    let parts = [
        (State::Done, counts.done),
        (State::InProgress, counts.in_progress),
        (State::Waiting, counts.waiting),
        (State::ToDo, counts.to_do),
        (State::Cancelled, counts.cancelled),
    ];
    let all: usize = parts.iter().map(|(_, count)| count).sum();

    h.push_str("<div class=\"overall\">\n");
    let _ = writeln!(
        h,
        "<p class=\"big\">{} of {total} tasks done</p>",
        counts.done
    );

    let label: Vec<String> = parts
        .iter()
        .map(|(state, count)| format!("{count} {}", state.word()))
        .collect();
    let _ = writeln!(
        h,
        "<div class=\"seg\" role=\"img\" aria-label=\"{}\">",
        escape(&label.join(", "))
    );
    for (state, count) in parts {
        let _ = write!(
            h,
            "<i class=\"seg-{}\" style=\"width:{}%\"></i>",
            state.class(),
            share(count, all)
        );
    }
    h.push_str("</div>\n<ul class=\"legend\">\n");
    for (state, count) in parts {
        let _ = write!(
            h,
            "<li><span class=\"mark {}\">{}</span><b>{count}</b> {}",
            state.class(),
            state.mark(),
            state.word()
        );
        if state == State::Cancelled {
            let _ = write!(
                h,
                " <span class=\"muted\">(not counted in the {total})</span>"
            );
        }
        h.push_str("</li>\n");
    }
    h.push_str("</ul>\n</div>\n");
}

/// The width of a bar part in percent of all tasks, to one decimal; 0 when there are no tasks.
fn share(count: usize, all: usize) -> String {
    if all == 0 {
        "0".to_string()
    } else {
        format!("{:.1}", count as f64 * 100.0 / all as f64)
    }
}

/// Needs you (R-REPORT-4): the heading with its count, then one row per entry, or the empty words.
fn push_needs(h: &mut String, page: &PageData) {
    let _ = writeln!(h, "<h2>{}</h2>", escape(&needs::heading(&page.needs)));
    if page.needs.is_empty() {
        let _ = writeln!(h, "<p class=\"muted\">{}</p>", escape(needs::EMPTY));
        return;
    }
    h.push_str("<div class=\"box attn\">\n");
    for need in &page.needs {
        push_need(h, need, page.today);
    }
    h.push_str("</div>\n");
}

/// One row of Needs you: its tag, its title, and the words that say why it waits for a person.
fn push_need(h: &mut String, need: &Need, today: NaiveDate) {
    let (tag, title, text) = match need {
        Need::Question {
            title,
            asked,
            about,
            waiting,
        } => {
            let mut parts = Vec::new();
            if let Some(asked) = asked {
                parts.push(format!("Asked {}", date_short(*asked, today)));
            }
            if let Some(about) = about {
                parts.push(format!("about \u{201C}{about}\u{201D}"));
            }
            if let Some(waiting) = needs::waiting_text(waiting) {
                parts.push(waiting);
            }
            ("Question", title.as_str(), parts.join(" \u{00B7} "))
        }
        Need::GivenBack {
            title,
            date,
            problem,
        } => (
            "Given back",
            title.as_str(),
            format!(
                "A worker could not finish it ({}): {problem}",
                date_short(*date, today)
            ),
        ),
        Need::NoActivity { title, owner, days } => {
            let span = match days {
                Some(1) => " for 1 day".to_string(),
                Some(days) => format!(" for {days} days"),
                None => String::new(),
            };
            (
                "No activity",
                title.as_str(),
                format!("In progress{span} by {owner}. It may have been abandoned."),
            )
        }
    };
    let _ = write!(
        h,
        "<div class=\"need\"><span class=\"tag\">{tag}</span><span class=\"what\">{}</span>",
        escape(title)
    );
    if !text.is_empty() {
        let _ = write!(h, "\n<p class=\"muted\">{}</p>", escape(&text));
    }
    h.push_str("</div>\n");
}

/// Last 7 days (R-REPORT-5): the rows, newest day first, and the closed part with the rest.
fn push_recent(h: &mut String, page: &PageData) {
    h.push_str("<h2>Last 7 days</h2>\n");
    if page.recent.rows.is_empty() {
        let _ = writeln!(h, "<p class=\"muted\">{}</p>", escape(recent::EMPTY));
        return;
    }
    push_recent_rows(h, &page.recent.rows, page.today);
    if !page.recent.earlier.is_empty() {
        let _ = writeln!(
            h,
            "<details class=\"fold\"><summary>Earlier this week ({})</summary>",
            page.recent.earlier.len()
        );
        push_recent_rows(h, &page.recent.earlier, page.today);
        h.push_str("</details>\n");
    }
}

/// Rows of Last 7 days as one `ul.recent`: the date, the mark and the words of each event.
fn push_recent_rows(h: &mut String, rows: &[(NaiveDate, Event)], today: NaiveDate) {
    if rows.is_empty() {
        return;
    }
    h.push_str("<ul class=\"recent\">\n");
    for (date, event) in rows {
        let (class, mark, text) = event_html(event);
        let _ = writeln!(
            h,
            "<li><span class=\"d\">{}</span><span class=\"mark {class}\">{mark}</span><span>{text}</span></li>",
            date_short(*date, today)
        );
    }
    h.push_str("</ul>\n");
}

/// The state class, the mark and the escaped words of one event; the words are the ones
/// [`recent::row_text`] gives, with the milestone's title in bold and the cancelled reason muted.
fn event_html(event: &Event) -> (&'static str, char, String) {
    let words = recent::row_text(event);
    match event {
        Event::MilestoneFinished { title, .. } => (
            "done",
            MARK_DONE,
            format!(
                "<b>{}</b> {}",
                escape(title),
                escape(rest_of(&words, title))
            ),
        ),
        Event::Done { title } => ("done", MARK_DONE, escape(title)),
        Event::Cancelled { title, .. } => (
            "drop",
            MARK_CANCELLED,
            format!(
                "{} <span class=\"muted\">{}</span>",
                escape(title),
                escape(rest_of(&words, title))
            ),
        ),
        Event::Added { count: 1, .. } => ("todo", MARK_ADDED, escape(&words)),
        Event::Added {
            count, milestone, ..
        } => (
            "todo",
            MARK_ADDED,
            format!(
                "{count} tasks added to <b>{}</b>",
                escape(milestone.as_deref().unwrap_or("Other work"))
            ),
        ),
    }
}

/// The words of a row after its title, such as "finished — all 3 tasks done".
fn rest_of<'a>(words: &'a str, title: &str) -> &'a str {
    words.strip_prefix(title).unwrap_or_default().trim_start()
}

/// All work (R-REPORT-1, R-REPORT-6): the view buttons and the expand and collapse buttons, then the
/// tree. The buttons are hidden until the page has the class `js`.
fn push_all_work(h: &mut String, page: &PageData) {
    h.push_str(
        "<h2>All work</h2>\n<div class=\"controls\">\n\
<span class=\"muted\">Show</span>\n\
<button data-f=\"all\" aria-pressed=\"true\">Everything</button>\n\
<button data-f=\"left\" aria-pressed=\"false\">Still to do</button>\n\
<button data-f=\"done\" aria-pressed=\"false\">Done</button>\n\
<button data-f=\"drop\" aria-pressed=\"false\">Cancelled</button>\n\
<span class=\"sp\"></span>\n\
<button id=\"exp\" type=\"button\">Expand all</button>\n\
<button id=\"col\" type=\"button\">Collapse all</button>\n\
</div>\n",
    );
    if page.tree.milestones.is_empty() && page.tree.other.is_empty() {
        h.push_str("<p class=\"muted\">No tasks or milestones in TODO.md yet.</p>\n");
    }
    push_tree(h, &page.tree, page.today);
}

/// More (R-REPORT-1): the closed parts Ideas, Earlier work and For the planner.
fn push_more(h: &mut String, page: &PageData) {
    h.push_str("<h2>More</h2>\n");
    if !page.ideas.is_empty() {
        let _ = writeln!(
            h,
            "<details class=\"fold\"><summary>Ideas ({}) <span class=\"muted\">\u{2014} not planned yet</span></summary>",
            page.ideas.len()
        );
        h.push_str("<ul>\n");
        for idea in &page.ideas {
            let _ = writeln!(h, "<li>{}</li>", escape(idea));
        }
        h.push_str("</ul>\n</details>\n");
    }
    if let Some(earlier) = &page.earlier {
        push_earlier(h, earlier, page.today);
    }
    push_planner(h, page);
}

/// Earlier work (R-REPORT-1, More): the archive's tree, with every milestone closed. The archive's
/// tree has the same markup as All work, so its `id` is dropped to keep the page's IDs unique.
fn push_earlier(h: &mut String, earlier: &Earlier, today: NaiveDate) {
    let mut tree = String::new();
    push_tree(&mut tree, &earlier.tree, today);
    // Escaped text never holds these tags, so the replacements touch the markup only.
    let tree = tree
        .replace("<details class=\"ms\" open>", "<details class=\"ms\">")
        .replacen("<div id=\"tree\">", "<div class=\"tree\">", 1);
    let _ = writeln!(
        h,
        "<details class=\"fold\"><summary>Earlier work ({}, {}) <span class=\"muted\">\u{2014} from the archive</span></summary>",
        counted(
            earlier.finished_milestones,
            "finished milestone",
            "finished milestones"
        ),
        counted(earlier.tasks, "task", "tasks")
    );
    h.push_str(&tree);
    h.push_str("</details>\n");
}

/// For the planner (R-REPORT-3, More): the planning state and the unfinished tasks per area, from the
/// v0.8 page's parts. Closed by default.
fn push_planner(h: &mut String, page: &PageData) {
    h.push_str(
        "<details class=\"fold\"><summary>For the planner <span class=\"muted\">\u{2014} levels, parallel waves, tasks that still need detail</span></summary>\n",
    );
    push_planning(h, &page.planner, SINGLE);
    push_areas(h, &page.planner, SINGLE);
    h.push_str("</details>\n");
}

/// `count` with the singular or the plural word, such as `1 task` or `3 tasks`.
fn counted(count: usize, singular: &str, plural: &str) -> String {
    let word = if count == 1 { singular } else { plural };
    format!("{count} {word}")
}
