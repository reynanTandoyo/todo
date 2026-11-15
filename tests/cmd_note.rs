//! CLI integration tests for `todo note` (T-026; R-WRITE-7, R-IO-6, TEST-4).

mod common;

use std::fs;

use tempfile::TempDir;

use common::{parse_json, read_text, setup_proj, stderr_text, stdout_text};

#[test]
fn note_appends_after_last_line_on_any_status_by_any_agent() {
    let tmp = TempDir::new().unwrap();
    let initial = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
## Now\n\n\
### T-001 · Open task\n\
- status: open\n\
- owner: —\n\
\n\
### T-002 · Claimed by alice\n\
- status: claimed\n\
- owner: alice\n\
- claimed: 2026-11-15T10:00+01:00\n\
- note (2026-11-14, alice): started\n\
\n\
### T-003 · Done task\n\
- status: done\n\
- owner: alice\n\
\n\
### T-004 · Dropped task\n\
- status: dropped\n\
- owner: —\n";
    let (home, work, todo_file) = setup_proj(&tmp, initial);
    // 23:30 UTC is 2026-11-16 in +01:00, verifying local date (R-IO-6).
    let now = "2026-11-16T00:30:00+01:00";

    // Positional text on claimed task by another agent (bob) without --force.
    let n1 = common::run_todo(
        &home,
        &work,
        &["note", "T-002", "saw edge case", "--agent", "bob"],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert!(n1.status.success(), "stderr: {}", stderr_text(&n1));
    assert_eq!(stdout_text(&n1).trim(), "T-002: note ok");

    // --text flag on done and dropped tasks, plus open task.
    for (id, msg) in [
        ("T-001", "triaged"),
        ("T-003", "post-release check"),
        ("T-004", "confirmed duplicate"),
    ] {
        let res = common::run_todo(
            &home,
            &work,
            &["--json", "note", id, "--text", msg, "--agent", "reviewer"],
            &[("TODO_TEST_NOW", now)],
        )
        .unwrap();
        assert!(res.status.success());
        let j: serde_json::Value = parse_json(&res.stdout);
        assert_eq!(j["schema"], "todo.note.v1");
    }

    let after = read_text(&todo_file);
    assert!(after.contains(
        "### T-002 · Claimed by alice\n\
- status: claimed\n\
- owner: alice\n\
- claimed: 2026-11-15T10:00+01:00\n\
- note (2026-11-14, alice): started\n\
- note (2026-11-16, bob): saw edge case\n\n"
    ));
    assert!(after.contains(
        "### T-003 · Done task\n\
- status: done\n\
- owner: alice\n\
- note (2026-11-16, reviewer): post-release check\n\n"
    ));
    assert!(after.contains(
        "### T-004 · Dropped task\n\
- status: dropped\n\
- owner: —\n\
- note (2026-11-16, reviewer): confirmed duplicate\n"
    ));

    // Empty / missing note text is refused with exit 2 and leaves file untouched.
    let before_err = fs::read(&todo_file).unwrap();
    let missing =
        common::run_todo(&home, &work, &["note", "T-001", "--agent", "bob"], &[]).unwrap();
    assert_eq!(missing.status.code(), Some(2));
    let empty = common::run_todo(
        &home,
        &work,
        &["note", "T-001", "   ", "--agent", "bob"],
        &[],
    )
    .unwrap();
    assert_eq!(empty.status.code(), Some(2));
    assert_eq!(fs::read(&todo_file).unwrap(), before_err);
}
