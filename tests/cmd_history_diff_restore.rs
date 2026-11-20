mod common;

use std::fs;
use std::path::Path;

use serde_json::Value;
use tempfile::TempDir;

use common::{parse_json, read_todo, run_in, set_read_only, stderr_text, stdout_text, write_todo};

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
fn writes_create_snapshots_pruned_to_snapshots_keep_and_diff_restore_work() {
    let dir = TempDir::new().expect("tempdir");
    write_todo(dir.path(), SAMPLE_TODO);
    fs::write(dir.path().join("todo.toml"), "snapshots_keep = 3\n").expect("write todo.toml");

    // 4 writes -> snapshots 1, 2, 3, 4 created, only 2, 3, 4 kept.
    for i in 1..=4 {
        let out = run_in(
            dir.path(),
            &[
                "add",
                "--title",
                &format!("Task number {i}"),
                "--priority",
                "P2",
                "--difficulty",
                "3",
                "--agent",
                "agent-1",
            ],
        );
        assert!(
            out.status.success(),
            "add {i} failed: {}",
            stderr_text(&out)
        );
    }

    let hist_out = run_in(dir.path(), &["history", "--json"]);
    assert!(hist_out.status.success());
    let hist_json: Value = parse_json(&hist_out.stdout);
    assert_eq!(hist_json["ok"], true);
    let snaps = hist_json["data"]["snapshots"]
        .as_array()
        .expect("snapshots array");
    assert_eq!(snaps.len(), 3, "only newest 3 snapshots should remain");
    assert_eq!(snaps[0]["number"], 2);
    assert_eq!(snaps[1]["number"], 3);
    assert_eq!(snaps[2]["number"], 4);
    assert_eq!(snaps[2]["agent"], "agent-1");
    assert_eq!(snaps[2]["command"], "add");
    assert!(snaps[2]["sha256"].as_str().is_some_and(|s| s.len() == 64));

    assert!(
        !dir.path().join(".todo/history/000001.md").exists(),
        "pruned snapshot 1 file should be deleted"
    );
    assert!(dir.path().join(".todo/history/000002.md").exists());

    // Diff snapshot 2 against current.
    let diff_out = run_in(dir.path(), &["diff", "2"]);
    assert!(diff_out.status.success());
    let diff_text = stdout_text(&diff_out);
    assert!(diff_text.contains("Task number 3"));
    assert!(diff_text.contains("Task number 4"));

    // Diff snapshot 2 against 3.
    let diff_2_3 = run_in(dir.path(), &["diff", "2", "3", "--json"]);
    assert!(diff_2_3.status.success());
    let diff_json: Value = parse_json(&diff_2_3.stdout);
    assert_eq!(diff_json["data"]["from"], 2);
    assert_eq!(diff_json["data"]["to"], 3);
    assert_eq!(diff_json["data"]["changed"], true);

    // Restore without --yes previews only and does not modify TODO.md.
    let before_preview = read_todo(dir.path());
    let preview_out = run_in(dir.path(), &["restore", "2", "--agent", "agent-1"]);
    assert!(preview_out.status.success());
    let preview_stdout = stdout_text(&preview_out);
    assert!(preview_stdout.contains("pass --yes"));
    assert_eq!(read_todo(dir.path()), before_preview);

    // Restore with --yes applies snapshot 2 and records snapshot 5 (pre-restore) so it can be undone.
    let restore_out = run_in(dir.path(), &["restore", "2", "--yes", "--agent", "agent-1"]);
    assert!(
        restore_out.status.success(),
        "restore failed: {}",
        stderr_text(&restore_out)
    );
    let after_restore = read_todo(dir.path());
    assert!(!after_restore.contains("Task number 2"));
    assert!(!after_restore.contains("Task number 4"));

    // Undo the restore by restoring snapshot 5!
    let hist_after = run_in(dir.path(), &["history", "--json"]);
    let hist_after_json: Value = parse_json(&hist_after.stdout);
    let latest_num = hist_after_json["data"]["snapshots"]
        .as_array()
        .expect("snaps")
        .last()
        .expect("last")["number"]
        .as_u64()
        .expect("num");
    assert_eq!(latest_num, 5);

    let undo_out = run_in(dir.path(), &["restore", "5", "--yes", "--agent", "agent-1"]);
    assert!(undo_out.status.success());
    let after_undo = read_todo(dir.path());
    assert_eq!(after_undo, before_preview);
}

/// The number of snapshots `todo history` lists for `dir` (`R-HIST-2`).
fn snapshot_count(dir: &Path) -> u64 {
    let out = run_in(dir, &["history", "--json", "--agent", "bob"]);
    assert!(out.status.success(), "{}", stderr_text(&out));
    parse_json(&out.stdout)["data"]["count"]
        .as_u64()
        .expect("snapshot count")
}

#[test]
fn a_write_refused_for_a_read_only_todo_md_saves_no_snapshot() {
    let dir = TempDir::new().expect("tempdir");
    write_todo(dir.path(), SAMPLE_TODO);
    // A first write, so that the history already holds a snapshot to compare against.
    let first = run_in(dir.path(), &["note", "T-001", "first", "--agent", "bob"]);
    assert!(first.status.success(), "{}", stderr_text(&first));
    let before = snapshot_count(dir.path());
    assert!(before >= 1);

    let todo = dir.path().join("TODO.md");
    let todo_before = read_todo(dir.path());
    set_read_only(&todo, true);
    let refused = run_in(dir.path(), &["note", "T-001", "refused", "--agent", "bob"]);
    set_read_only(&todo, false);

    assert_eq!(refused.status.code(), Some(4), "{}", stderr_text(&refused));
    assert!(stderr_text(&refused).contains("read-only"));
    assert_eq!(snapshot_count(dir.path()), before);
    assert_eq!(read_todo(dir.path()), todo_before);
}

#[test]
fn writes_refused_for_a_read_only_todo_md_save_no_snapshot_when_there_is_no_history_yet() {
    let dir = TempDir::new().expect("tempdir");
    write_todo(dir.path(), SAMPLE_TODO);
    let todo = dir.path().join("TODO.md");
    set_read_only(&todo, true);
    // A write that changes text, and a migration of the format 1 file, are both refused.
    let note = run_in(dir.path(), &["note", "T-001", "refused", "--agent", "bob"]);
    let migrate = run_in(dir.path(), &["migrate", "--yes", "--agent", "bob"]);
    set_read_only(&todo, false);

    for out in [&note, &migrate] {
        assert_eq!(out.status.code(), Some(4), "{}", stderr_text(out));
    }
    assert_eq!(snapshot_count(dir.path()), 0);
}
