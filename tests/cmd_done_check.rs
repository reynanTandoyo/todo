//! CLI tests for what `todo done` says about the task's check (T-254; R-VERIFY-3).
//!
//! Each case runs `done` once in text and once with `--json`, each time in a fresh project. The
//! checks are the same tiny invented commands as in `cmd_verify.rs`. A check that must not run is a
//! failing one, so a `done` that succeeds also shows that nothing ran.

mod common;

use std::path::Path;
use std::process::Output;

use serde_json::Value;
use tempfile::TempDir;

use common::{
    FIXTURE_MARKER, parse_json, run_todo, setup_proj, stderr_text, stdout_text, write_toml,
};

/// The clock of every command in these tests (the `TODO_TEST_NOW` hook).
const NOW: &str = "2026-11-15T12:00:00+01:00";

/// Runs `todo` in `work` at the test clock.
fn todo(home: &Path, work: &Path, args: &[&str]) -> Output {
    run_todo(home, work, args, &[("TODO_TEST_NOW", NOW)]).unwrap()
}

/// A check that passes on this platform's default shell.
fn pass_check() -> &'static str {
    "exit 0"
}

/// A check that prints `message` and fails with exit code 1.
fn fail_check(message: &str) -> String {
    if cfg!(windows) {
        format!("Write-Output \"{message}\"; exit 1")
    } else {
        format!("echo \"{message}\"; exit 1")
    }
}

/// One task block: the heading, then one `- field` line per entry.
fn block(id: &str, title: &str, fields: &[&str]) -> String {
    let mut text = format!("### {id} · {title}\n");
    for field in fields {
        text.push_str("- ");
        text.push_str(field);
        text.push('\n');
    }
    text.push('\n');
    text
}

/// A TODO.md with the fixture marker and the given blocks, under one `##` section.
fn file(blocks: &[String]) -> String {
    let mut text = format!("{FIXTURE_MARKER}\n# Fixture\n\n## Now\n\n");
    for b in blocks {
        text.push_str(b);
    }
    text
}

/// T-001 claimed by alice, with `verify` as its check and a note that says it was added.
fn claimed_with_verify(verify: &str) -> String {
    let verify = format!("verify: {verify}");
    file(&[block(
        "T-001",
        "Build the parser",
        &[
            "status: claimed",
            "owner: alice",
            "claimed: 2026-11-15T10:00+01:00",
            "priority: P1 · difficulty: 2",
            &verify,
            "note (2026-11-15, alice): added",
        ],
    )])
}

/// Runs `todo <args>` in a fresh project: `TODO.md` is `body`, and `todo.toml` is `toml` when given.
fn run_in_fresh_project(body: &str, toml: Option<&str>, args: &[&str]) -> Output {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo_file) = setup_proj(&tmp, body);
    if let Some(toml) = toml {
        write_toml(&work, toml);
    }
    todo(&home, &work, args)
}

/// Runs `done T-001 --agent alice` with `extra` options, once in text and once with `--json`, each
/// in a fresh project. Returns the text output and the `data` of the JSON output.
fn done_text_and_data(body: &str, toml: Option<&str>, extra: &[&str]) -> (String, Value) {
    let mut args = vec!["done", "T-001", "--agent", "alice"];
    args.extend_from_slice(extra);
    let text = run_in_fresh_project(body, toml, &args);
    assert!(text.status.success(), "stderr: {}", stderr_text(&text));

    args.push("--json");
    let json = run_in_fresh_project(body, toml, &args);
    assert!(json.status.success(), "stderr: {}", stderr_text(&json));
    let parsed = parse_json(&json.stdout);
    (stdout_text(&text), parsed["data"].clone())
}

#[test]
fn a_passing_check_is_reported_as_passed_right_after_the_done_line() {
    let body = claimed_with_verify(pass_check());

    let (text, data) = done_text_and_data(&body, None, &[]);

    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.first(), Some(&"T-001: done ok"), "{text}");
    assert_eq!(lines.get(1), Some(&"verify: passed"), "{text}");
    assert_eq!(data["check"], "passed", "{data}");
}

#[test]
fn a_check_that_runs_later_in_wave_mode_is_reported_as_waiting() {
    // The check would fail, so a successful `done` shows that it did not run.
    let body = claimed_with_verify(&fail_check("checked later"));

    let (text, data) = done_text_and_data(&body, Some("verify_when = \"wave\"\n"), &[]);

    assert_eq!(text.lines().next(), Some("T-001: done ok"), "{text}");
    assert_eq!(
        text.lines().nth(1),
        Some("verify: not run yet (todo verify --wave runs it)"),
        "{text}"
    );
    assert_eq!(data["check"], "waiting", "{data}");
}

#[test]
fn a_check_switched_off_with_verify_false_is_reported_as_off() {
    // The check would fail, so a successful `done` shows that it did not run.
    let body = claimed_with_verify(&fail_check("would fail"));

    let (text, data) = done_text_and_data(&body, Some("verify = false\n"), &[]);

    assert_eq!(text.lines().next(), Some("T-001: done ok"), "{text}");
    assert_eq!(
        text.lines().nth(1),
        Some("verify: not run (verify = false in todo.toml)"),
        "{text}"
    );
    assert_eq!(data["check"], "off", "{data}");
}

#[test]
fn a_skipped_check_with_no_verify_and_force_is_reported_as_skipped() {
    // The check would fail, so a successful `done` shows that it did not run.
    let body = claimed_with_verify(&fail_check("would fail"));

    let (text, data) = done_text_and_data(&body, None, &["--no-verify", "--force"]);

    assert_eq!(text.lines().next(), Some("T-001: done ok"), "{text}");
    assert_eq!(
        text.lines().nth(1),
        Some("verify: skipped (--no-verify)"),
        "{text}"
    );
    assert_eq!(data["check"], "skipped", "{data}");
}

#[test]
fn a_task_without_a_verify_line_prints_no_check_line_and_reports_none() {
    let body = file(&[block(
        "T-001",
        "Plain task",
        &[
            "status: claimed",
            "owner: alice",
            "claimed: 2026-11-15T10:00+01:00",
            "priority: P1 · difficulty: 2",
            "note (2026-11-15, alice): added",
        ],
    )]);

    let (text, data) = done_text_and_data(&body, None, &[]);

    assert_eq!(text.lines().next(), Some("T-001: done ok"), "{text}");
    assert!(
        !text.lines().any(|line| line.starts_with("verify:")),
        "{text}"
    );
    assert_eq!(data["check"], "none", "{data}");
}

#[test]
fn drop_prints_no_check_line_and_its_json_has_no_check_key() {
    let body = claimed_with_verify(pass_check());
    let args = ["drop", "T-001", "--agent", "alice", "--note", "x"];

    let text = run_in_fresh_project(&body, None, &args);
    assert!(text.status.success(), "stderr: {}", stderr_text(&text));
    let text = stdout_text(&text);
    assert_eq!(text.lines().next(), Some("T-001: drop ok"), "{text}");
    assert!(
        !text.lines().any(|line| line.starts_with("verify:")),
        "{text}"
    );

    let mut json_args = args.to_vec();
    json_args.push("--json");
    let json = run_in_fresh_project(&body, None, &json_args);
    assert!(json.status.success(), "stderr: {}", stderr_text(&json));
    let parsed = parse_json(&json.stdout);
    assert!(parsed["data"].get("check").is_none(), "{parsed}");
}
