//! Tests for the `summary` warning of `check` and for `brief` (T-210; R-WRITE-19, R-CHECK-1): `check`
//! warns, without changing its exit code, about each task and milestone that is not done or dropped
//! and has no `summary`; a question, an idea, a done or a dropped entry is not warned about; `brief`
//! does not print the summary. The fixtures are invented.

mod common;

use tempfile::TempDir;

use common::{parse_json, read_todo, run_todo, setup_proj, stderr_text, stdout_text};

/// The header every fixture here starts with (AGENTS.md "Test rules"), then the `## Tasks` section.
const HEADER: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
<!-- todo-format: 3 -->\n\
## Tasks\n\n";

/// The 1-based line of the heading of task `id` in `text`.
fn heading_line(text: &str, id: &str) -> usize {
    let prefix = format!("### {id} ");
    text.lines()
        .position(|line| line.starts_with(&prefix))
        .unwrap_or_else(|| panic!("no heading for {id}"))
        + 1
}

/// The warning `check` prints on standard error for task or milestone `id` of `text`, which has no
/// summary: `warning: file:line: message` (`Output::warn`).
fn no_summary_warning(text: &str, id: &str) -> String {
    format!(
        "warning: TODO.md:{}: {id}: no summary: write one or two plain sentences for people with todo set {id} --summary",
        heading_line(text, id)
    )
}

/// The lines of `stderr` that are the no-summary warning.
fn no_summary_lines(stderr: &str) -> Vec<&str> {
    stderr
        .lines()
        .filter(|line| line.contains("no summary"))
        .collect()
}

#[test]
fn check_warns_about_open_task_and_milestone_without_summary_and_exits_0() {
    let body = format!(
        "{HEADER}\
### T-001 · Render the first frame\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: feature · area: renderer\n\
- why: the first frame shows that the pipeline runs\n\
- accept: a frame is written to disk\n\n\
### T-002 · Lighting\n\
- status: open\n\
- priority: P1 · type: milestone\n\
- goal: the scene renders with lighting\n\n\
### T-003 · Write the readme\n\
- status: open\n\
- owner: —\n\
- priority: P2 · type: feature · area: docs\n\
- why: a new reader needs a start\n\
- summary: The readme that tells a new reader how to start.\n\
- accept: the readme has a start section\n"
    );
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, &body);

    let out = run_todo(&home, &work, &["check"], &[]).unwrap();
    // A warning does not change the exit code: the file has no problem, so `check` exits 0.
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    let stderr = stderr_text(&out);
    assert_eq!(
        no_summary_lines(&stderr),
        vec![
            no_summary_warning(&body, "T-001").as_str(),
            no_summary_warning(&body, "T-002").as_str(),
        ],
        "{stderr}"
    );
    // T-003 has a summary, so it is not reported.
    assert!(!stderr.contains("T-003: no summary"), "{stderr}");
}

#[test]
fn check_warns_about_claimed_and_blocked_tasks_without_summary() {
    let body = format!(
        "{HEADER}\
### T-001 · Waiting for the font\n\
- status: blocked\n\
- owner: —\n\
- priority: P2 · type: feature\n\
- why: the labels need the font\n\
- accept: the labels are drawn\n\
- note (2026-11-15, alice): blocked — waiting on the font file\n\n\
### T-002 · Draw the grid\n\
- status: open\n\
- owner: —\n\
- priority: P2 · type: feature\n\
- why: the grid shows the scale\n\
- accept: the grid is drawn\n"
    );
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, &body);

    // Claiming T-002 makes it claimed; the claim is fresh, so `check` reports no stale claim.
    let claim = run_todo(&home, &work, &["--agent", "alice", "claim", "T-002"], &[]).unwrap();
    assert_eq!(claim.status.code(), Some(0), "{}", stderr_text(&claim));
    let text = read_todo(&work);
    assert!(text.contains("- status: claimed"), "{text}");

    let out = run_todo(&home, &work, &["check"], &[]).unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    let stderr = stderr_text(&out);
    assert_eq!(
        no_summary_lines(&stderr),
        vec![
            no_summary_warning(&text, "T-001").as_str(),
            no_summary_warning(&text, "T-002").as_str(),
        ],
        "{stderr}"
    );
}

#[test]
fn check_does_not_warn_about_done_dropped_questions_or_ideas_without_summary() {
    let body = format!(
        "{HEADER}\
### T-001 · Shipped already\n\
- status: done\n\
- owner: —\n\
- priority: P1 · type: feature · area: cli\n\
- why: the reader needed it first\n\
- accept: the command prints the list\n\n\
### T-002 · Not needed after all\n\
- status: dropped\n\
- owner: —\n\
- priority: P2 · type: feature\n\
- note (2026-11-15, alice): dropped — the plan changed\n\n\
### T-003 · Renderer\n\
- status: done\n\
- priority: P1 · type: milestone\n\
- goal: the renderer draws a frame\n\n\
### T-004 · Are shadows in scope?\n\
- status: open\n\
- owner: —\n\
- priority: P2 · type: question\n\n\
### T-005 · Fog between the trees\n\
- status: open\n\
- owner: —\n\
- priority: P3 · type: idea\n"
    );
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, &body);

    let out = run_todo(&home, &work, &["check"], &[]).unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    let stderr = stderr_text(&out);
    assert!(no_summary_lines(&stderr).is_empty(), "{stderr}");
}

#[test]
fn brief_does_not_print_the_summary() {
    let body = format!(
        "{HEADER}\
### T-001 · Render the first frame\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: feature · area: renderer\n\
- why: the first frame shows that the pipeline runs\n\
- summary: Plain words for people, with no ID in them.\n\
- accept: a frame is written to disk\n"
    );
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, &body);

    // The text work order prints the why, and not the summary.
    let text = run_todo(&home, &work, &["brief", "T-001"], &[]).unwrap();
    assert_eq!(text.status.code(), Some(0), "{}", stderr_text(&text));
    let printed = stdout_text(&text);
    assert!(printed.contains("why: the first frame shows"), "{printed}");
    assert!(!printed.contains("Plain words for people"), "{printed}");
    assert!(!printed.contains("summary"), "{printed}");

    // The JSON work order has the same parts: no summary key, and no summary text.
    let json = run_todo(&home, &work, &["--json", "brief", "T-001"], &[]).unwrap();
    assert_eq!(json.status.code(), Some(0), "{}", stderr_text(&json));
    let raw = stdout_text(&json);
    assert!(!raw.contains("Plain words for people"), "{raw}");
    let parsed = parse_json(&json.stdout);
    assert!(parsed["data"].get("summary").is_none(), "{parsed}");
}
