#![allow(clippy::expect_used)] // whole file is test code; helpers outside #[test] may expect()

//! Parser and whole-line edit tests for the `steps`, `read` and `verify` fields (T-152;
//! R-DETAIL-1, R-EDIT-1, R-EDIT-7, TEST-2, TEST-3, TEST-4). Fixtures are in `tests/fixtures/`.

mod common;

use common::fixture;

use todo::doc::parser::parse_step_line;
use todo::doc::{
    CANONICAL_FIELD_ORDER, Document, LineDoc, ProblemKind, ProblemSeverity, TaskChangeIntent,
    TaskId,
};
use todo::ops::{TaskEditBuilder, steps_from_text};

const UTF8_BOM: &str = "\u{feff}";

fn parse(text: &str) -> Document {
    Document::parse_bytes(text.as_bytes()).expect("valid UTF-8")
}

/// The fixture text without its byte-order mark, and the fixture as it is on disk.
fn fixture_text(name: &str) -> (Vec<u8>, String) {
    let bytes = fixture(name);
    let text = String::from_utf8(bytes.clone()).expect("UTF-8 fixture");
    (bytes, text.trim_start_matches(UTF8_BOM).to_string())
}

/// `lines` joined with the line ending of the file, and a final line ending.
fn block(lines: &[&str], eol: &str) -> String {
    lines.iter().map(|l| format!("{l}{eol}")).collect()
}

#[test]
fn steps_are_read_in_order_without_numbers_and_end_at_the_first_other_line() {
    let text = "## Now\n### T-001 · Steps\n- status: open\n- steps:\n  1. first\n  2. second\n    3. third, indented by four spaces\n- a bullet ends the list\n  4. fourth is plain text now\n- accept: done\n";
    let doc = parse(text);
    assert!(doc.problems.is_empty(), "{:?}", doc.problems);

    let task = doc.task_by_id(TaskId(1)).expect("T-001");
    assert_eq!(
        task.steps,
        vec![
            "first".to_string(),
            "second".to_string(),
            "third, indented by four spaces".to_string()
        ]
    );
    // The field line is line 3 (0-based) and the three step lines are 4 to 6; the bullet on
    // line 7 ends the list, so the block is 3..7 (half-open).
    assert_eq!(task.steps_line, Some(3));
    assert_eq!(task.steps_block, Some(3..7));
    assert_eq!(task.accept.as_deref(), Some("done"));
}

#[test]
fn step_lines_need_two_to_four_spaces_a_number_a_dot_and_a_space() {
    assert_eq!(parse_step_line("  1. one"), Some("one"));
    assert_eq!(parse_step_line("    12. twelve"), Some("twelve"));
    assert_eq!(parse_step_line("  3.  padded  "), Some("padded"));
    assert_eq!(parse_step_line("  1. "), Some(""));
    assert_eq!(parse_step_line(" 1. one space"), None);
    assert_eq!(parse_step_line("     1. five spaces"), None);
    assert_eq!(parse_step_line("  1.no space"), None);
    assert_eq!(parse_step_line("  1) closing paren"), None);
    assert_eq!(parse_step_line("  a. letter"), None);
    assert_eq!(parse_step_line("- 1. dash"), None);
}

#[test]
fn a_value_on_the_steps_line_is_the_first_step_and_is_reported_without_blocking() {
    let doc = parse("## Now\n### T-001 · Inline\n- steps: inline value\n  1. second\n");
    let task = doc.task_by_id(TaskId(1)).expect("T-001");
    assert_eq!(
        task.steps,
        vec!["inline value".to_string(), "second".to_string()]
    );

    let problem = doc
        .problems
        .iter()
        .find(|p| matches!(p.kind, ProblemKind::StepsValueOnFieldLine { .. }))
        .expect("a problem for the value on the steps line");
    assert_eq!(problem.severity, ProblemSeverity::Warning);
    assert_eq!(problem.line, 3);
    assert!(!doc.blocks_task_write(TaskId(1)));
}

#[test]
fn steps_on_the_priority_line_are_one_step_and_are_reported() {
    let doc = parse("## Now\n### T-001 · Part\n- priority: P1 · steps: one step\n- accept: done\n");
    let task = doc.task_by_id(TaskId(1)).expect("T-001");
    assert_eq!(task.steps, vec!["one step".to_string()]);
    assert_eq!(task.steps_line, Some(2));
    assert_eq!(task.steps_block, None);
    assert!(
        doc.problems
            .iter()
            .any(|p| matches!(p.kind, ProblemKind::StepsValueOnFieldLine { .. })),
        "{:?}",
        doc.problems
    );
}

#[test]
fn read_and_verify_are_single_fields_and_read_splits_at_commas() {
    let doc = parse(
        "## Now\n### T-001 · Fields\n- priority: P1 · read: docs/a.md, src/b.rs · verify: cargo test\n- files: src/b.rs\n",
    );
    assert!(doc.problems.is_empty(), "{:?}", doc.problems);
    let task = doc.task_by_id(TaskId(1)).expect("T-001");
    assert_eq!(task.read.as_deref(), Some("docs/a.md, src/b.rs"));
    assert_eq!(task.read_line, Some(2));
    assert_eq!(task.read_paths(), vec!["docs/a.md", "src/b.rs"]);
    assert_eq!(task.verify.as_deref(), Some("cargo test"));
    assert_eq!(task.verify_line, Some(2));
    assert_eq!(task.field_value("read"), Some("docs/a.md, src/b.rs"));
    assert_eq!(task.field_value("verify"), Some("cargo test"));

    let own = parse("## Now\n### T-002 · Own lines\n- read: a.md\n- verify: make test\n");
    let task = own.task_by_id(TaskId(2)).expect("T-002");
    assert_eq!(task.read_paths(), vec!["a.md"]);
    assert_eq!(task.verify.as_deref(), Some("make test"));
    assert_eq!(task.verify_line, Some(3));
}

#[test]
fn repeated_steps_read_and_verify_lines_are_repeated_field_problems() {
    for field in ["steps", "read", "verify"] {
        let text = format!("## Now\n### T-001 · Twice\n- {field}: a\n- {field}: b\n");
        let doc = parse(&text);
        assert!(
            doc.problems.iter().any(|p| matches!(
                &p.kind,
                ProblemKind::RepeatedField { field: f, .. } if f == field
            )),
            "expected a repeated `{field}` problem, got {:?}",
            doc.problems
        );
        assert!(
            doc.blocks_task_write(TaskId(1)),
            "{field} repeated must block writes"
        );
    }
}

#[test]
fn a_field_inserted_after_steps_comes_after_its_step_lines() {
    let doc = parse(
        "## Now\n### T-001 · Order\n- status: open\n- steps:\n  1. one\n  2. two\n- note (2026-11-19, t): added\n",
    );
    let task = doc.task_by_id(TaskId(1)).expect("T-001");
    // `accept` and `verify` come after `steps`, so they go below its last step line (line 5).
    assert_eq!(task.insert_line_for_field("accept"), 6);
    assert_eq!(task.insert_line_for_field("verify"), 6);
    // `read` and `fix` come before `steps`; the nearest field before them is `status` (line 2).
    assert_eq!(task.insert_line_for_field("read"), 3);
    assert_eq!(task.insert_line_for_field("fix"), 3);
}

#[test]
fn canonical_order_has_read_steps_and_verify_where_format_section_10_puts_them() {
    let position = |key: &str| {
        CANONICAL_FIELD_ORDER
            .iter()
            .position(|k| *k == key)
            .expect(key)
    };
    assert!(position("files") < position("read"));
    assert!(position("read") < position("problem"));
    assert!(position("fix") < position("steps"));
    assert!(position("steps") < position("accept"));
    assert!(position("accept") < position("verify"));
    assert!(position("verify") < position("attempts"));
    assert!(position("verify") < position("commit"));
}

#[test]
fn steps_from_text_takes_one_step_per_line_and_removes_numbers() {
    assert_eq!(
        steps_from_text("1. one\r\n2) two\n\n  3.   three  \n2.5 GB\n4.\n- kept dash\n"),
        vec!["one", "two", "three", "2.5 GB", "- kept dash"]
    );
    assert!(steps_from_text(" \n\r\n").is_empty());
}

#[test]
fn fixtures_with_steps_round_trip_byte_identical_and_read_the_same_steps() {
    let mut texts = Vec::new();
    for name in ["steps_lf.md", "steps_crlf.md", "steps_bom.md"] {
        let (bytes, text) = fixture_text(name);
        let line_doc = LineDoc::from_bytes(&bytes).expect("UTF-8");
        assert_eq!(line_doc.to_bytes(), bytes, "{name} must round-trip");

        let doc = Document::parse_bytes(&bytes).expect("UTF-8");
        assert!(doc.problems.is_empty(), "{name}: {:?}", doc.problems);
        let task = doc.task_by_id(TaskId(1)).expect("T-001");
        assert_eq!(task.steps.len(), 3, "{name}");
        assert_eq!(
            task.read_paths(),
            vec!["docs/demo.md", "src/lib.rs"],
            "{name}"
        );
        assert_eq!(
            task.verify.as_deref(),
            Some("cargo test --all-features"),
            "{name}"
        );
        let no_steps = doc.task_by_id(TaskId(2)).expect("T-002");
        assert!(no_steps.steps.is_empty(), "{name}");
        texts.push(text);
    }
    assert_eq!(
        texts[0].replace("\r\n", "\n"),
        texts[1].replace("\r\n", "\n")
    );
    assert_eq!(texts[0], texts[2]);
}

#[test]
fn set_steps_replaces_only_the_step_lines_in_every_line_ending() {
    let step_lines = [
        "- steps:",
        "  1. Open the invented file src/demo.rs.",
        "  2. Add a function that returns 42, with `code`, \"quotes\" and $dollars.",
        "  3. Add a test that calls it.",
    ];
    for name in ["steps_lf.md", "steps_crlf.md", "steps_bom.md"] {
        let (bytes, text) = fixture_text(name);
        let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
        let doc = Document::parse_bytes(&bytes).expect("UTF-8");
        let task = doc.task_by_id(TaskId(1)).expect("T-001");

        let mut builder = TaskEditBuilder::new(&doc, task);
        builder.set_steps(&["new one".to_string(), "new two".to_string()]);
        let (edits, intent) = builder.build();
        let applied = doc
            .apply_and_verify(&edits, &intent)
            .expect("verified write");

        let expected = text.replacen(
            &block(&step_lines, eol),
            &block(&["- steps:", "  1. new one", "  2. new two"], eol),
            1,
        );
        let got = String::from_utf8(applied.bytes).expect("UTF-8");
        assert_eq!(got.trim_start_matches(UTF8_BOM), expected, "{name}");
        assert_eq!(
            applied
                .doc
                .task_by_id(TaskId(1))
                .expect("T-001")
                .steps
                .len(),
            2
        );
    }
}

#[test]
fn unset_steps_removes_the_field_with_all_its_lines_and_empty_steps_do_too() {
    let (bytes, text) = fixture_text("steps_lf.md");
    let doc = Document::parse_bytes(&bytes).expect("UTF-8");
    let task = doc.task_by_id(TaskId(1)).expect("T-001");

    for steps_text in [None, Some(Vec::new())] {
        let mut builder = TaskEditBuilder::new(&doc, task);
        match steps_text {
            None => builder.unset_field("steps"),
            Some(list) => builder.set_steps(&list),
        }
        let (edits, intent) = builder.build();
        let applied = doc
            .apply_and_verify(&edits, &intent)
            .expect("verified write");
        let got = String::from_utf8(applied.bytes).expect("UTF-8");
        let want = text.replacen(
            "- steps:\n  1. Open the invented file src/demo.rs.\n  2. Add a function that returns 42, with `code`, \"quotes\" and $dollars.\n  3. Add a test that calls it.\n",
            "",
            1,
        );
        assert_eq!(got.trim_start_matches(UTF8_BOM), want);
        assert!(
            applied
                .doc
                .task_by_id(TaskId(1))
                .expect("T-001")
                .steps
                .is_empty()
        );
    }
}

#[test]
fn a_new_steps_block_goes_in_canonical_order_and_read_verify_replace_their_own_lines() {
    let (bytes, _) = fixture_text("steps_lf.md");
    let doc = Document::parse_bytes(&bytes).expect("UTF-8");
    let task = doc.task_by_id(TaskId(2)).expect("T-002");

    let mut builder = TaskEditBuilder::new(&doc, task);
    builder.set_steps(&["only step".to_string()]);
    builder.set_field("verify", "make test");
    builder.set_field("read", "docs/x.md");
    let (edits, intent) = builder.build();
    let applied = doc
        .apply_and_verify(&edits, &TaskChangeIntent::modify_one(TaskId(2)))
        .expect("verified write");
    assert!(intent.modified_ids.contains(&TaskId(2)));

    let text = String::from_utf8(applied.bytes).expect("UTF-8");
    let at = |needle: &str| {
        text.find(needle)
            .unwrap_or_else(|| panic!("{needle} missing"))
    };
    let priority = at("- priority: P2 · difficulty: 1 · type: chore · area: demo\n");
    let steps = at("- steps:\n  1. only step\n");
    let accept = at("- accept: nothing to do\n");
    let verify = at("- verify: make test\n");
    let read = at("- read: docs/x.md\n");
    assert!(
        priority < steps && steps < accept && accept < verify,
        "{text}"
    );
    assert!(priority < read && read < steps, "{text}");

    let t2 = applied.doc.task_by_id(TaskId(2)).expect("T-002");
    assert_eq!(t2.steps, vec!["only step".to_string()]);
    assert_eq!(t2.read_paths(), vec!["docs/x.md"]);
    assert_eq!(t2.verify.as_deref(), Some("make test"));
}

#[test]
fn verification_rejects_a_changed_steps_read_or_verify_of_an_untouched_task() {
    let (bytes, _) = fixture_text("steps_lf.md");
    let doc = Document::parse_bytes(&bytes).expect("UTF-8");
    let t1 = doc.task_by_id(TaskId(1)).expect("T-001");

    // An edit that changes T-001's steps while the intent says only T-002 may change.
    let mut builder = TaskEditBuilder::new(&doc, t1);
    builder.set_steps(&["sneaky".to_string()]);
    let (edits, _) = builder.build();
    let refused = doc.apply_and_verify(&edits, &TaskChangeIntent::modify_one(TaskId(2)));
    assert!(
        refused.is_err(),
        "a steps change to an untouched task must be refused"
    );

    let mut builder = TaskEditBuilder::new(&doc, t1);
    builder.set_field("verify", "sneaky");
    let (edits, _) = builder.build();
    assert!(
        doc.apply_and_verify(&edits, &TaskChangeIntent::modify_one(TaskId(2)))
            .is_err(),
        "a verify change to an untouched task must be refused"
    );
}

#[test]
fn a_line_ending_and_a_missing_final_line_survive_a_steps_edit() {
    // A last line without a line ending: the step lines go, the rest keeps its bytes.
    let text = "## Now\n### T-001 · Last\n- status: open\n- steps:\n  1. one\n  2. two";
    let doc = parse(text);
    let task = doc.task_by_id(TaskId(1)).expect("T-001");
    let mut builder = TaskEditBuilder::new(&doc, task);
    builder.unset_steps();
    let (edits, intent) = builder.build();
    let applied = doc
        .apply_and_verify(&edits, &intent)
        .expect("verified write");
    assert_eq!(
        String::from_utf8(applied.bytes).expect("UTF-8"),
        "## Now\n### T-001 · Last\n- status: open\n"
    );
}

#[test]
fn read_and_verify_parts_of_the_priority_line_are_edited_in_place() {
    let text = "## Now
### T-001 · Part
- priority: P1 · read: a.md · verify: old check
- accept: done
";
    let doc = parse(text);
    let task = doc.task_by_id(TaskId(1)).expect("T-001");
    let mut builder = TaskEditBuilder::new(&doc, task);
    builder.set_field("verify", "new check");
    builder.set_field("read", "b.md");
    let (edits, intent) = builder.build();
    let applied = doc
        .apply_and_verify(&edits, &intent)
        .expect("verified write");
    assert_eq!(
        String::from_utf8(applied.bytes).expect("UTF-8"),
        "## Now
### T-001 · Part
- priority: P1 · read: b.md · verify: new check
- accept: done
"
    );
}

#[test]
fn a_steps_part_of_the_priority_line_moves_below_it_when_steps_are_set() {
    let text = "## Now
### T-001 · Part
- priority: P1 · steps: one step
- accept: done
";
    let doc = parse(text);
    let task = doc.task_by_id(TaskId(1)).expect("T-001");
    let mut builder = TaskEditBuilder::new(&doc, task);
    builder.set_steps(&["two".to_string(), "three".to_string()]);
    let (edits, intent) = builder.build();
    let applied = doc
        .apply_and_verify(&edits, &intent)
        .expect("verified write");
    assert_eq!(
        String::from_utf8(applied.bytes).expect("UTF-8"),
        "## Now
### T-001 · Part
- priority: P1
- steps:
  1. two
  2. three
- accept: done
"
    );
    let moved = applied.doc.task_by_id(TaskId(1)).expect("T-001");
    assert_eq!(moved.steps, vec!["two".to_string(), "three".to_string()]);
    assert!(
        applied.doc.problems.is_empty(),
        "{:?}",
        applied.doc.problems
    );
}

#[test]
fn unsetting_steps_on_the_priority_line_removes_only_that_part() {
    let text = "## Now
### T-001 · Part
- priority: P1 · steps: one step · area: demo
";
    let doc = parse(text);
    let task = doc.task_by_id(TaskId(1)).expect("T-001");
    let mut builder = TaskEditBuilder::new(&doc, task);
    builder.unset_steps();
    let (edits, intent) = builder.build();
    let applied = doc
        .apply_and_verify(&edits, &intent)
        .expect("verified write");
    assert_eq!(
        String::from_utf8(applied.bytes).expect("UTF-8"),
        "## Now
### T-001 · Part
- priority: P1 · area: demo
"
    );
}

#[test]
fn setting_steps_twice_in_one_edit_keeps_the_last_list() {
    let (bytes, _) = fixture_text("steps_lf.md");
    let doc = Document::parse_bytes(&bytes).expect("UTF-8");
    let task = doc.task_by_id(TaskId(1)).expect("T-001");
    let mut builder = TaskEditBuilder::new(&doc, task);
    builder.set_steps(&["first try".to_string()]);
    builder.set_steps(&["second try".to_string(), "again".to_string()]);
    let (edits, intent) = builder.build();
    let applied = doc
        .apply_and_verify(&edits, &intent)
        .expect("verified write");
    let text = String::from_utf8(applied.bytes).expect("UTF-8");
    assert!(
        text.contains("- steps:\n  1. second try\n  2. again\n- accept"),
        "{text}"
    );
    assert!(!text.contains("first try"), "{text}");
}

#[test]
fn setting_then_unsetting_steps_of_a_task_without_them_changes_nothing() {
    let (bytes, _) = fixture_text("steps_lf.md");
    let doc = Document::parse_bytes(&bytes).expect("UTF-8");
    let task = doc.task_by_id(TaskId(2)).expect("T-002");
    let mut builder = TaskEditBuilder::new(&doc, task);
    builder.set_steps(&["never kept".to_string()]);
    builder.unset_steps();
    let (edits, intent) = builder.build();
    let applied = doc
        .apply_and_verify(&edits, &intent)
        .expect("verified write");
    assert!(!applied.changed, "nothing may change");
    assert_eq!(applied.bytes, bytes);
}
