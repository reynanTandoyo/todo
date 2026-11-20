//! Tests for `--dry-run` on every write command and `--yes` helper (T-035; R-EDIT-10, R-WRITE-14).

mod common;

use common::{parse_json, stderr_text, stdout_text};

use std::fs;

use serde_json::Value;
use tempfile::TempDir;
use todo::cli::Context;
use todo::ops::require_yes_or_preview;
use todo::output::Output;

const FIXTURE: &str = "\
<!-- todo-format: 1 -->

## Tasks

### T-001 · Open task
- status: open
- owner: —
- priority: P1 · difficulty: 2

### T-002 · Claimed task
- status: claimed
- owner: alice
- claimed: 2026-01-01T10:00+00:00
- priority: P1

### T-003 · Blocked task
- status: blocked
- owner: alice
- priority: P2

### T-004 · Done task
- status: done
- owner: alice
- priority: P2

### T-005 · Open question
- status: open
- owner: —
- priority: P1 · type: question

## Later
";

#[test]
fn dry_run_prints_diff_and_leaves_file_byte_identical_across_all_write_commands() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("TODO.md");
    fs::write(&path, FIXTURE).unwrap();
    let orig_bytes = fs::read(&path).unwrap();

    let cases: &[&[&str]] = &[
        &[
            "add",
            "--title",
            "New task",
            "--priority",
            "P1",
            "--difficulty",
            "3",
            "--section",
            "Tasks",
        ],
        &["answer", "T-005", "Use 42"],
        &["ask", "What is the timeout?"],
        &["block", "T-002", "--note", "waiting on review"],
        &["claim", "T-001"],
        &["done", "T-002", "--note", "finished"],
        &["drop", "T-001", "--note", "obsolete"],
        &["move", "T-001", "--section", "Later"],
        &["next", "--claim"],
        &["note", "T-001", "hello"],
        &["release", "T-002"],
        &["reopen", "T-004", "--note", "regression"],
        &["set", "T-001", "--priority", "P0"],
        &["unblock", "T-003", "--note", "unblocked"],
    ];

    for cmd_args in cases {
        let mut full_args = vec!["--dry-run", "--agent", "alice"];
        full_args.extend_from_slice(cmd_args);

        let out = common::assert_todo_bin()
            .current_dir(dir.path())
            .args(&full_args)
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(0),
            "command {cmd_args:?} failed: {}",
            stderr_text(&out)
        );
        let stdout = stdout_text(&out);
        assert!(
            stdout.contains("--- a/") && stdout.contains("+++ b/") && stdout.contains("@@ "),
            "expected unified diff for {cmd_args:?}, got:\n{stdout}"
        );
        assert_eq!(
            fs::read(&path).unwrap(),
            orig_bytes,
            "TODO.md changed during --dry-run of {cmd_args:?}"
        );
    }

    // Also check JSON mode with --dry-run on `add` (and verify ids.json was not created).
    let json_out = common::assert_todo_bin()
        .current_dir(dir.path())
        .args([
            "--json",
            "--dry-run",
            "--agent",
            "alice",
            "add",
            "--title",
            "Dry JSON",
            "--priority",
            "P1",
            "--difficulty",
            "3",
            "--section",
            "Tasks",
        ])
        .output()
        .unwrap();
    assert_eq!(json_out.status.code(), Some(0));
    let json: Value = parse_json(&json_out.stdout);
    assert_eq!(json["ok"], true);
    assert_eq!(json["dry_run"], true);
    assert!(json["diff"].as_str().unwrap().contains("+### T-006"));
    assert_eq!(fs::read(&path).unwrap(), orig_bytes);
    assert!(!dir.path().join(".todo/ids.json").exists());
}

#[test]
fn require_yes_or_preview_gates_bulk_changes() {
    let base_ctx = Context {
        file: None,
        file_source: None,
        json: false,
        agent: Some("alice".to_string()),
        dry_run: false,
        yes: false,
        force: false,
        no_color: true,
        session: None,
        session_arg: None,
    };
    let mut out = Output::with_writers(false, Box::new(Vec::new()), Box::new(Vec::new()));
    assert!(!require_yes_or_preview(
        &base_ctx,
        "would archive 5 tasks",
        &mut out
    ));

    let mut yes_ctx = base_ctx.clone();
    yes_ctx.yes = true;
    assert!(require_yes_or_preview(
        &yes_ctx,
        "would archive 5 tasks",
        &mut out
    ));

    yes_ctx.dry_run = true;
    assert!(!require_yes_or_preview(
        &yes_ctx,
        "would archive 5 tasks",
        &mut out
    ));
}
