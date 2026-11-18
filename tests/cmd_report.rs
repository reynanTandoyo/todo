//! CLI integration tests for `todo report --html` (T-092; R-REPORT-1, R-LOCK-5): the page it writes,
//! where it writes it, what it shows and what it leaves alone. Every fixture is invented.

mod common;

use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::{Duration, Instant};

use tempfile::TempDir;

use common::{
    fixture, parse_json, run_todo, setup_fixture_proj, setup_proj, stderr_text, stdout_text,
};

const FIXTURE_MARKER: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->\n";

/// The clock every test runs with, so the page and its claim ages do not depend on the day.
const NOW: &str = "2026-11-19T10:00:00+01:00";

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

/// The position of the row of `id` in `html`, which must be there once.
fn row_at(html: &str, id: &str) -> usize {
    let needle = format!("id=\"e-{id}\"");
    assert_eq!(html.matches(&needle).count(), 1, "{id} should have one row");
    html.find(&needle).unwrap()
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
    let (home, work, _todo) = setup_fixture_proj(&tmp, "report_basic.md");
    let out = report(&home, &work, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    insta::assert_snapshot!("report_html", default_html(&work));
}

#[test]
fn page_shows_the_work_the_lists_and_the_facts_the_accept_line_asks_for() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo) = setup_fixture_proj(&tmp, "report_basic.md");
    assert_eq!(report(&home, &work, &[]).status.code(), Some(0));
    let html = default_html(&work);

    // The work is a tree: milestones with their tasks indented below, top-level tasks last.
    let order = [
        "T-001", "T-002", "T-003", "T-006", "T-004", "T-005", "T-015",
    ];
    let positions: Vec<usize> = order.iter().map(|id| row_at(&html, id)).collect();
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "work rows out of order: {order:?}"
    );

    // A milestone shows its goal and its progress over the tasks below it (not the dropped one).
    assert!(html.contains("the test scene renders with lighting at 60 frames per second"));
    assert!(html.contains("<progress value=\"1\" max=\"4\">1 of 4 done</progress>"));

    // Priority, difficulty and area are shown on each task.
    assert!(html.contains("<span class=\"chip\" title=\"priority\">P1</span>"));
    assert!(html.contains("<span class=\"chip\" title=\"difficulty\">hard</span>"));
    assert!(html.contains("<span class=\"chip\" title=\"area\">gpu</span>"));

    // A claim shows its owner and age; a stale claim is marked.
    assert!(html.contains("claimed by alice, 2 h"));
    assert!(html.contains("claimed by carol, 216 h"));
    assert!(html.contains("<span class=\"chip stale\">stale</span>"));

    // A blocked task shows its latest reason, not an earlier note.
    assert!(html.contains(
        "<span class=\"reason\">blocked: the vendor driver crashes on the test scene</span>"
    ));
    assert!(!html.contains("blocked: waiting for the driver update"));

    // Unfinished tasks per area; the done task T-011 is not counted, the dropped T-014 is not either.
    assert!(html.contains("<li><span>docs</span><span class=\"count\">2</span></li>"));
    assert!(html.contains("<li><span>gpu</span><span class=\"count\">1</span></li>"));
    assert!(html.contains("<li><span>ui</span><span class=\"count\">2</span></li>"));

    // Open questions link to the entry they are about; a missing entry is named but not linked.
    assert!(html.contains("about <a href=\"#e-T-002\">T-002 · Lighting</a>"));
    assert!(html.contains("about <a href=\"#e-T-001\">T-001 · Develop the renderer</a>"));
    assert!(html.contains("about T-099</span>"));
    assert!(!html.contains("href=\"#e-T-099\""));

    // Ideas, done and dropped lists. The done task and the dropped tasks below a milestone are
    // shown once, in their list; the reason of each drop is shown.
    assert!(html.contains("Hot-reload shaders while the scene runs"));
    assert!(
        html.contains("<span class=\"reason dropped\">dropped: replaced by the C shader</span>")
    );
    assert!(html.contains("dropped: the scene no longer needs it"));
    assert!(html.contains("Set up the test scene"));
    assert!(html.contains("<h2>Done (2)</h2>"));
    assert!(html.contains("<h2>Dropped (2)</h2>"));

    // Light and dark colours, and a narrow layout, are in the inline style.
    assert!(html.contains("prefers-color-scheme: dark"));
    assert!(html.contains("max-width: 600px"));
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
    assert!(!html.contains("<script"));
    assert!(!html.contains("<img"));
    assert!(!html.contains("<i>"));
    assert!(!html.contains("<b>"));
}

#[test]
fn page_has_no_script_no_network_load_and_no_controls() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo) = setup_fixture_proj(&tmp, "report_basic.md");
    assert_eq!(report(&home, &work, &[]).status.code(), Some(0));
    let html = default_html(&work);

    for banned in [
        "<script",
        "<link",
        "<img",
        "<iframe",
        "<object",
        "<embed",
        "<form",
        "<button",
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
fn archived_done_and_dropped_entries_are_listed_and_marked_archived() {
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
    assert!(html.contains("TODO.md and TODO-archive.md"));
    assert!(html.contains("Archived done task"));
    assert!(html.contains("dropped: no longer wanted"));
    assert_eq!(
        html.matches("<span class=\"chip\">archived</span>").count(),
        2
    );
    assert!(html.contains("<h2>Done (1)</h2>"));
    assert!(html.contains("<h2>Dropped (1)</h2>"));
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
    assert!(html.contains("Nothing unfinished."));
    assert!(html.contains("No unfinished tasks."));
    assert!(html.contains("No open questions."));
    assert!(html.contains("No ideas waiting."));
    assert!(html.contains("Nothing done yet."));
    assert!(html.contains("Nothing dropped."));
}
