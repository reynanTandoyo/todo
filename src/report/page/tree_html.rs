//! The part *All work* of the v1.0 page as HTML (T-213; R-REPORT-1, "All work", "An entry" and
//! "Details"; section 5.23).
//!
//! [`push_tree`] writes the tree of [`super::tree::Tree`] in the markup of `docs/report-mockup.html`:
//! a milestone is a `details.ms` row with its title, its state, "N of M done" and a bar; an entry is
//! one `li.item` with its mark, title and ID, whose details open on click. The stylesheet (T-214)
//! gives the colours and the layout, so the markup carries classes and bar widths only. Every text
//! from the file is escaped with [`escape`]; the texts a person reads as prose (titles, goals,
//! summaries, reasons and notes) go through [`inline_code`], which also shows text between backticks
//! as code (T-233).

use std::fmt::Write as _;

use chrono::{Datelike, NaiveDate};

use super::entry::{
    Dependency, Details, EntryView, NoteLine, SecondLine, Since, State, WaitReason,
};
use super::text::inline_code;
use super::tree::{MilestoneState, MilestoneView, Tree};
use crate::report::html::escape;

/// The date in the page's short form: "21 Nov", with the year after it when `date` is not in the
/// year of `today` ("21 Nov 2025").
#[must_use]
pub fn date_short(date: NaiveDate, today: NaiveDate) -> String {
    let day_and_month = date.format("%-d %b").to_string();
    if date.year() == today.year() {
        day_and_month
    } else {
        format!("{day_and_month} {}", date.year())
    }
}

/// Writes one entry as one `li.item` (R-REPORT-1, "An entry"): its mark, its title, the date when it
/// is done or cancelled, and its ID; then, on click, its details; then its second line, for the
/// states that have one.
pub fn push_entry(h: &mut String, entry: &EntryView, today: NaiveDate) {
    let _ = write!(
        h,
        "<li class=\"item {}\"><details><summary><span class=\"mark\">{}</span><span class=\"t\">{}</span>",
        entry.state.class(),
        entry.state.mark(),
        inline_code(&entry.title),
    );
    if let Some(date) = entry.date {
        let _ = write!(h, "<span class=\"when\">{}</span>", date_short(date, today));
    }
    let _ = write!(
        h,
        "<span class=\"id\">{}</span></summary>",
        escape(&entry.id)
    );
    push_details(h, entry, today);
    h.push_str("</details>");
    if let Some(second) = &entry.second {
        let _ = write!(h, "\n<p class=\"line2\">{}</p>", second_line_html(second));
    }
    h.push_str("</li>\n");
}

/// Writes one milestone as one `details.ms` (R-REPORT-1, "All work"): open unless it is finished or
/// cancelled; its title, state, "N of M done" and bar; then its goal and summary, the milestones
/// below it, and its tasks.
pub fn push_milestone(h: &mut String, milestone: &MilestoneView, today: NaiveDate) {
    let open = !matches!(
        milestone.state,
        MilestoneState::Finished(_) | MilestoneState::Cancelled(_)
    );
    h.push_str(if open {
        "<details class=\"ms\" open>\n"
    } else {
        "<details class=\"ms\">\n"
    });
    let (state_class, state_text) = milestone_state(milestone.state, today);
    push_summary(
        h,
        Some(&milestone.id),
        &milestone.title,
        state_class,
        &state_text,
        milestone.done,
        milestone.total,
    );
    h.push_str("<div class=\"ms-body\">\n");
    if let Some(goal) = &milestone.goal {
        let _ = writeln!(h, "<p class=\"goal\">Goal: {}</p>", inline_code(goal));
    }
    if let Some(summary) = &milestone.summary {
        h.push_str("<div class=\"more\">");
        push_labelled(h, "Summary", summary);
        h.push_str("</div>\n");
    }
    for nested in &milestone.milestones {
        push_milestone(h, nested, today);
    }
    push_items(h, &milestone.tasks, today);
    h.push_str("</div>\n</details>\n");
}

/// Writes the whole tree (R-REPORT-1, "All work"): every milestone in its order, then the group
/// *Other work* when some task is below no milestone.
pub fn push_tree(h: &mut String, tree: &Tree, today: NaiveDate) {
    h.push_str("<div id=\"tree\">\n");
    for milestone in &tree.milestones {
        push_milestone(h, milestone, today);
    }
    if !tree.other.is_empty() {
        push_other_work(h, &tree.other, today);
    }
    h.push_str("</div>\n");
}

/// The group *Other work*: a milestone row with no goal, always open, that counts its tasks the way
/// a milestone does.
fn push_other_work(h: &mut String, tasks: &[EntryView], today: NaiveDate) {
    let done = tasks
        .iter()
        .filter(|task| task.state == State::Done)
        .count();
    let total = tasks
        .iter()
        .filter(|task| task.state != State::Cancelled)
        .count();
    h.push_str("<details class=\"ms\" open>\n");
    push_summary(
        h,
        None,
        "Other work",
        "st-none",
        "not part of a milestone",
        done,
        total,
    );
    h.push_str("<div class=\"ms-body\">\n");
    push_items(h, tasks, today);
    h.push_str("</div>\n</details>\n");
}

/// The summary row of a milestone or of *Other work*: the marker and the ID when there is one (a
/// milestone; *Other work* has none), then title, state, "N of M done" and the bar.
fn push_summary(
    h: &mut String,
    id: Option<&str>,
    title: &str,
    state_class: &str,
    state_text: &str,
    done: usize,
    total: usize,
) {
    h.push_str("<summary>");
    if let Some(id) = id {
        let _ = write!(
            h,
            "<span class=\"ms-mark\" role=\"img\" aria-label=\"Milestone\">M</span><span class=\"ms-id\">{}</span>",
            escape(id),
        );
    }
    let _ = writeln!(
        h,
        "<span class=\"ms-title\">{}</span><span class=\"ms-state {state_class}\">{}</span><span class=\"ms-count\">{done} of {total} done</span><span class=\"bar\"><i style=\"width:{}%\"></i></span></summary>",
        inline_code(title),
        escape(state_text),
        percent(done, total),
    );
}

/// The state of a milestone in words and the class of its colour (R-REPORT-1, "All work").
fn milestone_state(state: MilestoneState, today: NaiveDate) -> (&'static str, String) {
    match state {
        MilestoneState::NotStarted => ("st-none", "not started".to_string()),
        MilestoneState::InProgress => ("st-prog", "in progress".to_string()),
        MilestoneState::Finished(date) => ("st-done", word_with_date("finished", date, today)),
        MilestoneState::Cancelled(date) => ("st-drop", word_with_date("cancelled", date, today)),
    }
}

/// `word` and the short date after it ("finished 19 Nov"), or `word` alone without a date.
fn word_with_date(word: &str, date: Option<NaiveDate>, today: NaiveDate) -> String {
    match date {
        Some(date) => format!("{word} {}", date_short(date, today)),
        None => word.to_string(),
    }
}

/// The tasks of a group as one `ul.items`; nothing when there are none.
fn push_items(h: &mut String, entries: &[EntryView], today: NaiveDate) {
    if entries.is_empty() {
        return;
    }
    h.push_str("<ul class=\"items\">\n");
    for entry in entries {
        push_entry(h, entry, today);
    }
    h.push_str("</ul>\n");
}

/// The width of a bar in whole percent, rounded to the nearest; 0 when there are no tasks.
fn percent(done: usize, total: usize) -> usize {
    if total == 0 {
        0
    } else {
        (done * 200 + total) / (total * 2)
    }
}

/// The part of an entry that opens on click (R-REPORT-1, "Details"): "Summary", or "Why" when there
/// is no summary; "Result" for a done entry; the closed "Technical details"; and, last, the closed
/// "Show chain" of R-REPORT-7 when the task has a chain. Nothing is written when none of them is
/// present.
fn push_details(h: &mut String, entry: &EntryView, today: NaiveDate) {
    let mut body = String::new();
    if let Some(summary) = &entry.summary {
        push_labelled(&mut body, "Summary", summary);
    } else if let Some(why) = &entry.why {
        push_labelled(&mut body, "Why", why);
    }
    if let (State::Done, Some(result)) = (entry.state, &entry.result) {
        push_labelled(&mut body, "Result", result);
    }
    if let Some(technical) = technical_details(&entry.details, today) {
        body.push_str(&technical);
    }
    if let Some(chain) = &entry.chain {
        super::chain_html::push_chain(&mut body, chain);
    }
    if !body.is_empty() {
        let _ = write!(h, "<div class=\"more\">{body}</div>");
    }
}

/// A paragraph with a small label, for "Summary", "Why", "Result" and the like.
fn push_labelled(out: &mut String, label: &str, text: &str) {
    let _ = write!(
        out,
        "<p><span class=\"lbl\">{label}</span><br>{}</p>",
        inline_code(text)
    );
}

/// The closed part "Technical details" (R-REPORT-1, "Details"): the fields that are present, and the
/// notes. `None` when none of them is present.
fn technical_details(details: &Details, today: NaiveDate) -> Option<String> {
    let mut rows: Vec<(&'static str, String)> = Vec::new();
    push_row(&mut rows, "Level", details.level.as_deref());
    push_row(&mut rows, "Priority", details.priority.as_deref());
    push_row(&mut rows, "Type", details.task_type.as_deref());
    push_row(&mut rows, "Area", details.area.as_deref());
    push_row(&mut rows, "Files", details.files.as_deref());
    if let Some(verify) = &details.verify {
        let check = check_html(verify, details.verified.as_deref(), today);
        rows.push(("Check", check));
    }
    push_row(&mut rows, "Commit", details.commit.as_deref());
    push_row(&mut rows, "Done by", details.done_by.as_deref());
    if !details.depends.is_empty() {
        let depends: Vec<String> = details.depends.iter().map(dependency_html).collect();
        rows.push(("Depends", depends.join("<br>")));
    }
    push_row(&mut rows, "Attempts", details.attempts.as_deref());
    if !details.notes.is_empty() {
        rows.push(("Notes", notes_html(&details.notes, today)));
    }
    if rows.is_empty() {
        return None;
    }
    let mut html = String::from("<details class=\"tech\"><summary>Technical details</summary><dl>");
    for (label, value) in rows {
        let _ = write!(html, "<dt>{label}</dt><dd>{value}</dd>");
    }
    html.push_str("</dl></details>");
    Some(html)
}

/// Adds one row of the technical details when the field is present.
fn push_row(rows: &mut Vec<(&'static str, String)>, label: &'static str, value: Option<&str>) {
    if let Some(value) = value {
        rows.push((label, escape(value)));
    }
}

/// The check and, when it has passed, its date: "cargo test — passed 21 Nov". A date that is not
/// written as `yyyy-mm-dd` is shown as written.
fn check_html(verify: &str, verified: Option<&str>, today: NaiveDate) -> String {
    let mut html = escape(verify);
    if let Some(verified) = verified {
        let passed = NaiveDate::parse_from_str(verified, "%Y-%m-%d")
            .map_or_else(|_| verified.to_string(), |date| date_short(date, today));
        let _ = write!(html, " — passed {}", escape(&passed));
    }
    html
}

/// A dependency as "T-002 Shipped part", or its ID alone when no task list holds it.
fn dependency_html(dependency: &Dependency) -> String {
    match &dependency.title {
        Some(title) => format!("{} {}", escape(&dependency.id), inline_code(title)),
        None => escape(&dependency.id),
    }
}

/// The notes as a list: each with its date, its agent and its text.
fn notes_html(notes: &[NoteLine], today: NaiveDate) -> String {
    let mut html = String::from("<ul>");
    for note in notes {
        let mut parts = vec![format!(
            "<span class=\"when\">{}</span>",
            date_short(note.date, today)
        )];
        let agent = note.agent.trim();
        if !agent.is_empty() {
            parts.push(escape(agent));
        }
        let text = note.text.trim();
        if !text.is_empty() {
            parts.push(inline_code(text));
        }
        let _ = write!(html, "<li>{}</li>", parts.join(" · "));
    }
    html.push_str("</ul>");
    html
}

/// The second line of an entry, with the state in words first (R-REPORT-1, "An entry").
fn second_line_html(second: &SecondLine) -> String {
    match second {
        SecondLine::InProgress {
            owner,
            since,
            stale,
        } => {
            let mut parts = vec!["<b>In progress</b>".to_string(), escape(owner)];
            if let Some(since) = since_text(*since) {
                parts.push(since);
            }
            if *stale {
                parts.push("<span class=\"stale\">no activity</span>".to_string());
            }
            parts.join(" · ")
        }
        SecondLine::Waiting(reason) => format!("<b>Waiting</b> · {}", waiting_html(reason)),
        SecondLine::Cancelled(reason) => format!("<b>Cancelled</b> · {}", inline_code(reason)),
    }
}

/// How long a claim has been held, in words: "since today", "since yesterday" or "for N days".
fn since_text(since: Option<Since>) -> Option<String> {
    since.map(|since| match since {
        Since::Today => "since today".to_string(),
        Since::Yesterday => "since yesterday".to_string(),
        Since::Days(days) => format!("for {days} days"),
    })
}

/// Why an entry waits, in words: the problem a worker gave back, the question it waits for, or its
/// block reason.
fn waiting_html(reason: &WaitReason) -> String {
    match reason {
        WaitReason::GivenBack(problem) => {
            format!("given back by a worker: {}", inline_code(problem))
        }
        WaitReason::Question(title) => format!("for your answer: {}", inline_code(title)),
        WaitReason::Blocked(reason) => inline_code(reason),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, FixedOffset};

    use super::*;
    use crate::doc::Document;
    use crate::ops::OpsConfig;
    use crate::report::page::entry::{Ctx, entry_view};
    use crate::report::page::tree::build_tree;

    /// The clock of every test: 21 Nov 2026, 15:00, in the +01:00 offset.
    const NOW: &str = "2026-11-21T15:00:00+01:00";

    fn today() -> NaiveDate {
        day(21)
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

    /// Runs `check` on the invented `fixture`, with a context read at [`NOW`] and no archive.
    fn with_doc<R>(fixture: &str, check: impl FnOnce(&Document, &Ctx<'_>) -> R) -> R {
        let doc = Document::parse_bytes(fixture.as_bytes()).unwrap();
        let cfg = OpsConfig::default();
        let now = DateTime::<FixedOffset>::parse_from_rfc3339(NOW).unwrap();
        let ctx = Ctx {
            doc: &doc,
            archive: None,
            cfg: &cfg,
            now,
        };
        check(&doc, &ctx)
    }

    /// The HTML of every task and milestone of `fixture` in file order, one entry at a time.
    fn entries_html(fixture: &str) -> String {
        with_doc(fixture, |doc, ctx| {
            let mut html = String::new();
            for task in &doc.tasks {
                push_entry(&mut html, &entry_view(task, ctx), today());
            }
            html
        })
    }

    /// The HTML of the whole tree of `fixture`.
    fn tree_html(fixture: &str) -> String {
        with_doc(fixture, |doc, ctx| {
            let mut html = String::new();
            push_tree(&mut html, &build_tree(doc, ctx), today());
            html
        })
    }

    /// The `li.item` of the entry with `id`, as written. An entry ends with a newline after its
    /// `</li>`; the notes inside it end their own `<li>` without one.
    fn item_of(html: &str, id: &str) -> String {
        let needle = format!("<span class=\"id\">{id}</span>");
        let at = html.find(&needle).unwrap();
        let start = html[..at].rfind("<li class=\"item").unwrap();
        let end = html[at..].find("</li>\n").unwrap() + at + "</li>\n".len();
        html[start..end].to_string()
    }

    /// The opening tag of the milestone row titled `title`, as written.
    fn opening_tag_of(html: &str, title: &str) -> String {
        let needle = format!("class=\"ms-title\">{title}</span>");
        let at = html.find(&needle).unwrap();
        let start = html[..at].rfind("<details").unwrap();
        let end = html[start..].find('>').unwrap() + start + 1;
        html[start..end].to_string()
    }

    /// The `summary` row of the milestone or group titled `title`, as written.
    fn summary_of(html: &str, title: &str) -> String {
        let needle = format!("<span class=\"ms-title\">{title}</span>");
        let at = html.find(&needle).unwrap();
        let start = html[..at].rfind("<summary>").unwrap();
        let end = html[at..].find("</summary>").unwrap() + at + "</summary>".len();
        html[start..end].to_string()
    }

    #[test]
    fn dates_leave_out_the_year_of_today_and_have_no_zero_before_the_day() {
        assert_eq!(date_short(day(21), today()), "21 Nov");
        assert_eq!(date_short(day(5), today()), "5 Nov");
        let march = NaiveDate::from_ymd_opt(2026, 3, 5).unwrap();
        assert_eq!(date_short(march, today()), "5 Mar");
        let last_year = NaiveDate::from_ymd_opt(2025, 11, 21).unwrap();
        assert_eq!(date_short(last_year, today()), "21 Nov 2025");
    }

    #[test]
    fn a_bar_is_as_wide_as_the_share_done_rounded_and_empty_without_tasks() {
        assert_eq!(percent(8, 13), 62);
        assert_eq!(percent(2, 5), 40);
        assert_eq!(percent(2, 3), 67);
        assert_eq!(percent(1, 3), 33);
        assert_eq!(percent(5, 5), 100);
        assert_eq!(percent(0, 0), 0);
    }

    #[test]
    fn each_state_has_its_mark_class_and_second_line() {
        let html = entries_html(&text(&[
            "### T-001 · Open task",
            "- status: open",
            "### T-002 · Claimed task",
            "- status: claimed",
            "- owner: worker-3",
            "- claimed: 2026-11-21T09:00+01:00",
            "### T-003 · Blocked task",
            "- status: blocked",
            "- note (2026-11-20, bob): blocked — waiting for the review",
            "### T-004 · Done task",
            "- status: done",
            "- note (2026-11-20, bob): done — shipped",
            "### T-005 · Dropped task",
            "- status: dropped",
            "- note (2026-11-19, bob): dropped — not needed",
        ]));

        let open = item_of(&html, "T-001");
        assert!(open.starts_with("<li class=\"item todo\">"));
        assert!(open.contains("<span class=\"mark\">○</span>"));
        assert!(!open.contains("line2"));

        let claimed = item_of(&html, "T-002");
        assert!(claimed.starts_with("<li class=\"item prog\">"));
        assert!(claimed.contains("<span class=\"mark\">●</span>"));
        assert!(
            claimed.contains("<p class=\"line2\"><b>In progress</b> · worker-3 · since today</p>")
        );

        let blocked = item_of(&html, "T-003");
        assert!(blocked.starts_with("<li class=\"item wait\">"));
        assert!(blocked.contains("<span class=\"mark\">◐</span>"));
        assert!(blocked.contains("<p class=\"line2\"><b>Waiting</b> · waiting for the review</p>"));

        let done = item_of(&html, "T-004");
        assert!(done.starts_with("<li class=\"item done\">"));
        assert!(done.contains("<span class=\"mark\">✓</span>"));
        assert!(done.contains("<span class=\"when\">20 Nov</span>"));
        assert!(!done.contains("line2"));

        let dropped = item_of(&html, "T-005");
        assert!(dropped.starts_with("<li class=\"item drop\">"));
        assert!(dropped.contains("<span class=\"mark\">✕</span>"));
        assert!(dropped.contains("<span class=\"when\">19 Nov</span>"));
        assert!(dropped.contains("<p class=\"line2\"><b>Cancelled</b> · not needed</p>"));
    }

    #[test]
    fn a_done_entry_shows_its_summary_then_its_result_and_no_second_line() {
        let html = entries_html(&text(&[
            "### T-001 · Shipped part",
            "- status: done",
            "- summary: The part that shipped.",
            "- note (2026-11-19, bob): added",
            "- note (2026-11-20, bob): done — first try",
        ]));
        let item = item_of(&html, "T-001");
        assert!(
            item.contains("<p><span class=\"lbl\">Summary</span><br>The part that shipped.</p>")
        );
        assert!(item.contains("<p><span class=\"lbl\">Result</span><br>first try</p>"));
        assert!(!item.contains("Why"));
        assert!(!item.contains("line2"));
    }

    #[test]
    fn an_entry_without_a_summary_shows_its_why_and_a_summary_wins_over_it() {
        let why_only = entries_html(&text(&[
            "### T-001 · Build",
            "- status: open",
            "- why: the page needs it",
        ]));
        let item = item_of(&why_only, "T-001");
        assert!(item.contains("<span class=\"lbl\">Why</span><br>the page needs it"));
        assert!(!item.contains("Summary"));

        let both = entries_html(&text(&[
            "### T-001 · Build",
            "- status: open",
            "- why: the page needs it",
            "- summary: The page shows the work.",
        ]));
        let item = item_of(&both, "T-001");
        assert!(item.contains("<span class=\"lbl\">Summary</span><br>The page shows the work."));
        assert!(!item.contains("Why"));
    }

    #[test]
    fn technical_details_leave_out_the_missing_fields_and_name_the_dependency() {
        let html = entries_html(&text(&[
            "### T-001 · Parse the source",
            "- status: done",
            "- area: report",
            "- difficulty: 2",
            "- verify: cargo test --lib report::page",
            "- verified: 2026-11-21",
            "- owner: bob",
            "- depends: T-002",
            "### T-002 · Shipped part",
            "- status: done",
            "- note (2026-11-20, bob): added",
            "- note (2026-11-21, bob): done — parsed",
        ]));

        let first = item_of(&html, "T-001");
        assert!(first.contains("<details class=\"tech\"><summary>Technical details</summary><dl>"));
        assert!(first.contains("<dt>Level</dt><dd>2</dd>"));
        assert!(first.contains("<dt>Area</dt><dd>report</dd>"));
        assert!(
            first.contains("<dt>Check</dt><dd>cargo test --lib report::page — passed 21 Nov</dd>")
        );
        assert!(first.contains("<dt>Depends</dt><dd>T-002 Shipped part</dd>"));
        assert!(first.contains("<dt>Done by</dt><dd>bob</dd>"));
        for absent in ["Priority", "Type", "Files", "Commit", "Attempts", "Notes"] {
            assert!(!first.contains(&format!("<dt>{absent}</dt>")), "{absent}");
        }

        let second = item_of(&html, "T-002");
        assert!(second.contains(
            "<dt>Notes</dt><dd><ul><li><span class=\"when\">20 Nov</span> · bob · added</li>\
             <li><span class=\"when\">21 Nov</span> · bob · done — parsed</li></ul></dd>"
        ));
        assert!(!second.contains("<dt>Check</dt>"));
    }

    #[test]
    fn an_entry_with_no_details_has_no_details_part() {
        let html = entries_html(&text(&["### T-001 · Bare task", "- status: open"]));
        let item = item_of(&html, "T-001");
        assert!(!item.contains("class=\"more\""));
        assert!(!item.contains("tech"));
    }

    #[test]
    fn a_waiting_entry_names_its_question_or_its_problem_in_words() {
        let question = entries_html(&text(&[
            "### T-001 · Link the stages",
            "- status: open",
            "- depends: T-002",
            "### T-002 · Vulkan only, or also DirectX 12?",
            "- type: question",
            "- status: open",
        ]));
        assert!(item_of(&question, "T-001").contains(
            "<p class=\"line2\"><b>Waiting</b> · for your answer: Vulkan only, or also DirectX 12?</p>"
        ));

        let given_back = entries_html(&text(&[
            "### T-001 · Draw a textured triangle",
            "- status: blocked",
            "- note (2026-11-20, worker-2): stuck — the schema file is missing",
        ]));
        assert!(item_of(&given_back, "T-001").contains(
            "<p class=\"line2\"><b>Waiting</b> · given back by a worker: the schema file is missing</p>"
        ));
    }

    #[test]
    fn an_in_progress_entry_says_since_when_and_flags_a_stale_claim() {
        let html = entries_html(&text(&[
            "### T-001 · Update the instructions",
            "- status: claimed",
            "- owner: worker-1",
            "- claimed: 2026-11-18T10:00+01:00",
            "### T-002 · Emit the vertex stage",
            "- status: claimed",
            "- owner: worker-3",
            "- claimed: 2026-11-20T18:00+01:00",
        ]));
        assert!(item_of(&html, "T-001").contains(
            "<p class=\"line2\"><b>In progress</b> · worker-1 · for 3 days · <span class=\"stale\">no activity</span></p>"
        ));
        assert!(
            item_of(&html, "T-002")
                .contains("<p class=\"line2\"><b>In progress</b> · worker-3 · since yesterday</p>")
        );
    }

    #[test]
    fn a_finished_milestone_is_closed_and_an_unfinished_one_is_open() {
        let html = tree_html(&text(&[
            "### T-001 · Finished milestone",
            "- status: done",
            "- type: milestone",
            "- note (2026-11-19, bob): done — all in",
            "### T-002 · Open milestone",
            "- status: open",
            "- type: milestone",
            "- goal: the goal of the open one",
            "### T-003 · Work under it",
            "- status: claimed",
            "- owner: keeper-1",
            "- claimed: 2026-11-21T09:00+01:00",
            "- milestone: T-002",
            "### T-004 · Empty milestone",
            "- status: open",
            "- type: milestone",
            "### T-005 · Dropped milestone",
            "- status: dropped",
            "- type: milestone",
            "- note (2026-11-20, bob): dropped — not needed",
        ]));
        assert_eq!(
            opening_tag_of(&html, "Finished milestone"),
            "<details class=\"ms\">"
        );
        assert_eq!(
            opening_tag_of(&html, "Open milestone"),
            "<details class=\"ms\" open>"
        );
        assert_eq!(
            opening_tag_of(&html, "Empty milestone"),
            "<details class=\"ms\" open>"
        );
        assert_eq!(
            opening_tag_of(&html, "Dropped milestone"),
            "<details class=\"ms\">"
        );
        assert!(html.contains("<span class=\"ms-state st-done\">finished 19 Nov</span>"));
        assert!(html.contains("<span class=\"ms-state st-prog\">in progress</span>"));
        assert!(html.contains("<span class=\"ms-state st-none\">not started</span>"));
        assert!(html.contains("<span class=\"ms-state st-drop\">cancelled 20 Nov</span>"));
        assert!(html.contains("<p class=\"goal\">Goal: the goal of the open one</p>"));
    }

    #[test]
    fn a_milestone_shows_its_count_its_bar_and_its_summary() {
        let html = tree_html(&text(&[
            "### T-001 · Counted milestone",
            "- status: open",
            "- type: milestone",
            "- summary: The count is over every task below.",
            "### T-002 · First",
            "- status: done",
            "- milestone: T-001",
            "### T-003 · Second",
            "- status: done",
            "- milestone: T-001",
            "### T-004 · Third",
            "- status: open",
            "- milestone: T-001",
            "### T-005 · Fourth, dropped",
            "- status: dropped",
            "- milestone: T-001",
            "### T-006 · Empty milestone",
            "- status: open",
            "- type: milestone",
        ]));
        assert!(html.contains(
            "<span class=\"ms-count\">2 of 3 done</span><span class=\"bar\"><i style=\"width:67%\"></i></span>"
        ));
        assert!(html.contains(
            "<span class=\"ms-count\">0 of 0 done</span><span class=\"bar\"><i style=\"width:0%\"></i></span>"
        ));
        assert!(html.contains(
            "<div class=\"more\"><p><span class=\"lbl\">Summary</span><br>The count is over every task below.</p></div>"
        ));
    }

    #[test]
    fn other_work_comes_last_and_appears_only_when_there_are_top_level_tasks() {
        let with_loose = tree_html(&text(&[
            "### T-001 · A milestone",
            "- status: open",
            "- type: milestone",
            "### T-002 · A loose task",
            "- status: open",
        ]));
        let milestone_at = with_loose.find("A milestone").unwrap();
        let other_at = with_loose.find("Other work").unwrap();
        assert!(other_at > milestone_at);
        assert_eq!(
            opening_tag_of(&with_loose, "Other work"),
            "<details class=\"ms\" open>"
        );
        assert!(with_loose.contains(
            "<span class=\"ms-state st-none\">not part of a milestone</span><span class=\"ms-count\">0 of 1 done</span><span class=\"bar\"><i style=\"width:0%\"></i></span>"
        ));
        assert!(item_of(&with_loose, "T-002").starts_with("<li class=\"item todo\">"));

        let milestones_only = tree_html(&text(&[
            "### T-001 · A milestone",
            "- status: open",
            "- type: milestone",
        ]));
        assert!(!milestones_only.contains("Other work"));
    }

    #[test]
    fn every_text_from_the_file_is_escaped() {
        let fixture = text(&[
            "### T-001 · <script>alert(1)</script>",
            "- status: open",
            "- why: <b>bold</b> & \"quoted\"",
            "### T-002 · Milestone <script>",
            "- status: open",
            "- type: milestone",
            "- goal: <i>goal</i>",
        ]);
        let entries = entries_html(&fixture);
        assert!(!entries.contains("<script>"));
        assert!(entries.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(entries.contains("&lt;b&gt;bold&lt;/b&gt; &amp; &quot;quoted&quot;"));

        let tree = tree_html(&fixture);
        assert!(!tree.contains("<script>"));
        assert!(!tree.contains("<i>goal"));
        assert!(tree.contains("Milestone &lt;script&gt;"));
        assert!(tree.contains("Goal: &lt;i&gt;goal&lt;/i&gt;"));
    }

    #[test]
    fn text_between_backticks_in_a_title_is_shown_as_code() {
        let html = entries_html(&text(&[
            "### T-001 · Add the `--summary` option",
            "- status: open",
        ]));
        let item = item_of(&html, "T-001");
        assert!(item.contains("<span class=\"t\">Add the <code>--summary</code> option</span>"));
        assert!(item.contains("<span class=\"id\">T-001</span>"));
    }

    #[test]
    fn a_milestone_row_holds_its_marker_and_its_id_before_its_title() {
        let html = tree_html(&text(&[
            "### T-001 · Develop the renderer",
            "- status: open",
            "- type: milestone",
            "### T-002 · Work under it",
            "- status: open",
            "- milestone: T-001",
        ]));
        let row = summary_of(&html, "Develop the renderer");
        assert!(
            row.starts_with(
                "<summary><span class=\"ms-mark\" role=\"img\" aria-label=\"Milestone\">M</span>\
                 <span class=\"ms-id\">T-001</span><span class=\"ms-title\">Develop the renderer</span>"
            ),
            "{row}"
        );
    }

    #[test]
    fn a_nested_milestone_row_holds_its_marker_and_its_id_too() {
        let html = tree_html(&text(&[
            "### T-001 · Develop the renderer",
            "- status: open",
            "- type: milestone",
            "### T-002 · C shader",
            "- status: open",
            "- type: milestone",
            "- milestone: T-001",
        ]));
        let row = summary_of(&html, "C shader");
        assert!(
            row.starts_with(
                "<summary><span class=\"ms-mark\" role=\"img\" aria-label=\"Milestone\">M</span>\
                 <span class=\"ms-id\">T-002</span><span class=\"ms-title\">C shader</span>"
            ),
            "{row}"
        );
    }

    #[test]
    fn the_other_work_row_has_neither_marker_nor_id() {
        let html = tree_html(&text(&[
            "### T-001 · A milestone",
            "- status: open",
            "- type: milestone",
            "### T-002 · A loose task",
            "- status: open",
        ]));
        let row = summary_of(&html, "Other work");
        assert!(
            row.starts_with("<summary><span class=\"ms-title\">Other work</span>"),
            "{row}"
        );
        assert!(!row.contains("ms-mark"), "{row}");
        assert!(!row.contains("ms-id"), "{row}");
    }
}
