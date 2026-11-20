//! Integration tests for the level of the tasks that `promote` and `apply` make (T-169; R-LEVEL-1,
//! R-WRITE-18, TEST-4): a promoted idea or an `add` operation of `apply` needs a level, as `add`
//! requires one, unless `todo.toml` drops `difficulty` from `required_fields`. Milestones need none.
//!
//! The helpers panic on setup failures, which is acceptable in test code (see `tests/common`).
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

mod common;

use std::fs;
use std::path::Path;
use std::process::Output;

use common::{parse_json, read_text, run_todo, setup_proj, stderr_text, stdout_text, write_toml};
use tempfile::TempDir;

const MARKER: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->";

/// The idea `T-002` with no level, and no priority, as an idea is written.
const PLAIN_IDEA: &str = "### T-002 · Cache parsed files\n- status: open\n- owner: —\n- type: idea\n- note (2026-11-15, planner): added\n";

/// A TODO.md in the current format: one leveled task in `## Work` and the ideas in `## Ideas`.
fn doc_with_ideas(ideas: &str) -> String {
    format!(
        "{MARKER}\n<!-- todo-format: 3 -->\n\n## Work\n\n\
         ### T-001 · Draw the scene\n- status: open\n- owner: —\n- priority: P1 · difficulty: 2\n\n\
         ## Ideas\n\n{ideas}"
    )
}

/// Runs `todo` with `args` and expects a refusal with exit code 1.
fn refused(home: &Path, work: &Path, args: &[&str]) -> Output {
    let out = run_todo(home, work, args, &[]).unwrap();
    assert_eq!(
        out.status.code(),
        Some(1),
        "{args:?} should be refused:\n{}{}",
        stdout_text(&out),
        stderr_text(&out)
    );
    out
}

/// Runs `todo` with `args` and succeeds, returning its output.
fn ok(home: &Path, work: &Path, args: &[&str]) -> Output {
    let out = run_todo(home, work, args, &[]).unwrap();
    assert!(
        out.status.success(),
        "{args:?} failed:\n{}{}",
        stdout_text(&out),
        stderr_text(&out)
    );
    out
}

#[test]
fn promote_without_a_level_is_refused_with_missing_field_and_the_file_stays_byte_identical() {
    let tmp = TempDir::new().unwrap();
    let (home, work, file) = setup_proj(&tmp, &doc_with_ideas(PLAIN_IDEA));
    let before = fs::read(&file).unwrap();

    let out = refused(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "alice",
            "promote",
            "T-002",
            "--priority",
            "P1",
        ],
    );
    assert_eq!(parse_json(&out.stdout)["error"]["code"], "MISSING_FIELD");
    assert_eq!(fs::read(&file).unwrap(), before, "nothing is written");
}

#[test]
fn promote_with_a_level_writes_it_on_the_new_task() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, &doc_with_ideas(PLAIN_IDEA));

    let out = ok(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "alice",
            "promote",
            "T-002",
            "--priority",
            "P1",
            "--difficulty",
            "3",
        ],
    );
    assert_eq!(parse_json(&out.stdout)["data"]["difficulty"], "3");
    assert!(read_text(&work.join("TODO.md")).contains("- priority: P1 · difficulty: 3"));
}

#[test]
fn promote_keeps_the_level_the_idea_already_has() {
    let idea = "### T-002 · Cache parsed files\n- status: open\n- owner: —\n- priority: P3 · difficulty: 4 · type: idea\n";
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, &doc_with_ideas(idea));

    let out = ok(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "alice",
            "promote",
            "T-002",
            "--priority",
            "P1",
        ],
    );
    assert_eq!(parse_json(&out.stdout)["data"]["difficulty"], "4");
}

#[test]
fn promote_refuses_a_level_outside_the_configured_ones_before_writing() {
    let tmp = TempDir::new().unwrap();
    let (home, work, file) = setup_proj(&tmp, &doc_with_ideas(PLAIN_IDEA));
    let before = fs::read(&file).unwrap();

    let out = refused(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "alice",
            "promote",
            "T-002",
            "--priority",
            "P1",
            "--difficulty",
            "9",
        ],
    );
    assert_eq!(parse_json(&out.stdout)["error"]["code"], "BAD_VALUE");
    assert_eq!(fs::read(&file).unwrap(), before, "nothing is written");
}

#[test]
fn apply_add_without_a_level_is_refused_and_the_whole_batch_is_left_alone() {
    let tmp = TempDir::new().unwrap();
    let (home, work, file) = setup_proj(&tmp, &doc_with_ideas(""));
    let before = fs::read(&file).unwrap();
    fs::write(
        work.join("ops.jsonl"),
        concat!(
            r#"{"op":"add","title":"Leveled","priority":"P1","difficulty":"2","section":"Work"}"#,
            "\n",
            r#"{"op":"add","title":"No level","priority":"P1","section":"Work"}"#,
            "\n",
        ),
    )
    .unwrap();

    let out = refused(
        &home,
        &work,
        &["--json", "--agent", "alice", "apply", "ops.jsonl"],
    );
    assert_eq!(parse_json(&out.stdout)["error"]["code"], "MISSING_FIELD");
    assert_eq!(fs::read(&file).unwrap(), before, "no operation is applied");
}

#[test]
fn apply_add_of_a_milestone_needs_no_level() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, &doc_with_ideas(""));
    fs::write(
        work.join("ops.jsonl"),
        concat!(
            r#"{"op":"add","title":"Lighting","type":"milestone","priority":"P1","goal":"the scene is lit","section":"Work"}"#,
            "\n",
        ),
    )
    .unwrap();

    ok(&home, &work, &["--agent", "alice", "apply", "ops.jsonl"]);
    assert!(read_text(&work.join("TODO.md")).contains("### T-002 · Lighting"));
}

#[test]
fn todo_toml_without_difficulty_in_required_fields_lets_promote_and_apply_skip_the_level() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, &doc_with_ideas(PLAIN_IDEA));
    write_toml(&work, "required_fields = [\"priority\"]\n");

    ok(
        &home,
        &work,
        &["--agent", "alice", "promote", "T-002", "--priority", "P1"],
    );
    fs::write(
        work.join("ops.jsonl"),
        concat!(
            r#"{"op":"add","title":"No level","priority":"P2","section":"Work"}"#,
            "\n"
        ),
    )
    .unwrap();
    ok(&home, &work, &["--agent", "alice", "apply", "ops.jsonl"]);
    assert!(read_text(&work.join("TODO.md")).contains("### T-003 · No level"));
}
