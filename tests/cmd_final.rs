//! Integration tests for the `final` field (T-150; R-LEVEL-3): `set --final`, `set --unset final`,
//! its refusals, `check`, `show --json`, `schema`, and the count `summary` shows apart.
//!
//! The helpers panic on setup failures, which is acceptable in test code (see `tests/common`).
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

mod common;

use std::fs;
use std::path::Path;
use std::process::Output;

use common::{parse_json, read_text, run_todo, setup_proj, stderr_text, stdout_text};
use tempfile::TempDir;

const MARKER: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->";

/// A format 3 TODO.md with one `## Tasks` section that holds `body`.
fn doc(body: &str) -> String {
    format!("{MARKER}\n<!-- todo-format: 3 -->\n\n## Tasks\n\n{body}")
}

/// The same file after a write: the running tool stamp directly after the format line (R-VER-1).
fn written(body: &str) -> String {
    format!(
        "{MARKER}\n<!-- todo-format: 3 -->\n<!-- todo-tool: {} -->\n\n## Tasks\n\n{body}",
        env!("CARGO_PKG_VERSION")
    )
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

/// Runs `todo --json` with `args`, expects a refusal (exit 1, `BAD_VALUE`) and returns its message.
fn refused_bad_value(home: &Path, work: &Path, args: &[&str]) -> String {
    let mut full = vec!["--json", "--agent", "t-agent"];
    full.extend_from_slice(args);
    let out = run_todo(home, work, &full, &[]).unwrap();
    assert_eq!(
        out.status.code(),
        Some(1),
        "{args:?} should be refused:\n{}{}",
        stdout_text(&out),
        stderr_text(&out)
    );
    let json = parse_json(&out.stdout);
    assert_eq!(json["error"]["code"], "BAD_VALUE", "{json}");
    json["error"]["message"]
        .as_str()
        .expect("error message")
        .to_string()
}

#[test]
fn set_final_is_written_in_canonical_order_replaced_in_place_and_unset() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Parse the config\n- status: open\n- owner: —\n\
                - priority: P1 · difficulty: 3\n- why: the parser is part of the plan\n\
                - accept: it parses\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));

    ok(
        &home,
        &work,
        &[
            "--agent",
            "t-agent",
            "set",
            "T-001",
            "--final",
            "the design is still open",
        ],
    );
    assert_eq!(
        read_text(&file),
        written(
            "### T-001 · Parse the config\n- status: open\n- owner: —\n\
             - priority: P1 · difficulty: 3\n- why: the parser is part of the plan\n\
             - final: the design is still open\n- accept: it parses\n"
        ),
        "final goes right after why, the field before it in format.md §10"
    );

    ok(
        &home,
        &work,
        &[
            "--agent",
            "t-agent",
            "set",
            "T-001",
            "--final",
            "the parser needs a decision first",
        ],
    );
    assert_eq!(
        read_text(&file),
        written(
            "### T-001 · Parse the config\n- status: open\n- owner: —\n\
             - priority: P1 · difficulty: 3\n- why: the parser is part of the plan\n\
             - final: the parser needs a decision first\n- accept: it parses\n"
        ),
        "a second set replaces the line in place"
    );

    ok(
        &home,
        &work,
        &["--agent", "t-agent", "set", "T-001", "--unset", "final"],
    );
    assert_eq!(
        read_text(&file),
        written(body),
        "--unset final removes its line and nothing else"
    );
}

#[test]
fn set_final_is_refused_at_the_target_level_and_leaves_the_file_alone() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Rename a constant\n- status: open\n- owner: —\n\
                - priority: P2 · difficulty: 1\n- accept: the build passes\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));
    let before = read_text(&file);

    let message = refused_bad_value(
        &home,
        &work,
        &["set", "T-001", "--final", "needs more thought"],
    );
    assert!(message.contains("at the target level 1"), "{message}");

    let message = refused_bad_value(
        &home,
        &work,
        &["set", "T-001", "--field", "final=needs more thought"],
    );
    assert!(
        message.contains("at the target level 1"),
        "--field final= takes the same path: {message}"
    );
    assert_eq!(
        read_text(&file),
        before,
        "a refused set leaves TODO.md alone"
    );
}

#[test]
fn set_final_needs_a_reason_and_leaves_the_file_alone() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Renderer lighting\n- status: open\n- owner: —\n\
                - priority: P1 · difficulty: 3\n- accept: the scene is lit\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));
    let before = read_text(&file);

    for args in [
        vec!["set", "T-001", "--final", ""],
        vec!["set", "T-001", "--final", "   "],
        vec!["set", "T-001", "--field", "final="],
    ] {
        let message = refused_bad_value(&home, &work, &args);
        assert!(message.contains("needs a reason"), "{args:?}: {message}");
    }
    assert_eq!(read_text(&file), before, "an empty reason is never written");
}

#[test]
fn set_final_is_refused_for_a_milestone_question_and_idea() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Renderer\n- status: open\n- owner: —\n\
                - priority: P1 · type: milestone\n- goal: the scene renders\n\n\
                ### T-002 · Should shadows be in scope?\n- status: open\n- owner: —\n\
                - type: question\n\n\
                ### T-003 · Maybe a ray tracer\n- status: open\n- owner: —\n- type: idea\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));
    let before = read_text(&file);

    for id in ["T-001", "T-002", "T-003"] {
        let message = refused_bad_value(&home, &work, &["set", id, "--final", "kept on purpose"]);
        assert!(
            message.contains("applies only to a task"),
            "{id}: {message}"
        );
    }
    assert_eq!(read_text(&file), before, "no entry of another kind changes");
}

#[test]
fn set_final_takes_the_level_given_in_the_same_set_and_refuses_without_a_level() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · No level yet\n- status: open\n- owner: —\n- priority: P1\n\
                - accept: a level is given\n\n\
                ### T-002 · Parse the config\n- status: open\n- owner: —\n\
                - priority: P2 · difficulty: 1\n- accept: it parses\n\n\
                ### T-003 · Renderer lighting\n- status: open\n- owner: —\n\
                - priority: P1 · difficulty: 3\n- accept: the scene is lit\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));

    let message = refused_bad_value(&home, &work, &["set", "T-001", "--final", "later"]);
    assert!(
        message.contains("has no configured level above the target 1"),
        "{message}"
    );

    let message = refused_bad_value(
        &home,
        &work,
        &["set", "T-003", "--difficulty", "1", "--final", "later"],
    );
    assert!(message.contains("at the target level 1"), "{message}");
    assert_eq!(
        read_text(&file),
        doc(body),
        "the two refused sets wrote nothing, so the file has no tool stamp yet"
    );

    ok(
        &home,
        &work,
        &[
            "--agent",
            "t-agent",
            "set",
            "T-002",
            "--difficulty",
            "3",
            "--final",
            "the design is open",
        ],
    );
    assert_eq!(
        read_text(&file),
        written(
            "### T-001 · No level yet\n- status: open\n- owner: —\n- priority: P1\n\
             - accept: a level is given\n\n\
             ### T-002 · Parse the config\n- status: open\n- owner: —\n\
             - priority: P2 · difficulty: 3\n- final: the design is open\n\
             - accept: it parses\n\n\
             ### T-003 · Renderer lighting\n- status: open\n- owner: —\n\
             - priority: P1 · difficulty: 3\n- accept: the scene is lit\n"
        ),
        "a level above the target given in the same set makes the final allowed"
    );
}

#[test]
fn final_on_the_combined_priority_line_is_read_and_unset_in_place() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Parse the config\n- status: open\n- owner: —\n\
                - priority: P1 · difficulty: 3 · final: the design is still open\n\
                - accept: it parses\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));

    let show = parse_json(&ok(&home, &work, &["--json", "show", "T-001"]).stdout);
    assert_eq!(show["data"]["final"], "the design is still open");
    assert!(
        show["data"]["fields"].get("final").is_none(),
        "final is a known field, not an unknown one: {}",
        show["data"]["fields"]
    );

    ok(
        &home,
        &work,
        &["--agent", "t-agent", "set", "T-001", "--unset", "final"],
    );
    assert_eq!(
        read_text(&file),
        written(
            "### T-001 · Parse the config\n- status: open\n- owner: —\n\
             - priority: P1 · difficulty: 3\n- accept: it parses\n"
        ),
        "only the final part leaves the priority line"
    );
}

#[test]
fn set_final_reads_its_reason_from_a_file_with_at_path() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Renderer lighting\n- status: open\n- owner: —\n\
                - priority: P1 · difficulty: 4\n- accept: the scene is lit\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));
    fs::write(
        work.join("why.txt"),
        "the lighting model needs a decision\n",
    )
    .unwrap();

    ok(
        &home,
        &work,
        &["--agent", "t-agent", "set", "T-001", "--final", "@why.txt"],
    );
    assert_eq!(
        read_text(&file),
        written(
            "### T-001 · Renderer lighting\n- status: open\n- owner: —\n\
             - priority: P1 · difficulty: 4\n\
             - final: the lighting model needs a decision\n- accept: the scene is lit\n"
        )
    );
}

#[test]
fn check_reports_a_final_on_an_entry_that_cannot_carry_one_and_on_a_task_at_the_target() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Renderer\n- status: open\n- owner: —\n\
                - priority: P1 · type: milestone\n- goal: the scene renders\n\
                - final: not for a milestone\n\n\
                ### T-002 · Should shadows be in scope?\n- status: open\n- owner: —\n\
                - type: question\n- final: not for a question\n\n\
                ### T-003 · Maybe a ray tracer\n- status: open\n- owner: —\n\
                - type: idea\n- final: not for an idea\n\n\
                ### T-004 · Parse the config\n- status: open\n- owner: —\n\
                - priority: P1 · difficulty: 1\n- final: written by hand\n\n\
                ### T-005 · Renderer lighting\n- status: open\n- owner: —\n\
                - priority: P2 · difficulty: 3\n- final: the design is open\n";
    let (home, work, _) = setup_proj(&tmp, &doc(body));

    let out = run_todo(&home, &work, &["check"], &[]).unwrap();
    let text = format!("{}{}", stdout_text(&out), stderr_text(&out));
    assert_eq!(out.status.code(), Some(1), "{text}");
    for expected in [
        "T-001: final on milestone entry; only a task can stay above the target level",
        "T-002: final on question entry; only a task can stay above the target level",
        "T-003: final on idea entry; only a task can stay above the target level",
        "T-004: final on a task at the target level 1; split it or give it a higher level",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in:\n{text}");
    }
    assert!(
        !text.contains("T-005: final"),
        "a task above the target may keep its final: {text}"
    );
}

#[test]
fn show_json_and_schema_list_final() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Renderer lighting\n- status: open\n- owner: —\n\
                - priority: P1 · difficulty: 4\n- final: the design is open\n\n\
                ### T-002 · Parse the config\n- status: open\n- owner: —\n\
                - priority: P2 · difficulty: 2\n";
    let (home, work, _) = setup_proj(&tmp, &doc(body));

    let show = parse_json(&ok(&home, &work, &["--json", "show", "T-001"]).stdout);
    assert_eq!(show["data"]["final"], "the design is open");
    let plain = parse_json(&ok(&home, &work, &["--json", "show", "T-002"]).stdout);
    assert!(plain["data"]["final"].is_null(), "{}", plain["data"]);

    let schema = parse_json(&ok(&home, &work, &["--json", "schema"]).stdout);
    let fields: Vec<&str> = schema["data"]["task"]["fields"]
        .as_array()
        .expect("task fields")
        .iter()
        .filter_map(|f| f.as_str())
        .collect();
    assert!(fields.contains(&"final"), "{fields:?}");
}

#[test]
fn summary_counts_open_and_blocked_tasks_kept_above_the_target_apart() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Renderer lighting\n- status: open\n- owner: —\n\
                - priority: P1 · difficulty: 3\n- final: the design is open\n\n\
                ### T-002 · Claimed work\n- status: claimed\n- owner: alice\n\
                - claimed: 2026-11-15T18:00+01:00\n- priority: P1 · difficulty: 3\n\
                - final: kept by alice\n\n\
                ### T-003 · Finished work\n- status: done\n- owner: alice\n\
                - priority: P1 · difficulty: 3\n- final: kept once\n\n\
                ### T-004 · Level one\n- status: open\n- owner: —\n\
                - priority: P1 · difficulty: 1\n- final: wrongly kept\n\n\
                ### T-005 · Blocked work\n- status: blocked\n- owner: —\n\
                - priority: P2 · difficulty: 4\n- final: the scope is open\n\
                - note (2026-11-15, alice): blocked — waiting for a decision\n";
    let (home, work, _) = setup_proj(&tmp, &doc(body));

    let text = stdout_text(&ok(&home, &work, &["summary"]));
    assert!(
        text.contains("kept above the target level on purpose: 2"),
        "{text}"
    );

    let json = parse_json(&ok(&home, &work, &["--json", "summary"]).stdout);
    assert_eq!(json["data"]["kept_on_purpose"], 2, "{json}");
}

#[test]
fn summary_leaves_the_count_out_when_no_task_is_kept_above_the_target() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Renderer lighting\n- status: open\n- owner: —\n\
                - priority: P1 · difficulty: 3\n- accept: the scene is lit\n";
    let (home, work, _) = setup_proj(&tmp, &doc(body));

    let text = stdout_text(&ok(&home, &work, &["summary"]));
    assert!(!text.contains("kept above the target level"), "{text}");

    let json = parse_json(&ok(&home, &work, &["--json", "summary"]).stdout);
    assert!(json["data"].get("kept_on_purpose").is_none(), "{json}");
}

#[test]
fn split_removes_final_when_the_task_becomes_a_milestone() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Renderer lighting\n- status: open\n- owner: —\n\
                - priority: P1 · difficulty: 4\n- final: the design is still open\n\
                - accept: the scene is lit\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));
    fs::write(
        work.join("children.jsonl"),
        "{\"title\":\"Light the floor\",\"priority\":\"P1\",\"difficulty\":\"2\"}\n",
    )
    .unwrap();

    ok(
        &home,
        &work,
        &[
            "--agent",
            "planner",
            "split",
            "T-001",
            "--from",
            "children.jsonl",
        ],
    );
    let text = read_text(&file);
    assert!(
        text.contains("type: milestone"),
        "the task became a milestone: {text}"
    );
    assert!(
        !text.contains("- final:"),
        "split removes final, which belongs to a task: {text}"
    );
}
