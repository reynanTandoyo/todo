//! Writes the report page as one HTML file (T-092; R-REPORT-1).
//!
//! The page has inline CSS, no script, no form, no button and nothing loaded from the network. Each
//! task opens with `<details>` to show its fields and notes. Every text that comes from a task list
//! goes through [`escape`], so a title such as `<script>` is shown as text.

use std::fmt::Write as _;

use crate::doc::{EntryKind, Progress, Status};
use crate::report::model::{About, AreaCount, Claim, Counts, Entry, Page, WorkNode};

/// The stylesheet, copied into the page (`style.css`, light and dark mode, narrow windows).
pub const STYLE: &str = include_str!("style.css");

/// Renders the whole page.
#[must_use]
pub fn render_page(page: &Page) -> String {
    let mut h = String::with_capacity(48 * 1024);
    let _ = write!(
        h,
        "<!doctype html>\n<html lang=\"en\">\n<head>\n\
<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
<title>{}</title>\n\
<style>\n{STYLE}</style>\n\
</head>\n<body>\n",
        escape(&page.title)
    );
    push_header(&mut h, page);
    h.push_str("<main>\n");
    push_work(&mut h, page);
    push_areas(&mut h, page);
    push_questions(&mut h, page);
    push_ideas(&mut h, page);
    push_done(&mut h, page);
    push_dropped(&mut h, page);
    h.push_str("</main>\n");
    let _ = writeln!(
        h,
        "<footer><p class=\"muted\">Read-only overview of {}. It changes nothing; to change a task, ask an agent.</p></footer>",
        escape(&page.file_label)
    );
    h.push_str("</body>\n</html>\n");
    h
}

/// Escapes text for an HTML text node or a quoted attribute value.
#[must_use]
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

fn push_header(h: &mut String, page: &Page) {
    h.push_str("<header>\n<p class=\"kicker\">Task overview</p>\n");
    let _ = writeln!(h, "<h1>{}</h1>", escape(&page.title));
    let source = match &page.archive_label {
        Some(archive) => format!("{} and {}", page.file_label, archive),
        None => page.file_label.clone(),
    };
    let _ = writeln!(
        h,
        "<p class=\"source\">{} \u{00B7} as of {}</p>",
        escape(&source),
        escape(&page.generated)
    );
    push_counts(h, &page.counts);
    h.push_str(
        "<nav class=\"jump\" aria-label=\"Sections\">\
<a href=\"#work\">Work</a>\
<a href=\"#areas\">Areas</a>\
<a href=\"#questions\">Questions</a>\
<a href=\"#ideas\">Ideas</a>\
<a href=\"#done\">Done</a>\
<a href=\"#dropped\">Dropped</a>\
</nav>\n</header>\n",
    );
}

fn push_counts(h: &mut String, counts: &Counts) {
    let items = [
        (counts.open, "open"),
        (counts.claimed, "claimed"),
        (counts.blocked, "blocked"),
        (counts.milestones, "milestones unfinished"),
        (counts.questions, "open questions"),
        (counts.ideas, "ideas"),
        (counts.done, "done"),
        (counts.dropped, "dropped"),
    ];
    h.push_str("<ul class=\"counts\">\n");
    for (n, label) in items {
        let _ = writeln!(h, "<li><span class=\"n\">{n}</span> {label}</li>");
    }
    h.push_str("</ul>\n");
}

fn push_work(h: &mut String, page: &Page) {
    h.push_str(
        "<section id=\"work\">\n<h2>Work</h2>\n\
<p class=\"hint\">Milestones with their goal and progress. The tasks below a milestone are indented under it. Open a task to see its fields and notes.</p>\n",
    );
    if page.work.is_empty() {
        h.push_str("<p class=\"empty\">Nothing unfinished.</p>\n");
    } else {
        push_nodes(h, &page.work, "tree");
    }
    h.push_str("</section>\n");
}

fn push_nodes(h: &mut String, nodes: &[WorkNode], class: &str) {
    let _ = writeln!(h, "<ol class=\"{class}\">");
    for node in nodes {
        push_node(h, node);
    }
    h.push_str("</ol>\n");
}

fn push_node(h: &mut String, node: &WorkNode) {
    let entry = &node.entry;
    if entry.kind == EntryKind::Milestone {
        push_milestone(h, node);
    } else {
        let _ = writeln!(
            h,
            "<li class=\"node task {}\" id=\"e-{}\">",
            status_class(entry.status),
            escape(&entry.id)
        );
        h.push_str("<details>\n<summary>");
        push_task_row(h, entry);
        h.push_str("</summary>\n");
        push_body(h, entry);
        h.push_str("</details>\n</li>\n");
    }
}

fn push_milestone(h: &mut String, node: &WorkNode) {
    let entry = &node.entry;
    let _ = writeln!(
        h,
        "<li class=\"node milestone {}\" id=\"e-{}\">",
        status_class(entry.status),
        escape(&entry.id)
    );
    h.push_str("<div class=\"milestone-head\">");
    let _ = write!(
        h,
        "<span class=\"id\">{}</span><span class=\"title\">{}</span>",
        escape(&entry.id),
        escape(&entry.title)
    );
    push_chips(h, entry);
    push_state(h, entry);
    push_progress(h, node.progress);
    h.push_str("</div>\n");
    match &entry.goal {
        Some(goal) => {
            let _ = writeln!(h, "<p class=\"goal\">{}</p>", escape(goal));
        }
        None => h.push_str("<p class=\"goal muted\">No goal recorded.</p>\n"),
    }
    h.push_str("<details>\n<summary>Fields and notes</summary>\n");
    push_body(h, entry);
    h.push_str("</details>\n");
    if !node.children.is_empty() {
        push_nodes(h, &node.children, "children");
    }
    h.push_str("</li>\n");
}

/// The row of a task in the work tree, shown on the `<summary>` that opens its fields and notes.
fn push_task_row(h: &mut String, entry: &Entry) {
    let _ = write!(
        h,
        "<span class=\"id\">{}</span><span class=\"title\">{}</span>",
        escape(&entry.id),
        escape(&entry.title)
    );
    push_chips(h, entry);
    push_state(h, entry);
}

/// The priority, difficulty and area of an entry, when it has them.
fn push_chips(h: &mut String, entry: &Entry) {
    if let Some(priority) = &entry.priority {
        let _ = write!(
            h,
            "<span class=\"chip\" title=\"priority\">{}</span>",
            escape(priority)
        );
    }
    if let Some(difficulty) = &entry.difficulty {
        let _ = write!(
            h,
            "<span class=\"chip\" title=\"difficulty\">{}</span>",
            escape(difficulty)
        );
    }
    if let Some(area) = &entry.area {
        let _ = write!(
            h,
            "<span class=\"chip\" title=\"area\">{}</span>",
            escape(area)
        );
    }
}

/// The state of an entry: who holds a claim and how old it is (with `stale` when it is stale), or
/// the latest reason for a block. Open entries show nothing.
fn push_state(h: &mut String, entry: &Entry) {
    if let Some(claim) = &entry.claim {
        let _ = write!(
            h,
            "<span class=\"chip\">{}</span>",
            escape(&claim_text(claim))
        );
        if claim.stale {
            h.push_str("<span class=\"chip stale\">stale</span>");
        }
    }
    match entry.status {
        Status::Blocked => {
            let reason = entry.reason.as_deref().unwrap_or_default();
            let _ = write!(
                h,
                "<span class=\"reason\">blocked: {}</span>",
                escape(reason)
            );
        }
        Status::Dropped => {
            let reason = entry.reason.as_deref().unwrap_or_default();
            let _ = write!(
                h,
                "<span class=\"reason dropped\">dropped: {}</span>",
                escape(reason)
            );
        }
        Status::Open => {}
        Status::Claimed => {}
        Status::Done => h.push_str("<span class=\"chip\">done</span>"),
    }
}

fn claim_text(claim: &Claim) -> String {
    let who = claim.owner.as_deref().unwrap_or("no owner");
    match claim.age_hours {
        Some(hours) => format!("claimed by {who}, {} h", hours.max(0)),
        None => format!("claimed by {who}, time unknown"),
    }
}

fn push_progress(h: &mut String, progress: Option<Progress>) {
    let Some(progress) = progress else {
        return;
    };
    if progress.total == 0 {
        h.push_str("<span class=\"progress-text\">no tasks below yet</span>");
        return;
    }
    let _ = write!(
        h,
        "<progress value=\"{}\" max=\"{}\">{} of {} done</progress><span class=\"progress-text\">{} of {} done</span>",
        progress.done, progress.total, progress.done, progress.total, progress.done, progress.total
    );
}

/// The fields as written and the dated notes of an entry, shown when it is opened.
fn push_body(h: &mut String, entry: &Entry) {
    h.push_str("<div class=\"more\">\n");
    if entry.fields.is_empty() {
        h.push_str("<p class=\"muted\">No fields.</p>\n");
    } else {
        h.push_str("<dl class=\"fields\">\n");
        for (key, value) in &entry.fields {
            let _ = writeln!(h, "<dt>{}</dt><dd>{}</dd>", escape(key), escape(value));
        }
        h.push_str("</dl>\n");
    }
    h.push_str("<h4>Notes</h4>\n");
    if entry.notes.is_empty() {
        h.push_str("<p class=\"muted\">No notes.</p>\n");
    } else {
        h.push_str("<ul class=\"notes\">\n");
        for note in &entry.notes {
            let _ = writeln!(
                h,
                "<li><span class=\"when\">{}</span> <span class=\"who\">{}</span>: {}</li>",
                escape(&note.date),
                escape(&note.agent),
                escape(&note.text)
            );
        }
        h.push_str("</ul>\n");
    }
    h.push_str("</div>\n");
}

fn push_areas(h: &mut String, page: &Page) {
    h.push_str(
        "<section id=\"areas\">\n<h2>Unfinished tasks per area</h2>\n\
<p class=\"hint\">Open, claimed or blocked tasks, counted by their area.</p>\n",
    );
    if page.areas.is_empty() && page.no_area == 0 {
        h.push_str("<p class=\"empty\">No unfinished tasks.</p>\n");
    } else {
        h.push_str("<ul class=\"areas\">\n");
        for AreaCount { area, count } in &page.areas {
            let _ = writeln!(
                h,
                "<li><span>{}</span><span class=\"count\">{count}</span></li>",
                escape(area)
            );
        }
        if page.no_area > 0 {
            let _ = writeln!(
                h,
                "<li><span class=\"muted\">no area</span><span class=\"count\">{}</span></li>",
                page.no_area
            );
        }
        h.push_str("</ul>\n");
    }
    h.push_str("</section>\n");
}

fn push_questions(h: &mut String, page: &Page) {
    let _ = writeln!(
        h,
        "<section id=\"questions\">\n<h2>Open questions ({})</h2>",
        page.questions.len()
    );
    if page.questions.is_empty() {
        h.push_str("<p class=\"empty\">No open questions.</p>\n");
    } else {
        h.push_str(
            "<p class=\"hint\">Each question links to the milestone or task it is about.</p>\n",
        );
        push_list(h, &page.questions, false);
    }
    h.push_str("</section>\n");
}

fn push_ideas(h: &mut String, page: &Page) {
    let _ = writeln!(
        h,
        "<section id=\"ideas\">\n<h2>Ideas ({})</h2>",
        page.ideas.len()
    );
    if page.ideas.is_empty() {
        h.push_str("<p class=\"empty\">No ideas waiting.</p>\n");
    } else {
        h.push_str(
            "<p class=\"hint\">Ideas are not work until someone turns one into a task.</p>\n",
        );
        push_list(h, &page.ideas, false);
    }
    h.push_str("</section>\n");
}

fn push_done(h: &mut String, page: &Page) {
    let _ = writeln!(
        h,
        "<section id=\"done\">\n<h2>Done ({})</h2>",
        page.done.len()
    );
    if page.done.is_empty() {
        h.push_str("<p class=\"empty\">Nothing done yet.</p>\n");
    } else {
        h.push_str(
            "<p class=\"hint\">Finished entries of every kind that the work above does not show. Entries from the archive are marked archived.</p>\n",
        );
        push_list(h, &page.done, true);
    }
    h.push_str("</section>\n");
}

fn push_dropped(h: &mut String, page: &Page) {
    let _ = writeln!(
        h,
        "<section id=\"dropped\">\n<h2>Dropped ({})</h2>",
        page.dropped.len()
    );
    if page.dropped.is_empty() {
        h.push_str("<p class=\"empty\">Nothing dropped.</p>\n");
    } else {
        h.push_str("<p class=\"hint\">Each entry shows the reason it was dropped.</p>\n");
        push_list(h, &page.dropped, true);
    }
    h.push_str("</section>\n");
}

/// A list of entries that are not part of the work tree: each opens to show its fields and notes.
fn push_list(h: &mut String, entries: &[Entry], show_kind: bool) {
    h.push_str("<ul class=\"list\">\n");
    for entry in entries {
        let _ = writeln!(h, "<li class=\"item\" id=\"e-{}\">", escape(&entry.id));
        h.push_str("<details>\n<summary>");
        if show_kind {
            let _ = write!(
                h,
                "<span class=\"kind-label\">{}</span> ",
                entry.kind.as_str()
            );
        }
        let _ = write!(
            h,
            "<span class=\"id\">{}</span><span class=\"title\">{}</span>",
            escape(&entry.id),
            escape(&entry.title)
        );
        push_chips(h, entry);
        if entry.archived {
            h.push_str("<span class=\"chip\">archived</span>");
        }
        match entry.status {
            Status::Dropped => {
                let reason = entry.reason.as_deref().unwrap_or_default();
                let _ = write!(
                    h,
                    "<span class=\"reason dropped\">dropped: {}</span>",
                    escape(reason)
                );
            }
            Status::Open | Status::Claimed | Status::Blocked => push_state(h, entry),
            Status::Done => {}
        }
        if let Some(about) = &entry.about {
            push_about(h, about);
        }
        h.push_str("</summary>\n");
        push_body(h, entry);
        h.push_str("</details>\n</li>\n");
    }
    h.push_str("</ul>\n");
}

/// "about T-001 · Develop the renderer": a link when the page shows that entry.
fn push_about(h: &mut String, about: &About) {
    let name = match &about.title {
        Some(title) => format!("{} \u{00B7} {}", about.id, title),
        None => about.id.clone(),
    };
    if about.linked {
        let _ = write!(
            h,
            "<span class=\"chip\">about <a href=\"#e-{}\">{}</a></span>",
            escape(&about.id),
            escape(&name)
        );
    } else {
        let _ = write!(h, "<span class=\"chip\">about {}</span>", escape(&name));
    }
}

/// The CSS class that goes with a status.
fn status_class(status: Status) -> String {
    format!("status-{}", status.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_covers_every_markup_character() {
        assert_eq!(
            escape("<script>alert(\"x\" & 'y')</script>"),
            "&lt;script&gt;alert(&quot;x&quot; &amp; &#39;y&#39;)&lt;/script&gt;"
        );
    }

    #[test]
    fn style_has_light_dark_and_narrow_rules() {
        assert!(STYLE.contains("prefers-color-scheme: dark"));
        assert!(STYLE.contains("max-width: 600px"));
        assert!(!STYLE.contains("@import"));
        assert!(!STYLE.contains("url("));
    }
}
