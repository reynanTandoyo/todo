#![allow(clippy::expect_used)] // whole file is test code

//! The tool version stamp at the document level (R-VER-1, `docs/format.md` §4): which edit a write
//! gets, how versions compare, and which warnings a stamp produces.

use todo::doc::{Document, LineEdit, ProblemKind, ProblemSeverity, stamp_bytes, tool_line_text};
use todo::version::{RUNNING_VERSION, ToolVersion};

fn v(major: u32, minor: u32, patch: u32) -> ToolVersion {
    ToolVersion {
        major,
        minor,
        patch,
    }
}

fn parse(text: &str) -> Document {
    Document::parse_bytes(text.as_bytes()).expect("valid UTF-8")
}

const TASK: &str = "### T-001 · Sort the lantern parts\n- status: open\n- owner: —\n";

#[test]
fn a_missing_stamp_is_inserted_after_the_format_line() {
    let doc = parse(&format!("<!-- todo-format: 1 -->\n# Title\n\n{TASK}"));
    assert_eq!(
        doc.tool_stamp_edit(v(0, 6, 0)),
        Some(LineEdit::insert(1, vec![tool_line_text(v(0, 6, 0))]))
    );
}

#[test]
fn a_missing_stamp_without_a_format_line_goes_on_the_first_line() {
    let doc = parse(&format!("# Title\n\n{TASK}"));
    assert_eq!(
        doc.tool_stamp_edit(v(0, 6, 0)),
        Some(LineEdit::insert(
            0,
            vec!["<!-- todo-tool: 0.6.0 -->".to_string()]
        ))
    );
}

#[test]
fn a_lower_stamp_is_replaced_on_its_own_line() {
    let doc = parse("<!-- todo-format: 1 -->\n<!-- todo-tool: 0.4.0 -->\n# Title\n");
    assert_eq!(
        doc.tool_stamp_edit(v(0, 6, 0)),
        Some(LineEdit::replace_line(1, "<!-- todo-tool: 0.6.0 -->"))
    );
}

#[test]
fn a_stamp_equal_to_or_above_the_running_version_is_never_lowered() {
    for stamp in ["0.6.0", "0.6.1", "0.7.0", "1.0.0"] {
        let doc = parse(&format!(
            "<!-- todo-format: 1 -->\n<!-- todo-tool: {stamp} -->\n# Title\n"
        ));
        assert_eq!(doc.tool_stamp_edit(v(0, 6, 0)), None, "stamp {stamp}");
    }
}

#[test]
fn versions_compare_number_by_number() {
    // Compared as text, "0.9.0" would be above "0.10.0"; numerically it is below.
    let stamp_0_9 = parse("<!-- todo-format: 1 -->\n<!-- todo-tool: 0.9.0 -->\n");
    assert!(stamp_0_9.tool_stamp_edit(v(0, 10, 0)).is_some());
    assert_eq!(stamp_0_9.tool_stamp_edit(v(0, 9, 0)), None);

    let stamp_0_10 = parse("<!-- todo-format: 1 -->\n<!-- todo-tool: 0.10.0 -->\n");
    assert_eq!(stamp_0_10.tool_stamp_edit(v(0, 9, 0)), None);

    // A stamp above the running version warns; 10.0.0 stays above it for every 1.x to 9.x release.
    let stamp_10 = parse("<!-- todo-format: 1 -->\n<!-- todo-tool: 10.0.0 -->\n");
    assert!(
        stamp_10
            .newer_tool_warning("TODO.md")
            .is_some_and(|w| w.starts_with("TODO.md:2:"))
    );
}

#[test]
fn only_the_first_stamp_in_the_preamble_counts() {
    let doc = parse(
        "<!-- todo-format: 1 -->\n<!-- todo-tool: 0.4.0 -->\n<!-- todo-tool: 0.9.0 -->\n# Title\n",
    );
    let stamp = doc.tool_stamp.as_ref().expect("stamp");
    assert_eq!(stamp.line, 1);
    assert_eq!(stamp.value, "0.4.0");
}

#[test]
fn a_stamp_after_the_first_heading_is_not_a_stamp() {
    let doc = parse(&format!(
        "<!-- todo-format: 1 -->\n# Title\n\n## Now\n\n<!-- todo-tool: 0.9.0 -->\n\n{TASK}"
    ));
    assert!(doc.tool_stamp.is_none());
    assert!(doc.newer_tool_warning("TODO.md").is_none());
}

#[test]
fn a_value_that_is_not_a_version_is_kept_and_warned_about() {
    let doc = parse("<!-- todo-format: 1 -->\n<!-- todo-tool: soon -->\n# Title\n");
    assert_eq!(doc.tool_stamp_edit(v(0, 6, 0)), None);
    let problem = doc
        .problems
        .iter()
        .find(|p| matches!(p.kind, ProblemKind::InvalidToolVersion { .. }))
        .expect("invalid stamp problem");
    assert_eq!(problem.severity, ProblemSeverity::Warning);
    assert_eq!(problem.line, 2);
    assert!(!doc.blocks_all_writes());
}

#[test]
fn a_newer_stamp_is_a_warning_that_does_not_block_writes() {
    let doc = parse("<!-- todo-format: 1 -->\n<!-- todo-tool: 99.0.0 -->\n# Title\n");
    let problem = doc
        .problems
        .iter()
        .find(|p| matches!(p.kind, ProblemKind::NewerToolVersion { .. }))
        .expect("newer stamp problem");
    assert_eq!(problem.severity, ProblemSeverity::Warning);
    assert!(!doc.blocks_all_writes());
    let warning = doc.newer_tool_warning("TODO.md").expect("warning");
    assert!(warning.starts_with("TODO.md:2: this todo ("), "{warning}");
    assert!(warning.contains("is behind"));
    assert!(warning.contains("todo help update"));
}

#[test]
fn a_document_with_a_format_line_it_cannot_write_is_never_stamped() {
    let text = "<!-- todo-format: 99 -->\n# Title\n";
    let doc = parse(text);
    assert!(doc.blocks_all_writes());
    assert_eq!(
        doc.with_tool_stamp(RUNNING_VERSION).expect("no error"),
        None
    );
    assert_eq!(
        stamp_bytes(text.as_bytes(), Default::default()).expect("no error"),
        text.as_bytes()
    );
}

#[test]
fn with_tool_stamp_adds_the_line_and_keeps_every_task() {
    let text = format!("<!-- todo-format: 1 -->\n# Title\n\n## Now\n\n{TASK}");
    let doc = parse(&text);
    let applied = doc
        .with_tool_stamp(v(0, 6, 0))
        .expect("no error")
        .expect("stamp needed");
    assert_eq!(
        applied.bytes,
        format!("<!-- todo-format: 1 -->\n<!-- todo-tool: 0.6.0 -->\n# Title\n\n## Now\n\n{TASK}")
            .into_bytes()
    );
    assert_eq!(applied.doc.tasks.len(), doc.tasks.len());
    assert!(applied.changed);
}

#[test]
fn stamp_bytes_is_idempotent_and_leaves_non_utf8_alone() {
    let stamped = stamp_bytes(
        format!("<!-- todo-format: 1 -->\n{TASK}").as_bytes(),
        Default::default(),
    )
    .expect("no error");
    let text = String::from_utf8(stamped.clone()).expect("utf-8");
    assert!(text.contains(&tool_line_text(RUNNING_VERSION)));

    let again = stamp_bytes(&stamped, Default::default()).expect("no error");
    assert_eq!(again, stamped);

    let not_utf8 = [0xff_u8, 0xfe, b'\n'];
    assert_eq!(
        stamp_bytes(&not_utf8, Default::default()).expect("no error"),
        not_utf8.to_vec()
    );
}
