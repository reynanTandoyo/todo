//! Integration tests for journal and `todo log` (T-047; R-HIST-4, R-READ-7).

use std::fs;

use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn writes_append_journal_and_log_shows_task_entries() {
    let dir = TempDir::new().unwrap();
    let home = dir.path().join("home");
    let work = dir.path().join("work");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&work).unwrap();

    fs::write(
        work.join("TODO.md"),
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
         <!-- todo-format: 1 -->\n\n\
         ## Tasks\n\n\
         ### T-001 · First\n\
         - status: open\n\
         - priority: P1\n\n\
         ### T-002 · Second\n\
         - status: open\n\
         - priority: P2\n",
    )
    .unwrap();

    // 1. Claim T-001
    let c1 = cargo_bin_cmd!("todo")
        .current_dir(&work)
        .env("HOME", &home)
        .env("APPDATA", &home)
        .args(["claim", "T-001", "--agent", "alice"])
        .output()
        .unwrap();
    assert!(c1.status.success());

    // 2. Claim T-002
    let c2 = cargo_bin_cmd!("todo")
        .current_dir(&work)
        .env("HOME", &home)
        .env("APPDATA", &home)
        .args(["claim", "T-002", "--agent", "bob"])
        .output()
        .unwrap();
    assert!(c2.status.success());

    // 3. Done T-001
    let d1 = cargo_bin_cmd!("todo")
        .current_dir(&work)
        .env("HOME", &home)
        .env("APPDATA", &home)
        .args(["done", "T-001", "--agent", "alice", "--note", "finished"])
        .output()
        .unwrap();
    assert!(d1.status.success());

    // Check `journal.jsonl` has 3 lines
    let journal_path = work.join(".todo").join("journal.jsonl");
    let journal_text = fs::read_to_string(&journal_path).unwrap();
    let lines: Vec<&str> = journal_text.lines().collect();
    assert_eq!(lines.len(), 3, "journal:\n{journal_text}");
    let first: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(first["agent"], "alice");
    assert_eq!(first["command"], "claim");
    assert_eq!(first["tasks"], serde_json::json!(["T-001"]));
    assert!(first["hash_before"].is_string());
    assert!(first["hash_after"].is_string());
    assert_ne!(first["hash_before"], first["hash_after"]);

    // `todo log T-001 --json` shows only the 2 entries touching T-001
    let log_json = cargo_bin_cmd!("todo")
        .current_dir(&work)
        .env("HOME", &home)
        .env("APPDATA", &home)
        .args(["--json", "log", "t1"])
        .output()
        .unwrap();
    assert!(log_json.status.success());
    let parsed: Value = serde_json::from_slice(&log_json.stdout).unwrap();
    assert_eq!(parsed["schema"], "todo.log.v1");
    assert_eq!(parsed["data"]["task"], "T-001");
    assert_eq!(parsed["data"]["count"], 2);
    assert_eq!(parsed["data"]["entries"][0]["command"], "claim");
    assert_eq!(parsed["data"]["entries"][1]["command"], "done");

    // Text mode
    let log_txt = cargo_bin_cmd!("todo")
        .current_dir(&work)
        .env("HOME", &home)
        .env("APPDATA", &home)
        .args(["log", "T-002"])
        .output()
        .unwrap();
    assert!(log_txt.status.success());
    let txt = String::from_utf8(log_txt.stdout).unwrap();
    assert!(txt.contains("bob  claim  [T-002]"), "{txt}");
}
