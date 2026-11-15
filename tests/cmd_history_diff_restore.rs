mod common;

use std::fs;

use serde_json::Value;
use tempfile::TempDir;

use common::{todo_bin, write_todo};

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
        let out = todo_bin()
            .current_dir(dir.path())
            .args([
                "add",
                "--title",
                &format!("Task number {i}"),
                "--priority",
                "P2",
                "--agent",
                "agent-1",
            ])
            .output()
            .expect("run add");
        assert!(
            out.status.success(),
            "add {i} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    let hist_out = todo_bin()
        .current_dir(dir.path())
        .args(["history", "--json"])
        .output()
        .expect("run history");
    assert!(hist_out.status.success());
    let hist_json: Value = serde_json::from_slice(&hist_out.stdout).expect("parse history json");
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
    let diff_out = todo_bin()
        .current_dir(dir.path())
        .args(["diff", "2"])
        .output()
        .expect("run diff 2");
    assert!(diff_out.status.success());
    let diff_text = String::from_utf8_lossy(&diff_out.stdout);
    assert!(diff_text.contains("Task number 3"));
    assert!(diff_text.contains("Task number 4"));

    // Diff snapshot 2 against 3.
    let diff_2_3 = todo_bin()
        .current_dir(dir.path())
        .args(["diff", "2", "3", "--json"])
        .output()
        .expect("run diff 2 3");
    assert!(diff_2_3.status.success());
    let diff_json: Value = serde_json::from_slice(&diff_2_3.stdout).expect("parse diff json");
    assert_eq!(diff_json["data"]["from"], 2);
    assert_eq!(diff_json["data"]["to"], 3);
    assert_eq!(diff_json["data"]["changed"], true);

    // Restore without --yes previews only and does not modify TODO.md.
    let before_preview = fs::read_to_string(dir.path().join("TODO.md")).expect("read before");
    let preview_out = todo_bin()
        .current_dir(dir.path())
        .args(["restore", "2", "--agent", "agent-1"])
        .output()
        .expect("run restore preview");
    assert!(preview_out.status.success());
    let preview_stdout = String::from_utf8_lossy(&preview_out.stdout);
    assert!(preview_stdout.contains("pass --yes"));
    assert_eq!(
        fs::read_to_string(dir.path().join("TODO.md")).expect("read after preview"),
        before_preview
    );

    // Restore with --yes applies snapshot 2 and records snapshot 5 (pre-restore) so it can be undone.
    let restore_out = todo_bin()
        .current_dir(dir.path())
        .args(["restore", "2", "--yes", "--agent", "agent-1"])
        .output()
        .expect("run restore --yes");
    assert!(
        restore_out.status.success(),
        "restore failed: {}",
        String::from_utf8_lossy(&restore_out.stderr)
    );
    let after_restore = fs::read_to_string(dir.path().join("TODO.md")).expect("read after restore");
    assert!(!after_restore.contains("Task number 2"));
    assert!(!after_restore.contains("Task number 4"));

    // Undo the restore by restoring snapshot 5!
    let hist_after = todo_bin()
        .current_dir(dir.path())
        .args(["history", "--json"])
        .output()
        .expect("run history after restore");
    let hist_after_json: Value =
        serde_json::from_slice(&hist_after.stdout).expect("parse history json");
    let latest_num = hist_after_json["data"]["snapshots"]
        .as_array()
        .expect("snaps")
        .last()
        .expect("last")["number"]
        .as_u64()
        .expect("num");
    assert_eq!(latest_num, 5);

    let undo_out = todo_bin()
        .current_dir(dir.path())
        .args(["restore", "5", "--yes", "--agent", "agent-1"])
        .output()
        .expect("undo restore");
    assert!(undo_out.status.success());
    let after_undo = fs::read_to_string(dir.path().join("TODO.md")).expect("read after undo");
    assert_eq!(after_undo, before_preview);
}
