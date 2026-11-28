//! Tests for atomic file replacement and IO safety (T-014; R-IO-1..5, TEST-7).

mod common;

use std::path::Path;

use tempfile::TempDir;
use todo::atomic::{atomic_replace, read_file_with_snapshot, validate_utf8};
use todo::error::{ErrorCode, ExitCode};

fn count_tmp_files(dir: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
        .count()
}

#[test]
fn atomic_replace_succeeds_and_leaves_no_tmp_files() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("TODO.md");
    let initial = b"<!-- TEST FIXTURE: invented data, not a real task list -->\n# V1\n";
    let updated = b"<!-- TEST FIXTURE: invented data, not a real task list -->\n# V2\n";
    std::fs::write(&file, initial).unwrap();

    let read = read_file_with_snapshot(&file).unwrap();
    assert_eq!(read.bytes, initial);

    atomic_replace(&file, &read.snapshot, updated).unwrap();
    assert_eq!(std::fs::read(&file).unwrap(), updated);
    assert_eq!(count_tmp_files(tmp.path()), 0);
}

#[test]
fn external_modification_detected_as_changed_during_write() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("TODO.md");
    let initial = b"<!-- TEST FIXTURE: invented data, not a real task list -->\n# V1\n";
    let external_same_len = b"<!-- TEST FIXTURE: invented data, not a real task list -->\n# X1\n";
    let our_update = b"<!-- TEST FIXTURE: invented data, not a real task list -->\n# V2\n";
    std::fs::write(&file, initial).unwrap();

    let read = read_file_with_snapshot(&file).unwrap();

    // Another program modifies the file with the exact same byte length.
    std::fs::write(&file, external_same_len).unwrap();

    let err = atomic_replace(&file, &read.snapshot, our_update).unwrap_err();
    assert_eq!(err.code, ErrorCode::ChangedDuringWrite);
    assert_eq!(err.exit_code, ExitCode::Busy);
    assert_eq!(std::fs::read(&file).unwrap(), external_same_len);
    assert_eq!(count_tmp_files(tmp.path()), 0);
}

#[test]
fn read_only_file_is_refused_with_exit_4_and_no_tmp() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("TODO.md");
    let initial = b"<!-- TEST FIXTURE: invented data, not a real task list -->\n# RO\n";
    std::fs::write(&file, initial).unwrap();

    let read = read_file_with_snapshot(&file).unwrap();

    let mut perms = std::fs::metadata(&file).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&file, perms).unwrap();

    let err = atomic_replace(&file, &read.snapshot, b"changed\n").unwrap_err();
    assert_eq!(err.code, ErrorCode::ReadOnly);
    assert_eq!(err.exit_code, ExitCode::FileError);
    assert_eq!(std::fs::read(&file).unwrap(), initial);
    assert_eq!(count_tmp_files(tmp.path()), 0);

    // Restore write permission so TempDir cleanup succeeds on Windows.
    let mut perms = std::fs::metadata(&file).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);
    std::fs::set_permissions(&file, perms).unwrap();
}

#[test]
fn invalid_utf8_reports_byte_position_and_line() {
    let bad = b"first line\nsecond \xff byte\n";
    let err = validate_utf8(bad, "TODO.md").unwrap_err();
    assert_eq!(err.code, ErrorCode::UnsafeFile);
    assert_eq!(err.exit_code, ExitCode::FileError);
    assert_eq!(err.details.line, Some(2));
    assert!(
        err.message.contains("18"),
        "expected byte offset 18 in message, got: {}",
        err.message
    );
}

#[cfg(windows)]
#[test]
fn windows_sharing_violation_retries_and_succeeds_once_released() {
    use std::os::windows::fs::OpenOptionsExt;

    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("TODO.md");
    let initial = b"<!-- TEST FIXTURE: invented data, not a real task list -->\n# Held\n";
    let updated = b"<!-- TEST FIXTURE: invented data, not a real task list -->\n# Updated\n";
    std::fs::write(&file, initial).unwrap();

    let read = read_file_with_snapshot(&file).unwrap();

    // FILE_SHARE_READ = 1 (no FILE_SHARE_DELETE) forces a sharing violation on rename.
    let handle = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&file)
        .unwrap();

    let releaser = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(250));
        drop(handle);
    });

    atomic_replace(&file, &read.snapshot, updated).unwrap();
    releaser.join().unwrap();

    assert_eq!(std::fs::read(&file).unwrap(), updated);
    assert_eq!(count_tmp_files(tmp.path()), 0);
}

#[cfg(windows)]
#[test]
fn windows_persistent_sharing_violation_fails_after_2s_and_cleans_tmp() {
    use std::os::windows::fs::OpenOptionsExt;

    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("TODO.md");
    let initial = b"<!-- TEST FIXTURE: invented data, not a real task list -->\n# Locked\n";
    std::fs::write(&file, initial).unwrap();

    let read = read_file_with_snapshot(&file).unwrap();

    // Hold without FILE_SHARE_DELETE for longer than the 2s retry window.
    let _handle = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&file)
        .unwrap();

    let err = atomic_replace(&file, &read.snapshot, b"never written\n").unwrap_err();
    assert_eq!(err.code, ErrorCode::WriteFailed);
    assert_eq!(err.exit_code, ExitCode::FileError);
    assert_eq!(std::fs::read(&file).unwrap(), initial);
    assert_eq!(count_tmp_files(tmp.path()), 0);
}

#[test]
fn target_removed_after_its_snapshot_is_refused_with_no_file_and_no_tmp() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("TODO.md");
    let initial = b"<!-- TEST FIXTURE: invented data, not a real task list -->\n# Removed\n";
    std::fs::write(&file, initial).unwrap();

    let read = read_file_with_snapshot(&file).unwrap();
    std::fs::remove_file(&file).unwrap();

    // The existence check at the start of the write answers first: nothing is made, and the file
    // is not recreated.
    let err = atomic_replace(&file, &read.snapshot, b"never written\n").unwrap_err();
    assert_eq!(err.code, ErrorCode::NoFile);
    assert_eq!(err.exit_code, ExitCode::FileError);
    assert!(!file.exists());
    assert_eq!(count_tmp_files(tmp.path()), 0);
}

/// The check just before the rename finds `TODO.md` gone: another program deleted it while the
/// write waited, so the write must exit 3, write nothing and leave no temporary file (R-IO-2).
#[cfg(feature = "test-hooks")]
#[test]
fn target_removed_while_the_write_waits_for_its_rename_exits_3_and_leaves_no_tmp() {
    let tmp = TempDir::new().unwrap();
    let body = "<!-- TEST FIXTURE: invented data, not a real task list -->\n# Removed late\n";
    let (home, work, todo_file) = common::setup_proj(&tmp, body);
    let ready = tmp.path().join("before-rename.ready");
    let ready_str = ready.to_string_lossy().into_owned();

    let writer = common::spawn_todo(
        &home,
        &work,
        &[
            "--json",
            "__test-append",
            "--agent",
            "writer-1",
            "--text",
            "our-line",
        ],
        &[
            ("TODO_TEST_BEFORE_RENAME_READY_FILE", ready_str.as_str()),
            ("TODO_TEST_BEFORE_RENAME_HOLD_MS", "1000"),
        ],
    )
    .unwrap();

    common::wait_for_file(&ready, std::time::Duration::from_secs(10)).unwrap();
    std::fs::remove_file(&todo_file).unwrap();

    let output = writer.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(3));
    let parsed = common::parse_json(&output.stdout);
    assert_eq!(parsed["error"]["code"], "CHANGED_DURING_WRITE");
    assert!(!todo_file.exists(), "the write must not recreate TODO.md");
    assert_eq!(count_tmp_files(&work), 0);
}

/// A program holds the file with no sharing at all for 250 ms. The check before the rename cannot
/// read the file until it is released, and then the replace succeeds.
#[cfg(windows)]
#[test]
fn windows_check_before_rename_waits_for_a_file_held_with_no_sharing() {
    use std::os::windows::fs::OpenOptionsExt;
    use std::time::Duration;

    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("TODO.md");
    let initial = b"<!-- TEST FIXTURE: invented data, not a real task list -->\n# Exclusive\n";
    let updated = b"<!-- TEST FIXTURE: invented data, not a real task list -->\n# Exclusive V2\n";
    std::fs::write(&file, initial).unwrap();

    let read = read_file_with_snapshot(&file).unwrap();

    let handle = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&file)
        .unwrap();
    let releaser = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        drop(handle);
    });

    atomic_replace(&file, &read.snapshot, updated).unwrap();
    releaser.join().unwrap();

    assert_eq!(std::fs::read(&file).unwrap(), updated);
    assert_eq!(count_tmp_files(tmp.path()), 0);
}

/// A program holds the file with no sharing for the whole call: the check before the rename gives
/// up only after the 2-second budget, reports WRITE_FAILED and leaves the old bytes.
#[cfg(windows)]
#[test]
fn windows_check_before_rename_gives_up_after_the_retry_budget() {
    use std::os::windows::fs::OpenOptionsExt;
    use std::time::{Duration, Instant};

    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("TODO.md");
    let initial = b"<!-- TEST FIXTURE: invented data, not a real task list -->\n# Exclusive\n";
    std::fs::write(&file, initial).unwrap();

    let read = read_file_with_snapshot(&file).unwrap();

    let handle = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&file)
        .unwrap();

    let started = Instant::now();
    let err = atomic_replace(&file, &read.snapshot, b"never written\n").unwrap_err();
    let waited = started.elapsed();
    drop(handle);

    assert_eq!(err.code, ErrorCode::WriteFailed);
    assert_eq!(err.exit_code, ExitCode::FileError);
    assert!(
        waited >= Duration::from_millis(1500),
        "gave up after {waited:?}, before the retry budget"
    );
    assert_eq!(std::fs::read(&file).unwrap(), initial);
    assert_eq!(count_tmp_files(tmp.path()), 0);
}

/// The rename waits for a program that holds the file without FILE_SHARE_DELETE. While it waits,
/// the temporary file must be in the folder of `TODO.md`, the one folder the test counts.
#[cfg(windows)]
#[test]
fn windows_temporary_file_is_next_to_todo_md_while_the_rename_waits() {
    use std::os::windows::fs::OpenOptionsExt;
    use std::time::Duration;

    // The temporary folder is not the working folder of the test (the package root).
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("TODO.md");
    let initial = b"<!-- TEST FIXTURE: invented data, not a real task list -->\n# Folder\n";
    let updated = b"<!-- TEST FIXTURE: invented data, not a real task list -->\n# Folder V2\n";
    std::fs::write(&file, initial).unwrap();

    let read = read_file_with_snapshot(&file).unwrap();

    let handle = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&file)
        .unwrap();
    let releaser = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(600));
        drop(handle);
    });

    let folder = tmp.path().to_path_buf();
    let looker = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        count_tmp_files(&folder)
    });

    atomic_replace(&file, &read.snapshot, updated).unwrap();
    let seen = looker.join().unwrap();
    releaser.join().unwrap();

    assert_eq!(
        seen, 1,
        "the temporary file must be in the folder of TODO.md while the rename waits"
    );
    assert_eq!(std::fs::read(&file).unwrap(), updated);
    assert_eq!(count_tmp_files(tmp.path()), 0);
}
