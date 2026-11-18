//! Integration tests for the combined `- priority:` line: its parts are read as the fields they name and
//! edited where they are (T-114, T-117; `docs/format.md` §8).
//!
//! The helpers panic on setup failures, which is acceptable in test code (see `tests/common`).
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

mod common;

use std::path::Path;
use std::process::Output;

use common::{parse_json, read_text, run_todo, setup_proj, stderr_text, stdout_text};
use tempfile::TempDir;

const MARKER: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->";

/// A TODO.md with one `## Tasks` section that holds `body`.
fn doc(body: &str) -> String {
    format!("{MARKER}\n<!-- todo-format: 1 -->\n\n## Tasks\n\n{body}")
}

/// The same file after a write: migrated to the current format (R-MIG-1), with the running tool stamp
/// directly after the format line (R-VER-1).
fn written(body: &str) -> String {
    format!(
        "{MARKER}\n<!-- todo-format: 2 -->\n<!-- todo-tool: {} -->\n\n## Tasks\n\n{body}",
        env!("CARGO_PKG_VERSION")
    )
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

#[test]
fn why_problem_fix_accept_files_depends_and_commit_on_the_combined_line_are_read_as_fields() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Parse the config\n- status: open\n- owner: —\n\
                - priority: P0 · why: keep the parser simple · files: src/config.rs · problem: no parser yet · \
                fix: add one · accept: the test passes · depends: T-002 · commit: abc1234\n\
                ### T-002 · Write the parser\n- status: open\n- owner: —\n- priority: P2\n";
    let (home, work, _) = setup_proj(&tmp, &doc(body));

    let show = parse_json(&ok(&home, &work, &["--json", "show", "T-001"]).stdout);
    let data = &show["data"];
    assert_eq!(data["why"], "keep the parser simple");
    assert_eq!(data["files"], "src/config.rs");
    assert_eq!(data["problem"], "no parser yet");
    assert_eq!(data["fix"], "add one");
    assert_eq!(data["accept"], "the test passes");
    assert_eq!(data["depends"][0], "T-002");
    assert_eq!(data["commit"][0], "abc1234");

    let listed = parse_json(
        &ok(
            &home,
            &work,
            &["--json", "list", "--where", "problem=no parser yet"],
        )
        .stdout,
    );
    assert_eq!(listed["data"]["count"], 1, "--where finds a combined part");

    // The dependency on T-002 is read, so the P0 task is not ready while T-002 is open.
    let next = parse_json(&ok(&home, &work, &["--json", "next"]).stdout);
    assert_eq!(next["data"]["task"]["id"], "T-002");
}

#[test]
fn set_edits_combined_parts_where_they_are_and_unset_removes_only_them() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Parse the config\n- status: open\n- owner: —\n\
                - priority: P1 · why: old why · area: ui · files: old.rs · problem: old problem · \
                fix: old fix · accept: old accept · depends: T-002 · commit: aaa111\n\
                ### T-002 · Second\n- status: open\n- owner: —\n- priority: P2\n\
                ### T-003 · Third\n- status: open\n- owner: —\n- priority: P2\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));

    ok(
        &home,
        &work,
        &[
            "--agent",
            "t-agent",
            "set",
            "T-001",
            "--why",
            "new why",
            "--files",
            "new.rs",
            "--problem",
            "new problem",
            "--fix",
            "new fix",
            "--accept",
            "new accept",
            "--depends",
            "T-003",
            "--field",
            "commit=bbb222",
        ],
    );
    let text = read_text(&file);
    assert!(
        text.contains(
            "- priority: P1 · why: new why · area: ui · files: new.rs · problem: new problem · \
             fix: new fix · accept: new accept · depends: T-003 · commit: bbb222\n"
        ),
        "{text}"
    );
    assert_eq!(
        text.matches("- priority:").count(),
        3,
        "no second copy of the line: {text}"
    );

    ok(
        &home,
        &work,
        &[
            "--agent", "t-agent", "set", "T-001", "--unset", "why", "--unset", "files", "--unset",
            "commit",
        ],
    );
    assert!(
        read_text(&file).contains(
            "- priority: P1 · area: ui · problem: new problem · fix: new fix · accept: new accept · \
             depends: T-003\n"
        ),
        "{}",
        read_text(&file)
    );
    let show = parse_json(&ok(&home, &work, &["--json", "show", "T-001"]).stdout);
    assert!(show["data"]["why"].is_null(), "{show}");
    assert!(show["data"]["files"].is_null(), "{show}");
    assert_eq!(show["data"]["problem"], "new problem");
}

#[test]
fn a_value_with_the_part_separator_moves_to_a_line_of_its_own() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Parse the config\n- status: open\n- owner: —\n\
                - priority: P1 · why: old · area: ui\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));

    ok(
        &home,
        &work,
        &["--agent", "t-agent", "set", "T-001", "--why", "new · value"],
    );
    assert_eq!(
        read_text(&file),
        written(
            "### T-001 · Parse the config\n- status: open\n- owner: —\n\
             - priority: P1 · area: ui\n- why: new · value\n"
        ),
        "the separator would split the value on the combined line, so it gets a line of its own"
    );
    let show = parse_json(&ok(&home, &work, &["--json", "show", "T-001"]).stdout);
    assert_eq!(show["data"]["why"], "new · value");
}

#[test]
fn a_combined_part_repeated_on_its_own_line_is_a_repeated_field() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Parse the config\n- status: open\n- owner: —\n\
                - priority: P1 · why: one\n- why: two\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));

    let out = run_todo(&home, &work, &["check"], &[]).unwrap();
    let text = format!("{}{}", stdout_text(&out), stderr_text(&out));
    assert!(text.contains("repeated field 'why'"), "{text}");

    let before = read_text(&file);
    let out = run_todo(
        &home,
        &work,
        &["--agent", "t-agent", "set", "T-001", "--priority", "P0"],
        &[],
    )
    .unwrap();
    assert!(!out.status.success(), "a repeated field blocks the write");
    assert_eq!(read_text(&file), before);
}

#[test]
fn unset_priority_moves_a_combined_part_to_its_own_line() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Parse the config\n- status: open\n- owner: —\n\
                - priority: P1 · why: keep it · area: ui\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));

    ok(
        &home,
        &work,
        &["--agent", "t-agent", "set", "T-001", "--unset", "priority"],
    );
    assert_eq!(
        read_text(&file),
        written(
            "### T-001 · Parse the config\n- status: open\n- owner: —\n\
             - area: ui\n- why: keep it\n"
        )
    );
}
