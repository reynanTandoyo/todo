//! Tests for `todo ask` and `todo answer` (T-031; R-WRITE-11) and questions about an entry
//! (T-098; R-KIND-2).

mod common;

use common::{parse_json, read_text, read_todo, run_todo, setup_proj, stderr_text, stdout_text};

use std::fs;

use serde_json::Value;
use tempfile::TempDir;

#[test]
fn ask_adds_questions_section_next_skips_questions_and_answer_completes() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("TODO.md");
    fs::write(&path, "<!-- todo-format: 1 -->\n\n## Tasks\n").unwrap();

    // Ask a question when `Questions` section does not exist yet.
    let ask_out = common::assert_todo_bin()
        .current_dir(dir.path())
        .args([
            "--json",
            "--agent",
            "agent-1",
            "ask",
            "Should retries use backoff?",
        ])
        .output()
        .unwrap();
    assert_eq!(
        ask_out.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&ask_out)
    );
    let ask_json: Value = parse_json(&ask_out.stdout);
    assert_eq!(ask_json["data"]["id"], "T-001");
    assert_eq!(ask_json["data"]["type"], "question");
    assert_eq!(ask_json["data"]["section"], "Questions");
    assert_eq!(ask_json["data"]["status"], "open");

    let content = read_text(&path);
    assert!(content.contains("## Questions\n"));

    // `next` never hands out questions, even when it's the only open task.
    let next_out = common::assert_todo_bin()
        .current_dir(dir.path())
        .args(["--json", "next"])
        .output()
        .unwrap();
    assert_eq!(next_out.status.code(), Some(0));
    let next_json: Value = parse_json(&next_out.stdout);
    assert!(next_json["data"]["task"].is_null());

    // `answer` records the answer note and marks it done.
    let ans_out = common::assert_todo_bin()
        .current_dir(dir.path())
        .args([
            "--json",
            "--agent",
            "human",
            "answer",
            "T-001",
            "Yes, cap at 30s",
        ])
        .output()
        .unwrap();
    assert_eq!(ans_out.status.code(), Some(0));
    let ans_json: Value = parse_json(&ans_out.stdout);
    assert_eq!(ans_json["data"]["status"], "done");
    let notes = ans_json["data"]["notes"].as_array().unwrap();
    assert!(
        notes
            .iter()
            .any(|n| n["text"].as_str().unwrap().contains("Yes, cap at 30s"))
    );
}

#[test]
fn ask_respects_question_section_in_todo_toml() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("TODO.md"),
        "<!-- todo-format: 1 -->\n\n## Tasks\n\n## Open Questions\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("todo.toml"),
        "question_section = \"Open Questions\"\n",
    )
    .unwrap();

    let ask_out = common::assert_todo_bin()
        .current_dir(dir.path())
        .args(["--json", "--agent", "agent-1", "ask", "Which API version?"])
        .output()
        .unwrap();
    assert_eq!(ask_out.status.code(), Some(0));
    let ask_json: Value = parse_json(&ask_out.stdout);
    assert_eq!(ask_json["data"]["section"], "Open Questions");
}

/// Base file for the `--about` tests: one task, `T-001 · C shader`, and no questions yet.
const ABOUT_BASE: &str = "<!-- todo-format: 1 -->\n\n## Tasks\n\n### T-001 · C shader\n- status: open\n- owner: —\n- priority: P1 · type: feature\n- note (2026-11-18, agent-1): added\n";

#[test]
fn ask_about_writes_about_line_after_the_type_line_and_json_names_the_entry() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, ABOUT_BASE);

    // `t1` is accepted and written as the canonical ID.
    let out = run_todo(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "agent-1",
            "ask",
            "Should the shader support shadows?",
            "--about",
            "t1",
        ],
        &[],
    )
    .unwrap();
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));
    let json = parse_json(&out.stdout);
    assert_eq!(json["data"]["id"], "T-002");
    assert_eq!(json["data"]["type"], "question");
    assert_eq!(json["data"]["fields"]["about"], "T-001");

    let text = read_todo(&work);
    assert!(
        text.contains(
            "### T-002 · Should the shader support shadows?\n- status: open\n- owner: —\n- type: question\n- about: T-001\n- note ("
        ),
        "unexpected file:\n{text}"
    );
}

#[test]
fn ask_about_refuses_an_unknown_id_and_leaves_the_file_and_ids_alone() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, ABOUT_BASE);
    let before = read_todo(&work);

    let refused = run_todo(
        &home,
        &work,
        &[
            "--json", "--agent", "agent-1", "ask", "Which?", "--about", "T-042",
        ],
        &[],
    )
    .unwrap();
    assert_eq!(refused.status.code(), Some(1));
    assert_eq!(parse_json(&refused.stdout)["error"]["code"], "NOT_FOUND");
    assert_eq!(read_todo(&work), before, "a refused ask changed TODO.md");

    // The refused ask did not use up an ID: the next question is T-002.
    let next = run_todo(
        &home,
        &work,
        &["--agent", "agent-1", "ask", "Plain question?"],
        &[],
    )
    .unwrap();
    assert_eq!(
        next.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&next)
    );
    assert_eq!(stdout_text(&next), "T-002: ask ok\n");
}

#[test]
fn ask_about_refuses_a_value_that_is_not_an_id() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, ABOUT_BASE);
    let before = read_todo(&work);

    let out = run_todo(
        &home,
        &work,
        &[
            "--json", "--agent", "agent-1", "ask", "Which?", "--about", "C shader",
        ],
        &[],
    )
    .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(parse_json(&out.stdout)["error"]["code"], "BAD_VALUE");
    assert_eq!(read_todo(&work), before);
}

#[test]
fn ask_about_accepts_an_entry_that_is_only_in_the_archive() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, ABOUT_BASE);
    fs::write(
        work.join("TODO-archive.md"),
        "<!-- todo-format: 1 -->\n\n## Done\n\n### T-005 · Old shader\n- status: done\n- owner: agent-1\n- note (2026-11-01, agent-1): done\n",
    )
    .unwrap();

    let out = run_todo(
        &home,
        &work,
        &[
            "--agent",
            "agent-1",
            "ask",
            "Still relevant?",
            "--about",
            "T-005",
        ],
        &[],
    )
    .unwrap();
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));
    assert_eq!(stdout_text(&out), "T-006: ask ok\n");
    assert!(read_todo(&work).contains("- about: T-005\n"));
}

#[test]
fn ask_without_about_writes_no_about_line() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, ABOUT_BASE);

    let out = run_todo(
        &home,
        &work,
        &["--json", "--agent", "agent-1", "ask", "Plain question?"],
        &[],
    )
    .unwrap();
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));
    let json = parse_json(&out.stdout);
    assert!(json["data"]["fields"].get("about").is_none());
    assert!(!read_todo(&work).contains("about:"));
}

#[test]
fn answer_prints_the_entry_the_question_is_about_in_text_and_json() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, ABOUT_BASE);
    let asked = run_todo(
        &home,
        &work,
        &["--agent", "agent-1", "ask", "Shadows?", "--about", "T-001"],
        &[],
    )
    .unwrap();
    assert_eq!(
        asked.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&asked)
    );

    let text = run_todo(
        &home,
        &work,
        &["--agent", "human", "answer", "T-002", "Yes"],
        &[],
    )
    .unwrap();
    assert_eq!(
        text.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&text)
    );
    assert_eq!(
        stdout_text(&text),
        "T-002: answer ok\n\nAbout T-001:\n### T-001 · C shader\n- status: open\n- owner: —\n- priority: P1 · type: feature\n- note (2026-11-18, agent-1): added\n"
    );

    // JSON: the question itself, with the entry it is about under `about`.
    let tmp2 = TempDir::new().unwrap();
    let (home2, work2, _) = setup_proj(&tmp2, ABOUT_BASE);
    run_todo(
        &home2,
        &work2,
        &["--agent", "agent-1", "ask", "Shadows?", "--about", "T-001"],
        &[],
    )
    .unwrap();
    let json_out = run_todo(
        &home2,
        &work2,
        &["--json", "--agent", "human", "answer", "T-002", "Yes"],
        &[],
    )
    .unwrap();
    assert_eq!(
        json_out.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&json_out)
    );
    let json = parse_json(&json_out.stdout);
    assert_eq!(json["data"]["id"], "T-002");
    assert_eq!(json["data"]["status"], "done");
    assert_eq!(json["data"]["about"]["id"], "T-001");
    assert_eq!(json["data"]["about"]["title"], "C shader");
}

#[test]
fn answer_to_a_question_without_about_prints_only_the_result() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, ABOUT_BASE);
    run_todo(&home, &work, &["--agent", "agent-1", "ask", "Plain?"], &[]).unwrap();

    let text = run_todo(
        &home,
        &work,
        &["--agent", "human", "answer", "T-002", "Yes"],
        &[],
    )
    .unwrap();
    assert_eq!(stdout_text(&text), "T-002: answer ok\n");

    let tmp2 = TempDir::new().unwrap();
    let (home2, work2, _) = setup_proj(&tmp2, ABOUT_BASE);
    run_todo(
        &home2,
        &work2,
        &["--agent", "agent-1", "ask", "Plain?"],
        &[],
    )
    .unwrap();
    let json_out = run_todo(
        &home2,
        &work2,
        &["--json", "--agent", "human", "answer", "T-002", "Yes"],
        &[],
    )
    .unwrap();
    let json = parse_json(&json_out.stdout);
    assert!(json["data"].get("about").is_none());
}

#[test]
fn list_where_about_finds_the_questions_about_an_entry() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, ABOUT_BASE);
    run_todo(
        &home,
        &work,
        &["--agent", "agent-1", "ask", "Shadows?", "--about", "T-001"],
        &[],
    )
    .unwrap();
    run_todo(
        &home,
        &work,
        &["--agent", "agent-1", "ask", "Unrelated?"],
        &[],
    )
    .unwrap();

    let about_t1 = run_todo(
        &home,
        &work,
        &["--json", "list", "--where", "about=T-001"],
        &[],
    )
    .unwrap();
    assert_eq!(
        about_t1.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&about_t1)
    );
    let json = parse_json(&about_t1.stdout);
    assert_eq!(json["data"]["count"], 1);
    assert_eq!(json["data"]["tasks"][0]["id"], "T-002");

    let about_t2 = run_todo(
        &home,
        &work,
        &["--json", "list", "--where", "about=T-002"],
        &[],
    )
    .unwrap();
    assert_eq!(parse_json(&about_t2.stdout)["data"]["count"], 0);
}
