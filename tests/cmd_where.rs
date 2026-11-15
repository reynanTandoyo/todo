//! Tests for `todo where` (T-016; R-FIND-7).

mod common;

use std::fs;

use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use tempfile::TempDir;

use common::git;

#[test]
fn where_prints_all_r_find_7_items_in_text_and_json() {
    let dir = TempDir::new().unwrap();
    git(dir.path(), &["init", "-q"]);
    fs::write(
        dir.path().join("TODO.md"),
        "<!-- todo-format: 1 -->\n\n## Tasks\n\n\
         ### T-001 · Open task\n- status: open\n- owner: —\n- priority: P1\n\n\
         ### T-002 · Claimed task\n- status: claimed\n- owner: alice\n- priority: P1\n\n\
         ### T-003 · Blocked task\n- status: blocked\n- owner: —\n- priority: P2\n\n\
         ### T-004 · Done task\n- status: done\n- owner: alice\n- priority: P2\n\n\
         ### T-005 · Dropped task\n- status: dropped\n- owner: —\n- priority: P3\n",
    )
    .unwrap();

    // Acquire a manual lease so lease status is also populated.
    let lock_out = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args(["--agent", "alice", "lock", "--minutes", "5"])
        .output()
        .unwrap();
    assert_eq!(lock_out.status.code(), Some(0));

    // Text mode
    let text_out = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args(["where"])
        .output()
        .unwrap();
    assert_eq!(text_out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&text_out.stdout);
    assert!(stdout.contains("File:"), "stdout: {stdout}");
    assert!(stdout.contains("found by: walk"), "stdout: {stdout}");
    assert!(stdout.contains("Repo root:"), "stdout: {stdout}");
    assert!(stdout.contains("State dir:"), "stdout: {stdout}");
    assert!(stdout.contains("Write lock: free"), "stdout: {stdout}");
    assert!(stdout.contains("Lease: held by alice"), "stdout: {stdout}");
    assert!(stdout.contains("Format version: 1"), "stdout: {stdout}");
    assert!(
        stdout.contains("Tasks: 5 total (open: 1, claimed: 1, blocked: 1, done: 1, dropped: 1)"),
        "stdout: {stdout}"
    );

    // JSON mode
    let json_out = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args(["--json", "where"])
        .output()
        .unwrap();
    assert_eq!(json_out.status.code(), Some(0));
    let json: Value = serde_json::from_slice(&json_out.stdout).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["data"]["found_by"], "walk");
    assert!(json["data"]["file"].as_str().unwrap().ends_with("TODO.md"));
    assert!(json["data"]["repo_root"].is_string());
    assert!(json["data"]["state_dir"].is_string());
    assert_eq!(json["data"]["write_lock"]["held"], false);
    assert_eq!(json["data"]["lease"]["agent"], "alice");
    assert_eq!(json["data"]["format_version"], 1);
    assert_eq!(json["data"]["counts"]["open"], 1);
    assert_eq!(json["data"]["counts"]["claimed"], 1);
    assert_eq!(json["data"]["counts"]["blocked"], 1);
    assert_eq!(json["data"]["counts"]["done"], 1);
    assert_eq!(json["data"]["counts"]["dropped"], 1);
    assert_eq!(json["data"]["counts"]["total"], 5);
}

#[test]
fn where_works_outside_git_repo() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("TODO.md"), "## Tasks\n").unwrap();

    let json_out = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args(["--json", "where"])
        .output()
        .unwrap();
    assert_eq!(json_out.status.code(), Some(0));
    let json: Value = serde_json::from_slice(&json_out.stdout).unwrap();
    assert_eq!(json["data"]["found_by"], "walk");
    assert!(json["data"]["repo_root"].is_null());
    assert!(json["data"]["format_version"].is_null());
    assert_eq!(json["data"]["counts"]["total"], 0);
}

#[test]
fn warns_on_onedrive_and_dropbox_in_where_and_writes() {
    let dir = TempDir::new().unwrap();
    let onedrive = dir.path().join("OneDrive - Personal").join("proj");
    fs::create_dir_all(&onedrive).unwrap();
    fs::write(onedrive.join("TODO.md"), "## Tasks\n").unwrap();

    let where_out = cargo_bin_cmd!("todo")
        .current_dir(&onedrive)
        .args(["where"])
        .output()
        .unwrap();
    assert!(where_out.status.success());
    let where_err = String::from_utf8(where_out.stderr).unwrap();
    assert!(where_err.contains("OneDrive"), "stderr was: {where_err}");

    let dropbox = dir.path().join("Dropbox").join("proj");
    fs::create_dir_all(&dropbox).unwrap();
    fs::write(dropbox.join("TODO.md"), "## Tasks\n").unwrap();

    let add_out = cargo_bin_cmd!("todo")
        .current_dir(&dropbox)
        .args([
            "--json",
            "--agent",
            "alice",
            "add",
            "--title",
            "First",
            "--priority",
            "P2",
        ])
        .output()
        .unwrap();
    assert!(
        add_out.status.success(),
        "add failed: stdout={}, stderr={}",
        String::from_utf8_lossy(&add_out.stdout),
        String::from_utf8_lossy(&add_out.stderr)
    );
    let json: Value = serde_json::from_slice(&add_out.stdout).unwrap();
    let warnings = json["warnings"].as_array().unwrap();
    assert!(
        warnings
            .iter()
            .any(|w| w.as_str().unwrap_or("").contains("Dropbox")),
        "expected Dropbox warning, got: {warnings:?}"
    );
}
