//! Integration tests for `todo show` (T-021; R-READ-2, R-DEP-2).

mod common;

use common::{parse_json, read_todo, setup_fixture_proj, stderr_text, stdout_text};

use std::fs;

use tempfile::TempDir;

#[test]
fn show_text_output_equals_task_lines_in_file_and_accepts_flexible_ids() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_fixture_proj(&tmp, "clean_basic.md");

    let file_text = read_todo(&work);
    let file_lines: Vec<&str> = file_text.lines().collect();
    // T-001 occupies 1-based lines 9..=24 (0-based 8..24).
    let expected = format!("{}\n", file_lines[8..24].join("\n"));

    for query in ["T-001", "t1", "1", "T-01"] {
        let out = common::run_todo(&home, &work, &["show", query], &[]).unwrap();
        assert!(
            out.status.success(),
            "show {query} failed: {}",
            stderr_text(&out)
        );
        let stdout = String::from_utf8(out.stdout).unwrap();
        assert_eq!(stdout, expected, "mismatch for query `{query}`");
    }
}

#[cfg(feature = "test-hooks")]
#[test]
fn show_json_includes_unknown_fields_notes_lines_dependencies_and_dependents() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_fixture_proj(&tmp, "clean_basic.md");

    // T-001 has unknown field `custom_tag`, 2 notes, lines 9..24, and T-002 depends on it.
    let out1 = common::run_todo(
        &home,
        &work,
        &["--json", "show", "T-001"],
        &[("TODO_TEST_NOW", "2026-11-15T15:00:00+01:00")],
    )
    .unwrap();
    assert!(out1.status.success());
    let j1: serde_json::Value = parse_json(&out1.stdout);
    assert_eq!(j1["schema"], "todo.show.v1");
    assert_eq!(j1["ok"], true);
    assert_eq!(j1["data"]["id"], "T-001");
    assert_eq!(j1["data"]["section"], "1. Now");
    assert_eq!(j1["data"]["archived"], false);
    assert_eq!(j1["data"]["line_start"], 9);
    assert_eq!(j1["data"]["line_end"], 24);
    assert_eq!(j1["data"]["fields"]["custom_tag"], "morning-shift");
    assert_eq!(j1["data"]["notes"].as_array().unwrap().len(), 2);
    assert_eq!(j1["data"]["notes"][0]["date"], "2026-11-15");
    assert_eq!(j1["data"]["notes"][0]["agent"], "coordinator");
    assert_eq!(j1["data"]["notes"][0]["text"], "added");
    assert_eq!(j1["data"]["notes"][0]["line"], 19);
    assert_eq!(j1["data"]["dependents"].as_array().unwrap().len(), 1);
    assert_eq!(j1["data"]["dependents"][0]["id"], "T-002");
    assert_eq!(j1["data"]["dependents"][0]["status"], "open");

    // T-002 depends on T-001 (which is claimed -> not satisfied).
    let out2 = common::run_todo(
        &home,
        &work,
        &["--json", "show", "T-002"],
        &[("TODO_TEST_NOW", "2026-11-15T15:00:00+01:00")],
    )
    .unwrap();
    assert!(out2.status.success());
    let j2: serde_json::Value = parse_json(&out2.stdout);
    assert_eq!(j2["data"]["dependencies"].as_array().unwrap().len(), 1);
    assert_eq!(j2["data"]["dependencies"][0]["id"], "T-001");
    assert_eq!(j2["data"]["dependencies"][0]["exists"], true);
    assert_eq!(j2["data"]["dependencies"][0]["status"], "claimed");
    assert_eq!(j2["data"]["dependencies"][0]["satisfied"], false);
}

#[test]
fn show_finds_archived_tasks_and_archived_dependencies() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let work = tmp.path().join("work");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&work).unwrap();

    fs::write(
        work.join("TODO.md"),
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n## Now\n\n\
         ### T-002 · Follows archived work\n- status: open\n- depends: T-001, T-999\n",
    )
    .unwrap();
    fs::write(
        work.join("TODO-archive.md"),
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n## Now\n\n\
         ### T-001 · Archived step\n- status: done\n- owner: alice\n",
    )
    .unwrap();

    let arch_show = common::run_todo(&home, &work, &["--json", "show", "T-001"], &[]).unwrap();
    assert!(arch_show.status.success());
    let aj: serde_json::Value = parse_json(&arch_show.stdout);
    assert_eq!(aj["data"]["archived"], true);
    assert_eq!(aj["data"]["status"], "done");
    assert_eq!(aj["data"]["dependents"][0]["id"], "T-002");

    let active_show = common::run_todo(&home, &work, &["--json", "show", "T-002"], &[]).unwrap();
    assert!(active_show.status.success());
    let tj: serde_json::Value = parse_json(&active_show.stdout);
    assert_eq!(tj["data"]["dependencies"][0]["id"], "T-001");
    assert_eq!(tj["data"]["dependencies"][0]["archived"], true);
    assert_eq!(tj["data"]["dependencies"][0]["status"], "done");
    assert_eq!(tj["data"]["dependencies"][0]["satisfied"], true);
    assert_eq!(tj["data"]["dependencies"][1]["id"], "T-999");
    assert_eq!(tj["data"]["dependencies"][1]["exists"], false);
    assert_eq!(tj["data"]["dependencies"][1]["satisfied"], false);
}

#[test]
fn show_missing_task_returns_not_found_and_unsafe_file_still_warns() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_fixture_proj(&tmp, "clean_basic.md");

    let missing = common::run_todo(&home, &work, &["--json", "show", "T-999"], &[]).unwrap();
    assert_eq!(missing.status.code(), Some(1));
    let mj: serde_json::Value = parse_json(&missing.stdout);
    assert_eq!(mj["error"]["code"], "NOT_FOUND");
    assert_eq!(mj["error"]["task"], "T-999");

    // Unsafe file (conflict marker) still allows show with a warning.
    fs::write(
        work.join("TODO.md"),
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n## Now\n\n\
         ### T-001 · Readable despite marker\n- status: open\n\n<<<<<<< HEAD\n",
    )
    .unwrap();
    let unsafe_show = common::run_todo(&home, &work, &["show", "T-001"], &[]).unwrap();
    assert!(unsafe_show.status.success());
    assert!(stdout_text(&unsafe_show).contains("Readable despite marker"));
    assert!(stderr_text(&unsafe_show).contains("warning: TODO.md:"));
}

/// A milestone (T-001) with a task (T-002) below it, and two questions about them: T-003 about
/// the task is open, T-004 about the milestone is done.
const QUESTIONS_BODY: &str = "<!-- todo-format: 1 -->\n\n## Tasks\n\n### T-001 · Develop the renderer\n- status: open\n- owner: —\n- priority: P1 · type: milestone\n- goal: the test scene renders with lighting at 60 frames per second\n- note (2026-11-18, planner): added\n\n### T-002 · C shader\n- status: open\n- owner: —\n- priority: P1 · type: feature\n- milestone: T-001\n- note (2026-11-18, planner): added\n\n## Questions\n\n### T-003 · Should the C shader support shadows?\n- status: open\n- owner: —\n- type: question\n- about: T-002\n- note (2026-11-18, agent-1): asked\n\n### T-004 · Is the goal reachable on the test PC?\n- status: done\n- owner: human\n- type: question\n- about: T-001\n- note (2026-11-18, human): answer — yes\n";

#[test]
fn show_lists_questions_about_a_task_or_milestone_in_text_and_json() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = common::setup_proj(&tmp, QUESTIONS_BODY);

    // Text: the block as in the file, then the questions about the task with their status.
    let task_text = common::run_todo(&home, &work, &["show", "T-002"], &[]).unwrap();
    assert!(task_text.status.success(), "{}", stderr_text(&task_text));
    assert_eq!(
        stdout_text(&task_text),
        "### T-002 · C shader\n- status: open\n- owner: —\n- priority: P1 · type: feature\n- milestone: T-001\n- note (2026-11-18, planner): added\n\nQuestions about T-002:\n- T-003 · open · Should the C shader support shadows?\n"
    );

    // Text: a milestone lists its own questions too, including a done one.
    let milestone_text = common::run_todo(&home, &work, &["show", "T-001"], &[]).unwrap();
    assert!(
        stdout_text(&milestone_text).ends_with(
            "\nQuestions about T-001:\n- T-004 · done · Is the goal reachable on the test PC?\n"
        ),
        "unexpected output:\n{}",
        stdout_text(&milestone_text)
    );

    // JSON: the same questions under `questions`, with ID, title, status and archived.
    let task_json = common::run_todo(&home, &work, &["--json", "show", "T-002"], &[]).unwrap();
    let tj = parse_json(&task_json.stdout);
    assert_eq!(tj["data"]["questions"].as_array().unwrap().len(), 1);
    assert_eq!(tj["data"]["questions"][0]["id"], "T-003");
    assert_eq!(tj["data"]["questions"][0]["status"], "open");
    assert_eq!(
        tj["data"]["questions"][0]["title"],
        "Should the C shader support shadows?"
    );
    assert_eq!(tj["data"]["questions"][0]["archived"], false);

    let milestone_json = common::run_todo(&home, &work, &["--json", "show", "T-001"], &[]).unwrap();
    let mj = parse_json(&milestone_json.stdout);
    assert_eq!(mj["data"]["questions"][0]["id"], "T-004");
    assert_eq!(mj["data"]["questions"][0]["status"], "done");

    // A question itself, and an entry with no questions, get an empty list and no extra text.
    let question_json = common::run_todo(&home, &work, &["--json", "show", "T-003"], &[]).unwrap();
    assert_eq!(
        parse_json(&question_json.stdout)["data"]["questions"],
        serde_json::json!([])
    );
    let plain = common::run_todo(&home, &work, &["show", "T-003"], &[]).unwrap();
    assert!(!stdout_text(&plain).contains("Questions about"));
}

#[test]
fn show_lists_archived_questions_marked_as_archived() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = common::setup_proj(
        &tmp,
        "<!-- todo-format: 1 -->\n\n## Tasks\n\n### T-002 · C shader\n- status: open\n- owner: —\n- priority: P1 · type: feature\n",
    );
    fs::write(
        work.join("TODO-archive.md"),
        "<!-- todo-format: 1 -->\n\n## Questions\n\n### T-009 · Old shadow question\n- status: done\n- owner: human\n- type: question\n- about: T-002\n- note (2026-11-01, human): answer — no\n",
    )
    .unwrap();

    let out = common::run_todo(&home, &work, &["show", "T-002"], &[]).unwrap();
    assert!(out.status.success(), "{}", stderr_text(&out));
    assert!(
        stdout_text(&out).ends_with(
            "\nQuestions about T-002:\n- T-009 · done · Old shadow question (archived)\n"
        ),
        "unexpected output:\n{}",
        stdout_text(&out)
    );

    let json = common::run_todo(&home, &work, &["--json", "show", "T-002"], &[]).unwrap();
    let j = parse_json(&json.stdout);
    assert_eq!(j["data"]["questions"][0]["id"], "T-009");
    assert_eq!(j["data"]["questions"][0]["archived"], true);
}
