//! `todo check` reports a blocked task that has no block-reason note, at the task's line (T-142;
//! R-CHECK-1). The reason is the one `Task::blocked_reason` reads, as `summary` and the report show
//! it. The fixture is invented.

mod common;

use std::fs;

use tempfile::TempDir;

use common::{run_todo, setup_fixture_proj, stdout_text};

/// The 1-based line of the heading of task `id` in `text`.
fn heading_line(text: &str, id: &str) -> usize {
    let prefix = format!("### {id} ");
    text.lines()
        .position(|line| line.starts_with(&prefix))
        .unwrap_or_else(|| panic!("no heading for {id}"))
        + 1
}

#[test]
fn check_reports_blocked_tasks_without_a_block_reason_note_at_their_line() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo) = setup_fixture_proj(&tmp, "check_blocked_reason.md");
    let text = fs::read_to_string(work.join("TODO.md")).unwrap();

    let out = run_todo(&home, &work, &["check"], &[]).unwrap();
    // A problem, like the other task state problems: `check` exits 1 when it finds any.
    assert_eq!(out.status.code(), Some(1), "{}", stdout_text(&out));
    let stdout = stdout_text(&out);

    let reported: Vec<&str> = stdout
        .lines()
        .filter(|line| line.contains("blocked task has no reason note"))
        .collect();
    let expected: Vec<String> = ["T-002", "T-003", "T-004", "T-005"]
        .iter()
        .map(|id| {
            format!(
                "TODO.md:{}: {id}: blocked task has no reason note",
                heading_line(&text, id)
            )
        })
        .collect();
    let expected: Vec<&str> = expected.iter().map(String::as_str).collect();
    assert_eq!(reported, expected, "{stdout}");
}

#[test]
fn a_blocked_task_with_a_reason_is_not_reported() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo) = setup_fixture_proj(&tmp, "check_blocked_reason.md");
    let out = run_todo(&home, &work, &["check"], &[]).unwrap();
    let stdout = stdout_text(&out);
    for id in ["T-001", "T-006"] {
        assert!(
            !stdout.contains(&format!("{id}: blocked task has no reason note")),
            "{stdout}"
        );
    }
}
