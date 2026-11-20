//! Format migrations at the document level (`docs/format.md` §18, R-MIG-1): the steps on LF, CRLF,
//! byte-order-mark and no-format-line fixtures, the order of steps, the format 2 to 3 rewrite of
//! difficulty names to levels (R-LEVEL-1), and a step that changes task lines. Every byte outside
//! the changed lines must stay identical.

mod common;

use todo::doc::migrate::FORMAT_2_TO_3;
use todo::doc::{
    CURRENT_FORMAT_VERSION, Document, LineEdit, MigrationError, MigrationSettings, MigrationStep,
    STEPS, Status, migrate_document,
};

/// The running version, as Cargo.toml gives it.
const RUNNING: &str = env!("CARGO_PKG_VERSION");

/// The settings of a `todo.toml` that sets no `difficulties` (the defaults).
const DEFAULTS: MigrationSettings = MigrationSettings {
    difficulties_set: false,
};

fn text_of(name: &str) -> String {
    String::from_utf8(common::fixture(name)).expect("UTF-8 fixture")
}

/// The expected result of migrating `before` (a format 1 file) to the current format: the format
/// line raised to 3, the tool line added directly after it, and nothing else changed. Every other
/// line stays as it was, with its own line ending.
fn expected_after_migration(before: &str, eol: &str) -> String {
    let format_1 = format!("<!-- todo-format: 1 -->{eol}");
    let format_3 = format!(
        "<!-- todo-format: {CURRENT_FORMAT_VERSION} -->{eol}<!-- todo-tool: {RUNNING} -->{eol}"
    );
    assert!(before.contains(&format_1), "fixture has no format 1 line");
    before.replacen(&format_1, &format_3, 1)
}

fn migrate_to_current(text: &str) -> todo::doc::Migrated {
    let doc = Document::parse_bytes(text.as_bytes()).expect("valid UTF-8");
    migrate_document(&doc, STEPS, CURRENT_FORMAT_VERSION, &DEFAULTS)
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
fn format_one_to_current_on_an_lf_file_changes_only_the_format_and_tool_lines() {
    let before = text_of("migrate_format1_lf.md");
    let migrated = migrate_to_current(&before);
    assert_eq!((migrated.from, migrated.to), (1, CURRENT_FORMAT_VERSION));
    assert_eq!(
        String::from_utf8(migrated.doc.to_bytes()).expect("UTF-8"),
        expected_after_migration(&before, "\n")
    );
}

#[test]
fn format_one_to_current_keeps_crlf_endings_on_every_line() {
    let before = text_of("migrate_format1_crlf.md");
    let migrated = migrate_to_current(&before);
    assert_eq!(
        String::from_utf8(migrated.doc.to_bytes()).expect("UTF-8"),
        expected_after_migration(&before, "\r\n")
    );
}

#[test]
fn format_one_to_current_keeps_the_byte_order_mark_and_crlf_endings() {
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
        expected_after_migration(&before, "\r\n")
    );
}

#[test]
fn a_file_without_a_format_line_gets_the_format_line_at_the_top_and_the_stamp_after_it() {
    let before = text_of("migrate_format1_no_format_line.md");
    let migrated = migrate_to_current(&before);
    let after = String::from_utf8(migrated.doc.to_bytes()).expect("UTF-8");
    let expected = format!(
        "<!-- todo-format: {CURRENT_FORMAT_VERSION} -->\n<!-- todo-tool: {RUNNING} -->\n{before}"
    );
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
        migrate_document(&doc, STEPS, CURRENT_FORMAT_VERSION, &DEFAULTS).expect("no error"),
        None
    );
}

#[test]
fn a_step_may_change_a_task_line_and_then_only_that_line_changes() {
    fn rename_first_task(doc: &Document, _settings: &MigrationSettings) -> Vec<LineEdit> {
        vec![LineEdit::replace_line(
            doc.tasks[0].block_range.start,
            "### T-999 · Renamed",
        )]
    }
    const RENAMES_TASK: MigrationStep = MigrationStep {
        from: 1,
        to: 2,
        edits: rename_first_task,
    };

    let before = text_of("migrate_format1_lf.md");
    let doc = Document::parse_bytes(before.as_bytes()).expect("UTF-8");
    let migrated = migrate_document(&doc, &[RENAMES_TASK], 2, &DEFAULTS)
        .expect("no error")
        .expect("format 1 is migrated");
    let after = String::from_utf8(migrated.doc.to_bytes()).expect("UTF-8");
    let heading = doc.tasks[0].heading_line;
    let expected: String = before
        .split_inclusive('\n')
        .enumerate()
        .map(|(index, line)| {
            if index == heading {
                "### T-999 · Renamed\n".to_string()
            } else {
                line.to_string()
            }
        })
        .collect();
    assert_eq!(after, expected);
}

#[test]
fn a_chain_with_a_missing_step_is_refused() {
    let doc = Document::parse_bytes(b"<!-- todo-format: 1 -->\n# Title\n").expect("UTF-8");
    let err = migrate_document(&doc, &[], CURRENT_FORMAT_VERSION, &DEFAULTS).expect_err("no step");
    assert_eq!(err, MigrationError::NoStep { version: 1 });
}

/// The expected text of `migrate_format2_levels.md` after format 2 to 3, with the LF line endings
/// of the fixture: the format line raised to 3 and only the difficulty of the open entries with an
/// old name rewritten (`docs/format.md` §18). Done and dropped entries and the entries without a
/// difficulty keep their lines.
fn expected_levels_after_format_2_to_3() -> String {
    let before = text_of("migrate_format2_levels.md");
    let changes = [
        // T-001: a combined priority line; only the part after `difficulty:` changes.
        (
            "- priority: P1 · difficulty: easy · type: feature · area: buoys",
            "- priority: P1 · difficulty: 2 · type: feature · area: buoys",
        ),
        // T-002: an old name in another case, on the combined line after `area`.
        (
            "- priority: P2 · area: pier · difficulty: Medium",
            "- priority: P2 · area: pier · difficulty: 3",
        ),
        // T-003: a claimed task, which is not done, on its own line (T-007, dropped, has the same
        // line and keeps it).
        (
            "- priority: P1\n- difficulty: hard\n",
            "- priority: P1\n- difficulty: 4\n",
        ),
    ];
    let mut text = before.replacen(
        "<!-- todo-format: 2 -->\n",
        &format!("<!-- todo-format: {CURRENT_FORMAT_VERSION} -->\n"),
        1,
    );
    for (old, new) in changes {
        assert_eq!(
            text.matches(old).count(),
            1,
            "fixture must hold {old:?} once"
        );
        text = text.replacen(old, new, 1);
    }
    text
}

#[test]
fn format_two_to_three_rewrites_the_old_names_of_open_entries_and_nothing_else() {
    let before = text_of("migrate_format2_levels.md");
    let doc = Document::parse_bytes(before.as_bytes()).expect("UTF-8");
    let migrated = migrate_document(&doc, &[FORMAT_2_TO_3], CURRENT_FORMAT_VERSION, &DEFAULTS)
        .expect("no error")
        .expect("format 2 is migrated");
    assert_eq!((migrated.from, migrated.to), (2, CURRENT_FORMAT_VERSION));
    assert_eq!(
        String::from_utf8(migrated.doc.to_bytes()).expect("UTF-8"),
        expected_levels_after_format_2_to_3()
    );
}

#[test]
fn format_two_to_three_reads_the_rewritten_levels_back_from_the_parsed_tasks() {
    let doc = Document::parse_bytes(&common::fixture("migrate_format2_levels.md")).expect("UTF-8");
    let migrated = migrate_document(&doc, &[FORMAT_2_TO_3], CURRENT_FORMAT_VERSION, &DEFAULTS)
        .expect("no error")
        .expect("format 2 is migrated");
    let level = |id: u64| {
        migrated
            .doc
            .tasks
            .iter()
            .find(|t| t.id.0 == id)
            .map(|t| (t.difficulty.clone(), t.status))
            .expect("task is there")
    };
    assert_eq!(level(1), (Some("2".to_string()), Status::Open));
    assert_eq!(level(2), (Some("3".to_string()), Status::Open));
    assert_eq!(level(3), (Some("4".to_string()), Status::Claimed));
    // No difficulty: left for the planner's list (R-LEVEL-2).
    assert_eq!(level(4), (None, Status::Open));
    // A number that is already a level is not an old name.
    assert_eq!(level(5), (Some("5".to_string()), Status::Open));
    // Done and dropped entries keep their old names as a record (format.md §18).
    assert_eq!(level(6), (Some("easy".to_string()), Status::Done));
    assert_eq!(level(7), (Some("hard".to_string()), Status::Dropped));
}

#[test]
fn format_two_to_three_keeps_crlf_endings_on_the_rewritten_lines() {
    let before = text_of("migrate_format2_levels.md").replace('\n', "\r\n");
    let doc = Document::parse_bytes(before.as_bytes()).expect("UTF-8");
    let migrated = migrate_document(&doc, &[FORMAT_2_TO_3], CURRENT_FORMAT_VERSION, &DEFAULTS)
        .expect("no error")
        .expect("format 2 is migrated");
    assert_eq!(
        String::from_utf8(migrated.doc.to_bytes()).expect("UTF-8"),
        expected_levels_after_format_2_to_3().replace('\n', "\r\n")
    );
}

#[test]
fn format_two_to_three_keeps_the_old_names_when_todo_toml_sets_difficulties() {
    let before = text_of("migrate_format2_levels.md");
    let doc = Document::parse_bytes(before.as_bytes()).expect("UTF-8");
    let settings = MigrationSettings {
        difficulties_set: true,
    };
    let migrated = migrate_document(&doc, &[FORMAT_2_TO_3], CURRENT_FORMAT_VERSION, &settings)
        .expect("no error")
        .expect("format 2 is migrated");
    let expected = before.replacen(
        "<!-- todo-format: 2 -->\n",
        &format!("<!-- todo-format: {CURRENT_FORMAT_VERSION} -->\n"),
        1,
    );
    assert_eq!(
        String::from_utf8(migrated.doc.to_bytes()).expect("UTF-8"),
        expected,
        "only the format line changes"
    );
}

#[cfg(feature = "test-hooks")]
#[test]
fn two_steps_run_in_order_when_a_test_only_step_follows_format_one_to_two() {
    use todo::doc::migrate::{FORMAT_1_TO_2, TEST_STEP_2_TO_3};

    let doc = Document::parse_bytes(&common::fixture("migrate_format1_lf.md")).expect("UTF-8");
    let migrated = migrate_document(&doc, &[FORMAT_1_TO_2, TEST_STEP_2_TO_3], 3, &DEFAULTS)
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
    let err =
        migrate_document(&doc, &[TEST_STEP_2_TO_3], 3, &DEFAULTS).expect_err("format 2 is needed");
    assert_eq!(err, MigrationError::NoStep { version: 1 });
}
