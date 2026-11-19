//! Tests for the write lock (T-013; R-LOCK-1..5, TEST-5).

mod common;

#[cfg(feature = "test-hooks")]
use common::{parse_json, read_text, stderr_text};

use std::time::Duration;

use tempfile::TempDir;
use todo::atomic::read_file_with_snapshot;
use todo::error::{ErrorCode, ExitCode};
use todo::find::{FindOptions, find_todo};
use todo::lock::{LockConfig, WriteLockGuard, WriteOutcome, locked_read_modify_write};
use todo::state::StateDir;

#[test]
fn unchanged_outcome_does_not_rewrite_file() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("proj");
    std::fs::create_dir_all(&dir).unwrap();
    let todo_file = dir.join("TODO.md");
    let initial = b"<!-- TEST FIXTURE: invented data, not a real task list -->\n# Keep\n";
    std::fs::write(&todo_file, initial).unwrap();

    let found = find_todo(&FindOptions {
        start_dir: Some(&dir),
        ..Default::default()
    })
    .unwrap();
    let state = StateDir::from_found(&found).unwrap();

    let res = locked_read_modify_write(
        &found.path,
        &state,
        "agent-1",
        "noop",
        &LockConfig::default(),
        |_bytes, _snap| Ok(WriteOutcome::Unchanged(42)),
    )
    .unwrap();
    assert_eq!(res, 42);
    assert_eq!(std::fs::read(&todo_file).unwrap(), initial);
    assert!(state.read_last_write().unwrap().is_none());
}

#[test]
fn changed_outcome_writes_and_updates_last_write_and_releases_holder() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("proj");
    std::fs::create_dir_all(&dir).unwrap();
    let todo_file = dir.join("TODO.md");
    let initial = b"<!-- TEST FIXTURE: invented data, not a real task list -->\n# One\n";
    std::fs::write(&todo_file, initial).unwrap();

    let found = find_todo(&FindOptions {
        start_dir: Some(&dir),
        ..Default::default()
    })
    .unwrap();
    let state = StateDir::from_found(&found).unwrap();

    locked_read_modify_write(
        &found.path,
        &state,
        "agent-1",
        "append",
        &LockConfig::default(),
        |bytes, _snap| {
            let mut out = bytes.to_vec();
            out.extend_from_slice(b"added line\n");
            Ok(WriteOutcome::Changed {
                new_bytes: out,
                value: (),
            })
        },
    )
    .unwrap();

    assert_eq!(
        std::fs::read(&todo_file).unwrap(),
        b"<!-- TEST FIXTURE: invented data, not a real task list -->\n# One\nadded line\n"
    );
    // holder.json must be cleaned up when the lock is released.
    assert!(state.read_holder().unwrap().is_none());
    let lw = state.read_last_write().unwrap().unwrap();
    let snap = read_file_with_snapshot(&todo_file).unwrap().snapshot;
    assert_eq!(lw.size, snap.size);
    assert_eq!(lw.sha256, snap.sha256_hex());
}

#[test]
fn held_in_process_lock_times_out_with_busy_naming_holder() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("proj");
    std::fs::create_dir_all(&dir).unwrap();
    let todo_file = dir.join("TODO.md");
    std::fs::write(
        &todo_file,
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n# Lock\n",
    )
    .unwrap();

    let found = find_todo(&FindOptions {
        start_dir: Some(&dir),
        ..Default::default()
    })
    .unwrap();
    let state = StateDir::from_found(&found).unwrap();

    let _guard =
        WriteLockGuard::acquire(&state, "holder-x", "claim", Duration::from_secs(1)).unwrap();
    let err = WriteLockGuard::acquire(&state, "waiter-y", "done", Duration::from_millis(120))
        .err()
        .unwrap();
    assert_eq!(err.code, ErrorCode::Busy);
    assert_eq!(err.exit_code, ExitCode::Busy);
    assert_eq!(err.details.owner.as_deref(), Some("holder-x"));
    assert!(err.message.contains("holder-x"));
    assert!(err.message.contains("claim"));
}

#[cfg(feature = "test-hooks")]
#[test]
fn killed_holder_delays_next_writer_by_less_than_2_seconds() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let work = tmp.path().join("work");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&work).unwrap();
    let todo_file = work.join("TODO.md");
    let initial = "<!-- TEST FIXTURE: invented data, not a real task list -->\n# Kill test\n";
    std::fs::write(&todo_file, initial).unwrap();

    let ready = tmp.path().join("holder.ready");
    let ready_str = ready.to_string_lossy().into_owned();

    let mut holder = common::spawn_todo(
        &home,
        &work,
        &[
            "__test-append",
            "--agent",
            "crash-agent",
            "--text",
            "never-committed",
            "--hold-ms",
            "30000",
            "--ready-file",
            &ready_str,
        ],
        &[],
    )
    .unwrap();

    common::wait_for_file(&ready, Duration::from_secs(5)).unwrap();

    // Kill the holder abruptly while it holds write.lock.
    holder.kill().unwrap();
    let _ = holder.wait();

    let start = std::time::Instant::now();
    let out = common::run_todo(
        &home,
        &work,
        &[
            "__test-append",
            "--agent",
            "survivor-agent",
            "--text",
            "after-kill",
        ],
        &[],
    )
    .unwrap();
    let elapsed = start.elapsed();

    assert!(
        out.status.success(),
        "expected survivor to succeed, stderr: {}",
        stderr_text(&out)
    );
    assert!(
        elapsed < Duration::from_secs(2),
        "expected < 2s delay after killed holder, took {elapsed:?}"
    );
    let content = read_text(&todo_file);
    assert!(content.contains("after-kill"));
    assert!(!content.contains("never-committed"));
}

#[cfg(feature = "test-hooks")]
#[test]
fn held_lock_causes_exit_3_naming_holder_without_writing_and_reads_do_not_wait() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let work = tmp.path().join("work");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&work).unwrap();
    let todo_file = work.join("TODO.md");
    let initial = "<!-- TEST FIXTURE: invented data, not a real task list -->\n# Held test\n";
    std::fs::write(&todo_file, initial).unwrap();

    let ready = tmp.path().join("held.ready");
    let ready_str = ready.to_string_lossy().into_owned();

    let mut holder = common::spawn_todo(
        &home,
        &work,
        &[
            "__test-append",
            "--agent",
            "busy-holder",
            "--text",
            "holder-write",
            "--hold-ms",
            "20000",
            "--ready-file",
            &ready_str,
        ],
        &[],
    )
    .unwrap();
    let holder_pid = holder.id();

    common::wait_for_file(&ready, Duration::from_secs(5)).unwrap();

    // Reads must never wait for the write lock.
    let read_start = std::time::Instant::now();
    let read = read_file_with_snapshot(&todo_file).unwrap();
    assert_eq!(read.bytes, initial.as_bytes());
    assert!(read_start.elapsed() < Duration::from_millis(500));

    // Competing write exits 3 (BUSY) naming the holder and leaves TODO.md unchanged.
    let waiter_json = common::run_todo(
        &home,
        &work,
        &[
            "--json",
            "__test-append",
            "--agent",
            "waiter-agent",
            "--text",
            "should-not-appear",
        ],
        &[("TODO_TEST_LOCK_WAIT_MS", "250")],
    )
    .unwrap();

    assert_eq!(waiter_json.status.code(), Some(3));
    let parsed: serde_json::Value = parse_json(&waiter_json.stdout);
    assert_eq!(parsed["ok"], false);
    assert_eq!(parsed["error"]["code"], "BUSY");
    assert_eq!(parsed["error"]["owner"], "busy-holder");
    assert_eq!(parsed["error"]["command"], "__test-append");
    assert_eq!(parsed["error"]["pid"], holder_pid);
    assert!(parsed["error"]["started_at"].as_str().is_some());

    let waiter_text = common::run_todo(
        &home,
        &work,
        &[
            "__test-append",
            "--agent",
            "waiter-agent",
            "--text",
            "should-not-appear-2",
        ],
        &[("TODO_TEST_LOCK_WAIT_MS", "250")],
    )
    .unwrap();
    assert_eq!(waiter_text.status.code(), Some(3));
    let stderr = stderr_text(&waiter_text);
    assert!(stderr.contains("busy-holder"), "stderr: {stderr}");
    assert!(stderr.contains("__test-append"), "stderr: {stderr}");
    assert!(stderr.contains(&holder_pid.to_string()), "stderr: {stderr}");

    // Kill holder before it writes; file must remain untouched.
    holder.kill().unwrap();
    let _ = holder.wait();
    assert_eq!(read_text(&todo_file), initial);
}

#[cfg(feature = "test-hooks")]
#[test]
fn parallel_writes_serialize_without_losing_updates() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let work = tmp.path().join("work");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&work).unwrap();
    let todo_file = work.join("TODO.md");
    std::fs::write(
        &todo_file,
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n# Parallel\n",
    )
    .unwrap();

    let count = 8;
    let invocations: Vec<Vec<String>> = (0..count)
        .map(|i| {
            vec![
                "__test-append".to_owned(),
                "--agent".to_owned(),
                format!("worker-{i}"),
                "--text".to_owned(),
                format!("entry-{i}"),
            ]
        })
        .collect();

    let outputs = common::run_parallel_todos(&home, &work, &invocations).unwrap();
    for (i, out) in outputs.iter().enumerate() {
        assert!(
            out.status.success(),
            "worker-{i} failed ({:?}): {}",
            out.status.code(),
            stderr_text(out)
        );
    }

    let final_text = read_text(&todo_file);
    for i in 0..count {
        assert!(
            final_text.contains(&format!("entry-{i}")),
            "missing entry-{i} in:\n{final_text}"
        );
    }
}
