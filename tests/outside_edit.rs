mod common;

use std::fs;

use serde_json::Value;
use tempfile::TempDir;

use common::{parse_json, read_text, run_in, write_todo};

const SAMPLE_TODO: &str = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo:format=1 -->
# Test Tasks

## Tasks

### T-001 · First task
- status: open
- owner: —
- priority: P1
";

#[test]
fn hand_edit_between_commands_is_saved_and_reported_once() {
    let dir = TempDir::new().expect("tempdir");
    write_todo(dir.path(), SAMPLE_TODO);

    // Initial write via todo so last-write.json is recorded.
    let add = run_in(
        dir.path(),
        &[
            "add",
            "--title",
            "Second task",
            "--priority",
            "P2",
            "--agent",
            "agent-1",
        ],
    );
    assert!(add.status.success());

    // Simulate a hand edit outside the tool.
    let path = dir.path().join("TODO.md");
    let mut text = read_text(&path);
    text.push_str("- note (2026-11-15, human): edited by hand\n");
    fs::write(&path, text).expect("write hand edit");

    // Next command (read) detects it once, warns once, and saves an outside-edit snapshot.
    let list1 = run_in(dir.path(), &["list", "--json"]);
    assert!(list1.status.success());
    let list1_json: Value = parse_json(&list1.stdout);
    let warnings1 = list1_json["warnings"].as_array().expect("warnings array");
    assert_eq!(
        warnings1.len(),
        1,
        "expected 1 outside-edit warning: {warnings1:?}"
    );
    assert!(
        warnings1[0]
            .as_str()
            .is_some_and(|w| w.contains("edited outside the tool")),
        "unexpected warning: {warnings1:?}"
    );

    // Second read command must NOT warn again.
    let list2 = run_in(dir.path(), &["list", "--json"]);
    assert!(list2.status.success());
    let list2_json: Value = parse_json(&list2.stdout);
    let warnings2 = list2_json["warnings"].as_array().expect("warnings array");
    assert!(
        warnings2.is_empty(),
        "expected no second warning: {warnings2:?}"
    );

    // History contains the outside-edit snapshot.
    let hist = run_in(dir.path(), &["history", "--json"]);
    assert!(hist.status.success());
    let hist_json: Value = parse_json(&hist.stdout);
    let snaps = hist_json["data"]["snapshots"].as_array().expect("snaps");
    assert_eq!(snaps.len(), 2);
    assert_eq!(snaps[1]["command"], "outside edit");
    assert_eq!(snaps[1]["outside_edit"], true);
}

#[test]
fn edits_during_lease_are_recorded_without_warning() {
    let dir = TempDir::new().expect("tempdir");
    write_todo(dir.path(), SAMPLE_TODO);

    // Initial write.
    let add = run_in(
        dir.path(),
        &[
            "add",
            "--title",
            "Second task",
            "--priority",
            "P2",
            "--agent",
            "agent-1",
        ],
    );
    assert!(add.status.success());

    // Acquire lease.
    let lock = run_in(dir.path(), &["lock", "--agent", "human"]);
    assert!(lock.status.success());

    // Edit by hand while lease is live.
    let path = dir.path().join("TODO.md");
    let mut text = read_text(&path);
    text.push_str("- note (2026-11-15, human): edited under lease\n");
    fs::write(&path, text).expect("write lease edit");

    // Read during live lease emits no outside-edit warning.
    let list_during = run_in(dir.path(), &["list", "--json"]);
    assert!(list_during.status.success());
    let list_during_json: Value = parse_json(&list_during.stdout);
    assert!(
        list_during_json["warnings"]
            .as_array()
            .expect("warnings")
            .is_empty()
    );

    // Unlock records the lease edit snapshot without a warning.
    let unlock = run_in(dir.path(), &["unlock", "--agent", "human", "--json"]);
    assert!(unlock.status.success());
    let unlock_json: Value = parse_json(&unlock.stdout);
    assert!(
        unlock_json["warnings"]
            .as_array()
            .expect("warnings")
            .is_empty()
    );

    // Subsequent list emits no warning either.
    let list_after = run_in(dir.path(), &["list", "--json"]);
    assert!(list_after.status.success());
    let list_after_json: Value = parse_json(&list_after.stdout);
    assert!(
        list_after_json["warnings"]
            .as_array()
            .expect("warnings")
            .is_empty()
    );

    // History has the lease edit snapshot.
    let hist = run_in(dir.path(), &["history", "--json"]);
    let hist_json: Value = parse_json(&hist.stdout);
    let snaps = hist_json["data"]["snapshots"].as_array().expect("snaps");
    assert_eq!(snaps.len(), 2);
    assert_eq!(snaps[1]["command"], "lease edit");
    assert_eq!(snaps[1]["agent"], "human");
    assert!(snaps[1].get("outside_edit").is_none());
}
