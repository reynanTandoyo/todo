//! Tests for `-` (stdin) and `@path` text options (T-032; R-WRITE-12).

mod common;

use common::{parse_json, run_with_stdin, stderr_text};

use std::fs;

use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use tempfile::TempDir;

const TRICKY: &str = r#"Keep "double", 'single', `backticks`, and $env:VAR $(sub) intact!"#;

#[test]
fn text_with_quotes_backticks_and_dollar_arrives_intact_via_stdin_and_at_file() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("TODO.md"),
        "<!-- todo-format: 1 -->\n\n## Tasks\n\n### T-001 · Initial\n- status: open\n- owner: —\n- priority: P1\n",
    )
    .unwrap();

    // 1. `note` reading from stdin (`-`).
    let note_out = run_with_stdin(
        dir.path(),
        &["--json", "--agent", "alice", "note", "T-001", "-"],
        TRICKY,
    );
    assert_eq!(note_out.status.code(), Some(0));
    let note_json: Value = parse_json(&note_out.stdout);
    assert_eq!(note_json["data"]["notes"][0]["text"], TRICKY);

    // 2. `add` reading `--title` from `@path` (with spaces and non-ASCII in path).
    let sub = dir.path().join("my notes café");
    fs::create_dir_all(&sub).unwrap();
    let msg_file = sub.join("title.txt");
    fs::write(&msg_file, format!("\u{feff}{TRICKY}\n")).unwrap();
    let at_arg = format!("@{}", msg_file.display());

    let add_out = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args([
            "--json",
            "--agent",
            "alice",
            "add",
            "--title",
            &at_arg,
            "--priority",
            "P1",
            "--problem",
            &at_arg,
        ])
        .output()
        .unwrap();
    assert_eq!(
        add_out.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&add_out)
    );
    let add_json: Value = parse_json(&add_out.stdout);
    assert_eq!(add_json["data"]["title"], TRICKY);
    assert_eq!(add_json["data"]["problem"], TRICKY);

    // 3. `done --note -` from stdin.
    let done_out = run_with_stdin(
        dir.path(),
        &["--json", "--agent", "alice", "done", "T-001", "--note", "-"],
        TRICKY,
    );
    assert_eq!(done_out.status.code(), Some(0));
    let done_json: Value = parse_json(&done_out.stdout);
    let notes = done_json["data"]["notes"].as_array().unwrap();
    assert!(
        notes
            .iter()
            .any(|n| n["text"].as_str().unwrap().contains(TRICKY))
    );

    // 4. Missing `@path` returns exit 4 (`NO_FILE`).
    let missing = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args(["--agent", "alice", "note", "T-002", "@does-not-exist.txt"])
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(4));
}
