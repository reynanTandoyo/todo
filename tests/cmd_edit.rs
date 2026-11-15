//! Integration tests for `todo edit` (T-048; R-LOCK-8).

mod common;

use common::{parse_json, stderr_text, stdout_text};

use std::fs;

use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn edit_takes_lease_runs_editor_checks_snapshots_and_unlocks() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("TODO.md");
    fs::write(
        &path,
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
         <!-- todo-format: 1 -->\n\n\
         ## Tasks\n\n\
         ### T-001 · Initial\n\
         - status: open\n\
         - priority: P1\n",
    )
    .unwrap();

    // Create an editor script that verifies a lease is held while the editor is open,
    // then appends a new valid task to TODO.md.
    let lease_file = dir.path().join(".todo").join("lease.json");
    let flag_file = dir.path().join("lease_was_live.txt");
    let bin = assert_cmd::cargo::cargo_bin!("todo");

    #[cfg(windows)]
    let editor_path = {
        let p = dir.path().join("mock_editor.cmd");
        let script = format!(
            "@echo off\r\n\
             \"{}\" status --json > \"{}\" 2>nul\r\n\
             echo.>> %1\r\n\
             echo ### T-002 · Added in editor>> %1\r\n\
             echo - status: open>> %1\r\n\
             echo - priority: P2>> %1\r\n",
            bin.display(),
            flag_file.display(),
        );
        fs::write(&p, script).unwrap();
        p
    };

    #[cfg(not(windows))]
    let editor_path = {
        use std::os::unix::fs::PermissionsExt;
        let p = dir.path().join("mock_editor.sh");
        let script = format!(
            "#!/bin/sh\n\
             \"{}\" status --json > \"{}\" 2>/dev/null\n\
             printf '\\n### T-002 · Added in editor\\n- status: open\\n- priority: P2\\n' >> \"$1\"\n",
            bin.display(),
            flag_file.display(),
        );
        fs::write(&p, script).unwrap();
        let mut perms = fs::metadata(&p).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&p, perms).unwrap();
        p
    };

    let out = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .env("EDITOR", format!("\"{}\"", editor_path.display()))
        .env_remove("VISUAL")
        .args(["--json", "--agent", "alice", "edit"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout={}, stderr={}",
        stdout_text(&out),
        stderr_text(&out)
    );

    let json: Value = parse_json(&out.stdout);
    assert_eq!(json["schema"], "todo.edit.v1");
    assert_eq!(json["data"]["clean"], true);

    // Lease was live while editor ran.
    let status_during: Value = parse_json(&fs::read(&flag_file).unwrap());
    assert_eq!(status_during["data"]["lease"]["agent"], "alice");

    // Lease is ended after edit finishes.
    assert!(!lease_file.exists(), "lease.json should be removed");

    // Snapshot was recorded.
    let hist = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args(["--json", "history"])
        .output()
        .unwrap();
    let hist_json: Value = parse_json(&hist.stdout);
    assert_eq!(hist_json["data"]["count"], 1);
}
