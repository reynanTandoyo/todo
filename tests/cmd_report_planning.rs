//! CLI tests for the planning state of the HTML report (T-164; R-REPORT-3, R-LEVEL-2, R-WAVE-1): the
//! open tasks per level, the tasks not ready for lack of detail, the tasks given back with their
//! attempts, the tasks kept above the target level, the waves, and what an opened task shows. The
//! counts must be the ones `todo plan` and `todo waves` give. Every fixture is invented (TEST-4).

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use common::{parse_json, run_todo, setup_proj, stderr_text};
use serde_json::Value;
use tempfile::TempDir;

/// The clock every test runs with, so the page and its claim ages do not depend on the day.
const NOW: &str = "2026-11-20T10:00:00+01:00";

/// An invented task list with every planning state: a level-1 task whose detail is complete, a
/// level-1 task that lacks detail, a blocked task given back twice, a level-3 task kept above the
/// target level with a reason that holds HTML, a task without a level, and a done task with a passed
/// check.
const PLANNING: &str = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 3 -->
# Planning fixture

## Work

### T-001 · Renderer
- status: open
- owner: —
- priority: P1 · type: milestone
- goal: the scene renders

### T-002 · Parse the config
- status: open
- owner: —
- priority: P1 · difficulty: 1
- milestone: T-001
- files: src/config.rs
- why: the loader needs the values
- accept: config tests pass
- steps:
  1. open the parser
- verify: cargo test config

### T-003 · Load the config
- status: open
- owner: —
- priority: P1 · difficulty: 1
- milestone: T-001
- files: src/loader.rs
- depends: T-002
- why: the program reads the file

### T-004 · Write the schema
- status: blocked
- owner: —
- priority: P2 · difficulty: 2
- attempts: 2
- note (2026-11-19, bob): stuck — the schema file does not exist

### T-005 · Render the report
- status: open
- owner: —
- priority: P2 · difficulty: 3
- final: needs the <b>layout</b> decision

### T-006 · Pick a colour theme
- status: open
- owner: —
- priority: P3

### T-007 · Ship the parser
- status: done
- owner: alice
- priority: P1 · difficulty: 1
- files: src/config.rs
- verify: cargo test config
- verified: 2026-11-20
- commit: 1a2b3c4d
- note (2026-11-20, alice): done — the parser reads every key
";

/// Sets up the project with [`PLANNING`] and builds its page with the fixed clock. Returns the home
/// and work folders, and the page as text.
fn built(tmp: &TempDir) -> (PathBuf, PathBuf, String) {
    let (home, work, _file) = setup_proj(tmp, PLANNING);
    let out = report(&home, &work);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    let html = fs::read_to_string(work.join(".todo").join("report.html")).expect("read the page");
    (home, work, html)
}

/// Runs `todo report --html` in the project with the fixed clock.
fn report(home: &Path, work: &Path) -> Output {
    run_todo(home, work, &["report", "--html"], &[("TODO_TEST_NOW", NOW)]).expect("run todo report")
}

/// Runs `todo --json <args>` in the project and returns the parsed envelope. `plan` exits 1 while its
/// list is not empty (R-LEVEL-2), so the exit code is not checked here; the envelope is.
fn json(home: &Path, work: &Path, args: &[&str]) -> Value {
    let mut all = vec!["--json"];
    all.extend_from_slice(args);
    let out = run_todo(home, work, &all, &[]).expect("run todo");
    let envelope = parse_json(&out.stdout);
    assert_eq!(envelope["ok"], true, "stderr: {}", stderr_text(&out));
    envelope
}

/// The planning section of the page: from its anchor to its closing tag.
fn planning_section(html: &str) -> &str {
    let start = html
        .find("<section id=\"planning\">")
        .expect("the page has a planning section");
    let end = html[start..]
        .find("</section>")
        .expect("the planning section closes");
    &html[start..start + end]
}

/// The row of the entry `id` in All work, from its ID to the end of its row. A row closes with
/// `</li>` and a line break; the notes of an opened row close with `</li>` before `</ul>`.
fn row_of<'a>(html: &'a str, id: &str) -> &'a str {
    let needle = format!("<span class=\"id\">{id}</span>");
    assert_eq!(html.matches(&needle).count(), 1, "{id} has one row");
    let start = html.find(&needle).expect("the entry has a row");
    let end = html[start..].find("</li>\n").expect("the row closes");
    &html[start..start + end]
}

#[test]
fn the_planning_section_counts_what_the_planner_counts() {
    let tmp = TempDir::new().expect("tempdir");
    let (_home, _work, html) = built(&tmp);
    let planning = planning_section(&html);

    // Open, claimed or blocked tasks by level: two at level 1, one at 2, one at 3, one without a level.
    assert!(
        planning.contains("<h3>Open tasks per level</h3>"),
        "{planning}"
    );
    assert!(
        planning.contains("<li><span>1</span><span class=\"count\">2</span></li>"),
        "{planning}"
    );
    assert!(
        planning.contains("<li><span>2</span><span class=\"count\">1</span></li>"),
        "{planning}"
    );
    assert!(
        planning.contains("<li><span>3</span><span class=\"count\">1</span></li>"),
        "{planning}"
    );
    assert!(
        planning.contains(
            "<li><span class=\"muted\">no level</span><span class=\"count\">1</span></li>"
        ),
        "{planning}"
    );

    // T-003 is at the target level and lacks its accept line, steps and check (R-DETAIL-2).
    assert!(
        planning.contains("Not ready for lack of detail (1)"),
        "{planning}"
    );
    assert!(
        planning.contains("missing: accept, steps, verify"),
        "{planning}"
    );

    // T-004 is given back with its problem and its two attempts (R-STUCK-1).
    assert!(
        planning.contains("Given back to the planner (1)"),
        "{planning}"
    );
    assert!(
        planning.contains("the schema file does not exist"),
        "{planning}"
    );
    assert!(planning.contains("attempts 2"), "{planning}");

    // T-005 stays above the target level with its final reason (R-LEVEL-3).
    assert!(
        planning.contains("Kept above the target level on purpose (1)"),
        "{planning}"
    );

    // T-002 and T-005 are wave 1, in two lanes; T-003 waits for T-002, so there are two waves.
    assert!(
        planning.contains("<p>2 waves in all. Wave 1 holds 2 tasks in 2 lanes.</p>"),
        "{planning}"
    );
}

#[test]
fn the_planning_counts_are_the_ones_todo_plan_and_todo_waves_give() {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work, html) = built(&tmp);

    // The commands give the numbers the page prints.
    let plan = json(&home, &work, &["plan"]);
    let plan_data = &plan["data"];
    assert_eq!(
        plan_data["to_detail"].as_array().map(Vec::len),
        Some(1),
        "{plan}"
    );
    assert_eq!(
        plan_data["given_back"].as_array().map(Vec::len),
        Some(1),
        "{plan}"
    );
    assert_eq!(
        plan_data["kept_on_purpose"].as_array().map(Vec::len),
        Some(1),
        "{plan}"
    );
    let waves = json(&home, &work, &["waves"]);
    let wave_data = &waves["data"];
    assert_eq!(wave_data["wave_count"], 2, "{waves}");
    assert_eq!(wave_data["waves"][0]["size"]["tasks"], 2, "{waves}");
    assert_eq!(wave_data["waves"][0]["size"]["lanes"], 2, "{waves}");

    // The page says the same.
    let planning = planning_section(&html);
    assert!(
        planning.contains("Not ready for lack of detail (1)"),
        "{planning}"
    );
    assert!(
        planning.contains("Given back to the planner (1)"),
        "{planning}"
    );
    assert!(
        planning.contains("Kept above the target level on purpose (1)"),
        "{planning}"
    );
    assert!(
        planning.contains("<p>2 waves in all. Wave 1 holds 2 tasks in 2 lanes.</p>"),
        "{planning}"
    );
}

#[test]
fn an_opened_task_shows_its_level_files_check_and_whether_it_was_verified() {
    let tmp = TempDir::new().expect("tempdir");
    let (_home, _work, html) = built(&tmp);

    // The technical details of an opened task give its level, its files and its check (R-REPORT-3).
    let t002 = row_of(&html, "T-002");
    assert!(t002.contains("<dt>Level</dt><dd>1</dd>"), "{t002}");
    assert!(
        t002.contains("<dt>Files</dt><dd>src/config.rs</dd>"),
        "{t002}"
    );
    // A check that has not passed yet is shown without a date.
    assert!(
        t002.contains("<dt>Check</dt><dd>cargo test config</dd>"),
        "{t002}"
    );
    assert!(!t002.contains("passed"), "{t002}");

    // A done task that passed its check shows the date it passed.
    let t007 = row_of(&html, "T-007");
    assert!(
        t007.contains("<dt>Check</dt><dd>cargo test config \u{2014} passed 20 Nov</dd>"),
        "{t007}"
    );

    // A task with no level, files or check shows none of those rows.
    let t006 = row_of(&html, "T-006");
    for label in ["Level", "Files", "Check"] {
        assert!(
            !t006.contains(&format!("<dt>{label}</dt>")),
            "{label} in {t006}"
        );
    }
}

#[test]
fn text_from_the_planning_state_is_escaped_and_the_section_has_no_control() {
    let tmp = TempDir::new().expect("tempdir");
    let (_home, _work, html) = built(&tmp);
    let planning = planning_section(&html);

    assert!(
        planning.contains("needs the &lt;b&gt;layout&lt;/b&gt; decision"),
        "{planning}"
    );
    assert!(
        !html.contains("<b>layout</b>"),
        "the reason was not escaped"
    );
    for control in [
        "<button",
        "<form",
        "<input",
        "<select",
        "<textarea",
        "<script",
        "onclick",
    ] {
        assert!(
            !planning.contains(control),
            "{control} in the planning section"
        );
    }
}

#[test]
fn a_task_given_back_or_not_ready_is_named_in_the_planner_and_has_its_row_in_all_work() {
    let tmp = TempDir::new().expect("tempdir");
    let (_home, _work, html) = built(&tmp);
    let planning = planning_section(&html);
    // The planner names each task by its ID and title. The page writes no anchor for a row, so the
    // name is not a link.
    assert!(
        planning.contains("<span>T-003 \u{00B7} Load the config</span>"),
        "{planning}"
    );
    assert!(
        planning.contains("<span>T-004 \u{00B7} Write the schema</span>"),
        "{planning}"
    );
    assert!(
        planning.contains("<span>T-005 \u{00B7} Render the report</span>"),
        "{planning}"
    );
    assert!(!planning.contains("href="), "{planning}");
    // Each of them has exactly one row in All work.
    for id in ["T-003", "T-004", "T-005"] {
        assert_eq!(
            html.matches(&format!("<span class=\"id\">{id}</span>"))
                .count(),
            1,
            "{id} should have one row"
        );
    }
}

#[test]
fn the_planner_part_is_the_last_closed_part_of_more() {
    let tmp = TempDir::new().expect("tempdir");
    let (_home, _work, html) = built(&tmp);
    let at = |text: &str| {
        html.find(text)
            .unwrap_or_else(|| panic!("{text} is missing"))
    };
    let order = [
        at("<h2>Needs you ("),
        at("<h2>Last 7 days</h2>"),
        at("<h2>All work</h2>"),
        at("<h2>More</h2>"),
        at("<summary>For the planner"),
        at("<section id=\"planning\">"),
    ];
    assert!(
        order.windows(2).all(|pair| pair[0] < pair[1]),
        "the sections are out of order: {order:?}"
    );
    // The part is closed by default.
    assert!(!html.contains("<details class=\"fold\" open"));
}

#[test]
fn a_project_without_unfinished_tasks_says_so() {
    let body = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 3 -->
# Empty fixture

## Work

### T-001 · Finished
- status: done
- owner: alice
- priority: P1 · difficulty: 1
- commit: 1a2b3c4d
- note (2026-11-20, alice): done
";
    let tmp = TempDir::new().expect("tempdir");
    let (home, work, _file) = setup_proj(&tmp, body);
    assert_eq!(report(&home, &work).status.code(), Some(0));
    let html = fs::read_to_string(work.join(".todo").join("report.html")).expect("read the page");
    let planning = planning_section(&html);
    assert!(
        planning.contains("No unfinished task, so no waves."),
        "{planning}"
    );
    assert!(
        planning.contains("Every task at or below the target level has its detail."),
        "{planning}"
    );
    assert!(planning.contains("Nothing given back."), "{planning}");
    assert!(
        planning.contains("Nothing kept above the target level."),
        "{planning}"
    );
}
