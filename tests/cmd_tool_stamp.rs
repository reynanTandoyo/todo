#![allow(clippy::expect_used)] // whole file is test code; helpers outside #[test] may expect()

//! The tool version stamp `<!-- todo-tool: X -->` (R-VER-1, `docs/format.md` §4) through the CLI:
//! writes add or raise it, reads never write it, older and invalid stamps are reported, `where`
//! shows it, and the archive file, restore, apply and the merge driver follow the same rules.

mod common;

use std::fs;

use serde_json::Value;
use similar::{ChangeTag, TextDiff};
use tempfile::TempDir;

use common::{
    copy_fixture, parse_json, read_todo, run_in, run_with_stdin, stderr_text, stdout_text,
};

/// The version of the running todo, as Cargo.toml gives it.
const RUNNING: &str = env!("CARGO_PKG_VERSION");

fn stamp_line(version: &str) -> String {
    format!("<!-- todo-tool: {version} -->")
}

/// Lines added and removed going from `before` to `after` (line endings ignored).
fn added_and_removed(before: &str, after: &str) -> (Vec<String>, usize) {
    let mut added = Vec::new();
    let mut removed = 0;
    for change in TextDiff::from_lines(before, after).iter_all_changes() {
        match change.tag() {
            ChangeTag::Insert => {
                added.push(change.value().trim_end_matches(['\r', '\n']).to_string());
            }
            ChangeTag::Delete => removed += 1,
            ChangeTag::Equal => {}
        }
    }
    (added, removed)
}

/// Runs a write command that must succeed, then returns the new TODO.md text.
fn write_ok(dir: &std::path::Path, args: &[&str]) -> String {
    let out = run_in(dir, args);
    assert!(
        out.status.success(),
        "{args:?} failed: {}",
        stderr_text(&out)
    );
    read_todo(dir)
}

#[test]
fn init_writes_the_stamp_directly_after_the_format_line() {
    let dir = TempDir::new().expect("tempdir");
    let out = run_in(dir.path(), &["init", "--title", "Sample Project"]);
    assert!(out.status.success(), "init failed: {}", stderr_text(&out));

    let created = read_todo(dir.path());
    let lines: Vec<&str> = created.lines().collect();
    assert_eq!(lines[0], "<!-- todo-format: 3 -->");
    assert_eq!(lines[1], stamp_line(RUNNING));
    assert_eq!(lines[2], "# Sample Project");
}

#[test]
fn a_write_adds_the_missing_stamp_and_changes_no_other_line() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "tool_stamp_missing.md");
    let before = read_todo(dir.path());

    let after = write_ok(
        dir.path(),
        &["note", "T-001", "checked the lanterns", "--agent", "bob"],
    );

    // The write also raises the format line from 2 to 3 (R-MIG-1); no other line is removed.
    let (added, removed) = added_and_removed(&before, &after);
    assert_eq!(removed, 1, "only the format line is replaced");
    assert_eq!(added.len(), 3, "added lines: {added:?}");
    assert_eq!(added[0], "<!-- todo-format: 3 -->");
    assert_eq!(added[1], stamp_line(RUNNING));
    assert!(added[2].starts_with("- note ("), "added: {:?}", added[2]);
    assert!(after.contains(&format!(
        "<!-- todo-format: 3 -->\n{}\n",
        stamp_line(RUNNING)
    )));
}

#[test]
fn a_write_raises_an_older_stamp_to_the_running_version() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "tool_stamp_older.md");
    let before = read_todo(dir.path());

    let after = write_ok(
        dir.path(),
        &["note", "T-001", "checked the lanterns", "--agent", "bob"],
    );

    assert!(after.contains(&stamp_line(RUNNING)), "{after}");
    assert!(!after.contains("<!-- todo-tool: 0.4.0 -->"));
    // The old stamp and the format line (raised from 2 to 3, R-MIG-1) are replaced, nothing else.
    let (added, removed) = added_and_removed(&before, &after);
    assert_eq!(
        removed, 2,
        "only the old stamp and the format line are replaced"
    );
    assert!(
        added.contains(&"<!-- todo-format: 3 -->".to_string()),
        "{added:?}"
    );
    assert!(added.contains(&stamp_line(RUNNING)));
}

#[test]
fn a_write_never_lowers_a_newer_stamp_and_warns_on_standard_error() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "tool_stamp_newer.md");

    // 0.10.0 is above the running version when compared number by number; compared as text it
    // would look older, so this also checks the numeric comparison.
    let out = run_in(
        dir.path(),
        &["note", "T-001", "checked the lanterns", "--agent", "bob"],
    );
    assert!(out.status.success(), "note failed: {}", stderr_text(&out));
    let stderr = stderr_text(&out);
    assert!(stderr.contains("is behind"), "stderr: {stderr}");
    assert!(stderr.contains("todo help update"), "stderr: {stderr}");
    // The fixture's stamp is on line 3 (line 1 is the fixture marker).
    assert!(stderr.contains("TODO.md:3:"), "stderr: {stderr}");

    let after = read_todo(dir.path());
    assert!(after.contains("<!-- todo-tool: 0.10.0 -->"), "{after}");
    assert!(!after.contains(&stamp_line(RUNNING)));
}

#[test]
fn a_newer_stamp_still_lets_read_commands_work() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "tool_stamp_newer.md");

    // In --json mode the warning goes to the envelope's `warnings` array, not to standard error.
    let out = run_in(dir.path(), &["--json", "list"]);
    assert!(out.status.success(), "list failed: {}", stderr_text(&out));
    let json: Value = parse_json(&out.stdout);
    assert_eq!(json["ok"], true);
    let warnings = json["warnings"].as_array().expect("warnings array");
    assert!(
        warnings
            .iter()
            .any(|w| w.as_str().is_some_and(|w| w.contains("is behind"))),
        "warnings: {warnings:?}"
    );
}

#[test]
fn read_commands_never_write_the_stamp() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "tool_stamp_missing.md");
    let before = read_todo(dir.path());

    for args in [
        vec!["list"],
        vec!["summary"],
        vec!["where"],
        vec!["show", "T-001"],
        vec!["next"],
    ] {
        let out = run_in(dir.path(), &args);
        assert!(
            out.status.success(),
            "{args:?} failed: {}",
            stderr_text(&out)
        );
        assert_eq!(read_todo(dir.path()), before, "{args:?} wrote TODO.md");
    }
}

#[test]
fn a_command_that_changes_nothing_writes_no_stamp() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "tool_stamp_missing.md");
    let before = read_todo(dir.path());

    // T-001 is already a chore, so this changes no line (R-EDIT-6).
    let out = run_in(
        dir.path(),
        &["set", "T-001", "--type", "chore", "--agent", "bob"],
    );
    assert!(out.status.success(), "set failed: {}", stderr_text(&out));
    assert_eq!(read_todo(dir.path()), before);
}

#[test]
fn a_file_without_a_format_line_gets_the_format_line_first_and_the_stamp_after_it() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "tool_stamp_no_format.md");

    let after = write_ok(
        dir.path(),
        &["note", "T-001", "checked the lanterns", "--agent", "bob"],
    );

    // The first write migrates a file without a format line (format.md §18): the format line goes
    // first, and the stamp directly after it.
    let lines: Vec<&str> = after.lines().collect();
    assert_eq!(lines[0], "<!-- todo-format: 3 -->");
    assert_eq!(lines[1], stamp_line(RUNNING));
}

#[test]
fn the_stamp_uses_the_file_line_ending_and_keeps_the_byte_order_mark() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "tool_stamp_crlf_bom.md");

    let out = run_in(
        dir.path(),
        &["note", "T-001", "checked the lanterns", "--agent", "bob"],
    );
    assert!(out.status.success(), "note failed: {}", stderr_text(&out));
    let bytes = fs::read(dir.path().join("TODO.md")).expect("read TODO.md");
    assert!(bytes.starts_with(b"\xEF\xBB\xBF"), "BOM must stay");
    let text = String::from_utf8(bytes).expect("utf-8");
    let stamp_crlf = format!("{}\r\n", stamp_line(RUNNING));
    assert!(
        text.contains(&stamp_crlf),
        "stamp must end with CRLF: {text:?}"
    );
    assert_eq!(
        text.matches('\n').count(),
        text.matches("\r\n").count(),
        "no bare LF may be added"
    );
}

#[test]
fn an_invalid_stamp_is_kept_and_reported() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "tool_stamp_invalid.md");

    let list = run_in(dir.path(), &["list"]);
    assert!(list.status.success());
    assert!(
        stderr_text(&list).contains("is not MAJOR.MINOR.PATCH"),
        "stderr: {}",
        stderr_text(&list)
    );

    let after = write_ok(
        dir.path(),
        &["note", "T-001", "checked the lanterns", "--agent", "bob"],
    );
    assert!(after.contains("<!-- todo-tool: soon -->"));
    assert!(!after.contains(&stamp_line(RUNNING)));
}

#[test]
fn where_shows_the_stamp_and_the_running_version_in_text_and_json() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "tool_stamp_older.md");

    let text = run_in(dir.path(), &["where"]);
    assert!(
        text.status.success(),
        "where failed: {}",
        stderr_text(&text)
    );
    let text = stdout_text(&text);
    assert!(text.contains("Tool stamp: 0.4.0"), "{text}");
    assert!(
        text.contains(&format!("Running todo version: {RUNNING}")),
        "{text}"
    );

    let json = run_in(dir.path(), &["--json", "where"]);
    assert!(json.status.success());
    let json = parse_json(&json.stdout);
    assert_eq!(json["data"]["tool_stamp"], "0.4.0");
    assert_eq!(json["data"]["running_version"], RUNNING);

    copy_fixture(dir.path(), "tool_stamp_missing.md");
    let json = run_in(dir.path(), &["--json", "where"]);
    let json = parse_json(&json.stdout);
    assert!(json["data"]["tool_stamp"].is_null());
    assert_eq!(json["data"]["running_version"], RUNNING);
}

#[test]
fn archive_stamps_both_files() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "tool_stamp_done.md");

    let out = run_in(dir.path(), &["archive", "--yes", "--agent", "coordinator"]);
    assert!(
        out.status.success(),
        "archive failed: {}",
        stderr_text(&out)
    );

    let todo = read_todo(dir.path());
    assert!(todo.contains(&stamp_line(RUNNING)), "{todo}");
    assert!(!todo.contains("<!-- todo-tool: 0.4.0 -->"));

    let archive = fs::read_to_string(dir.path().join("TODO-archive.md")).expect("archive");
    let head: Vec<&str> = archive.lines().take(2).collect();
    assert_eq!(
        head,
        ["<!-- todo-format: 3 -->", stamp_line(RUNNING).as_str()]
    );
}

#[test]
fn restoring_an_old_snapshot_gets_the_running_stamp() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "tool_stamp_older.md");
    let original = read_todo(dir.path());

    // The first write stores the 0.4.0 file as snapshot 1 and raises the stamp.
    write_ok(
        dir.path(),
        &["note", "T-001", "checked the lanterns", "--agent", "bob"],
    );
    let history = run_in(dir.path(), &["--json", "history"]);
    let history = parse_json(&history.stdout);
    let number = history["data"]["snapshots"][0]["number"]
        .as_u64()
        .expect("snapshot number");

    let number = number.to_string();
    let restored = write_ok(dir.path(), &["restore", &number, "--yes", "--agent", "bob"]);
    assert!(restored.contains(&stamp_line(RUNNING)), "{restored}");
    assert_ne!(restored, original, "the snapshot has the older stamp");
    assert!(!restored.contains("<!-- todo-tool: 0.4.0 -->"));
}

#[test]
fn apply_stamps_the_file_it_writes() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "tool_stamp_missing.md");

    let out = run_with_stdin(
        dir.path(),
        &["--agent", "alice", "apply", "-"],
        "{\"op\":\"claim\",\"task\":\"T-001\"}\n",
    );
    assert!(out.status.success(), "apply failed: {}", stderr_text(&out));

    let text = read_todo(dir.path());
    assert!(text.contains(&stamp_line(RUNNING)), "{text}");
    assert!(text.contains("- status: claimed"));
}

#[test]
fn the_merge_driver_takes_the_higher_stamp_instead_of_a_conflict() {
    let dir = TempDir::new().expect("tempdir");
    let base = fs::read_to_string("tests/fixtures/tool_stamp_older.md").expect("fixture");
    // Compared as text "0.9.0" would beat "0.10.0"; the merge must compare numbers.
    let ours = base.replace("0.4.0", "0.9.0");
    let theirs = base.replace("0.4.0", "0.10.0");
    fs::write(dir.path().join("base.md"), &base).expect("base");
    fs::write(dir.path().join("ours.md"), &ours).expect("ours");
    fs::write(dir.path().join("theirs.md"), &theirs).expect("theirs");

    let out = run_in(
        dir.path(),
        &[
            "merge-driver",
            "run",
            "base.md",
            "ours.md",
            "theirs.md",
            "TODO.md",
        ],
    );
    assert!(
        out.status.success(),
        "merge reported a conflict: {}{}",
        stdout_text(&out),
        stderr_text(&out)
    );
    let merged = fs::read_to_string(dir.path().join("ours.md")).expect("merged");
    assert_eq!(merged, theirs);
    assert!(!merged.contains("<<<<<<<"));
}
