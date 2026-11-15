//! Tests for `todo add --from <file|->` with `@ref` dependencies (T-034; R-WRITE-13).

mod common;

use common::{parse_json, read_text, stderr_text};

use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn add_from_adds_30_tasks_with_at_refs_in_one_step() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("TODO.md");
    fs::write(&path, "<!-- todo-format: 1 -->\n\n## Tasks\n\n## Later\n").unwrap();

    let mut lines = Vec::new();
    for i in 1..=30 {
        if i == 1 {
            lines.push(
                r#"{"ref":"step1","title":"Task 1","priority":"P0","section":"Tasks"}"#.to_string(),
            );
        } else if i == 30 {
            lines.push(
                r#"{"title":"Task 30","priority":"P2","section":"Later","depends":["@step1","@step29"]}"#
                    .to_string(),
            );
        } else {
            let prev = i - 1;
            lines.push(format!(
                r#"{{"ref":"step{i}","title":"Task {i}","priority":"P1","section":"Tasks","depends":"@step{prev}"}}"#
            ));
        }
    }
    let payload = lines.join("\n");

    let bin = assert_cmd::cargo::cargo_bin!("todo");
    let mut child = Command::new(bin)
        .current_dir(dir.path())
        .args(["--json", "--agent", "coordinator", "add", "--from", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(payload.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));

    let json: Value = parse_json(&out.stdout);
    assert_eq!(json["data"]["count"], 30);
    assert_eq!(json["data"]["ids"][0], "T-001");
    assert_eq!(json["data"]["ids"][29], "T-030");

    let task30 = &json["data"]["added"][29];
    assert_eq!(task30["section"], "Later");
    assert_eq!(task30["dependencies"][0]["id"], "T-001");
    assert_eq!(task30["dependencies"][1]["id"], "T-029");
}

#[test]
fn add_from_rejects_whole_batch_on_one_bad_entry() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("TODO.md");
    let initial = "<!-- todo-format: 1 -->\n\n## Tasks\n";
    fs::write(&path, initial).unwrap();

    // Entry 2 has an unknown @ref -> whole batch must fail and TODO.md must stay unchanged.
    let batch_file = dir.path().join("batch.jsonl");
    fs::write(
        &batch_file,
        "{\"ref\":\"a\",\"title\":\"Good task\",\"priority\":\"P1\"}\n\
         {\"title\":\"Bad ref task\",\"priority\":\"P1\",\"depends\":\"@missing\"}\n",
    )
    .unwrap();

    let out = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args([
            "--json",
            "--agent",
            "coordinator",
            "add",
            "--from",
            batch_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(read_text(&path), initial);

    // Next real `add` still gets T-001 (ids.json was not bumped).
    let next_add = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args([
            "--json",
            "--agent",
            "coordinator",
            "add",
            "--title",
            "First real task",
            "--priority",
            "P1",
        ])
        .output()
        .unwrap();
    assert_eq!(next_add.status.code(), Some(0));
    let next_json: Value = parse_json(&next_add.stdout);
    assert_eq!(next_json["data"]["id"], "T-001");
}
