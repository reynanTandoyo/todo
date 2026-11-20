//! Integration tests for `todo apply <file|->` (T-036; R-WRITE-15).

mod common;

use common::{parse_json, read_text, stderr_text, stdout_text};

use std::fs;
use std::io::Write;
use std::process::Stdio;

use serde_json::Value;
use tempfile::TempDir;

#[test]
fn apply_runs_multiple_ops_with_one_snapshot_and_is_atomic_on_error() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("TODO.md");
    let initial = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
                   <!-- todo-format: 1 -->\n\n\
                   ## Tasks\n\n\
                   ### T-001 · Existing task\n\
                   - status: open\n\
                   - priority: P1\n\n\
                   ### T-002 · Claimed by bob\n\
                   - status: claimed\n\
                   - owner: bob\n\
                   - claimed: 2026-11-15T10:00+01:00\n\
                   - priority: P2\n";
    fs::write(&path, initial).unwrap();

    // 1. Batch where op 2 fails (claim T-002 owned by bob) -> nothing applies, no snapshot created.
    let bad_ops = "{\"op\":\"claim\",\"task\":\"T-001\"}\n\
                   {\"op\":\"claim\",\"task\":\"T-002\"}\n";
    let bad_file = dir.path().join("bad.jsonl");
    fs::write(&bad_file, bad_ops).unwrap();

    let bad_out = common::assert_todo_bin()
        .current_dir(dir.path())
        .env("TODO_TEST_NOW", "2026-11-15T12:00:00+01:00")
        .args([
            "--json",
            "--agent",
            "alice",
            "apply",
            bad_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(bad_out.status.code(), Some(1));
    assert_eq!(read_text(&path), initial);
    let hist_out = common::assert_todo_bin()
        .current_dir(dir.path())
        .env("TODO_TEST_NOW", "2026-11-15T12:00:00+01:00")
        .args(["--json", "history"])
        .output()
        .unwrap();
    let hist_json: Value = parse_json(&hist_out.stdout);
    assert_eq!(hist_json["data"]["count"], 0);

    // 2. Valid batch via stdin: add @new, claim T-001, note T-001, done T-001, claim @new
    let good_ops = "{\"op\":\"add\",\"ref\":\"new\",\"title\":\"Follow-up\",\"priority\":\"P1\",\"difficulty\":\"3\",\"depends\":\"T-001\"}\n\
                    {\"op\":\"claim\",\"task\":\"T-001\"}\n\
                    {\"op\":\"note\",\"task\":\"T-001\",\"text\":\"investigated\"}\n\
                    {\"op\":\"done\",\"task\":\"T-001\",\"note\":\"fixed\"}\n\
                    {\"op\":\"claim\",\"task\":\"@new\"}\n";

    let mut child = common::todo_bin()
        .current_dir(dir.path())
        .env("TODO_TEST_NOW", "2026-11-15T12:00:00+01:00")
        .args(["--json", "--agent", "alice", "apply", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(good_ops.as_bytes())
        .unwrap();
    let good_out = child.wait_with_output().unwrap();
    assert_eq!(
        good_out.status.code(),
        Some(0),
        "stdout={}, stderr={}",
        stdout_text(&good_out),
        stderr_text(&good_out)
    );
    let json: Value = parse_json(&good_out.stdout);
    assert_eq!(json["schema"], "todo.apply.v1");
    assert_eq!(json["data"]["count"], 5);
    assert_eq!(json["data"]["applied"][0]["task"], "T-003");
    assert_eq!(json["data"]["applied"][4]["task"], "T-003");

    // Exactly one snapshot was recorded.
    let hist_out2 = common::assert_todo_bin()
        .current_dir(dir.path())
        .env("TODO_TEST_NOW", "2026-11-15T12:00:00+01:00")
        .args(["--json", "history"])
        .output()
        .unwrap();
    let hist_json2: Value = parse_json(&hist_out2.stdout);
    assert_eq!(hist_json2["data"]["count"], 1);
}

#[test]
fn apply_refuses_an_unconfigured_priority_in_an_add_or_a_set_and_writes_nothing() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("TODO.md");
    let initial = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
                   <!-- todo-format: 3 -->\n\n\
                   ## Tasks\n\n\
                   ### T-001 · Existing task\n\
                   - status: open\n\
                   - owner: —\n\
                   - priority: P1 · difficulty: 2\n";
    fs::write(&path, initial).unwrap();

    for (name, line) in [
        (
            "add",
            "{\"op\":\"add\",\"title\":\"New one\",\"priority\":\"P9\",\"difficulty\":\"2\"}\n",
        ),
        (
            "set",
            "{\"op\":\"set\",\"task\":\"T-001\",\"priority\":\"P9\"}\n",
        ),
    ] {
        let file = dir.path().join(format!("{name}.jsonl"));
        fs::write(&file, line).unwrap();
        let out = common::assert_todo_bin()
            .current_dir(dir.path())
            .args([
                "--json",
                "--agent",
                "alice",
                "apply",
                file.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(1), "{name}: {}", stdout_text(&out));
        let json = parse_json(&out.stdout);
        assert_eq!(json["error"]["code"], "BAD_VALUE", "{name}");
        // An add names its operation in the message; the message is otherwise the one `add` gives.
        let message = json["error"]["message"].as_str().unwrap();
        assert!(
            message.contains("invalid priority 'P9'; expected one of: P0, P1, P2, P3"),
            "{name}: {json}"
        );
        if name == "add" {
            assert!(
                message.starts_with("operation #1 (add): "),
                "{name}: {json}"
            );
        }
        assert_eq!(read_text(&path), initial, "{name} changed TODO.md");
    }
}
