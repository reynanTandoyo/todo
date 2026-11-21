//! Integration tests for `todo apply`: a line break in a free-text value of an `add` or `set`
//! operation becomes a single space, as `todo add` and `todo set` make it (`docs/format.md` §10,
//! R-EDIT-5). Steps are the exception: each line of their text is one step (R-DETAIL-1).

mod common;

use std::fs;
use std::process::Output;

use serde_json::{Value, json};
use tempfile::TempDir;

use common::{stderr_text, stdout_text, write_todo};

const EMPTY_TODO: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
                          <!-- todo-format: 1 -->\n\n\
                          ## Tasks\n";

const ONE_TASK_TODO: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
                             <!-- todo-format: 1 -->\n\n\
                             ## Tasks\n\n\
                             ### T-001 · Existing task\n\
                             - status: open\n\
                             - owner: —\n\
                             - priority: P1 · difficulty: 2\n\
                             - why: old reason\n";

/// Writes `ops` as JSON lines to `ops.jsonl` in `dir` and runs `todo --json apply` on that file.
fn apply_ops(dir: &TempDir, ops: &[Value]) -> Output {
    let text: String = ops.iter().map(|op| format!("{op}\n")).collect();
    let file = dir.path().join("ops.jsonl");
    fs::write(&file, text).expect("write ops file");
    common::assert_todo_bin()
        .current_dir(dir.path())
        .env("TODO_TEST_NOW", "2026-11-15T12:00:00+01:00")
        .args([
            "--json",
            "--agent",
            "alice",
            "apply",
            file.to_str().expect("utf-8 path"),
        ])
        .output()
        .expect("run todo apply")
}

/// Asserts that `output` succeeded and returns the `TODO.md` text it left in `dir`.
fn applied_todo(dir: &TempDir, output: &Output) -> String {
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={}, stderr={}",
        stdout_text(output),
        stderr_text(output)
    );
    common::read_todo(dir.path())
}

/// Whether `text` has a line that is exactly `line`.
fn has_line(text: &str, line: &str) -> bool {
    text.lines().any(|l| l == line)
}

#[test]
fn apply_add_turns_line_breaks_in_free_text_into_single_spaces() {
    let dir = TempDir::new().expect("tempdir");
    write_todo(dir.path(), EMPTY_TODO);

    let output = apply_ops(
        &dir,
        &[
            json!({
                "op": "add",
                "ref": "m",
                "title": "Shelves\nand lights",
                "type": "milestone",
                "priority": "P1",
                "goal": "the room\r\nis lit"
            }),
            json!({
                "op": "add",
                "ref": "t",
                "title": "Hang\nthe lamp",
                "priority": "P2",
                "difficulty": "2",
                "milestone": "@m",
                "why": "the room\nneeds light",
                "files": "src/lamp.rs\nsrc/wall.rs",
                "read": "lamp.md\r\nwall.md",
                "problem": "no light\r\nin the room",
                "fix": "add a lamp\nand wire it",
                "accept": "first line\nsecond line",
                "verify": "cargo test --test lamp\nand more"
            }),
            json!({
                "op": "add",
                "title": "Second\ntask",
                "priority": "P2",
                "difficulty": "2",
                "fields": {"why": "from\nfields", "custom": "one\ntwo"}
            }),
        ],
    );
    let text = applied_todo(&dir, &output);

    assert!(has_line(&text, "### T-001 · Shelves and lights"), "{text}");
    assert!(has_line(&text, "- goal: the room is lit"), "{text}");
    assert!(has_line(&text, "### T-002 · Hang the lamp"), "{text}");
    assert!(has_line(&text, "- milestone: T-001"), "{text}");
    assert!(has_line(&text, "- why: the room needs light"), "{text}");
    assert!(
        has_line(&text, "- files: src/lamp.rs src/wall.rs"),
        "{text}"
    );
    assert!(has_line(&text, "- read: lamp.md wall.md"), "{text}");
    assert!(has_line(&text, "- problem: no light in the room"), "{text}");
    assert!(has_line(&text, "- fix: add a lamp and wire it"), "{text}");
    assert!(
        has_line(&text, "- accept: first line second line"),
        "{text}"
    );
    assert!(
        has_line(&text, "- verify: cargo test --test lamp and more"),
        "{text}"
    );
    assert!(has_line(&text, "### T-003 · Second task"), "{text}");
    assert!(has_line(&text, "- why: from fields"), "{text}");
    assert!(has_line(&text, "- custom: one two"), "{text}");
}

#[test]
fn apply_set_turns_a_crlf_in_free_text_into_a_single_space() {
    let dir = TempDir::new().expect("tempdir");
    write_todo(dir.path(), ONE_TASK_TODO);

    let output = apply_ops(
        &dir,
        &[json!({
            "op": "set",
            "task": "T-001",
            "title": "Renamed\r\ntask",
            "why": "first\r\nsecond",
            "read": "x.md\r\ny.md",
            "accept": "a\nb",
            "verify": "run\ncheck",
            "fields": {"problem": "p1\r\np2"}
        })],
    );
    let text = applied_todo(&dir, &output);

    assert!(has_line(&text, "### T-001 · Renamed task"), "{text}");
    assert!(has_line(&text, "- why: first second"), "{text}");
    assert!(has_line(&text, "- read: x.md y.md"), "{text}");
    assert!(has_line(&text, "- accept: a b"), "{text}");
    assert!(has_line(&text, "- verify: run check"), "{text}");
    assert!(has_line(&text, "- problem: p1 p2"), "{text}");
}

#[test]
fn apply_add_keeps_each_line_of_steps_text_as_its_own_step() {
    let dir = TempDir::new().expect("tempdir");
    write_todo(dir.path(), EMPTY_TODO);

    let output = apply_ops(
        &dir,
        &[json!({
            "op": "add",
            "title": "Stepped task",
            "priority": "P2",
            "difficulty": "2",
            "why": "steps\nare kept",
            "steps": "first step\r\nsecond step\nthird step"
        })],
    );
    let text = applied_todo(&dir, &output);

    assert!(has_line(&text, "- why: steps are kept"), "{text}");
    assert!(has_line(&text, "- steps:"), "{text}");
    assert!(has_line(&text, "  1. first step"), "{text}");
    assert!(has_line(&text, "  2. second step"), "{text}");
    assert!(has_line(&text, "  3. third step"), "{text}");
}

#[test]
fn apply_set_keeps_each_line_of_steps_text_as_its_own_step() {
    let dir = TempDir::new().expect("tempdir");
    write_todo(dir.path(), ONE_TASK_TODO);

    let output = apply_ops(
        &dir,
        &[json!({
            "op": "set",
            "task": "T-001",
            "steps": "alpha\r\nbeta\ngamma"
        })],
    );
    let text = applied_todo(&dir, &output);

    assert!(has_line(&text, "- steps:"), "{text}");
    assert!(has_line(&text, "  1. alpha"), "{text}");
    assert!(has_line(&text, "  2. beta"), "{text}");
    assert!(has_line(&text, "  3. gamma"), "{text}");
}
