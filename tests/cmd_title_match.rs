//! Tests for title matching on task arguments (T-029; R-ID-4).

use std::fs;

use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use tempfile::TempDir;

const FIXTURE: &str = "\
<!-- todo-format: 1 -->

## Tasks

### T-001 · Fix login timeout
- status: open
- owner: —
- priority: P1

### T-002 · Fix signup timeout
- status: claimed
- owner: bob
- priority: P2

### T-003 · Fix old login bug
- status: done
- owner: alice
- priority: P2

### T-004 · Dropped login experiment
- status: dropped
- owner: —
- priority: P3
";

#[test]
fn title_matching_unique_ambiguous_none_and_skips_done_dropped() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("TODO.md"), FIXTURE).unwrap();

    // Unique match ignoring case ("LOGIN" matches only T-001 because T-003 is done and T-004 is dropped).
    let show_out = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args(["--json", "show", "LOGIN"])
        .output()
        .unwrap();
    assert_eq!(show_out.status.code(), Some(0));
    let show_json: Value = serde_json::from_slice(&show_out.stdout).unwrap();
    assert_eq!(show_json["data"]["id"], "T-001");

    // Write command by title ("login timeout") claims T-001.
    let claim_out = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args(["--json", "--agent", "alice", "claim", "login timeout"])
        .output()
        .unwrap();
    assert_eq!(claim_out.status.code(), Some(0));
    let claim_json: Value = serde_json::from_slice(&claim_out.stdout).unwrap();
    assert_eq!(claim_json["data"]["id"], "T-001");
    assert_eq!(claim_json["data"]["status"], "claimed");

    // Ambiguous match ("timeout" matches T-001 and T-002) exits 1 and lists candidates.
    let ambig_out = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args(["--json", "show", "timeout"])
        .output()
        .unwrap();
    assert_eq!(ambig_out.status.code(), Some(1));
    let ambig_json: Value = serde_json::from_slice(&ambig_out.stdout).unwrap();
    assert_eq!(ambig_json["error"]["code"], "AMBIGUOUS");
    let candidates = ambig_json["error"]["candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0], "T-001");
    assert_eq!(candidates[1], "T-002");

    // Done/dropped-only title ("old login" / "experiment") is never matched.
    let done_only = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args(["show", "old login"])
        .output()
        .unwrap();
    assert_eq!(done_only.status.code(), Some(1));

    let dropped_only = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args(["--agent", "alice", "note", "experiment", "--text", "hi"])
        .output()
        .unwrap();
    assert_eq!(dropped_only.status.code(), Some(1));

    // No matches exits 1.
    let none_out = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args(["show", "nonexistent"])
        .output()
        .unwrap();
    assert_eq!(none_out.status.code(), Some(1));
}
