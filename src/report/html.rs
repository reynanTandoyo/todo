//! Writes the report pages as HTML (T-092, T-094; R-REPORT-1, R-REPORT-2).
//!
//! The pages have inline CSS, no script, no form, no button and nothing loaded from the network. Each
//! task opens with `<details>` to show its fields and notes. Every text that comes from a task list
//! goes through [`escape`], so a title such as `<script>` is shown as text.
//!
//! The single-project page and each project on the all-projects page are written by the same
//! functions. [`Scope`] keeps their anchor IDs apart and sets the heading level of their sections.

use std::fmt::Write as _;

use crate::doc::{EntryKind, Progress, Status};
use crate::register::overview::TaskCounts;
use crate::report::all::{AllPage, Content, Lists, ProjectPage};
use crate::report::model::{
    About, AreaCount, Claim, Counts, Entry, Facts, LevelCount, LevelCounts, Page, PlanRow,
    Planning, WorkNode,
};

/// The stylesheet, copied into the page (`style.css`, light and dark mode, narrow windows).
pub const STYLE: &str = include_str!("style.css");

/// Where a set of lists sits on a page. `prefix` goes in front of every anchor ID, so that the lists
/// of several projects on one page do not clash; `level` is the heading level of their sections.
#[derive(Debug, Clone, Copy)]
struct Scope<'a> {
    prefix: &'a str,
    level: u8,
}

/// The scope of the single-project page: no prefix, sections under `h2`.
const SINGLE: Scope<'static> = Scope {
    prefix: "",
    level: 2,
};

impl Scope<'_> {
    /// The anchor ID of a section, such as `work`.
    fn section_id(&self, name: &str) -> String {
        format!("{}{name}", self.prefix)
    }

    /// The anchor ID of an entry, such as `e-T-001`.
    fn entry_id(&self, id: &str) -> String {
        format!("{}e-{id}", self.prefix)
    }
}

/// Renders the single-project page.
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
    push_lists(&mut h, page, SINGLE);
    h.push_str("</main>\n");
    let _ = writeln!(
        h,
        "<footer><p class=\"muted\">Read-only overview of {}. It changes nothing; to change a task, ask an agent.</p></footer>",
        escape(&page.file_label)
    );
    h.push_str("</body>\n</html>\n");
    h
}

/// Renders the page for every registered project (R-REPORT-2): the header, a card per project, then
/// each project's lists under its own heading.
#[must_use]
pub fn render_all_page(page: &AllPage) -> String {
    let mut h = String::with_capacity(64 * 1024);
    let _ = write!(
        h,
        "<!doctype html>\n<html lang=\"en\">\n<head>\n\
<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
<title>Task overview: all projects</title>\n\
<style>\n{STYLE}</style>\n\
</head>\n<body>\n"
    );
    push_all_header(&mut h, page);
    h.push_str("<main>\n");
    push_cards(&mut h, page);
    for (index, project) in page.projects.iter().enumerate() {
        if let Some(lists) = project.lists() {
            push_project(&mut h, project, lists, index);
        }
    }
    h.push_str("</main>\n");
    h.push_str(
        "<footer><p class=\"muted\">Read-only overview of the registered projects. It changes nothing; to change a task, ask an agent.</p></footer>\n",
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
<a href=\"#planning\">Planning</a>\
<a href=\"#work\">Work</a>\
<a href=\"#areas\">Areas</a>\
<a href=\"#questions\">Questions</a>\
<a href=\"#ideas\">Ideas</a>\
<a href=\"#done\">Done</a>\
<a href=\"#dropped\">Dropped</a>\
</nav>\n</header>\n",
    );
}

fn push_all_header(h: &mut String, page: &AllPage) {
    let count = page.projects.len();
    let noun = if count == 1 { "project" } else { "projects" };
    h.push_str(
        "<header>\n<p class=\"kicker\">Task overview</p>\n<h1>All registered projects</h1>\n",
    );
    let _ = writeln!(
        h,
        "<p class=\"source\">{count} {noun} \u{00B7} as of {}</p>\n</header>",
        escape(&page.generated)
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

/// The sections of one page's lists, in the order the single-project page shows them.
fn push_lists(h: &mut String, page: &Page, scope: Scope<'_>) {
    push_planning(h, page, scope);
    push_work(h, page, scope);
    push_areas(h, page, scope);
    push_questions(h, page, scope);
    push_ideas(h, page, scope);
    push_done(h, page, scope);
    push_dropped(h, page, scope);
}

/// The planning state (`R-REPORT-3`): open tasks per level, what is not ready for lack of detail, what
/// was given back and why, what is kept above the target level, and the waves. Each count is the one
/// `todo plan` and `todo waves` give.
fn push_planning(h: &mut String, page: &Page, scope: Scope<'_>) {
    section_start(h, scope, "planning", "Planning");
    let planning: &Planning = &page.planning;
    let sub = scope.level + 1;
    let _ = writeln!(
        h,
        "<p class=\"hint\">What the planner still has to do. The target level is {}: a task at or below it needs complete detail before a worker is handed it.</p>",
        escape(&planning.target)
    );

    let _ = writeln!(h, "<h{sub}>Open tasks per level</h{sub}>");
    h.push_str("<p class=\"hint\">Open, claimed or blocked tasks, counted by their level.</p>\n");
    push_level_counts(h, &page.levels);

    let _ = writeln!(
        h,
        "<h{sub}>Not ready for lack of detail ({})</h{sub}>",
        planning.not_ready.len()
    );
    if planning.not_ready.is_empty() {
        h.push_str(
            "<p class=\"empty\">Every task at or below the target level has its detail.</p>\n",
        );
    } else {
        h.push_str("<ul class=\"card-list\">\n");
        for item in &planning.not_ready {
            h.push_str("<li>");
            push_plan_row(h, &item.row, scope);
            if !item.missing.is_empty() {
                let missing: Vec<String> = item.missing.iter().map(|m| escape(m)).collect();
                let _ = write!(
                    h,
                    " <span class=\"reason\">missing: {}</span>",
                    missing.join(", ")
                );
            }
            h.push_str("</li>\n");
        }
        h.push_str("</ul>\n");
    }

    let _ = writeln!(
        h,
        "<h{sub}>Given back to the planner ({})</h{sub}>",
        planning.given_back.len()
    );
    if planning.given_back.is_empty() {
        h.push_str("<p class=\"empty\">Nothing given back.</p>\n");
    } else {
        h.push_str("<ul class=\"card-list\">\n");
        for item in &planning.given_back {
            h.push_str("<li>");
            push_plan_row(h, &item.row, scope);
            let _ = write!(
                h,
                " <span class=\"reason\">{}</span> <span class=\"chip\">attempts {}</span>",
                escape(&item.problem),
                escape(&item.attempts)
            );
            h.push_str("</li>\n");
        }
        h.push_str("</ul>\n");
    }

    let _ = writeln!(
        h,
        "<h{sub}>Kept above the target level on purpose ({})</h{sub}>",
        planning.kept_on_purpose.len()
    );
    if planning.kept_on_purpose.is_empty() {
        h.push_str("<p class=\"empty\">Nothing kept above the target level.</p>\n");
    } else {
        h.push_str("<ul class=\"card-list\">\n");
        for item in &planning.kept_on_purpose {
            h.push_str("<li>");
            push_plan_row(h, &item.row, scope);
            let _ = write!(h, " <span class=\"reason\">{}</span>", escape(&item.reason));
            h.push_str("</li>\n");
        }
        h.push_str("</ul>\n");
    }

    let _ = writeln!(h, "<h{sub}>Waves</h{sub}>");
    if planning.waves == 0 {
        h.push_str("<p class=\"empty\">No unfinished task, so no waves.</p>\n");
    } else {
        let _ = writeln!(
            h,
            "<p>{} in all. Wave 1 holds {} in {}.</p>",
            counted(planning.waves, "wave", "waves"),
            counted(planning.wave1_tasks, "task", "tasks"),
            counted(planning.wave1_lanes, "lane", "lanes")
        );
    }
    h.push_str("</section>\n");
}

/// A task of the planning state: its ID and title, linked to its row when the page shows it, then its
/// level.
fn push_plan_row(h: &mut String, row: &PlanRow, scope: Scope<'_>) {
    let name = format!("{} \u{00B7} {}", row.id, row.title);
    if row.linked {
        let _ = write!(
            h,
            "<a href=\"#{}\">{}</a>",
            escape(&scope.entry_id(&row.id)),
            escape(&name)
        );
    } else {
        let _ = write!(h, "<span>{}</span>", escape(&name));
    }
    if let Some(level) = &row.level {
        let _ = write!(
            h,
            " <span class=\"chip\" title=\"level\">{}</span>",
            escape(level)
        );
    }
}

/// Unfinished tasks per level: each configured level, then the tasks without a level and those whose
/// level is not configured, when there are any.
fn push_level_counts(h: &mut String, levels: &LevelCounts) {
    h.push_str("<ul class=\"areas levels\">\n");
    for LevelCount { level, count } in &levels.levels {
        let _ = writeln!(
            h,
            "<li><span>{}</span><span class=\"count\">{count}</span></li>",
            escape(level)
        );
    }
    if levels.no_level > 0 {
        let _ = writeln!(
            h,
            "<li><span class=\"muted\">no level</span><span class=\"count\">{}</span></li>",
            levels.no_level
        );
    }
    if levels.other > 0 {
        let _ = writeln!(
            h,
            "<li><span class=\"muted\">level not configured</span><span class=\"count\">{}</span></li>",
            levels.other
        );
    }
    h.push_str("</ul>\n");
}

/// `count` with the singular or the plural word, such as `1 wave` or `3 waves`.
fn counted(count: usize, singular: &str, plural: &str) -> String {
    let word = if count == 1 { singular } else { plural };
    format!("{count} {word}")
}

/// The level, files and check of a task, as an opened task shows them first (`R-REPORT-3`), on one
/// line to keep the page short. A missing part reads `none` (a task without files may change any
/// file, as the work section says); `Verified` is shown only when there is a check to run.
fn push_facts(h: &mut String, facts: &Facts) {
    let level = facts.level.as_deref().unwrap_or("none");
    let files = facts.files.as_deref().unwrap_or("none");
    let _ = write!(
        h,
        "<p class=\"facts\">Level: {} \u{00B7} Files: {}",
        escape(level),
        escape(files)
    );
    match facts.verify.as_deref() {
        None => h.push_str(" \u{00B7} Verify: none"),
        Some(command) => {
            let verified = facts.verified.as_deref().unwrap_or("not yet");
            let _ = write!(
                h,
                " \u{00B7} Verify: <code>{}</code> \u{00B7} Verified: {}",
                escape(command),
                escape(verified)
            );
        }
    }
    h.push_str("</p>\n");
}

/// Opens a section with its anchor and its heading, at the heading level of `scope`.
fn section_start(h: &mut String, scope: Scope<'_>, name: &str, heading: &str) {
    let level = scope.level;
    let _ = writeln!(
        h,
        "<section id=\"{}\">\n<h{level}>{}</h{level}>",
        escape(&scope.section_id(name)),
        escape(heading)
    );
}

fn push_work(h: &mut String, page: &Page, scope: Scope<'_>) {
    section_start(h, scope, "work", "Work");
    h.push_str(
        "<p class=\"hint\">Milestones with their goal and progress. The tasks below a milestone are indented under it. Open a task to see its level, files, check, fields and notes; files none means the task may change any file.</p>\n",
    );
    if page.work.is_empty() {
        h.push_str("<p class=\"empty\">Nothing unfinished.</p>\n");
    } else {
        push_nodes(h, &page.work, "tree", scope);
    }
    h.push_str("</section>\n");
}

fn push_nodes(h: &mut String, nodes: &[WorkNode], class: &str, scope: Scope<'_>) {
    let _ = writeln!(h, "<ol class=\"{class}\">");
    for node in nodes {
        push_node(h, node, scope);
    }
    h.push_str("</ol>\n");
}

fn push_node(h: &mut String, node: &WorkNode, scope: Scope<'_>) {
    let entry = &node.entry;
    if entry.kind == EntryKind::Milestone {
        push_milestone(h, node, scope);
    } else {
        let _ = writeln!(
            h,
            "<li class=\"node\" id=\"{}\">",
            escape(&scope.entry_id(&entry.id))
        );
        h.push_str("<details>\n<summary>");
        push_task_row(h, entry);
        h.push_str("</summary>\n");
        push_body(h, entry);
        h.push_str("</details>\n</li>\n");
    }
}

fn push_milestone(h: &mut String, node: &WorkNode, scope: Scope<'_>) {
    let entry = &node.entry;
    let _ = writeln!(
        h,
        "<li class=\"node milestone {}\" id=\"{}\">",
        status_class(entry.status),
        escape(&scope.entry_id(&entry.id))
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
        push_nodes(h, &node.children, "children", scope);
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
    // A priority code and an area name read as they are; a bare level number needs its label.
    if let Some(priority) = &entry.priority {
        let _ = write!(h, "<span class=\"chip\">{}</span>", escape(priority));
    }
    if let Some(difficulty) = &entry.difficulty {
        let _ = write!(
            h,
            "<span class=\"chip\" title=\"difficulty\">{}</span>",
            escape(difficulty)
        );
    }
    if let Some(area) = &entry.area {
        let _ = write!(h, "<span class=\"chip\">{}</span>", escape(area));
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
    if let Some(facts) = &entry.facts {
        push_facts(h, facts);
    }
    if entry.fields.is_empty() {
        h.push_str("<p class=\"muted\">No fields.</p>\n");
    } else {
        h.push_str("<dl class=\"fields\">");
        for (key, value) in &entry.fields {
            let _ = write!(h, "<dt>{}</dt><dd>{}</dd>", escape(key), escape(value));
        }
        h.push_str("</dl>\n");
    }
    // A task without notes has no notes block.
    if !entry.notes.is_empty() {
        h.push_str("<h4>Notes</h4>\n<ul class=\"notes\">\n");
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
}

fn push_areas(h: &mut String, page: &Page, scope: Scope<'_>) {
    section_start(h, scope, "areas", "Unfinished tasks per area");
    h.push_str("<p class=\"hint\">Open, claimed or blocked tasks, counted by their area.</p>\n");
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

fn push_questions(h: &mut String, page: &Page, scope: Scope<'_>) {
    section_start(
        h,
        scope,
        "questions",
        &format!("Open questions ({})", page.questions.len()),
    );
    if page.questions.is_empty() {
        h.push_str("<p class=\"empty\">No open questions.</p>\n");
    } else {
        h.push_str(
            "<p class=\"hint\">Each question links to the milestone or task it is about.</p>\n",
        );
        push_list(h, &page.questions, false, scope);
    }
    h.push_str("</section>\n");
}

fn push_ideas(h: &mut String, page: &Page, scope: Scope<'_>) {
    section_start(h, scope, "ideas", &format!("Ideas ({})", page.ideas.len()));
    if page.ideas.is_empty() {
        h.push_str("<p class=\"empty\">No ideas waiting.</p>\n");
    } else {
        h.push_str(
            "<p class=\"hint\">Ideas are not work until someone turns one into a task.</p>\n",
        );
        push_list(h, &page.ideas, false, scope);
    }
    h.push_str("</section>\n");
}

fn push_done(h: &mut String, page: &Page, scope: Scope<'_>) {
    section_start(h, scope, "done", &format!("Done ({})", page.done.len()));
    if page.done.is_empty() {
        h.push_str("<p class=\"empty\">Nothing done yet.</p>\n");
    } else {
        h.push_str(
            "<p class=\"hint\">Finished entries of every kind that the work above does not show. Entries from the archive are marked archived.</p>\n",
        );
        push_list(h, &page.done, true, scope);
    }
    h.push_str("</section>\n");
}

fn push_dropped(h: &mut String, page: &Page, scope: Scope<'_>) {
    section_start(
        h,
        scope,
        "dropped",
        &format!("Dropped ({})", page.dropped.len()),
    );
    if page.dropped.is_empty() {
        h.push_str("<p class=\"empty\">Nothing dropped.</p>\n");
    } else {
        h.push_str("<p class=\"hint\">Each entry shows the reason it was dropped.</p>\n");
        push_list(h, &page.dropped, true, scope);
    }
    h.push_str("</section>\n");
}

/// A list of entries that are not part of the work tree: each opens to show its fields and notes.
fn push_list(h: &mut String, entries: &[Entry], show_kind: bool, scope: Scope<'_>) {
    h.push_str("<ul class=\"list\">\n");
    for entry in entries {
        let _ = writeln!(
            h,
            "<li class=\"item\" id=\"{}\">",
            escape(&scope.entry_id(&entry.id))
        );
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
            push_about(h, about, scope);
        }
        h.push_str("</summary>\n");
        push_body(h, entry);
        h.push_str("</details>\n</li>\n");
    }
    h.push_str("</ul>\n");
}

/// "about T-001 · Develop the renderer": a link when the page shows that entry.
fn push_about(h: &mut String, about: &About, scope: Scope<'_>) {
    let name = match &about.title {
        Some(title) => format!("{} \u{00B7} {}", about.id, title),
        None => about.id.clone(),
    };
    if about.linked {
        let _ = write!(
            h,
            "<span class=\"chip\">about <a href=\"#{}\">{}</a></span>",
            escape(&scope.entry_id(&about.id)),
            escape(&name)
        );
    } else {
        let _ = write!(h, "<span class=\"chip\">about {}</span>", escape(&name));
    }
}

/// The tasks of a project per status, at the top of its card.
fn push_task_counts(h: &mut String, tasks: TaskCounts) {
    let items = [
        (tasks.open, "open"),
        (tasks.claimed, "claimed"),
        (tasks.blocked, "blocked"),
        (tasks.done, "done"),
        (tasks.dropped, "dropped"),
    ];
    h.push_str("<ul class=\"counts\">\n");
    for (n, label) in items {
        let _ = writeln!(h, "<li><span class=\"n\">{n}</span> {label}</li>");
    }
    h.push_str("</ul>\n");
}

/// The cards at the top of the all-projects page, one per project, in register order.
fn push_cards(h: &mut String, page: &AllPage) {
    let _ = writeln!(
        h,
        "<section id=\"projects\">\n<h2>Projects ({})</h2>",
        page.projects.len()
    );
    if page.projects.is_empty() {
        h.push_str(
            "<p class=\"empty\">No project is registered yet. A project is listed here once an agent has registered its folder: ask an agent working in that folder to register the project, then build this page again.</p>\n",
        );
    } else {
        h.push_str(
            "<p class=\"hint\">One card per project. A card links to the project's lists further down the page.</p>\n<ul class=\"cards\">\n",
        );
        for (index, project) in page.projects.iter().enumerate() {
            push_card(h, project, index);
        }
        h.push_str("</ul>\n");
    }
    h.push_str("</section>\n");
}

/// The anchor of a project's card and lists, such as `p2`.
fn project_anchor(index: usize) -> String {
    format!("p{}", index + 1)
}

/// The prefix of the anchor IDs in a project's lists, such as `p2-`.
fn project_prefix(index: usize) -> String {
    format!("p{}-", index + 1)
}

fn push_card(h: &mut String, project: &ProjectPage, index: usize) {
    h.push_str("<li class=\"card\">\n");
    let name = escape(&project.name);
    match project.lists() {
        Some(_) => {
            let _ = writeln!(
                h,
                "<h3><a href=\"#{}\">{name}</a></h3>",
                escape(&project_anchor(index))
            );
        }
        None => {
            let _ = writeln!(h, "<h3>{name}</h3>");
        }
    }
    let _ = writeln!(h, "<p class=\"path\">{}</p>", escape(&project.path));
    match &project.content {
        Content::Problem(problem) => {
            let _ = writeln!(
                h,
                "<p class=\"problem\">This project cannot be shown: {} <span class=\"chip\">{}</span></p>",
                escape(&problem.message),
                problem.code.as_str()
            );
        }
        Content::Lists(lists) => push_card_body(h, lists, index),
    }
    h.push_str("</li>\n");
}

/// What a card shows for a project that can be read: tasks per status, top-level milestones with
/// their progress, blocked tasks and open questions, each linked to its row in the lists below.
fn push_card_body(h: &mut String, lists: &Lists, index: usize) {
    let prefix = project_prefix(index);
    let scope = Scope {
        prefix: &prefix,
        level: 3,
    };
    let card = &lists.card;
    push_task_counts(h, card.tasks);

    h.push_str("<h4>Open tasks per level</h4>\n");
    push_level_counts(h, &card.levels);

    h.push_str("<h4>Top-level milestones</h4>\n");
    if card.milestones.is_empty() {
        h.push_str("<p class=\"muted\">No milestones.</p>\n");
    } else {
        h.push_str("<ul class=\"card-list\">\n");
        for milestone in &card.milestones {
            let _ = write!(
                h,
                "<li><a href=\"#{}\">{} \u{00B7} {}</a> ",
                escape(&scope.entry_id(&milestone.id)),
                escape(&milestone.id),
                escape(&milestone.title)
            );
            if milestone.status == Status::Done {
                h.push_str("<span class=\"chip\">done</span> ");
            }
            push_progress(h, Some(milestone.progress));
            h.push_str("</li>\n");
        }
        h.push_str("</ul>\n");
    }

    h.push_str("<h4>Blocked tasks</h4>\n");
    if card.blocked.is_empty() {
        h.push_str("<p class=\"muted\">No blocked tasks.</p>\n");
    } else {
        h.push_str("<ul class=\"card-list\">\n");
        for task in &card.blocked {
            let _ = writeln!(
                h,
                "<li><a href=\"#{}\">{} \u{00B7} {}</a> <span class=\"reason\">blocked: {}</span></li>",
                escape(&scope.entry_id(&task.id)),
                escape(&task.id),
                escape(&task.title),
                escape(&task.reason)
            );
        }
        h.push_str("</ul>\n");
    }

    h.push_str("<h4>Open questions</h4>\n");
    if card.questions.is_empty() {
        h.push_str("<p class=\"muted\">No open questions.</p>\n");
    } else {
        h.push_str("<ul class=\"card-list\">\n");
        for question in &card.questions {
            let _ = writeln!(
                h,
                "<li><a href=\"#{}\">{} \u{00B7} {}</a></li>",
                escape(&scope.entry_id(&question.id)),
                escape(&question.id),
                escape(&question.title)
            );
        }
        h.push_str("</ul>\n");
    }
}

/// One project's section on the all-projects page: its name, its path and its lists.
fn push_project(h: &mut String, project: &ProjectPage, lists: &Lists, index: usize) {
    let prefix = project_prefix(index);
    let scope = Scope {
        prefix: &prefix,
        level: 3,
    };
    let _ = writeln!(
        h,
        "<section class=\"project\" id=\"{}\">",
        escape(&project_anchor(index))
    );
    let _ = writeln!(h, "<h2>{}</h2>", escape(&project.name));
    let _ = writeln!(h, "<p class=\"path\">{}</p>", escape(&project.path));
    push_lists(h, &lists.page, scope);
    h.push_str("</section>\n");
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

    #[test]
    fn scopes_keep_the_ids_of_two_projects_apart() {
        assert_eq!(SINGLE.entry_id("T-001"), "e-T-001");
        assert_eq!(SINGLE.section_id("work"), "work");
        assert_eq!(project_prefix(1), "p2-");
        assert_eq!(project_anchor(1), "p2");
        let first = Scope {
            prefix: &project_prefix(0),
            level: 3,
        };
        assert_eq!(first.entry_id("T-001"), "p1-e-T-001");
        assert_eq!(first.section_id("work"), "p1-work");
    }
}
