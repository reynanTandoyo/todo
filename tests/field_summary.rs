//! Tests for the `summary` field (T-208; R-WRITE-19, TEST-4): a hand-written `- summary:` line is
//! read as the field, `show` and `show --json` carry it (not under `fields`), `schema` lists it,
//! it comes directly after `why` in the field order, a file without it works unchanged, and a
//! repeated summary is a repeated field.

mod common;

use serde_json::Value;
use tempfile::TempDir;

use common::{parse_json, read_todo, run_todo, setup_proj, stderr_text, stdout_text};
use todo::doc::canonical_field_rank;

/// One task with `why` and a hand-written `summary` line after it.
const WITH_SUMMARY: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
<!-- todo-format: 3 -->\n\
## Tasks\n\n\
### T-001 · Render the first frame\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: feature · area: renderer\n\
- why: the first frame shows that the pipeline runs\n\
- summary: The first picture the renderer draws, so progress can be seen.\n\
- accept: a frame is written to disk\n";

/// The same task without any summary line, and without a format line (format 1).
const WITHOUT_SUMMARY: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
## Tasks\n\n\
### T-001 · Render the first frame\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: feature · area: renderer\n\
- problem: nothing is drawn yet\n\
- accept: a frame is written to disk\n";

/// The file after a write to a format-1 fixture: the format line and the tool stamp come first
/// (R-MIG-1, R-VER-1), as in tests/cmd_why.rs.
fn stamped(text: &str) -> String {
    format!(
        "<!-- todo-format: 3 -->\n<!-- todo-tool: {} -->\n{text}",
        env!("CARGO_PKG_VERSION")
    )
}

#[test]
fn a_hand_written_summary_is_read_as_the_summary_field() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, WITH_SUMMARY);

    let shown = run_todo(&home, &work, &["--json", "show", "T-001"], &[]).unwrap();
    assert_eq!(
        shown.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&shown)
    );
    let json = parse_json(&shown.stdout);
    assert_eq!(
        json["data"]["summary"],
        "The first picture the renderer draws, so progress can be seen."
    );
    // A known field: it is not listed among the unknown fields.
    assert!(json["data"]["fields"].get("summary").is_none(), "{json}");
}

#[test]
fn show_text_prints_the_summary_line_after_why() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, WITH_SUMMARY);

    // `show` prints the task block as written, so the line stays where the file has it.
    let shown = run_todo(&home, &work, &["show", "T-001"], &[]).unwrap();
    assert_eq!(
        shown.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&shown)
    );
    let text = stdout_text(&shown);
    let why_at = text.find("- why:").expect("why line in show");
    let summary_at = text.find("- summary:").expect("summary line in show");
    assert!(why_at < summary_at, "{text}");
}

#[test]
fn a_summary_on_the_priority_line_is_read_like_a_hand_written_one() {
    let body = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
<!-- todo-format: 3 -->\n\
## Tasks\n\n\
### T-001 · Render the first frame\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: feature · summary: The first picture the renderer draws.\n\
- accept: a frame is written to disk\n";
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, body);

    let shown = run_todo(&home, &work, &["--json", "show", "T-001"], &[]).unwrap();
    assert_eq!(
        shown.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&shown)
    );
    let json = parse_json(&shown.stdout);
    assert_eq!(
        json["data"]["summary"],
        "The first picture the renderer draws."
    );
    assert!(json["data"]["fields"].get("summary").is_none(), "{json}");
}

#[test]
fn a_file_without_summary_works_unchanged() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, WITHOUT_SUMMARY);

    // `show --json` still has the key, with null, and it is not an unknown field.
    let shown = run_todo(&home, &work, &["--json", "show", "T-001"], &[]).unwrap();
    assert_eq!(
        shown.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&shown)
    );
    let json = parse_json(&shown.stdout);
    assert_eq!(json["data"].get("summary"), Some(&Value::Null), "{json}");
    assert!(json["data"]["fields"].get("summary").is_none(), "{json}");

    // A write adds the field it was asked for and no summary line.
    let set = run_todo(
        &home,
        &work,
        &[
            "--agent",
            "alice",
            "set",
            "T-001",
            "--why",
            "the first frame shows that the pipeline runs",
        ],
        &[],
    )
    .unwrap();
    assert_eq!(set.status.code(), Some(0), "stderr: {}", stderr_text(&set));
    let expected = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
## Tasks\n\n\
### T-001 · Render the first frame\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: feature · area: renderer\n\
- why: the first frame shows that the pipeline runs\n\
- problem: nothing is drawn yet\n\
- accept: a frame is written to disk\n";
    assert_eq!(read_todo(&work), stamped(expected));
}

#[test]
fn schema_lists_summary_and_its_example_has_it() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, WITH_SUMMARY);

    let schema = run_todo(&home, &work, &["--json", "schema"], &[]).unwrap();
    assert_eq!(
        schema.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&schema)
    );
    let task = parse_json(&schema.stdout)["data"]["task"].clone();
    let fields = task["fields"]
        .as_array()
        .expect("task fields array")
        .clone();
    assert!(fields.iter().any(|f| f == "summary"), "fields: {fields:?}");
    assert!(task["example"]["summary"].is_string(), "{task}");
}

#[test]
fn summary_comes_directly_after_why_in_the_field_order() {
    assert_eq!(
        canonical_field_rank("summary"),
        canonical_field_rank("why") + 1
    );
    assert_eq!(
        canonical_field_rank("final"),
        canonical_field_rank("summary") + 1
    );
}

#[test]
fn a_repeated_summary_is_a_repeated_field_and_blocks_writes() {
    let tmp = TempDir::new().unwrap();
    let body = WITH_SUMMARY.replace(
        "- accept:",
        "- summary: A second sentence for the same task.\n- accept:",
    );
    let (home, work, _) = setup_proj(&tmp, &body);

    let check = run_todo(&home, &work, &["check"], &[]).unwrap();
    let text = format!("{}{}", stdout_text(&check), stderr_text(&check));
    assert!(text.contains("repeated field 'summary'"), "{text}");

    let before = read_todo(&work);
    let set = run_todo(
        &home,
        &work,
        &["--agent", "alice", "set", "T-001", "--priority", "P0"],
        &[],
    )
    .unwrap();
    assert!(!set.status.success(), "a repeated field blocks the write");
    assert_eq!(read_todo(&work), before);
}
