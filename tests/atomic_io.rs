//! Tests for atomic file replacement and IO safety (T-014; R-IO-1..5, TEST-7).

use std::path::Path;
use std::time::Duration;

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
        std::thread::sleep(Duration::from_millis(250));
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
