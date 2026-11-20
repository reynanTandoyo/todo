//! Integration tests for the levels 1 to 5 (T-148; R-LEVEL-1, R-MIG-1, R-CFG-2, TEST-4): the default
//! `difficulties`, the fields a task needs to be added, what `next` hands out, what `check` reports,
//! the help text, and the format 2 to 3 migration on a write.
//!
//! The helpers panic on setup failures, which is acceptable in test code (see `tests/common`).
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

mod common;

use std::io::Write;
use std::path::Path;
use std::process::{Output, Stdio};

use common::{
    clean_todo_command, parse_json, read_todo, run_todo, setup_fixture_proj, setup_proj,
    stderr_text, stdout_text, write_toml,
};
use tempfile::TempDir;
use todo::ops::OpsConfig;

const MARKER: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->";

/// A TODO.md in the current format (3) with one `## Tasks` section that holds `body`.
fn doc(body: &str) -> String {
    format!("{MARKER}\n<!-- todo-format: 3 -->\n\n## Tasks\n\n{body}")
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

/// Runs `todo` with `args` and expects a refusal with exit code `code`.
fn refused_with(home: &Path, work: &Path, args: &[&str], code: i32) -> Output {
    let out = run_todo(home, work, args, &[]).unwrap();
    assert_eq!(
        out.status.code(),
        Some(code),
        "{args:?} should be refused with exit {code}:\n{}{}",
        stdout_text(&out),
        stderr_text(&out)
    );
    out
}

/// Runs `todo` with `args`, feeding `input` on standard input.
fn run_stdin(home: &Path, work: &Path, args: &[&str], input: &str) -> Output {
    let mut cmd = clean_todo_command(home, work);
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn the_default_levels_are_one_to_five_and_a_task_needs_a_priority_and_a_level() {
    let def = OpsConfig::default();
    assert_eq!(def.difficulties, vec!["1", "2", "3", "4", "5"]);
    assert_eq!(def.required_fields, vec!["priority", "difficulty"]);
    assert!(!def.difficulties_set, "todo.toml sets no difficulties");

    let set = OpsConfig::from_toml_str("difficulties = [\"easy\", \"hard\"]\n").expect("valid");
    assert!(set.difficulties_set, "a todo.toml that sets them is noted");
    assert_eq!(set.difficulties, vec!["easy", "hard"]);
}

#[test]
fn help_add_and_help_set_say_what_each_level_means_and_name_no_model() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, &doc(""));

    for command in ["add", "set"] {
        let out = ok(&home, &work, &["help", command]);
        let text = stdout_text(&out);
        for line in [
            "  1  Everything is written down",
            "  2  Small and clear",
            "  3  Needs an understanding",
            "  4  Spans several parts",
            "  5  Open: needs research",
        ] {
            assert!(
                text.contains(line),
                "todo help {command} lacks {line:?}:\n{text}"
            );
        }
        for model in ["Claude", "Haiku", "Sonnet", "Opus", "Fable", "GPT", "Codex"] {
            assert!(!text.contains(model), "todo help {command} names {model}");
        }
    }

    let out = ok(&home, &work, &["help", "list"]);
    assert!(
        !stdout_text(&out).contains("Levels ("),
        "only add and set print the levels"
    );

    let out = ok(&home, &work, &["--json", "help", "add"]);
    let details = parse_json(&out.stdout)["data"]["details"]
        .as_str()
        .expect("details")
        .to_string();
    assert!(details.contains("Everything is written down"), "{details}");
}

#[test]
fn add_without_a_difficulty_is_refused_with_missing_field_and_the_file_stays_byte_identical() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Existing\n- status: open\n- owner: —\n- priority: P1 · difficulty: 2\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));
    let before = std::fs::read(&file).unwrap();

    let out = refused_with(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "t-agent",
            "add",
            "--title",
            "Needs a level",
            "--priority",
            "P1",
        ],
        1,
    );
    assert_eq!(parse_json(&out.stdout)["error"]["code"], "MISSING_FIELD");
    assert_eq!(std::fs::read(&file).unwrap(), before);
}

#[test]
fn add_refuses_a_level_outside_the_configured_ones_and_keeps_the_file() {
    let tmp = TempDir::new().unwrap();
    let (home, work, file) = setup_proj(&tmp, &doc(""));
    let before = std::fs::read(&file).unwrap();

    let out = refused_with(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "t-agent",
            "add",
            "--title",
            "Too hard",
            "--priority",
            "P1",
            "--difficulty",
            "6",
        ],
        1,
    );
    assert_eq!(parse_json(&out.stdout)["error"]["code"], "BAD_VALUE");
    assert_eq!(std::fs::read(&file).unwrap(), before);
}

#[test]
fn add_with_a_priority_and_a_level_writes_the_task_with_its_level() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, &doc(""));

    ok(
        &home,
        &work,
        &[
            "--agent",
            "t-agent",
            "add",
            "--title",
            "Leveled",
            "--priority",
            "P1",
            "--difficulty",
            "2",
        ],
    );
    let text = read_todo(&work);
    assert!(text.contains("- priority: P1 · difficulty: 2\n"), "{text}");
}

#[test]
fn add_still_needs_a_priority_for_a_task_as_well_as_a_level() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, &doc(""));

    let out = refused_with(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "t-agent",
            "add",
            "--title",
            "No priority",
            "--difficulty",
            "1",
        ],
        1,
    );
    assert_eq!(parse_json(&out.stdout)["error"]["code"], "MISSING_FIELD");
}

#[test]
fn add_from_refuses_the_whole_batch_when_one_task_has_no_difficulty() {
    let tmp = TempDir::new().unwrap();
    let (home, work, file) = setup_proj(&tmp, &doc(""));
    let before = std::fs::read(&file).unwrap();

    let input = "{\"title\": \"First level\", \"priority\": \"P1\", \"difficulty\": \"2\"}\n\
                 {\"title\": \"No level\", \"priority\": \"P1\"}\n";
    let out = run_stdin(
        &home,
        &work,
        &["--json", "--agent", "t-agent", "add", "--from", "-"],
        input,
    );
    assert_eq!(out.status.code(), Some(1), "{}", stdout_text(&out));
    assert_eq!(parse_json(&out.stdout)["error"]["code"], "MISSING_FIELD");
    assert_eq!(std::fs::read(&file).unwrap(), before, "no task is written");
}

#[test]
fn a_milestone_a_question_and_an_idea_need_neither_a_priority_nor_a_level() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, &doc(""));

    ok(
        &home,
        &work,
        &[
            "--agent",
            "t-agent",
            "add",
            "--title",
            "Lighting",
            "--type",
            "milestone",
            "--goal",
            "the test scene is lit",
        ],
    );
    ok(
        &home,
        &work,
        &[
            "--agent",
            "t-agent",
            "add",
            "--title",
            "Which paint lasts?",
            "--type",
            "question",
        ],
    );
    ok(
        &home,
        &work,
        &[
            "--agent",
            "t-agent",
            "add",
            "--title",
            "Glow paint",
            "--type",
            "idea",
        ],
    );
    let text = read_todo(&work);
    assert!(text.contains("### T-001 · Lighting"), "{text}");
    assert!(text.contains("### T-002 · Which paint lasts?"), "{text}");
    assert!(text.contains("### T-003 · Glow paint"), "{text}");
}

#[test]
fn todo_toml_can_drop_the_difficulty_from_required_fields() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, &doc(""));
    write_toml(&work, "required_fields = [\"priority\"]\n");

    ok(
        &home,
        &work,
        &[
            "--agent",
            "t-agent",
            "add",
            "--title",
            "Without a level",
            "--priority",
            "P1",
        ],
    );
    assert!(read_todo(&work).contains("### T-001 · Without a level"));
}

#[test]
fn next_and_list_ready_never_hand_out_a_task_without_a_level() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Urgent without a level\n- status: open\n- priority: P0\n\n\
                ### T-002 · Level two\n- status: open\n- priority: P2 · difficulty: 2\n";
    let (home, work, _) = setup_proj(&tmp, &doc(body));

    let next = ok(&home, &work, &["--json", "next", "--agent", "t-other"]);
    assert_eq!(parse_json(&next.stdout)["data"]["task"]["id"], "T-002");

    let ready = ok(&home, &work, &["list", "--ready"]);
    let text = stdout_text(&ready);
    assert!(text.contains("T-002"), "{text}");
    assert!(!text.contains("T-001"), "{text}");

    // Once the only leveled task is taken, nothing is handed out: T-001 has no level.
    ok(&home, &work, &["--agent", "t-agent", "next", "--claim"]);
    let none = ok(&home, &work, &["--json", "next", "--agent", "t-other"]);
    assert!(parse_json(&none.stdout)["data"]["task"].is_null());
    assert!(read_todo(&work).contains("### T-001 · Urgent without a level\n- status: open\n"));
}

#[test]
fn check_does_not_report_an_old_name_on_a_done_or_dropped_entry() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Finished\n- status: done\n- priority: P1 · difficulty: easy\n\n\
                ### T-002 · Dropped\n- status: dropped\n- priority: P2 · difficulty: hard\n\n\
                ### T-003 · Still open\n- status: open\n- owner: —\n- priority: P2 · difficulty: easy\n";
    let (home, work, _) = setup_proj(&tmp, &doc(body));

    let out = run_todo(&home, &work, &["check"], &[]).unwrap();
    let text = format!("{}{}", stdout_text(&out), stderr_text(&out));
    assert!(
        text.contains("T-003: difficulty \"easy\" is not in configured difficulties"),
        "an open entry with an old name is still reported: {text}"
    );
    assert!(!text.contains("T-001: difficulty"), "{text}");
    assert!(!text.contains("T-002: difficulty"), "{text}");
}

#[test]
fn a_format_four_file_is_refused_with_tool_too_old() {
    let tmp = TempDir::new().unwrap();
    let body =
        "<!-- todo-format: 4 -->\n# Plans\n\n## Tasks\n\n### T-001 · Later\n- status: open\n";
    let (home, work, file) = setup_proj(&tmp, &format!("{MARKER}\n{body}"));
    let before = std::fs::read(&file).unwrap();

    let out = refused_with(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "t-agent",
            "add",
            "--title",
            "Nope",
            "--priority",
            "P1",
            "--difficulty",
            "1",
        ],
        4,
    );
    assert_eq!(parse_json(&out.stdout)["error"]["code"], "TOOL_TOO_OLD");
    assert_eq!(std::fs::read(&file).unwrap(), before, "nothing is written");
}

#[test]
fn a_write_migrates_format_two_to_three_and_rewrites_the_old_names_of_open_entries() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_fixture_proj(&tmp, "migrate_format2_levels.md");

    let out = ok(
        &home,
        &work,
        &[
            "--agent",
            "t-agent",
            "add",
            "--title",
            "Dredge the mouth",
            "--priority",
            "P1",
            "--difficulty",
            "3",
            "--section",
            "Now",
        ],
    );
    assert!(
        stderr_text(&out).contains("TODO.md: format 2 to 3"),
        "{}",
        stderr_text(&out)
    );

    let text = read_todo(&work);
    assert!(text.contains("<!-- todo-format: 3 -->\n"), "{text}");
    assert!(
        text.contains("- priority: P1 · difficulty: 2 · type: feature · area: buoys\n"),
        "{text}"
    );
    assert!(text.contains("- difficulty: 4\n"), "{text}");
    // Done and dropped entries keep their old names as a record.
    assert!(
        text.contains("- priority: P3 · difficulty: easy\n"),
        "{text}"
    );
    assert!(
        text.contains("- difficulty: hard\n- note (2026-11-16"),
        "{text}"
    );
    assert!(text.contains("### T-009 · Dredge the mouth"), "{text}");
}

#[test]
fn a_todo_toml_with_its_own_difficulties_keeps_the_old_names_and_only_moves_the_format() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_fixture_proj(&tmp, "migrate_format2_levels.md");
    write_toml(&work, "difficulties = [\"easy\", \"medium\", \"hard\"]\n");

    ok(
        &home,
        &work,
        &[
            "--agent",
            "t-agent",
            "add",
            "--title",
            "Dredge the mouth",
            "--priority",
            "P1",
            "--difficulty",
            "easy",
            "--section",
            "Now",
        ],
    );
    let text = read_todo(&work);
    assert!(text.contains("<!-- todo-format: 3 -->\n"), "{text}");
    assert!(
        text.contains("- priority: P1 · difficulty: easy · type: feature · area: buoys\n"),
        "{text}"
    );
    assert!(
        text.contains("- difficulty: hard\n"),
        "the claimed task keeps its name: {text}"
    );
}

#[test]
fn migrate_yes_moves_a_format_two_file_to_the_levels_and_a_second_run_changes_nothing() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_fixture_proj(&tmp, "migrate_format2_levels.md");

    let out = ok(&home, &work, &["migrate", "--yes", "--agent", "t-agent"]);
    assert!(
        stdout_text(&out).contains("format 2 to 3") || stderr_text(&out).contains("format 2 to 3"),
        "{}{}",
        stdout_text(&out),
        stderr_text(&out)
    );
    let once = read_todo(&work);
    assert!(
        once.contains("- priority: P1 · difficulty: 2 · type: feature"),
        "{once}"
    );

    ok(&home, &work, &["migrate", "--yes", "--agent", "t-agent"]);
    assert_eq!(read_todo(&work), once, "a second run changes nothing");
}
