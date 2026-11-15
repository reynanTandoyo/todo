//! Tests for `todo ask` and `todo answer` (T-031; R-WRITE-11).

use std::fs;

use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn ask_adds_questions_section_next_skips_questions_and_answer_completes() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("TODO.md");
    fs::write(&path, "<!-- todo-format: 1 -->\n\n## Tasks\n").unwrap();

    // Ask a question when `Questions` section does not exist yet.
    let ask_out = cargo_bin_cmd!("todo")
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
        String::from_utf8_lossy(&ask_out.stderr)
    );
    let ask_json: Value = serde_json::from_slice(&ask_out.stdout).unwrap();
    assert_eq!(ask_json["data"]["id"], "T-001");
    assert_eq!(ask_json["data"]["type"], "question");
    assert_eq!(ask_json["data"]["section"], "Questions");
    assert_eq!(ask_json["data"]["status"], "open");

    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("## Questions\n"));

    // `next` never hands out questions, even when it's the only open task.
    let next_out = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args(["--json", "next"])
        .output()
        .unwrap();
    assert_eq!(next_out.status.code(), Some(0));
    let next_json: Value = serde_json::from_slice(&next_out.stdout).unwrap();
    assert!(next_json["data"]["task"].is_null());

    // `answer` records the answer note and marks it done.
    let ans_out = cargo_bin_cmd!("todo")
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
    let ans_json: Value = serde_json::from_slice(&ans_out.stdout).unwrap();
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

    let ask_out = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args(["--json", "--agent", "agent-1", "ask", "Which API version?"])
        .output()
        .unwrap();
    assert_eq!(ask_out.status.code(), Some(0));
    let ask_json: Value = serde_json::from_slice(&ask_out.stdout).unwrap();
    assert_eq!(ask_json["data"]["section"], "Open Questions");
}
