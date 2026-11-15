//! Tests for `todo reopen` and `todo move` (T-030; R-WRITE-10).

use std::fs;

use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use tempfile::TempDir;

const FIXTURE: &str = "\
<!-- todo-format: 1 -->

## Backlog

### T-001 · Done item
- status: done
- owner: alice
- claimed: 2026-01-01T10:00+00:00
- priority: P1

### T-002 · Dropped item
- status: dropped
- owner: bob
- priority: P2

### T-003 · Open item with extra lines
- status: open
- owner: —
- priority: P1
- custom_field: keep-me
- note (2026-01-01, alice): original note

  Arbitrary prose paragraph inside T-003.

## Now
";

#[test]
fn reopen_requires_note_clears_owner_and_claimed_and_follows_transitions() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("TODO.md"), FIXTURE).unwrap();

    // Missing or empty --note is refused.
    let no_note = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args(["--agent", "carol", "reopen", "T-001"])
        .output()
        .unwrap();
    assert_eq!(no_note.status.code(), Some(2));

    let empty_note = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args(["--agent", "carol", "reopen", "T-001", "--note", "   "])
        .output()
        .unwrap();
    assert_eq!(empty_note.status.code(), Some(1));

    // Reopening an already-open task is refused with BAD_TRANSITION.
    let bad_trans = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args([
            "--json", "--agent", "carol", "reopen", "T-003", "--note", "why",
        ])
        .output()
        .unwrap();
    assert_eq!(bad_trans.status.code(), Some(1));
    let bad_json: Value = serde_json::from_slice(&bad_trans.stdout).unwrap();
    assert_eq!(bad_json["error"]["code"], "BAD_TRANSITION");

    // Reopen done task T-001.
    let ok_done = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args([
            "--json",
            "--agent",
            "carol",
            "reopen",
            "T-001",
            "--note",
            "regression in prod",
        ])
        .output()
        .unwrap();
    assert_eq!(ok_done.status.code(), Some(0));
    let done_json: Value = serde_json::from_slice(&ok_done.stdout).unwrap();
    assert_eq!(done_json["data"]["status"], "open");
    assert!(done_json["data"]["owner"].is_null());
    assert!(done_json["data"]["claimed"].is_null());

    // Reopen dropped task T-002.
    let ok_drop = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args([
            "--json",
            "--agent",
            "carol",
            "reopen",
            "T-002",
            "--note",
            "reconsidered",
        ])
        .output()
        .unwrap();
    assert_eq!(ok_drop.status.code(), Some(0));
    let drop_json: Value = serde_json::from_slice(&ok_drop.stdout).unwrap();
    assert_eq!(drop_json["data"]["status"], "open");
    assert!(drop_json["data"]["owner"].is_null());
}

#[test]
fn move_keeps_every_line_and_leaves_rest_of_file_unchanged() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("TODO.md");
    fs::write(&path, FIXTURE).unwrap();

    // Moving to the same section is a no-op (bytes unchanged).
    let same = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args(["--agent", "alice", "move", "T-003", "--section", "Backlog"])
        .output()
        .unwrap();
    assert_eq!(same.status.code(), Some(0));
    assert_eq!(fs::read_to_string(&path).unwrap(), FIXTURE);

    // Move T-003 to Now.
    let moved = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args([
            "--json",
            "--agent",
            "alice",
            "move",
            "T-003",
            "--section",
            "Now",
        ])
        .output()
        .unwrap();
    assert_eq!(
        moved.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&moved.stderr)
    );
    let moved_json: Value = serde_json::from_slice(&moved.stdout).unwrap();
    assert_eq!(moved_json["data"]["section"], "Now");

    let after = fs::read_to_string(&path).unwrap();
    assert!(after.contains("- custom_field: keep-me"));
    assert!(after.contains("- note (2026-01-01, alice): original note"));
    assert!(after.contains("  Arbitrary prose paragraph inside T-003."));
    // T-001 and T-002 remain untouched in Backlog.
    assert!(after.contains("### T-001 · Done item\n- status: done\n- owner: alice\n- claimed: 2026-01-01T10:00+00:00\n- priority: P1"));
    assert!(
        after.contains("### T-002 · Dropped item\n- status: dropped\n- owner: bob\n- priority: P2")
    );
    // T-003 is now after `## Now`.
    let now_pos = after.find("## Now").unwrap();
    let t3_pos = after
        .find("### T-003 · Open item with extra lines")
        .unwrap();
    assert!(t3_pos > now_pos);
}
