//! CLI integration tests for `todo report --html` (T-092, T-214; R-REPORT-1, R-LOCK-5): the page it
//! writes, where it writes it, what it shows and what it leaves alone. Every fixture is invented.

mod common;

use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::{Duration, Instant};

use tempfile::TempDir;

use common::{
    fixture, parse_json, run_todo, setup_fixture_proj, setup_proj, stderr_text, stdout_text,
    write_toml,
};

const FIXTURE_MARKER: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->\n";

/// The clock every test runs with, so the page and its claim ages do not depend on the day.
const NOW: &str = "2026-11-19T10:00:00+01:00";

/// The `description` of the fixture project (`todo.toml`, R-CFG-2).
const DESCRIPTION: &str = "Plans the lanterns for the club hall from a sky map.";

/// Runs `todo report --html` with `extra` options in the project, with the fixed clock.
fn report(home: &Path, work: &Path, extra: &[&str]) -> Output {
    let mut args = vec!["report", "--html"];
    args.extend_from_slice(extra);
    run_todo(home, work, &args, &[("TODO_TEST_NOW", NOW)]).unwrap()
}

/// The page `todo report --html` writes by default: `report.html` in the state folder.
fn default_page(work: &Path) -> PathBuf {
    work.join(".todo").join("report.html")
}

/// Reads the default page after a successful report.
fn default_html(work: &Path) -> String {
    fs::read_to_string(default_page(work)).unwrap()
}

/// Writes the invented page fixture `report_page.md` (with its archive and a description) into a
/// new project, runs the report with the fixed clock, and returns the project folder and the page.
fn fixture_report(tmp: &TempDir) -> (PathBuf, String) {
    let (home, work, _todo) = setup_fixture_proj(tmp, "report_page.md");
    fs::write(
        work.join("TODO-archive.md"),
        fixture("report_page_archive.md"),
    )
    .unwrap();
    write_toml(&work, &format!("description = \"{DESCRIPTION}\"\n"));
    let out = report(&home, &work, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    let html = default_html(&work);
    (work, html)
}

/// The part of `html` from the heading `heading` (the text inside `<h2>`) to the next `<h2>`, or to
/// the end of the page.
fn section<'a>(html: &'a str, heading: &str) -> &'a str {
    let start = html
        .find(&format!("<h2>{heading}"))
        .unwrap_or_else(|| panic!("no section {heading:?}"));
    let rest = &html[start..];
    let end = rest[4..].find("<h2>").map_or(rest.len(), |at| at + 4);
    &rest[..end]
}

/// The position of each text in `texts` in `haystack`, in order: each must be there and come after
/// the one before it.
fn in_order(haystack: &str, texts: &[&str]) {
    let mut last = 0;
    for text in texts {
        let at = haystack[last..]
            .find(text)
            .unwrap_or_else(|| panic!("{text:?} is missing or out of order"));
        last += at + text.len();
    }
}

#[test]
fn report_writes_the_page_to_the_state_folder_and_leaves_todo_md_alone() {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo) = setup_fixture_proj(&tmp, "report_basic.md");
    let before = fs::read(&todo).unwrap();

    let out = report(&home, &work, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));

    let printed = stdout_text(&out);
    let printed_path = Path::new(printed.trim_end());
    assert!(
        printed_path.ends_with(Path::new(".todo").join("report.html")),
        "printed {printed:?}"
    );
    assert!(printed_path.is_file());
    assert_eq!(fs::read(&todo).unwrap(), before, "TODO.md changed");
    assert_eq!(before, fixture("report_basic.md"));

    // Reads take no lock, so the report never creates lock or lease files (R-LOCK-5).
    let state = work.join(".todo");
    assert!(!state.join("write.lock").exists());
    assert!(!state.join("holder.json").exists());
    assert!(!state.join("lease.json").exists());
    assert!(
        state.join(".gitignore").is_file(),
        "state folder is not kept out of git"
    );
}

#[test]
fn report_html_matches_snapshot_with_a_fixed_clock() {
    let tmp = TempDir::new().unwrap();
    let (_work, html) = fixture_report(&tmp);
    insta::assert_snapshot!("report_html", html);
}

#[test]
fn the_header_shows_the_title_the_description_and_the_time_of_the_status() {
    let tmp = TempDir::new().unwrap();
    let (_work, html) = fixture_report(&tmp);
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("<title>Lantern tool \u{2014} status</title>"));
    assert!(html.contains("<h1>Lantern tool</h1>"));
    assert!(html.contains(&format!("<p class=\"desc\">{DESCRIPTION}</p>")));
    assert!(html.contains("Status as of 19 Nov 2026, 10:00 \u{00B7} from TODO.md"));
    // The two colour buttons sit beside the header.
    assert!(html.contains("<button id=\"theme\" type=\"button\">Dark mode</button>"));
    assert!(html.contains("<button id=\"cb\" type=\"button\" aria-pressed=\"false\""));
}

#[test]
fn the_overall_line_counts_the_tasks_and_the_legend_names_the_five_states() {
    let tmp = TempDir::new().unwrap();
    let (_work, html) = fixture_report(&tmp);
    assert!(html.contains("<p class=\"big\">5 of 12 tasks done</p>"));
    for legend in [
        "<li><span class=\"mark done\">\u{2713}</span><b>5</b> done</li>",
        "<li><span class=\"mark prog\">\u{25CF}</span><b>2</b> in progress</li>",
        "<li><span class=\"mark wait\">\u{25D0}</span><b>3</b> waiting</li>",
        "<li><span class=\"mark todo\">\u{25CB}</span><b>2</b> to do</li>",
        "<li><span class=\"mark drop\">\u{2715}</span><b>2</b> cancelled <span class=\"muted\">(not counted in the 12)</span></li>",
    ] {
        assert!(html.contains(legend), "{legend}");
    }
    assert!(html.contains("aria-label=\"5 done, 2 in progress, 3 waiting, 2 to do, 2 cancelled\""));
    for class in ["seg-done", "seg-prog", "seg-wait", "seg-todo", "seg-drop"] {
        assert!(
            html.contains(&format!("<i class=\"{class}\" style=\"width:")),
            "{class}"
        );
    }
}

#[test]
fn needs_you_lists_the_question_the_given_back_task_and_the_stale_claim_in_that_order() {
    let tmp = TempDir::new().unwrap();
    let (_work, html) = fixture_report(&tmp);
    let needs = section(&html, "Needs you (3)");
    in_order(
        needs,
        &[
            "<span class=\"tag\">Question</span><span class=\"what\">How many lanterns per room?</span>",
            "Asked 15 Nov \u{00B7} about \u{201C}Lamp plan\u{201D} \u{00B7} the task \u{201C}Count the lanterns per room\u{201D} waits for this answer",
            "<span class=\"tag\">Given back</span><span class=\"what\">Place the lanterns on the east wall</span>",
            "A worker could not finish it (16 Nov): the plan puts one lantern too many on the east wall",
            "<span class=\"tag\">No activity</span><span class=\"what\">Write the sky map test set</span>",
            "In progress for ",
            " days by sam. It may have been abandoned.",
        ],
    );
    assert!(!html.contains("Nothing needs you."));
}

#[test]
fn last_7_days_lists_the_events_newest_day_first_with_done_before_added() {
    let tmp = TempDir::new().unwrap();
    let (_work, html) = fixture_report(&tmp);
    let recent = section(&html, "Last 7 days");
    in_order(
        recent,
        &[
            "<span class=\"d\">18 Nov</span><span class=\"mark done\">\u{2713}</span><span><b>Club hall</b> finished \u{2014} all 2 tasks done</span>",
            "<span class=\"d\">18 Nov</span><span class=\"mark done\">\u{2713}</span><span>Mark the ceiling points</span>",
            "<span class=\"d\">17 Nov</span><span class=\"mark done\">\u{2713}</span><span>The star catalogue is read from its text file, with the position and brightness of each star.</span>",
            "<span class=\"d\">17 Nov</span><span class=\"mark done\">\u{2713}</span><span>The hall is measured wall to wall, with the height of the ceiling.</span>",
            "<span class=\"d\">17 Nov</span><span class=\"mark todo\">\u{FF0B}</span><span>Added: The hall is measured wall to wall, with the height of the ceiling.</span>",
            "<span class=\"d\">16 Nov</span><span class=\"mark done\">\u{2713}</span><span>Pick the font for the plan</span>",
            "<span class=\"d\">16 Nov</span><span class=\"mark todo\">\u{FF0B}</span><span>2 tasks added to <b>Sky map reader</b></span>",
            "<span class=\"d\">15 Nov</span><span class=\"mark done\">\u{2713}</span><span>Agree the lamp colour</span>",
            "<span class=\"d\">15 Nov</span><span class=\"mark todo\">\u{FF0B}</span><span>Added: Count the lanterns per room</span>",
            "<span class=\"d\">14 Nov</span><span class=\"mark drop\">\u{2715}</span><span>Print a poster <span class=\"muted\">\u{2014} cancelled: the plan on the screen is enough</span></span>",
            "<span class=\"d\">13 Nov</span><span class=\"mark todo\">\u{FF0B}</span><span>Added: The star catalogue is read from its text file, with the position and brightness of each star.</span>",
        ],
    );
    // Eleven rows fit in the first fifteen, so the closed part is not there.
    assert!(!recent.contains("Earlier this week"));
}

#[test]
fn all_work_is_one_tree_and_a_done_entry_stays_in_its_milestone() {
    let tmp = TempDir::new().unwrap();
    let (_work, html) = fixture_report(&tmp);
    // Each milestone holds its nested milestones, then its tasks, in file order; the top-level
    // tasks come last, under Other work. A milestone row names its title, a task row its ID.
    in_order(
        &html,
        &[
            "<span class=\"ms-title\">Sky map reader</span>",
            "<span class=\"id\">T-003</span>",
            "<span class=\"id\">T-004</span>",
            "<span class=\"id\">T-005</span>",
            "<span class=\"id\">T-006</span>",
            "<span class=\"id\">T-010</span>",
            "<span class=\"ms-title\">Lamp plan</span>",
            "<span class=\"id\">T-012</span>",
            "<span class=\"id\">T-013</span>",
            "<span class=\"id\">T-014</span>",
            "<span class=\"ms-title\">Club hall</span>",
            "<span class=\"id\">T-021</span>",
            "<span class=\"id\">T-022</span>",
            "<span class=\"id\">T-016</span>",
            "<span class=\"ms-title\">Other work</span>",
            "<span class=\"id\">T-030</span>",
            "<span class=\"id\">T-032</span>",
            "<span class=\"id\">T-033</span>",
        ],
    );

    // The finished milestone is closed and shows its two done tasks.
    assert!(html.contains(
        "<details class=\"ms\">\n<summary><span class=\"ms-title\">Club hall</span><span class=\"ms-state st-done\">finished 18 Nov</span><span class=\"ms-count\">2 of 2 done</span>"
    ));
    let club = section_from(&html, "Club hall");
    assert!(
        club.contains("<span class=\"t\">Measure the hall</span>"),
        "{club}"
    );
    assert!(club.contains("Result"), "{club}");

    // The milestone with work in progress is open and counts its tasks below at any depth.
    assert!(html.contains(
        "<details class=\"ms\" open>\n<summary><span class=\"ms-title\">Sky map reader</span><span class=\"ms-state st-prog\">in progress</span><span class=\"ms-count\">1 of 4 done</span>"
    ));
    // A cancelled task stays in its place, with its reason on the second line.
    assert!(html.contains("<li class=\"item drop\">"));
    assert!(html.contains("<b>Cancelled</b> \u{00B7} the club has no maps left in that format"));
}

#[test]
fn all_work_has_a_hidden_line_for_an_empty_view() {
    let tmp = TempDir::new().unwrap();
    let (_work, html) = fixture_report(&tmp);
    // The line sits after the tree, hidden, so the script can show it when a view matches nothing.
    let all = section(&html, "All work");
    assert!(
        all.trim_end()
            .ends_with("<p id=\"empty-view\" class=\"muted\" hidden></p>"),
        "{all}"
    );
}

#[test]
fn the_script_names_the_texts_of_an_empty_view() {
    let tmp = TempDir::new().unwrap();
    let (_work, html) = fixture_report(&tmp);
    // The page's own script is the last script element, and the texts of the empty view live there.
    let start = html.rfind("<script>").unwrap() + "<script>".len();
    let end = html.rfind("</script>").unwrap();
    let script = &html[start..end];
    assert!(script.contains("empty-view"), "{script}");
    assert!(script.contains("Nothing still to do."), "{script}");
    assert!(script.contains("Nothing done yet."), "{script}");
    assert!(script.contains("Nothing cancelled."), "{script}");
}

#[test]
fn the_second_line_names_each_waiting_and_claimed_state_in_words() {
    let tmp = TempDir::new().unwrap();
    let (_work, html) = fixture_report(&tmp);
    assert!(html.contains("<b>In progress</b> \u{00B7} ines \u{00B7} since yesterday"));
    assert!(html.contains("<b>In progress</b> \u{00B7} sam \u{00B7} for "));
    assert!(html.contains(" \u{00B7} <span class=\"stale\">no activity</span>"));
    assert!(html.contains("<b>Waiting</b> \u{00B7} waiting for the printer driver"));
    assert!(html.contains(
        "<b>Waiting</b> \u{00B7} given back by a worker: the plan puts one lantern too many on the east wall"
    ));
    assert!(html.contains("<b>Waiting</b> \u{00B7} for your answer: How many lanterns per room?"));
}

/// The part of `html` from the row of the milestone titled `title` to the end of the page.
fn section_from<'a>(html: &'a str, title: &str) -> &'a str {
    let start = html
        .find(&format!("<span class=\"ms-title\">{title}</span>"))
        .unwrap_or_else(|| panic!("no milestone {title:?}"));
    &html[start..]
}

#[test]
fn other_work_holds_the_tasks_below_no_milestone() {
    let tmp = TempDir::new().unwrap();
    let (_work, html) = fixture_report(&tmp);
    let other = section_from(&html, "Other work");
    assert!(other.contains("not part of a milestone"));
    in_order(other, &["T-030", "T-032", "T-033"]);
}

#[test]
fn more_holds_the_ideas_the_archive_and_the_planner_all_closed() {
    let tmp = TempDir::new().unwrap();
    let (_work, html) = fixture_report(&tmp);
    let more = &html[html.find("<h2>More</h2>").unwrap()..];
    assert!(more.contains(
        "<details class=\"fold\"><summary>Ideas (1) <span class=\"muted\">\u{2014} not planned yet</span></summary>"
    ));
    assert!(more.contains("<li>Show the lamp heat map</li>"));
    assert!(more.contains(
        "<details class=\"fold\"><summary>Earlier work (1 finished milestone, 2 tasks) <span class=\"muted\">\u{2014} from the archive</span></summary>"
    ));
    // The archive's tree is closed: its milestone and its tasks show, but nothing is open.
    assert!(more.contains("<span class=\"ms-title\">Lantern tool v0</span>"));
    assert!(more.contains("<span class=\"id\">T-041</span>"));
    assert!(more.contains("<span class=\"id\">T-042</span>"));
    assert!(!more.contains("<details class=\"ms\" open>"));
    assert!(more.contains(
        "<details class=\"fold\"><summary>For the planner <span class=\"muted\">\u{2014} levels, parallel waves, tasks that still need detail</span></summary>"
    ));
    assert!(!more.contains("<details class=\"fold\" open"));
    // The planner's part is the v0.8 planning state and the counts per area.
    assert!(more.contains("<section id=\"planning\">"));
    assert!(more.contains("Not ready for lack of detail (1)"));
    assert!(more.contains("Given back to the planner (1)"));
    assert!(more.contains("<section id=\"areas\">"));
    assert!(more.contains("<li><span>parser</span><span class=\"count\">2</span></li>"));
    // The planner's rows name the task and link to nothing: the page writes no entry anchors.
    assert!(!more.contains("href=\"#"));
}

#[test]
fn the_v0_8_sections_and_the_raw_field_lists_are_gone() {
    let tmp = TempDir::new().unwrap();
    let (_work, html) = fixture_report(&tmp);
    for gone in [
        "<nav class=\"jump\"",
        "<h2>Done (",
        "<h2>Dropped (",
        "<h2>Open questions (",
        "<h2>Ideas (",
        "class=\"facts\"",
        "<progress",
        "<ul class=\"counts\"",
        "class=\"chip stale\"",
    ] {
        assert!(!html.contains(gone), "the page still has {gone:?}");
    }
}

#[test]
fn an_archive_is_shown_under_earlier_work_and_one_without_it_is_left_out() {
    let body = format!(
        "{FIXTURE_MARKER}<!-- todo-format: 2 -->\n# Archive check\n\n## Now\n\n\
### T-001 · Open task\n- status: open\n- owner: —\n"
    );
    let archive = format!(
        "{FIXTURE_MARKER}<!-- todo-format: 2 -->\n# Archive check\n\n## Archive\n\n\
### T-002 · Archived done task\n- status: done\n- owner: —\n- note (2026-11-01, alice): done\n\n\
### T-003 · Archived dropped task\n- status: dropped\n- owner: —\n\
- note (2026-11-02, alice): dropped — no longer wanted\n"
    );
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo) = setup_proj(&tmp, &body);
    fs::write(work.join("TODO-archive.md"), archive).unwrap();

    let out = report(&home, &work, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    let html = default_html(&work);
    let earlier = &html[html.find("Earlier work (").unwrap()..];
    assert!(earlier.starts_with(
        "Earlier work (0 finished milestones, 2 tasks) <span class=\"muted\">\u{2014} from the archive</span>"
    ));
    assert!(earlier.contains("Archived done task"));
    assert!(earlier.contains("<b>Cancelled</b> \u{00B7} no longer wanted"));

    // Without an archive, the closed part is not there.
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo) = setup_proj(&tmp, &body);
    assert_eq!(report(&home, &work, &[]).status.code(), Some(0));
    assert!(!default_html(&work).contains("Earlier work"));
}

#[test]
fn text_from_the_task_list_is_escaped() {
    let body = format!(
        "{FIXTURE_MARKER}<!-- todo-format: 2 -->\n# Harbour <i>lights</i>\n\n## Now\n\n\
### T-001 · <script>alert(\"x\")</script>\n\
- status: open\n\
- owner: —\n\
- priority: P1 · area: <b>ui</b>\n\n\
### T-002 · <i>Milestone</i>\n\
- status: blocked\n\
- owner: —\n\
- priority: P1 · type: milestone\n\
- goal: <img src=x onerror=alert(1)> & more\n\
- note (2026-11-18, bob): blocked — <img src=x onerror=alert(1)> & more\n"
    );
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo) = setup_proj(&tmp, &body);
    let out = report(&home, &work, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    let html = default_html(&work);

    assert!(html.contains("&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt;"));
    assert!(html.contains("&lt;img src=x onerror=alert(1)&gt; &amp; more"));
    assert!(html.contains("Harbour &lt;i&gt;lights&lt;/i&gt;"));
    // The area of the open task is shown in its technical details, escaped.
    assert!(html.contains("<dt>Area</dt><dd>&lt;b&gt;ui&lt;/b&gt;</dd>"));
    // The page's own script is the only script element; the one in the title is escaped above.
    assert_eq!(html.matches("<script").count(), 1);
    assert!(!html.contains("<script>alert"));
    assert!(!html.contains("<img"));
    assert!(!html.contains("<i>"));
    assert!(!html.contains("<b>ui</b>"));
}

#[test]
fn ideas_show_text_between_backticks_as_code() {
    let body = format!(
        "{FIXTURE_MARKER}<!-- todo-format: 3 -->\n# Title\n\n## Ideas\n\n\
### T-001 · Try `todo add --template bug`\n\
- status: open\n\
- owner: —\n\
- type: idea\n"
    );
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo) = setup_proj(&tmp, &body);
    let out = report(&home, &work, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    let html = default_html(&work);

    assert!(html.contains("<code>todo add --template bug</code>"));
    assert!(!html.contains("`todo add --template bug`"));
}

#[test]
fn page_loads_nothing_from_the_network_and_its_buttons_are_hidden_without_js() {
    let tmp = TempDir::new().unwrap();
    let (_work, html) = fixture_report(&tmp);

    for banned in [
        "<link",
        "<img",
        "<iframe",
        "<object",
        "<embed",
        "<form",
        "<input",
        "<select",
        "<textarea",
        "@import",
        "url(",
        "src=",
        "http://",
        "https://",
    ] {
        assert!(!html.contains(banned), "the page contains {banned:?}");
    }
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("<style>"));
    // The buttons are there, and the stylesheet hides them until the page has the class `js`.
    assert!(html.contains("<div class=\"controls\">"));
    assert!(html.contains(".page:not(.js) .controls{display:none}"));
}

/// The page with its line endings as LF, so that the checks below read the same on every checkout.
fn lf(html: &str) -> String {
    html.replace("\r\n", "\n")
}

#[test]
fn the_page_has_one_inline_script_that_names_no_address_and_makes_no_network_call() {
    let tmp = TempDir::new().unwrap();
    let (_work, html) = fixture_report(&tmp);
    let html = lf(&html);
    assert_eq!(html.matches("<script").count(), 1, "one script element");
    assert_eq!(html.matches("</script>").count(), 1, "one closing tag");

    let open = "<script>\n";
    let start = html.find(open).unwrap() + open.len();
    let end = html.find("</script>").unwrap();
    let script = &html[start..end];
    for banned in [
        "http",
        "//",
        "fetch",
        "XMLHttpRequest",
        "import",
        "WebSocket",
        "sendBeacon",
        "eval(",
        "document.write",
    ] {
        assert!(!script.contains(banned), "the script contains {banned:?}");
    }
    // The first thing the script does is to mark the page as scripted, so the buttons appear.
    let marked = script.find("page.classList.add('js')").unwrap();
    assert!(script.find("addEventListener").unwrap() > marked);
    // Both choices are remembered under their keys, and every storage call is inside a try block.
    assert!(script.contains("'todo-report-theme'"));
    assert!(script.contains("'todo-report-cb'"));
    assert_eq!(script.matches("window.localStorage.").count(), 2);
    assert!(script.contains("try {\n      return window.localStorage.getItem(key);"));
    assert!(script.contains("try {\n      window.localStorage.setItem(key, value);"));
    // Without a choice the mode follows the system setting.
    assert!(script.contains("(prefers-color-scheme: dark)"));
}

#[test]
fn the_view_and_mode_buttons_are_in_order_and_left_out_when_printed() {
    let tmp = TempDir::new().unwrap();
    let (_work, html) = fixture_report(&tmp);
    let html = lf(&html);
    in_order(
        &html,
        &[
            "<button id=\"theme\" type=\"button\">Dark mode</button>",
            "<button id=\"cb\" type=\"button\" aria-pressed=\"false\"",
            "<button data-f=\"all\" aria-pressed=\"true\">Everything</button>",
            "<button data-f=\"left\" aria-pressed=\"false\">Still to do</button>",
            "<button data-f=\"done\" aria-pressed=\"false\">Done</button>",
            "<button data-f=\"drop\" aria-pressed=\"false\">Cancelled</button>",
            "<button id=\"exp\" type=\"button\">Expand all</button>",
            "<button id=\"col\" type=\"button\">Collapse all</button>",
            "<div id=\"tree\">",
        ],
    );
    assert!(html.contains("Colour-blind colours</button>"));
    assert!(html.contains("@media print{ .controls{display:none}"));
}

#[test]
fn the_colours_the_layout_and_the_print_rules_are_in_the_stylesheet() {
    let tmp = TempDir::new().unwrap();
    let (_work, html) = fixture_report(&tmp);
    // Light values of R-REPORT-6, the dark set for the system setting and for the chosen mode.
    assert!(html.contains("--c-wait:#b07800"));
    assert!(html.contains("--c-todo:#80868e"));
    assert!(html.contains("@media (prefers-color-scheme: dark)"));
    assert!(html.contains(".page:not([data-theme]){"));
    assert!(html.contains(".page[data-theme=\"dark\"]{"));
    assert!(html.contains(".page[data-cb=\"on\"]{"));
    // One column of at most 820 px, narrow windows and print.
    assert!(html.contains("max-width:820px"));
    assert!(html.contains("@media (max-width:560px)"));
    assert!(html.contains("@media print"));
    // The state classes of the milestone rows and of a stale claim.
    for class in [
        ".st-prog{",
        ".st-none{",
        ".st-done{",
        ".st-drop{",
        ".stale{",
    ] {
        assert!(html.contains(class), "{class}");
    }
    // The mockup's host fallbacks and its banner are not in the page.
    assert!(!html.contains("var(--background"));
    assert!(!html.contains("mockflag"));
}

#[test]
fn out_writes_to_the_given_path_with_spaces_and_non_ascii_and_not_to_the_state_folder() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo) = setup_fixture_proj(&tmp, "report_basic.md");
    let folder = work.join("Tâche finale");
    fs::create_dir_all(&folder).unwrap();
    let target = folder.join("mes tâches.html");

    let out = report(&home, &work, &["--out", target.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    assert_eq!(stdout_text(&out).trim_end(), target.display().to_string());
    let html = fs::read_to_string(&target).unwrap();
    assert!(html.starts_with("<!doctype html>"));
    assert!(!default_page(&work).exists());
}

#[test]
fn out_into_a_missing_folder_fails_and_writes_nothing() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo) = setup_fixture_proj(&tmp, "report_basic.md");
    let target = work.join("no such folder").join("tasks.html");
    let target_text = target.to_str().unwrap();

    let out = run_todo(
        &home,
        &work,
        &["--json", "report", "--html", "--out", target_text],
        &[("TODO_TEST_NOW", NOW)],
    )
    .unwrap();
    assert_ne!(out.status.code(), Some(0));
    let json = parse_json(&out.stdout);
    assert_eq!(json["ok"], false);
    assert_eq!(json["error"]["code"], "WRITE_FAILED");
    assert!(!target.exists());
}

#[test]
fn report_without_html_is_a_usage_error() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo) = setup_fixture_proj(&tmp, "report_basic.md");
    let out = run_todo(&home, &work, &["report"], &[]).unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr_text(&out).contains("--html"),
        "{}",
        stderr_text(&out)
    );
    assert!(!default_page(&work).exists());
}

#[test]
fn json_reports_the_path_the_size_and_whether_the_page_was_opened() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo) = setup_fixture_proj(&tmp, "report_basic.md");
    let out = run_todo(
        &home,
        &work,
        &["--json", "report", "--html"],
        &[("TODO_TEST_NOW", NOW)],
    )
    .unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    let json = parse_json(&out.stdout);
    assert_eq!(json["schema"], "todo.report.v1");
    assert_eq!(json["data"]["opened"], false);
    assert!(
        json["data"]["path"]
            .as_str()
            .unwrap()
            .ends_with("report.html")
    );
    assert_eq!(
        json["data"]["bytes"].as_u64().unwrap(),
        fs::metadata(default_page(&work)).unwrap().len()
    );
}

#[test]
fn report_takes_no_lock_even_while_the_write_lock_is_held() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo) = setup_fixture_proj(&tmp, "report_basic.md");
    let state = work.join(".todo");
    fs::create_dir_all(&state).unwrap();
    let lock = File::create(state.join("write.lock")).unwrap();
    lock.try_lock().expect("the test holds the write lock");

    let started = Instant::now();
    let out = report(&home, &work, &[]);
    let waited = started.elapsed();
    drop(lock);

    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    assert!(
        waited < Duration::from_secs(20),
        "report waited {waited:?} for the lock"
    );
}

#[test]
fn an_empty_list_says_what_is_missing_in_words() {
    let body = format!("{FIXTURE_MARKER}<!-- todo-format: 2 -->\n# Empty list\n");
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo) = setup_proj(&tmp, &body);
    let out = report(&home, &work, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    let html = default_html(&work);
    assert!(html.contains("<h1>Empty list</h1>"));
    assert!(html.contains("<p class=\"big\">0 of 0 tasks done</p>"));
    assert!(html.contains("Nothing needs you."));
    assert!(html.contains("Nothing changed in the last 7 days."));
    assert!(html.contains("No tasks or milestones in TODO.md yet."));
    assert!(!html.contains("Ideas ("));
    assert!(!html.contains("Earlier work"));
    assert!(html.contains("Every task at or below the target level has its detail."));
    assert!(html.contains("Nothing given back."));
}
