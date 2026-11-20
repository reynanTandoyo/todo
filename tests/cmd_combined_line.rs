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
        "{MARKER}\n<!-- todo-format: 3 -->\n<!-- todo-tool: {} -->\n\n## Tasks\n\n{body}",
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
fn set_edits_an_unknown_part_of_the_combined_line_in_place() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Parse the config\n- status: open\n- owner: —\n\
                - priority: P1 · custom_tag: a · area: ui\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));

    ok(
        &home,
        &work,
        &[
            "--agent",
            "t-agent",
            "set",
            "T-001",
            "--field",
            "custom_tag=b",
        ],
    );
    assert_eq!(
        read_text(&file),
        written(
            "### T-001 · Parse the config\n- status: open\n- owner: —\n\
             - priority: P1 · custom_tag: b · area: ui\n"
        ),
        "the part is changed where it is, with no second copy"
    );

    ok(
        &home,
        &work,
        &[
            "--agent",
            "t-agent",
            "set",
            "T-001",
            "--unset",
            "custom_tag",
        ],
    );
    assert_eq!(
        read_text(&file),
        written(
            "### T-001 · Parse the config\n- status: open\n- owner: —\n- priority: P1 · area: ui\n"
        ),
        "unset removes only that part of the line"
    );
}

#[test]
fn why_problem_fix_accept_files_depends_and_commit_on_the_combined_line_are_read_as_fields() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Parse the config\n- status: open\n- owner: —\n\
                - priority: P0 · why: keep the parser simple · files: src/config.rs · problem: no parser yet · \
                fix: add one · accept: the test passes · depends: T-002 · commit: abc1234\n\
                ### T-002 · Write the parser\n- status: open\n- owner: —\n- priority: P2 · difficulty: 2\n";
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

// T-133: status, owner and claimed on the combined priority line are their fields (`docs/format.md` §8).

#[test]
fn status_owner_and_claimed_on_the_combined_line_are_read_and_drive_next() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Claimed on the line\n\
                - priority: P1 · status: claimed · owner: alice · claimed: 2026-11-15T10:00+01:00\n\
                ### T-002 · Open\n- status: open\n- owner: —\n- priority: P2 · difficulty: 2\n";
    let (home, work, _) = setup_proj(&tmp, &doc(body));

    let show = parse_json(&ok(&home, &work, &["--json", "show", "T-001"]).stdout);
    assert_eq!(show["data"]["status"], "claimed", "{show}");
    assert_eq!(show["data"]["owner"], "alice", "{show}");
    assert_eq!(show["data"]["claimed"], "2026-11-15T10:00+01:00", "{show}");

    // A claimed task is not handed out, so the open P2 task is the next one. The claim is half an
    // hour old at the pinned time, so it is not stale.
    let out = run_todo(
        &home,
        &work,
        &["--json", "next"],
        &[("TODO_TEST_NOW", "2026-11-15T10:30:00+01:00")],
    )
    .unwrap();
    assert!(out.status.success(), "{}", stderr_text(&out));
    let next = parse_json(&out.stdout);
    assert_eq!(next["data"]["task"]["id"], "T-002", "{next}");
}

#[test]
fn check_sees_a_stale_claim_held_on_the_combined_line() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Stale\n\
                - priority: P1 · status: claimed · owner: bob · claimed: 2020-01-01T00:00+00:00\n";
    let (home, work, _) = setup_proj(&tmp, &doc(body));

    let out = run_todo(&home, &work, &["check"], &[]).unwrap();
    let text = format!("{}{}", stdout_text(&out), stderr_text(&out));
    assert!(text.contains("T-001: stale claim by bob"), "{text}");
}

#[test]
fn an_unknown_status_on_the_combined_line_is_reported() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Paused\n- priority: P1 · status: paused\n";
    let (home, work, _) = setup_proj(&tmp, &doc(body));

    let out = run_todo(&home, &work, &["check"], &[]).unwrap();
    let text = format!("{}{}", stdout_text(&out), stderr_text(&out));
    assert!(text.contains("unknown status 'paused' on T-001"), "{text}");
}

#[test]
fn a_status_on_the_combined_line_repeated_on_its_own_line_is_a_repeated_field() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Twice\n- priority: P1 · status: open\n- status: done\n";
    let (home, work, _) = setup_proj(&tmp, &doc(body));

    let out = run_todo(&home, &work, &["check"], &[]).unwrap();
    let text = format!("{}{}", stdout_text(&out), stderr_text(&out));
    assert!(text.contains("repeated field 'status'"), "{text}");
}

#[test]
fn release_and_done_edit_the_claim_on_the_combined_line_in_place() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Claimed\n\
                - priority: P1 · status: claimed · owner: alice · claimed: 2026-11-15T10:00+01:00 · area: ui\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));

    ok(&home, &work, &["--agent", "alice", "release", "T-001"]);
    assert_eq!(
        read_text(&file),
        written("### T-001 · Claimed\n- priority: P1 · status: open · owner: — · area: ui\n"),
        "release changes the claim where it is and removes the claimed part"
    );

    let (home, work, file) = setup_proj(&tmp, &doc(body));
    let out = run_todo(
        &home,
        &work,
        &["--agent", "alice", "done", "T-001"],
        &[("TODO_TEST_NOW", "2026-11-15T10:30:00+01:00")],
    )
    .unwrap();
    assert!(out.status.success(), "{}", stderr_text(&out));
    assert_eq!(
        read_text(&file),
        written(
            "### T-001 · Claimed\n- priority: P1 · status: done · owner: alice · area: ui\n\
             - note (2026-11-15, alice): done\n"
        ),
        "done changes the status and owner where they are"
    );
}

#[test]
fn claim_writes_the_status_on_the_combined_line_where_it_is() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Open\n- priority: P1 · status: open · area: ui\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));

    let out = run_todo(
        &home,
        &work,
        &["--agent", "bob", "claim", "T-001"],
        &[("TODO_TEST_NOW", "2026-11-15T14:00:00+01:00")],
    )
    .unwrap();
    assert!(out.status.success(), "{}", stderr_text(&out));
    let text = read_text(&file);
    assert!(
        text.contains("- priority: P1 · status: claimed · area: ui\n"),
        "the status is changed in place, with no second status line: {text}"
    );
    assert_eq!(text.matches("status:").count(), 1, "{text}");
}

// T-134: a segment of the combined priority line that is not `key: value` is never dropped: it is kept
// as written (R-EDIT-1) and `check` reports it (`docs/format.md` §8).

#[test]
fn a_part_without_key_value_survives_a_change_to_the_priority_or_another_part() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Notes\n- status: open\n- owner: —\n\
                - priority: P1 · see the design notes · area: ui\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));

    ok(
        &home,
        &work,
        &["--agent", "t-agent", "set", "T-001", "--priority", "P0"],
    );
    assert_eq!(
        read_text(&file),
        written(
            "### T-001 · Notes\n- status: open\n- owner: —\n\
             - priority: P0 · see the design notes · area: ui\n"
        ),
        "a change of priority keeps the part"
    );

    ok(
        &home,
        &work,
        &["--agent", "t-agent", "set", "T-001", "--area", "ops"],
    );
    assert_eq!(
        read_text(&file),
        written(
            "### T-001 · Notes\n- status: open\n- owner: —\n\
             - priority: P0 · see the design notes · area: ops\n"
        ),
        "a change of another part keeps the part"
    );
}

#[test]
fn removing_the_priority_keeps_a_part_without_key_value_on_a_line_of_its_own() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Notes\n- status: open\n- owner: —\n\
                - priority: P1 · see the design notes · area: ui\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));

    ok(
        &home,
        &work,
        &["--agent", "t-agent", "set", "T-001", "--unset", "priority"],
    );
    assert_eq!(
        read_text(&file),
        written(
            "### T-001 · Notes\n- status: open\n- owner: —\n\
             - see the design notes\n- area: ui\n"
        ),
        "the text stays where the priority was, and the field moves to its own line"
    );
}

#[test]
fn check_reports_a_part_without_key_value_on_its_line_as_a_warning() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Notes\n- status: open\n- owner: —\n\
                - priority: P1 · see the design notes · area: ui\n";
    let (home, work, _) = setup_proj(&tmp, &doc(body));

    let out = run_todo(&home, &work, &["check"], &[]).unwrap();
    let text = format!("{}{}", stdout_text(&out), stderr_text(&out));
    assert_eq!(out.status.code(), Some(1), "{text}");
    // The priority line is line 9 of this file.
    assert!(
        text.contains(
            "TODO.md:9: part 'see the design notes' on the priority line of T-001 is not key: value; it is kept as written"
        ),
        "{text}"
    );
}

// T-178: a value that holds the part separator gets a line of its own, wherever it is written.

/// The `check` output of a project, stdout and stderr together.
fn check_text(home: &Path, work: &Path) -> String {
    let out = run_todo(home, work, &["check"], &[]).unwrap();
    format!("{}{}", stdout_text(&out), stderr_text(&out))
}

#[test]
fn set_goal_with_the_separator_moves_out_of_the_combined_line() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Renderer\n- status: open\n- owner: —\n\
                - priority: P1 · type: milestone · goal: old goal · area: ui\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));

    ok(
        &home,
        &work,
        &[
            "--agent",
            "t-agent",
            "set",
            "T-001",
            "--goal",
            "lights · shadows",
        ],
    );
    assert_eq!(
        read_text(&file),
        written(
            "### T-001 · Renderer\n- status: open\n- owner: —\n\
             - priority: P1 · type: milestone · area: ui\n- goal: lights · shadows\n"
        ),
        "the goal keeps its whole value, on a line of its own"
    );
    let show = parse_json(&ok(&home, &work, &["--json", "show", "T-001"]).stdout);
    assert_eq!(show["data"]["goal"], "lights · shadows");
    assert_eq!(show["data"]["type"], "milestone");
    assert!(!check_text(&home, &work).contains("not key: value"));
}

#[test]
fn set_type_and_area_with_the_separator_move_out_of_the_combined_line() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Lighting\n- status: open\n- owner: —\n\
                - priority: P1 · difficulty: 2 · type: feature · area: ui\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));

    ok(
        &home,
        &work,
        &[
            "--agent",
            "t-agent",
            "set",
            "T-001",
            "--type",
            "dev · ops",
            "--area",
            "web · api",
        ],
    );
    let text = read_text(&file);
    assert!(
        text.contains("- priority: P1 · difficulty: 2\n- type: dev · ops\n- area: web · api\n"),
        "{text}"
    );
    let show = parse_json(&ok(&home, &work, &["--json", "show", "T-001"]).stdout);
    assert_eq!(show["data"]["type"], "dev · ops");
    assert_eq!(show["data"]["area"], "web · api");
    assert!(!check_text(&home, &work).contains("not key: value"));
}

#[test]
fn a_new_type_with_the_separator_is_not_added_to_the_combined_line() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Lighting\n- status: open\n- owner: —\n- priority: P1 · area: ui\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));

    ok(
        &home,
        &work,
        &["--agent", "t-agent", "set", "T-001", "--type", "dev · ops"],
    );
    assert_eq!(
        read_text(&file),
        written(
            "### T-001 · Lighting\n- status: open\n- owner: —\n\
             - priority: P1 · area: ui\n- type: dev · ops\n"
        )
    );
    assert!(!check_text(&home, &work).contains("not key: value"));
}

#[test]
fn a_priority_with_the_separator_is_refused_and_the_file_stays() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Lighting\n- status: open\n- owner: —\n- priority: P1\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));
    let before = read_text(&file);

    let out = run_todo(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "t-agent",
            "set",
            "T-001",
            "--priority",
            "P1 · x",
        ],
        &[],
    )
    .unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", stdout_text(&out));
    let json = parse_json(&out.stdout);
    assert_eq!(json["error"]["code"], "BAD_VALUE", "{json}");
    assert_eq!(
        read_text(&file),
        before,
        "a refused priority writes nothing"
    );
}

#[test]
fn add_puts_a_type_or_an_area_with_the_separator_on_lines_of_their_own() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Renderer\n- status: open\n- owner: —\n- priority: P1\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));

    ok(
        &home,
        &work,
        &[
            "--agent",
            "t-agent",
            "add",
            "--title",
            "Lighting",
            "--priority",
            "P1",
            "--difficulty",
            "2",
            "--type",
            "dev · ops",
            "--area",
            "web · api",
        ],
    );
    let text = read_text(&file);
    assert!(
        text.contains(
            "### T-002 · Lighting\n- status: open\n- owner: —\n\
             - priority: P1 · difficulty: 2\n- type: dev · ops\n- area: web · api\n"
        ),
        "{text}"
    );
    let show = parse_json(&ok(&home, &work, &["--json", "show", "T-002"]).stdout);
    assert_eq!(show["data"]["type"], "dev · ops");
    assert_eq!(show["data"]["area"], "web · api");
    assert!(!check_text(&home, &work).contains("not key: value"));
}

#[test]
fn ask_puts_an_area_with_the_separator_on_a_line_of_its_own() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Renderer\n- status: open\n- owner: —\n- priority: P1\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));

    ok(
        &home,
        &work,
        &[
            "--agent",
            "t-agent",
            "ask",
            "Should it glow?",
            "--area",
            "web · api",
            "--about",
            "T-001",
        ],
    );
    let text = read_text(&file);
    assert!(
        text.contains("- type: question\n- area: web · api\n- about: T-001\n"),
        "{text}"
    );
    assert!(!check_text(&home, &work).contains("not key: value"));
}
