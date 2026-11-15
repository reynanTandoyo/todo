mod common;

use std::fs;

use serde_json::Value;
use tempfile::TempDir;

use common::{parse_json, read_text, stderr_text, stdout_text, todo_bin};

const ARCHIVE_FIXTURE: &str = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 1 -->
# Project Tasks

## Backend

### T-001 · Old done backend task
- status: done
- owner: —
- priority: P1
- note (2026-10-01, alice): done — shipped

### T-002 · Recent done backend task
- status: done
- owner: —
- priority: P1
- note (2026-11-15, alice): done — just finished

### T-003 · Open task depending on T-001
- status: open
- owner: —
- priority: P1
- depends: T-001

## Frontend

### T-004 · Dropped UI task
- status: dropped
- owner: —
- priority: P2
- note (2026-10-15, bob): dropped — superseded

### T-005 · Highest ID open task
- status: open
- owner: —
- priority: P2
";

#[test]
fn archive_moves_done_and_dropped_to_matching_sections_and_preserves_lookups_and_ids() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("TODO.md"), ARCHIVE_FIXTURE).expect("write TODO.md");

    // Preview without --yes makes no changes.
    let preview = todo_bin()
        .current_dir(dir.path())
        .args(["archive", "--agent", "coordinator"])
        .output()
        .expect("run archive preview");
    assert!(preview.status.success());
    assert!(stdout_text(&preview).contains("pass --yes"));
    assert!(!dir.path().join("TODO-archive.md").exists());

    // Archive with --older-than 10 --yes: only T-001 and T-004 (Oct 2026) should move; T-002 (2026-11-15) stays.
    let arch_old = todo_bin()
        .current_dir(dir.path())
        .env("TODO_TEST_NOW", "2026-11-15T12:00:00+00:00")
        .args([
            "archive",
            "--older-than",
            "10",
            "--yes",
            "--agent",
            "coordinator",
            "--json",
        ])
        .output()
        .expect("run archive --older-than 10");
    assert!(
        arch_old.status.success(),
        "archive failed: {}",
        stderr_text(&arch_old)
    );
    let old_json: Value = parse_json(&arch_old.stdout);
    assert_eq!(old_json["data"]["count"], 2);

    let archive_text = read_text(&dir.path().join("TODO-archive.md"));
    assert!(archive_text.starts_with("<!-- todo-format: 1 -->"));
    assert!(archive_text.contains("## Backend"));
    assert!(archive_text.contains("### T-001 · Old done backend task"));
    assert!(archive_text.contains("## Frontend"));
    assert!(archive_text.contains("### T-004 · Dropped UI task"));
    assert!(!archive_text.contains("T-002"));

    // show still finds T-001 in archive.
    let show = todo_bin()
        .current_dir(dir.path())
        .args(["show", "T-001", "--json"])
        .output()
        .expect("run show T-001");
    assert!(show.status.success());

    // list --archive includes archived tasks.
    let list_arch = todo_bin()
        .current_dir(dir.path())
        .args(["list", "--archive", "--json"])
        .output()
        .expect("run list --archive");
    assert!(list_arch.status.success());
    let list_json: Value = parse_json(&list_arch.stdout);
    let tasks = list_json["data"]["tasks"].as_array().expect("tasks");
    assert!(tasks.iter().any(|t| t["id"] == "T-001"));
    assert!(tasks.iter().any(|t| t["id"] == "T-004"));

    // Dependency check: T-003 depends on T-001 (which is done in archive), so T-003 is ready and `check` is clean!
    let check = todo_bin()
        .current_dir(dir.path())
        .args(["check"])
        .output()
        .expect("run check");
    assert!(
        check.status.success(),
        "check failed: {}",
        stdout_text(&check)
    );

    // Archive remaining done task T-002 (into existing Backend section in TODO-archive.md).
    let arch_rest = todo_bin()
        .current_dir(dir.path())
        .args(["archive", "--yes", "--agent", "coordinator"])
        .output()
        .expect("run archive rest");
    assert!(arch_rest.status.success());
    let archive_text2 = read_text(&dir.path().join("TODO-archive.md"));
    assert!(archive_text2.contains("### T-002 · Recent done backend task"));

    // Adding a new task must pick T-006, never reusing archived IDs.
    let add = todo_bin()
        .current_dir(dir.path())
        .args([
            "add",
            "--title",
            "After archive",
            "--section",
            "Backend",
            "--priority",
            "P1",
            "--agent",
            "alice",
            "--json",
        ])
        .output()
        .expect("run add");
    assert!(add.status.success());
    let add_json: Value = parse_json(&add.stdout);
    assert_eq!(add_json["data"]["id"], "T-006");
}
