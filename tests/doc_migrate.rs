//! Format migrations at the document level (`docs/format.md` §18, R-MIG-1): the first step on LF,
//! CRLF, byte-order-mark and no-format-line fixtures, the order of steps, and the refusal of a step
//! that would change task lines. Every byte outside the changed lines must stay identical.

mod common;

use todo::doc::{
    CURRENT_FORMAT_VERSION, Document, LineEdit, MigrationError, MigrationStep, STEPS,
    migrate_document,
};

/// The running version, as Cargo.toml gives it.
const RUNNING: &str = env!("CARGO_PKG_VERSION");

fn text_of(name: &str) -> String {
    String::from_utf8(common::fixture(name)).expect("UTF-8 fixture")
}

/// The expected result of the first step on `before`: the format line raised to 2 and the tool
/// line added directly after it. Every other line stays as it was, with its own line ending.
fn expected_after_format_1_to_2(before: &str, eol: &str) -> String {
    let format_1 = format!("<!-- todo-format: 1 -->{eol}");
    let format_2 = format!("<!-- todo-format: 2 -->{eol}<!-- todo-tool: {RUNNING} -->{eol}");
    assert!(before.contains(&format_1), "fixture has no format 1 line");
    before.replacen(&format_1, &format_2, 1)
}

fn migrate_to_current(text: &str) -> todo::doc::Migrated {
    let doc = Document::parse_bytes(text.as_bytes()).expect("valid UTF-8");
    migrate_document(&doc, STEPS, CURRENT_FORMAT_VERSION)
        .expect("no error")
        .expect("format 1 is migrated")
}

#[test]
fn the_steps_go_one_format_version_at_a_time_up_to_the_current_format() {
    assert_eq!(STEPS.first().map(|s| s.from), Some(1));
    for pair in STEPS.windows(2) {
        assert_eq!(pair[0].to, pair[1].from, "steps must chain");
    }
    for step in STEPS {
        assert_eq!(step.to, step.from + 1, "a step moves one version");
    }
    assert_eq!(STEPS.last().map(|s| s.to), Some(CURRENT_FORMAT_VERSION));
}

#[test]
fn format_one_to_two_on_an_lf_file_changes_only_the_format_and_tool_lines() {
    let before = text_of("migrate_format1_lf.md");
    let migrated = migrate_to_current(&before);
    assert_eq!((migrated.from, migrated.to), (1, CURRENT_FORMAT_VERSION));
    assert_eq!(
        String::from_utf8(migrated.doc.to_bytes()).expect("UTF-8"),
        expected_after_format_1_to_2(&before, "\n")
    );
}

#[test]
fn format_one_to_two_keeps_crlf_endings_on_every_line() {
    let before = text_of("migrate_format1_crlf.md");
    let migrated = migrate_to_current(&before);
    assert_eq!(
        String::from_utf8(migrated.doc.to_bytes()).expect("UTF-8"),
        expected_after_format_1_to_2(&before, "\r\n")
    );
}

#[test]
fn format_one_to_two_keeps_the_byte_order_mark_and_crlf_endings() {
    let before = text_of("migrate_format1_bom_crlf.md");
    assert!(
        before.starts_with('\u{feff}'),
        "fixture must start with a BOM"
    );
    let migrated = migrate_to_current(&before);
    let after = migrated.doc.to_bytes();
    assert_eq!(&after[..3], &[0xEF, 0xBB, 0xBF], "the BOM stays first");
    assert_eq!(
        String::from_utf8(after).expect("UTF-8"),
        expected_after_format_1_to_2(&before, "\r\n")
    );
}

#[test]
fn a_file_without_a_format_line_gets_the_format_line_at_the_top_and_the_stamp_after_it() {
    let before = text_of("migrate_format1_no_format_line.md");
    let migrated = migrate_to_current(&before);
    let after = String::from_utf8(migrated.doc.to_bytes()).expect("UTF-8");
    let expected = format!("<!-- todo-format: 2 -->\n<!-- todo-tool: {RUNNING} -->\n{before}");
    assert_eq!(after, expected);
    assert_eq!(migrated.doc.format_line, Some(0));
    assert_eq!(migrated.doc.tool_stamp.as_ref().map(|s| s.line), Some(1));
}

#[test]
fn the_migrated_tasks_are_the_same_tasks() {
    let before = Document::parse_bytes(&common::fixture("migrate_format1_lf.md")).expect("UTF-8");
    let migrated = migrate_to_current(&text_of("migrate_format1_lf.md"));
    let ids_before: Vec<_> = before
        .tasks
        .iter()
        .map(|t| (t.id, t.title.clone(), t.status))
        .collect();
    let ids_after: Vec<_> = migrated
        .doc
        .tasks
        .iter()
        .map(|t| (t.id, t.title.clone(), t.status))
        .collect();
    assert_eq!(ids_before, ids_after);
    assert_eq!(ids_after.len(), 4);
    assert!(migrated.doc.problems.iter().all(|p| !p.blocks_all_writes()));
}

#[test]
fn a_document_already_in_the_current_format_has_nothing_to_migrate() {
    let text = format!("<!-- todo-format: {CURRENT_FORMAT_VERSION} -->\n# Title\n");
    let doc = Document::parse_bytes(text.as_bytes()).expect("UTF-8");
    assert_eq!(
        migrate_document(&doc, STEPS, CURRENT_FORMAT_VERSION).expect("no error"),
        None
    );
}

#[test]
fn a_step_that_would_change_a_task_line_is_refused_before_anything_is_applied() {
    fn rename_first_task(doc: &Document) -> Vec<LineEdit> {
        vec![LineEdit::replace_line(
            doc.tasks[0].block_range.start,
            "### T-999 · Renamed",
        )]
    }
    const TOUCHES_TASK: MigrationStep = MigrationStep {
        from: 1,
        to: 2,
        edits: rename_first_task,
    };

    let doc = Document::parse_bytes(&common::fixture("migrate_format1_lf.md")).expect("UTF-8");
    let err = migrate_document(&doc, &[TOUCHES_TASK], CURRENT_FORMAT_VERSION)
        .expect_err("task lines must not change");
    assert_eq!(err, MigrationError::ChangesTaskLines { from: 1 });
}

#[test]
fn a_chain_with_a_missing_step_is_refused() {
    let doc = Document::parse_bytes(b"<!-- todo-format: 1 -->\n# Title\n").expect("UTF-8");
    let err = migrate_document(&doc, &[], CURRENT_FORMAT_VERSION).expect_err("no step");
    assert_eq!(err, MigrationError::NoStep { version: 1 });
}

#[cfg(feature = "test-hooks")]
#[test]
fn two_steps_run_in_order_when_a_test_only_step_follows_format_one_to_two() {
    use todo::doc::migrate::{FORMAT_1_TO_2, TEST_STEP_2_TO_3};

    let doc = Document::parse_bytes(&common::fixture("migrate_format1_lf.md")).expect("UTF-8");
    let migrated = migrate_document(&doc, &[FORMAT_1_TO_2, TEST_STEP_2_TO_3], 3)
        .expect("no error")
        .expect("format 1 is migrated");
    assert_eq!((migrated.from, migrated.to), (1, 3));
    let text = String::from_utf8(migrated.doc.to_bytes()).expect("UTF-8");
    assert!(text.contains("<!-- todo-format: 3 -->\n"), "{text}");
    assert_eq!(
        text.matches("todo-tool:").count(),
        1,
        "the tool line is added once"
    );
    assert!(!text.contains("todo-format: 1"));

    // The test-only step cannot start a chain: it needs format 2 first.
    let err = migrate_document(&doc, &[TEST_STEP_2_TO_3], 3).expect_err("format 2 is needed");
    assert_eq!(err, MigrationError::NoStep { version: 1 });
}
