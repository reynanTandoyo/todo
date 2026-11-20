#![allow(clippy::expect_used)] // whole file is test code; helpers outside #[test] may expect()

//! A `TODO.md` (or archive) in a format version newer than this todo (R-VER-2, `docs/format.md`
//! §4): every command that interprets or changes task content refuses with exit 4 and
//! `TOOL_TOO_OLD`, nothing is written, `where` shows the format and the stamp without counting
//! tasks, and the commands that never read task content keep working.

mod common;

use std::fs;
use std::path::Path;

use serde_json::Value;
use tempfile::TempDir;

use common::{copy_fixture, fixture, parse_json, read_todo, run_in, stderr_text, stdout_text};

/// The version of the running todo, as Cargo.toml gives it.
const RUNNING: &str = env!("CARGO_PKG_VERSION");

/// The format version in `format_newer_99.md`, which this todo does not know.
const NEWER_FORMAT: &str = "99";

/// The tool line stamped in `format_newer_99.md`.
const NEWER_STAMP: &str = "0.9.0";

/// An archive in the same newer format, with one invented finished task.
const NEWER_ARCHIVE: &str = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 99 -->
<!-- todo-tool: 0.9.0 -->
# Archive

## Tasks

### T-090 · Carry the old lantern
- status: done
- owner: —
- priority: P3 · type: chore · area: shed
";

/// A project whose `TODO.md` is in format version 99.
fn newer_project() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "format_newer_99.md");
    dir
}

/// Runs `todo --json <args>` and returns the exit code and the parsed JSON envelope.
fn run_json(dir: &Path, args: &[&str]) -> (i32, Value) {
    let mut all = vec!["--json"];
    all.extend_from_slice(args);
    let out = run_in(dir, &all);
    (out.status.code().unwrap_or(-1), parse_json(&out.stdout))
}

/// Asserts that `todo --json <args>` is refused with exit 4 and `TOOL_TOO_OLD`, and that the
/// message names the stamped and the running version and says to update first.
fn assert_refused(dir: &Path, args: &[&str]) -> Value {
    let (code, json) = run_json(dir, args);
    assert_eq!(code, 4, "{args:?} exit code; envelope: {json}");
    assert_eq!(json["ok"], false, "{args:?}");
    assert_eq!(json["error"]["code"], "TOOL_TOO_OLD", "{args:?}: {json}");
    let message = json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains(NEWER_FORMAT), "{args:?}: {message}");
    assert!(message.contains(NEWER_STAMP), "{args:?}: {message}");
    assert!(message.contains(RUNNING), "{args:?}: {message}");
    assert!(
        message.contains("Update the todo executable first"),
        "{args:?}: {message}"
    );
    json
}

#[test]
fn every_command_that_interprets_or_changes_tasks_refuses_a_newer_format() {
    let dir = newer_project();
    let before = read_todo(dir.path());
    let commands: &[&[&str]] = &[
        &["list"],
        &["show", "T-001"],
        &["summary"],
        &["next"],
        &["search", "lantern"],
        &["log", "T-001"],
        &["check"],
        &["align"],
        &["scan"],
        &["edit", "--agent", "bob"],
        &["claim", "T-001", "--agent", "bob"],
        &["note", "T-001", "checked the lanterns", "--agent", "bob"],
        &["done", "T-001", "--agent", "bob"],
        &["set", "T-001", "--priority", "P1", "--agent", "bob"],
        &[
            "add",
            "--title",
            "New lantern task",
            "--priority",
            "P2",
            "--difficulty",
            "3",
            "--type",
            "chore",
            "--area",
            "shed",
            "--agent",
            "bob",
        ],
        &["archive", "--agent", "bob"],
        &["archive", "--agent", "bob", "--yes"],
        &[
            "idea",
            "Cache parsed files",
            "--about",
            "T-001",
            "--agent",
            "bob",
        ],
        &["promote", "T-002", "--priority", "P2", "--agent", "bob"],
        &["move", "T-001", "--section", "Later", "--agent", "bob"],
    ];
    for args in commands {
        assert_refused(dir.path(), args);
    }
    assert_eq!(
        read_todo(dir.path()),
        before,
        "a refused command writes nothing"
    );
    assert!(!dir.path().join("TODO-archive.md").exists());
}

#[test]
fn milestone_commands_refuse_a_newer_format_too() {
    let dir = newer_project();
    let before = read_todo(dir.path());
    fs::write(
        dir.path().join("batch.jsonl"),
        concat!(
            "{\"ref\":\"m\",\"title\":\"Shelves\",\"type\":\"milestone\",\"priority\":\"P1\",",
            "\"goal\":\"shelves are sorted\"}\n",
            "{\"title\":\"Shelf pass\",\"priority\":\"P2\",\"difficulty\":\"3\",\"milestone\":\"@m\"}\n",
        ),
    )
    .expect("write batch");
    let commands: &[&[&str]] = &[
        &[
            "add",
            "--title",
            "Shade the shelves",
            "--priority",
            "P2",
            "--difficulty",
            "3",
            "--type",
            "chore",
            "--area",
            "shed",
            "--milestone",
            "T-001",
            "--agent",
            "bob",
        ],
        &[
            "add",
            "--title",
            "Shelf milestone",
            "--priority",
            "P1",
            "--difficulty",
            "3",
            "--type",
            "milestone",
            "--goal",
            "shelves are sorted",
            "--area",
            "shed",
            "--agent",
            "bob",
        ],
        &["set", "T-001", "--milestone", "T-002", "--agent", "bob"],
        &[
            "drop",
            "T-001",
            "--tree",
            "--note",
            "scope cut",
            "--agent",
            "bob",
        ],
        &["add", "--from", "batch.jsonl", "--agent", "bob"],
    ];
    for args in commands {
        assert_refused(dir.path(), args);
    }
    assert_eq!(
        read_todo(dir.path()),
        before,
        "a refused command writes nothing"
    );
}

#[test]
fn the_refusal_text_names_the_file_line_and_both_versions() {
    let dir = newer_project();
    let out = run_in(dir.path(), &["list"]);
    assert_eq!(out.status.code(), Some(4));
    let stderr = stderr_text(&out);
    // The format line is line 2 of the fixture.
    let expected = format!(
        "error: TODO.md:2: format version {NEWER_FORMAT} is newer than this todo {RUNNING} supports; \
         the file was last written by todo {NEWER_STAMP}. Update the todo executable first"
    );
    assert!(stderr.contains(&expected), "stderr: {stderr}");
    assert!(stdout_text(&out).is_empty());
}

#[test]
fn where_shows_the_format_and_the_stamp_but_counts_no_tasks() {
    let dir = newer_project();

    let out = run_in(dir.path(), &["where"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    let text = stdout_text(&out);
    assert!(
        text.contains(&format!("Format version: {NEWER_FORMAT}")),
        "{text}"
    );
    assert!(
        text.contains(&format!("Tool stamp: {NEWER_STAMP}")),
        "{text}"
    );
    assert!(
        text.contains(&format!("Running todo version: {RUNNING}")),
        "{text}"
    );
    assert!(text.contains("Tasks: not counted"), "{text}");

    let (code, json) = run_json(dir.path(), &["where"]);
    assert_eq!(code, 0, "{json}");
    assert_eq!(json["data"]["format_version"], 99);
    assert_eq!(json["data"]["tool_stamp"], NEWER_STAMP);
    assert_eq!(json["data"]["running_version"], RUNNING);
    assert!(json["data"].get("counts").is_none(), "{json}");
}

#[test]
fn help_schema_completions_and_version_still_work() {
    let dir = newer_project();
    for args in [
        &["help"][..],
        &["help", "list"],
        &["schema"],
        &["completions", "powershell"],
        &["--version"],
    ] {
        let out = run_in(dir.path(), args);
        assert_eq!(
            out.status.code(),
            Some(0),
            "{args:?}: {}",
            stderr_text(&out)
        );
    }
    let version = run_in(dir.path(), &["--version"]);
    assert!(stdout_text(&version).contains(RUNNING));
}

#[test]
fn commands_that_never_read_task_content_keep_working() {
    let dir = newer_project();
    for args in [&["status"][..], &["history"]] {
        let out = run_in(dir.path(), args);
        assert_eq!(
            out.status.code(),
            Some(0),
            "{args:?}: {}",
            stderr_text(&out)
        );
    }
}

#[test]
fn a_newer_format_without_a_tool_line_says_the_file_counts_as_older() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("TODO.md"),
        "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 99 -->
# Lantern Project — tasks

## Now

### T-001 · Sort the lantern parts
- status: open
",
    )
    .expect("write TODO.md");
    let (code, json) = run_json(dir.path(), &["list"]);
    assert_eq!(code, 4, "{json}");
    assert_eq!(json["error"]["code"], "TOOL_TOO_OLD");
    let message = json["error"]["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("has no tool line, so it counts as written by todo 0.5.0 or older"),
        "{message}"
    );
    assert!(message.contains("todo help update"), "{message}");
}

#[test]
fn a_known_format_with_a_newer_stamp_still_lists_and_only_warns() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "tool_stamp_newer.md");
    let out = run_in(dir.path(), &["list"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    assert!(
        stderr_text(&out).contains("is behind"),
        "{}",
        stderr_text(&out)
    );
}

#[test]
fn a_newer_archive_refuses_the_commands_that_read_it() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "clean_basic.md");
    fs::write(dir.path().join("TODO-archive.md"), NEWER_ARCHIVE).expect("write archive");
    let before = read_todo(dir.path());

    for args in [
        &["list", "--archive"][..],
        &["show", "T-001"],
        &["next"],
        &["check"],
    ] {
        let json = assert_refused(dir.path(), args);
        assert_eq!(json["error"]["file"], "TODO-archive.md", "{args:?}");
    }

    // A command that does not need the archive still lists the file.
    let out = run_in(dir.path(), &["list"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    assert_eq!(read_todo(dir.path()), before);
}

#[test]
fn import_refuses_to_replace_a_newer_todo_file() {
    let dir = newer_project();
    let before = read_todo(dir.path());
    fs::write(
        dir.path().join("notes.md"),
        "## Now\n\n- [ ] Sort the lantern parts\n",
    )
    .expect("write notes");

    let (code, json) = run_json(
        dir.path(),
        &[
            "import", "--from", "checkbox", "notes.md", "--out", "TODO.md", "--yes",
        ],
    );
    assert_eq!(code, 4, "{json}");
    assert_eq!(json["error"]["code"], "TOOL_TOO_OLD", "{json}");
    assert_eq!(read_todo(dir.path()), before);
    assert!(!dir.path().join("TODO.md.pre-import").exists());
}

#[test]
fn restore_refuses_to_replace_a_newer_file_or_bring_back_a_newer_snapshot() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "clean_basic.md");
    // Snapshot 1 is the clean file, taken before this write.
    let note = run_in(
        dir.path(),
        &["note", "T-001", "checked the beds", "--agent", "bob"],
    );
    assert!(note.status.success(), "{}", stderr_text(&note));

    // The file is replaced by a newer format outside the tool. The next read records that
    // version as snapshot 2 and refuses it.
    fs::write(dir.path().join("TODO.md"), fixture("format_newer_99.md")).expect("replace");
    let newer = read_todo(dir.path());
    assert_refused(dir.path(), &["list"]);

    // Snapshot 1 cannot replace a newer file.
    assert_refused(dir.path(), &["restore", "1", "--agent", "bob", "--yes"]);
    assert_eq!(read_todo(dir.path()), newer);

    // Back to a readable file: snapshot 2 (the newer format) cannot be restored.
    let clean = fixture("clean_basic.md");
    fs::write(dir.path().join("TODO.md"), &clean).expect("restore clean");
    let before = read_todo(dir.path());
    assert_refused(dir.path(), &["restore", "2", "--agent", "bob", "--yes"]);
    assert_eq!(read_todo(dir.path()), before);
}
