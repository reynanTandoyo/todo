mod common;

use std::fs;
use std::path::Path;

use serde_json::Value;
use tempfile::TempDir;

use common::{
    count_tmp_files, parse_json, read_text, run_in, set_read_only, stderr_text, stdout_text,
    todo_bin,
};

const ARCHIVE_FIXTURE: &str = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 2 -->
# Project Tasks

## Backend

### T-001 · Old done backend task
- status: done
- owner: —
- priority: P1
- note (2026-10-01, alice): done — shipped

### T-002 · Recent done backend task
- status: done
- owner: —
- priority: P1
- note (2026-11-15, alice): done — just finished

### T-003 · Open task depending on T-001
- status: open
- owner: —
- priority: P1
- depends: T-001

## Frontend

### T-004 · Dropped UI task
- status: dropped
- owner: —
- priority: P2
- note (2026-10-15, bob): dropped — superseded

### T-005 · Highest ID open task
- status: open
- owner: —
- priority: P2
";

#[test]
fn archive_moves_done_and_dropped_to_matching_sections_and_preserves_lookups_and_ids() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("TODO.md"), ARCHIVE_FIXTURE).expect("write TODO.md");

    // Preview without --yes makes no changes.
    let preview = run_in(dir.path(), &["archive", "--agent", "coordinator"]);
    assert!(preview.status.success());
    assert!(stdout_text(&preview).contains("pass --yes"));
    assert!(!dir.path().join("TODO-archive.md").exists());

    // Archive with --older-than 10 --yes: only T-001 and T-004 (Oct 2026) should move; T-002 (2026-11-15) stays.
    let arch_old = todo_bin()
        .current_dir(dir.path())
        .env("TODO_TEST_NOW", "2026-11-15T12:00:00+00:00")
        .args([
            "archive",
            "--older-than",
            "10",
            "--yes",
            "--agent",
            "coordinator",
            "--json",
        ])
        .output()
        .expect("run archive --older-than 10");
    assert!(
        arch_old.status.success(),
        "archive failed: {}",
        stderr_text(&arch_old)
    );
    let old_json: Value = parse_json(&arch_old.stdout);
    assert_eq!(old_json["data"]["count"], 2);

    let archive_text = read_text(&dir.path().join("TODO-archive.md"));
    assert!(archive_text.starts_with("<!-- todo-format: 2 -->"));
    assert!(archive_text.contains("## Backend"));
    assert!(archive_text.contains("### T-001 · Old done backend task"));
    assert!(archive_text.contains("## Frontend"));
    assert!(archive_text.contains("### T-004 · Dropped UI task"));
    assert!(!archive_text.contains("T-002"));

    // show still finds T-001 in archive.
    let show = run_in(dir.path(), &["show", "T-001", "--json"]);
    assert!(show.status.success());

    // list --archive includes archived tasks.
    let list_arch = run_in(dir.path(), &["list", "--archive", "--json"]);
    assert!(list_arch.status.success());
    let list_json: Value = parse_json(&list_arch.stdout);
    let tasks = list_json["data"]["tasks"].as_array().expect("tasks");
    assert!(tasks.iter().any(|t| t["id"] == "T-001"));
    assert!(tasks.iter().any(|t| t["id"] == "T-004"));

    // Dependency check: T-003 depends on T-001 (which is done in archive), so T-003 is ready and `check` is clean!
    let check = run_in(dir.path(), &["check"]);
    assert!(
        check.status.success(),
        "check failed: {}",
        stdout_text(&check)
    );

    // Archive remaining done task T-002 (into existing Backend section in TODO-archive.md).
    let arch_rest = run_in(dir.path(), &["archive", "--yes", "--agent", "coordinator"]);
    assert!(arch_rest.status.success());
    let archive_text2 = read_text(&dir.path().join("TODO-archive.md"));
    assert!(archive_text2.contains("### T-002 · Recent done backend task"));

    // Adding a new task must pick T-006, never reusing archived IDs.
    let add = run_in(
        dir.path(),
        &[
            "add",
            "--title",
            "After archive",
            "--section",
            "Backend",
            "--priority",
            "P1",
            "--agent",
            "alice",
            "--json",
        ],
    );
    assert!(add.status.success());
    let add_json: Value = parse_json(&add.stdout);
    assert_eq!(add_json["data"]["id"], "T-006");
}

/// Makes the snapshot of TODO.md fail to record: the history folder is a file, so the write takes
/// the lock and then stops before TODO.md or the archive is replaced.
fn block_history(dir: &Path) {
    let state = dir.join(".todo");
    fs::create_dir_all(&state).expect("create state folder");
    fs::write(
        state.join("history"),
        "a file where the history folder belongs",
    )
    .expect("block history");
}

#[test]
fn archive_leaves_the_archive_alone_when_the_snapshot_of_todo_md_cannot_be_recorded() {
    let dir = TempDir::new().expect("tempdir");
    let archive_before = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
<!-- todo-format: 2 -->\n\
# Archive\n\
\n\
## Backend\n\
\n\
### T-900 · Archived earlier\n\
- status: done\n\
- owner: —\n\
- priority: P2\n";
    fs::write(dir.path().join("TODO.md"), ARCHIVE_FIXTURE).expect("write TODO.md");
    fs::write(dir.path().join("TODO-archive.md"), archive_before).expect("write archive");
    block_history(dir.path());

    let out = run_in(dir.path(), &["archive", "--yes", "--agent", "coordinator"]);
    assert!(
        !out.status.success(),
        "archive should fail: {}",
        stdout_text(&out)
    );
    assert_eq!(
        read_text(&dir.path().join("TODO-archive.md")),
        archive_before
    );
    assert_eq!(read_text(&dir.path().join("TODO.md")), ARCHIVE_FIXTURE);
}

#[test]
fn archive_creates_no_archive_file_when_the_snapshot_of_todo_md_cannot_be_recorded() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("TODO.md"), ARCHIVE_FIXTURE).expect("write TODO.md");
    block_history(dir.path());

    let out = run_in(dir.path(), &["archive", "--yes", "--agent", "coordinator"]);
    assert!(
        !out.status.success(),
        "archive should fail: {}",
        stdout_text(&out)
    );
    assert!(!dir.path().join("TODO-archive.md").exists());
    assert_eq!(read_text(&dir.path().join("TODO.md")), ARCHIVE_FIXTURE);
}

/// An archive file that already holds one archived task, which the tests below replace.
const OLD_ARCHIVE: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
<!-- todo-format: 2 -->\n\
# Archive\n\
\n\
## Backend\n\
\n\
### T-900 · Archived earlier\n\
- status: done\n\
- owner: —\n\
- priority: P2\n";

/// TODO.md as another program leaves it while `archive` waits to replace it.
#[cfg(feature = "test-hooks")]
const EXTERNAL_EDIT: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
<!-- todo-format: 2 -->\n\
# Project Tasks\n\
\n\
## Backend\n\
\n\
### T-001 · Edited by hand while archive ran\n\
- status: done\n\
- owner: —\n\
- priority: P1\n";

#[test]
fn archive_refuses_a_read_only_todo_md_before_it_writes_a_new_archive() {
    let dir = TempDir::new().expect("tempdir");
    let todo = dir.path().join("TODO.md");
    fs::write(&todo, ARCHIVE_FIXTURE).expect("write TODO.md");
    set_read_only(&todo, true);

    let out = run_in(dir.path(), &["archive", "--yes", "--agent", "coordinator"]);
    set_read_only(&todo, false);

    assert_eq!(out.status.code(), Some(4), "{}", stderr_text(&out));
    assert!(
        stderr_text(&out).contains("read-only"),
        "{}",
        stderr_text(&out)
    );
    assert!(
        !dir.path().join("TODO-archive.md").exists(),
        "the archive must not be written when TODO.md cannot be replaced"
    );
    assert_eq!(read_text(&todo), ARCHIVE_FIXTURE);
    assert_eq!(count_tmp_files(dir.path()), 0);
}

#[test]
fn archive_refuses_a_read_only_todo_md_before_it_writes_an_existing_archive() {
    let dir = TempDir::new().expect("tempdir");
    let todo = dir.path().join("TODO.md");
    let archive = dir.path().join("TODO-archive.md");
    fs::write(&todo, ARCHIVE_FIXTURE).expect("write TODO.md");
    fs::write(&archive, OLD_ARCHIVE).expect("write archive");
    set_read_only(&todo, true);

    let out = run_in(dir.path(), &["archive", "--yes", "--agent", "coordinator"]);
    set_read_only(&todo, false);

    assert_eq!(out.status.code(), Some(4), "{}", stderr_text(&out));
    assert_eq!(read_text(&archive), OLD_ARCHIVE);
    assert_eq!(read_text(&todo), ARCHIVE_FIXTURE);
    assert_eq!(count_tmp_files(dir.path()), 0);
}

/// Writes `ARCHIVE_FIXTURE` as TODO.md, and `archive_before` as the archive when it is given, then
/// starts `archive --yes` with the test pause before each rename. Returns the work folder and the
/// running command, once `archive` has reached its first rename.
#[cfg(feature = "test-hooks")]
fn start_archive_with_pause(
    tmp: &TempDir,
    archive_before: Option<&str>,
) -> (std::path::PathBuf, std::process::Child) {
    let home = tmp.path().join("home");
    let work = tmp.path().join("work");
    fs::create_dir_all(&home).expect("home");
    fs::create_dir_all(&work).expect("work");
    fs::write(work.join("TODO.md"), ARCHIVE_FIXTURE).expect("write TODO.md");
    if let Some(text) = archive_before {
        fs::write(work.join("TODO-archive.md"), text).expect("write archive");
    }
    let ready = tmp.path().join("rename.ready");
    let ready_str = ready.to_string_lossy().into_owned();
    let child = common::spawn_todo(
        &home,
        &work,
        &["archive", "--yes", "--agent", "coordinator"],
        &[
            ("TODO_TEST_BEFORE_RENAME_READY_FILE", ready_str.as_str()),
            ("TODO_TEST_BEFORE_RENAME_HOLD_MS", "1000"),
        ],
    )
    .expect("spawn archive");
    common::wait_for_file(&ready, std::time::Duration::from_secs(10))
        .expect("archive reached a rename");
    (work, child)
}

/// The write of `archive` that creates the archive, then fails to replace TODO.md because another
/// program changed it: the archive it created is removed again, so no task is in both files.
#[cfg(feature = "test-hooks")]
#[test]
fn archive_removes_the_new_archive_when_todo_md_changes_before_it_is_replaced() {
    let tmp = TempDir::new().expect("tempdir");
    let (work, child) = start_archive_with_pause(&tmp, None);
    // The new archive is created without a pause, so the first pause is the replace of TODO.md.
    fs::write(work.join("TODO.md"), EXTERNAL_EDIT).expect("external edit");
    let out = child.wait_with_output().expect("wait for archive");

    assert_eq!(out.status.code(), Some(3), "{}", stderr_text(&out));
    assert!(
        stderr_text(&out).contains("modified by another program"),
        "{}",
        stderr_text(&out)
    );
    assert!(
        !work.join("TODO-archive.md").exists(),
        "the archive created by the failed write must be removed"
    );
    assert_eq!(read_text(&work.join("TODO.md")), EXTERNAL_EDIT);
    assert_eq!(count_tmp_files(&work), 0);
}

/// The archive is replaced, TODO.md then changes, and the archive has become read-only by the time
/// it must be put back: the error says that the archive still holds the moved tasks.
#[cfg(feature = "test-hooks")]
#[test]
fn archive_says_so_when_the_archive_cannot_be_put_back() {
    let tmp = TempDir::new().expect("tempdir");
    let (work, child) = start_archive_with_pause(&tmp, Some(OLD_ARCHIVE));
    let ready = tmp.path().join("rename.ready");
    let archive = work.join("TODO-archive.md");
    // The first pause is the replace of the existing archive: TODO.md changes during it.
    fs::write(work.join("TODO.md"), EXTERNAL_EDIT).expect("external edit");
    fs::remove_file(&ready).expect("clear the first pause marker");
    // The second pause is the replace of TODO.md, which then fails: the archive is read-only by then.
    common::wait_for_file(&ready, std::time::Duration::from_secs(10))
        .expect("archive reached the replace of TODO.md");
    set_read_only(&archive, true);
    let out = child.wait_with_output().expect("wait for archive");
    set_read_only(&archive, false);

    let err = stderr_text(&out);
    assert_eq!(out.status.code(), Some(3), "{err}");
    assert!(err.contains("modified by another program"), "{err}");
    assert!(
        err.contains("could not be put back, so it still holds the moved tasks"),
        "{err}"
    );
    assert!(err.contains("read-only"), "{err}");
}

/// The write of `archive` that replaces an existing archive, then fails to replace TODO.md: the
/// archive is put back with the text it had before.
#[cfg(feature = "test-hooks")]
#[test]
fn archive_puts_the_old_archive_back_when_todo_md_changes_before_it_is_replaced() {
    let tmp = TempDir::new().expect("tempdir");
    let (work, child) = start_archive_with_pause(&tmp, Some(OLD_ARCHIVE));
    // The first pause is the replace of the existing archive, which is not yet done. TODO.md is
    // changed during that pause, so the replace of TODO.md after it finds the file changed.
    fs::write(work.join("TODO.md"), EXTERNAL_EDIT).expect("external edit");
    let out = child.wait_with_output().expect("wait for archive");

    assert_eq!(out.status.code(), Some(3), "{}", stderr_text(&out));
    assert_eq!(read_text(&work.join("TODO-archive.md")), OLD_ARCHIVE);
    assert_eq!(read_text(&work.join("TODO.md")), EXTERNAL_EDIT);
    assert_eq!(count_tmp_files(&work), 0);
}
