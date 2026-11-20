//! CLI tests for the `steps`, `read` and `verify` fields (T-152; R-DETAIL-1, R-WRITE-1, R-WRITE-8,
//! R-WRITE-12, R-WRITE-13, R-WRITE-15, R-READ-2, R-EDIT-7, TEST-4). Fixtures are in `tests/fixtures/`.

mod common;

use std::collections::BTreeSet;
use std::fs;

use common::{
    parse_json, read_text, run_todo, setup_fixture_proj, setup_proj, stderr_text, stdout_text,
};
use serde_json::{Value, json};
use tempfile::TempDir;

const BANNER: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->";

fn base_file() -> String {
    format!(
        "{BANNER}\n## Now\n\n### T-001 · Existing task\n- status: open\n- owner: —\n- priority: P1\n- accept: it works\n\n"
    )
}

/// The text from the `## Now` heading on: the part that a format migration never changes.
fn body(text: &str) -> &str {
    &text[text.find("## Now").expect("## Now heading")..]
}

/// The four lines of the steps field of T-001 in `steps_lf.md`, with the file's line ending.
fn fixture_steps_block(eol: &str) -> String {
    [
        "- steps:",
        "  1. Open the invented file src/demo.rs.",
        "  2. Add a function that returns 42, with `code`, \"quotes\" and $dollars.",
        "  3. Add a test that calls it.",
    ]
    .iter()
    .map(|line| format!("{line}{eol}"))
    .collect()
}

#[test]
fn add_writes_files_read_steps_accept_and_verify_in_canonical_order() {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo_file) = setup_proj(&tmp, &base_file());
    fs::write(
        work.join("steps.txt"),
        "1. first step\r\n2) second step\r\n\r\nthird step\r\n",
    )
    .unwrap();

    let out = run_todo(
        &home,
        &work,
        &[
            "add",
            "--title",
            "New task",
            "--priority",
            "P2",
            "--difficulty",
            "2",
            "--files",
            "src/new.rs",
            "--read",
            "docs/a.md, src/lib.rs",
            "--steps",
            "@steps.txt",
            "--accept",
            "tests pass",
            "--verify",
            "cargo test",
            "--agent",
            "alice",
        ],
        &[],
    )
    .unwrap();
    assert!(out.status.success(), "{}", stderr_text(&out));

    let text = read_text(&todo_file);
    assert!(
        text.contains(
            "### T-002 · New task\n- status: open\n- owner: —\n- priority: P2 · difficulty: 2\n- files: src/new.rs\n\
             - read: docs/a.md, src/lib.rs\n- steps:\n  1. first step\n  2. second step\n  3. third step\n\
             - accept: tests pass\n- verify: cargo test\n- note ("
        ),
        "{text}"
    );

    let show = run_todo(&home, &work, &["--json", "show", "T-002"], &[]).unwrap();
    assert!(show.status.success(), "{}", stderr_text(&show));
    let task = parse_json(&show.stdout)["data"].clone();
    assert_eq!(
        task["steps"],
        json!(["first step", "second step", "third step"])
    );
    assert_eq!(task["read"], json!(["docs/a.md", "src/lib.rs"]));
    assert_eq!(task["verify"], "cargo test");
}

#[test]
fn show_json_lists_steps_and_read_and_is_empty_without_them() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_fixture_proj(&tmp, "steps_lf.md");

    let with = run_todo(&home, &work, &["--json", "show", "T-001"], &[]).unwrap();
    let task = parse_json(&with.stdout)["data"].clone();
    assert_eq!(
        task["steps"],
        json!([
            "Open the invented file src/demo.rs.",
            "Add a function that returns 42, with `code`, \"quotes\" and $dollars.",
            "Add a test that calls it."
        ])
    );
    assert_eq!(task["read"], json!(["docs/demo.md", "src/lib.rs"]));
    assert_eq!(task["verify"], "cargo test --all-features");

    let without = run_todo(&home, &work, &["--json", "show", "T-002"], &[]).unwrap();
    let task = parse_json(&without.stdout)["data"].clone();
    assert_eq!(task["steps"], json!([]));
    assert_eq!(task["read"], json!([]));
    assert_eq!(task["verify"], Value::Null);
}

#[test]
fn set_steps_replaces_the_list_and_keeps_every_other_byte_in_each_line_ending() {
    for name in ["steps_lf.md", "steps_crlf.md", "steps_bom.md"] {
        let tmp = TempDir::new().unwrap();
        let (home, work, todo_file) = setup_fixture_proj(&tmp, name);
        let before = read_text(&todo_file);
        let eol = if before.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        fs::write(work.join("steps.txt"), "only one\nonly two").unwrap();

        let out = run_todo(
            &home,
            &work,
            &["set", "T-001", "--steps", "@steps.txt", "--agent", "alice"],
            &[],
        )
        .unwrap();
        assert!(out.status.success(), "{name}: {}", stderr_text(&out));

        let after = read_text(&todo_file);
        let want = body(&before).replacen(
            &fixture_steps_block(eol),
            &format!("- steps:{eol}  1. only one{eol}  2. only two{eol}"),
            1,
        );
        assert_eq!(body(&after), want, "{name}");
        if eol == "\r\n" {
            assert!(
                !after.replace("\r\n", "").contains('\n'),
                "{name} must stay CRLF"
            );
        }
    }
}

#[test]
fn set_unset_steps_removes_the_field_with_its_lines() {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo_file) = setup_fixture_proj(&tmp, "steps_crlf.md");
    let before = read_text(&todo_file);

    let out = run_todo(
        &home,
        &work,
        &["set", "T-001", "--unset", "steps", "--agent", "alice"],
        &[],
    )
    .unwrap();
    assert!(out.status.success(), "{}", stderr_text(&out));

    let after = read_text(&todo_file);
    assert_eq!(
        body(&after),
        body(&before).replacen(&fixture_steps_block("\r\n"), "", 1)
    );
    assert!(!after.contains("- steps:"), "{after}");
    assert!(!after.contains("  1. Open the invented"), "{after}");
}

#[test]
fn set_read_and_verify_change_their_own_lines_in_place_and_unset_removes_them() {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo_file) = setup_fixture_proj(&tmp, "steps_lf.md");
    let before = read_text(&todo_file);

    let out = run_todo(
        &home,
        &work,
        &[
            "set",
            "T-001",
            "--read",
            "docs/other.md",
            "--verify",
            "make test",
            "--agent",
            "alice",
        ],
        &[],
    )
    .unwrap();
    assert!(out.status.success(), "{}", stderr_text(&out));
    let after = read_text(&todo_file);
    assert_eq!(
        body(&after),
        body(&before)
            .replacen(
                "- read: docs/demo.md, src/lib.rs\n",
                "- read: docs/other.md\n",
                1
            )
            .replacen(
                "- verify: cargo test --all-features\n",
                "- verify: make test\n",
                1
            )
    );

    let out = run_todo(
        &home,
        &work,
        &[
            "set", "T-001", "--unset", "verify", "--unset", "read", "--agent", "alice",
        ],
        &[],
    )
    .unwrap();
    assert!(out.status.success(), "{}", stderr_text(&out));
    let after = read_text(&todo_file);
    assert!(
        !after.contains("- verify:") && !after.contains("- read:"),
        "{after}"
    );
    assert!(
        after.contains("- steps:\n  1. Open the invented file"),
        "{after}"
    );
}

#[test]
fn add_from_takes_steps_as_a_list_or_as_text_with_read_and_verify() {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo_file) = setup_proj(&tmp, &base_file());
    fs::write(
        work.join("batch.jsonl"),
        concat!(
            r#"{"title":"From list","priority":"P1","difficulty":"2","steps":["1. one","two\nthree"],"read":"a.md","verify":"make a"}"#,
            "\n",
            r#"{"title":"From text","priority":"P2","difficulty":"2","steps":"one\n2. two"}"#,
            "\n",
        ),
    )
    .unwrap();

    let out = run_todo(
        &home,
        &work,
        &["add", "--from", "batch.jsonl", "--agent", "alice"],
        &[],
    )
    .unwrap();
    assert!(out.status.success(), "{}", stderr_text(&out));

    let text = read_text(&todo_file);
    assert!(
        text.contains(
            "### T-002 · From list\n- status: open\n- owner: —\n- priority: P1 · difficulty: 2\n- read: a.md\n\
             - steps:\n  1. one\n  2. two\n  3. three\n- verify: make a\n- note ("
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "### T-003 · From text\n- status: open\n- owner: —\n- priority: P2 · difficulty: 2\n\
             - steps:\n  1. one\n  2. two\n- note ("
        ),
        "{text}"
    );
}

#[test]
fn apply_add_and_set_take_steps_read_and_verify() {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo_file) = setup_proj(&tmp, &base_file());
    fs::write(
        work.join("ops.jsonl"),
        concat!(
            r#"{"op":"add","title":"Applied add","priority":"P1","difficulty":"2","steps":["a step"],"read":"r.md","verify":"v"}"#,
            "\n",
            r#"{"op":"set","task":"T-001","steps":"x\ny","verify":"make it"}"#,
            "\n",
        ),
    )
    .unwrap();

    let out = run_todo(
        &home,
        &work,
        &["apply", "ops.jsonl", "--agent", "alice"],
        &[],
    )
    .unwrap();
    assert!(out.status.success(), "{}", stderr_text(&out));

    let text = read_text(&todo_file);
    assert!(
        text.contains(
            "### T-001 · Existing task\n- status: open\n- owner: —\n- priority: P1\n\
             - steps:\n  1. x\n  2. y\n- accept: it works\n- verify: make it\n"
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "### T-002 · Applied add\n- status: open\n- owner: —\n- priority: P1 · difficulty: 2\n- read: r.md\n\
             - steps:\n  1. a step\n- verify: v\n"
        ),
        "{text}"
    );
}

#[test]
fn add_field_steps_is_the_same_as_the_steps_option() {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo_file) = setup_proj(&tmp, &base_file());
    let out = run_todo(
        &home,
        &work,
        &[
            "add",
            "--title",
            "Via field",
            "--priority",
            "P1",
            "--difficulty",
            "2",
            "--field",
            "steps=only one",
            "--field",
            "read=a.md",
            "--agent",
            "alice",
        ],
        &[],
    )
    .unwrap();
    assert!(out.status.success(), "{}", stderr_text(&out));
    let text = read_text(&todo_file);
    assert!(
        text.contains(
            "- priority: P1 · difficulty: 2\n- read: a.md\n- steps:\n  1. only one\n- note ("
        ),
        "{text}"
    );
}

#[test]
fn check_reports_a_value_written_on_the_steps_line() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(
        &tmp,
        &format!(
            "{BANNER}\n## Now\n\n### T-001 · Inline\n- status: open\n- steps: inline value\n  1. second\n"
        ),
    );
    let out = run_todo(&home, &work, &["check"], &[]).unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", stderr_text(&out));
    let text = stdout_text(&out);
    assert!(
        text.contains("the value of steps on T-001 is read as a step"),
        "{text}"
    );
}

#[test]
fn schema_lists_the_three_fields_and_the_three_options() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, &base_file());

    let full = run_todo(&home, &work, &["--json", "schema"], &[]).unwrap();
    let fields: BTreeSet<String> = parse_json(&full.stdout)["data"]["task"]["fields"]
        .as_array()
        .expect("fields array")
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    for field in ["steps", "read", "verify"] {
        assert!(fields.contains(field), "schema lacks {field}: {fields:?}");
    }

    for command in ["add", "set"] {
        let out = run_todo(&home, &work, &["--json", "schema", command], &[]).unwrap();
        let names: BTreeSet<String> = parse_json(&out.stdout)["data"]["options"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(|o| o["name"].as_str().map(str::to_string))
            .collect();
        for option in ["--steps", "--read", "--verify"] {
            assert!(
                names.contains(option),
                "{command} lacks {option}: {names:?}"
            );
        }
    }
}
