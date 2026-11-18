mod common;

use std::fs;
use std::path::Path;

use serde_json::Value;
use tempfile::TempDir;

use common::{parse_json, read_text, run_in, stderr_text, stdout_text, todo_bin};

const ARCHIVE_FIXTURE: &str = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 2 -->
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
    let preview = run_in(dir.path(), &["archive", "--agent", "coordinator"]);
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
    assert!(archive_text.starts_with("<!-- todo-format: 2 -->"));
    assert!(archive_text.contains("## Backend"));
    assert!(archive_text.contains("### T-001 · Old done backend task"));
    assert!(archive_text.contains("## Frontend"));
    assert!(archive_text.contains("### T-004 · Dropped UI task"));
    assert!(!archive_text.contains("T-002"));

    // show still finds T-001 in archive.
    let show = run_in(dir.path(), &["show", "T-001", "--json"]);
    assert!(show.status.success());

    // list --archive includes archived tasks.
    let list_arch = run_in(dir.path(), &["list", "--archive", "--json"]);
    assert!(list_arch.status.success());
    let list_json: Value = parse_json(&list_arch.stdout);
    let tasks = list_json["data"]["tasks"].as_array().expect("tasks");
    assert!(tasks.iter().any(|t| t["id"] == "T-001"));
    assert!(tasks.iter().any(|t| t["id"] == "T-004"));

    // Dependency check: T-003 depends on T-001 (which is done in archive), so T-003 is ready and `check` is clean!
    let check = run_in(dir.path(), &["check"]);
    assert!(
        check.status.success(),
        "check failed: {}",
        stdout_text(&check)
    );

    // Archive remaining done task T-002 (into existing Backend section in TODO-archive.md).
    let arch_rest = run_in(dir.path(), &["archive", "--yes", "--agent", "coordinator"]);
    assert!(arch_rest.status.success());
    let archive_text2 = read_text(&dir.path().join("TODO-archive.md"));
    assert!(archive_text2.contains("### T-002 · Recent done backend task"));

    // Adding a new task must pick T-006, never reusing archived IDs.
    let add = run_in(
        dir.path(),
        &[
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
        ],
    );
    assert!(add.status.success());
    let add_json: Value = parse_json(&add.stdout);
    assert_eq!(add_json["data"]["id"], "T-006");
}

/// Makes the snapshot of TODO.md fail to record: the history folder is a file, so the write takes
/// the lock and then stops before TODO.md or the archive is replaced.
fn block_history(dir: &Path) {
    let state = dir.join(".todo");
    fs::create_dir_all(&state).expect("create state folder");
    fs::write(
        state.join("history"),
        "a file where the history folder belongs",
    )
    .expect("block history");
}

#[test]
fn archive_leaves_the_archive_alone_when_the_snapshot_of_todo_md_cannot_be_recorded() {
    let dir = TempDir::new().expect("tempdir");
    let archive_before = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
<!-- todo-format: 2 -->\n\
# Archive\n\
\n\
## Backend\n\
\n\
### T-900 · Archived earlier\n\
- status: done\n\
- owner: —\n\
- priority: P2\n";
    fs::write(dir.path().join("TODO.md"), ARCHIVE_FIXTURE).expect("write TODO.md");
    fs::write(dir.path().join("TODO-archive.md"), archive_before).expect("write archive");
    block_history(dir.path());

    let out = run_in(dir.path(), &["archive", "--yes", "--agent", "coordinator"]);
    assert!(
        !out.status.success(),
        "archive should fail: {}",
        stdout_text(&out)
    );
    assert_eq!(
        read_text(&dir.path().join("TODO-archive.md")),
        archive_before
    );
    assert_eq!(read_text(&dir.path().join("TODO.md")), ARCHIVE_FIXTURE);
}

#[test]
fn archive_creates_no_archive_file_when_the_snapshot_of_todo_md_cannot_be_recorded() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("TODO.md"), ARCHIVE_FIXTURE).expect("write TODO.md");
    block_history(dir.path());

    let out = run_in(dir.path(), &["archive", "--yes", "--agent", "coordinator"]);
    assert!(
        !out.status.success(),
        "archive should fail: {}",
        stdout_text(&out)
    );
    assert!(!dir.path().join("TODO-archive.md").exists());
    assert_eq!(read_text(&dir.path().join("TODO.md")), ARCHIVE_FIXTURE);
}
