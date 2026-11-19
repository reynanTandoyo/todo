//! Tests for manual leases, lock/unlock/status CLI commands, and git-free worktree execution (T-015, T-011; R-LOCK-5, R-LOCK-6, TEST-5, TEST-6, TEST-7).

mod common;

use std::path::Path;
#[cfg(feature = "test-hooks")]
use std::time::Duration;

use tempfile::TempDir;

#[cfg(feature = "test-hooks")]
use common::read_text;
use common::{git, parse_json, setup_proj, stderr_text, stdout_text};

const LEASE_BODY: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->
# Lease tests
";

#[test]
fn status_works_before_any_state_exists_without_creating_dot_todo() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo_file) = setup_proj(&tmp, LEASE_BODY);

    let out = common::run_todo(&home, &work, &["--json", "status"], &[]).unwrap();
    assert!(out.status.success());
    let parsed: serde_json::Value = parse_json(&out.stdout);
    assert_eq!(parsed["schema"], "todo.status.v1");
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["write_lock"]["held"], false);
    assert!(parsed["data"]["lease"].is_null());
    assert!(!work.join(".todo").exists());

    let text_out = common::run_todo(&home, &work, &["status"], &[]).unwrap();
    assert!(text_out.status.success());
    let stdout = stdout_text(&text_out);
    assert!(stdout.contains("Write lock: free"));
    assert!(stdout.contains("Lease: none"));
}

#[test]
fn lock_requires_agent_and_valid_minutes() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo_file) = setup_proj(&tmp, LEASE_BODY);

    let no_agent = common::run_todo(&home, &work, &["--json", "lock"], &[]).unwrap();
    assert_eq!(no_agent.status.code(), Some(2));
    let parsed: serde_json::Value = parse_json(&no_agent.stdout);
    assert_eq!(parsed["error"]["code"], "NO_AGENT");

    let zero_min = common::run_todo(
        &home,
        &work,
        &["--json", "lock", "--agent", "alice", "--minutes", "0"],
        &[],
    )
    .unwrap();
    assert_eq!(zero_min.status.code(), Some(1));
    let parsed: serde_json::Value = parse_json(&zero_min.stdout);
    assert_eq!(parsed["error"]["code"], "BAD_VALUE");

    let too_long = common::run_todo(
        &home,
        &work,
        &["--json", "lock", "--agent", "alice", "--minutes", "61"],
        &[],
    )
    .unwrap();
    assert_eq!(too_long.status.code(), Some(1));
    let parsed: serde_json::Value = parse_json(&too_long.stdout);
    assert_eq!(parsed["error"]["code"], "BAD_VALUE");
}

#[cfg(feature = "test-hooks")]
#[test]
fn lease_blocks_other_agents_allows_holder_and_expires() {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo_file) = setup_proj(&tmp, LEASE_BODY);
    let t0 = "2026-11-15T14:00:00+01:00";

    // Acquire 10-minute default lease as alice at t0.
    let lock_out = common::run_todo(
        &home,
        &work,
        &["--json", "lock", "--agent", "alice"],
        &[("TODO_TEST_NOW", t0)],
    )
    .unwrap();
    assert!(lock_out.status.success());
    let lock_json: serde_json::Value = parse_json(&lock_out.stdout);
    assert_eq!(lock_json["schema"], "todo.lock.v1");
    assert_eq!(lock_json["data"]["agent"], "alice");
    assert_eq!(lock_json["data"]["minutes"], 10);
    let expires_at = lock_json["data"]["expires_at"].as_str().unwrap().to_owned();

    // `todo status` shows the active lease (both JSON and text).
    let status_out = common::run_todo(
        &home,
        &work,
        &["--json", "status"],
        &[("TODO_TEST_NOW", "2026-11-15T14:02:00+01:00")],
    )
    .unwrap();
    assert!(status_out.status.success());
    let status_json: serde_json::Value = parse_json(&status_out.stdout);
    assert_eq!(status_json["data"]["lease"]["agent"], "alice");
    assert_eq!(status_json["data"]["lease"]["remaining_seconds"], 480);

    let status_text = common::run_todo(
        &home,
        &work,
        &["status"],
        &[("TODO_TEST_NOW", "2026-11-15T14:02:00+01:00")],
    )
    .unwrap();
    assert!(status_text.status.success());
    let st = stdout_text(&status_text);
    assert!(st.contains("Lease: held by alice"), "status:\n{st}");

    // Holder (alice, case-insensitive) can write normally.
    let holder_write = common::run_todo(
        &home,
        &work,
        &["__test-append", "--agent", "ALICE", "--text", "by-alice"],
        &[("TODO_TEST_NOW", "2026-11-15T14:05:00+01:00")],
    )
    .unwrap();
    assert!(holder_write.status.success());
    assert!(read_text(&todo_file).contains("by-alice"));

    // Another agent (bob) waits and exits 3 with LEASE_HELD naming alice and the expiry time.
    let bob_write = common::run_todo(
        &home,
        &work,
        &[
            "--json",
            "__test-append",
            "--agent",
            "bob",
            "--text",
            "by-bob-blocked",
        ],
        &[
            ("TODO_TEST_NOW", "2026-11-15T14:05:00+01:00"),
            ("TODO_TEST_LOCK_WAIT_MS", "200"),
        ],
    )
    .unwrap();
    assert_eq!(bob_write.status.code(), Some(3));
    let bob_json: serde_json::Value = parse_json(&bob_write.stdout);
    assert_eq!(bob_json["error"]["code"], "LEASE_HELD");
    assert_eq!(bob_json["error"]["owner"], "alice");
    assert_eq!(bob_json["error"]["expires_at"], expires_at);
    assert!(
        bob_json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("alice")
    );
    assert!(!read_text(&todo_file).contains("by-bob-blocked"));

    // After expiry (t0 + 11 minutes), status shows no lease and bob can write.
    let expired_status = common::run_todo(
        &home,
        &work,
        &["--json", "status"],
        &[("TODO_TEST_NOW", "2026-11-15T14:11:00+01:00")],
    )
    .unwrap();
    let exp_json: serde_json::Value = parse_json(&expired_status.stdout);
    assert!(exp_json["data"]["lease"].is_null());

    let bob_after_expiry = common::run_todo(
        &home,
        &work,
        &[
            "__test-append",
            "--agent",
            "bob",
            "--text",
            "by-bob-after-expiry",
        ],
        &[("TODO_TEST_NOW", "2026-11-15T14:11:00+01:00")],
    )
    .unwrap();
    assert!(bob_after_expiry.status.success());
    assert!(read_text(&todo_file).contains("by-bob-after-expiry"));
}

#[cfg(feature = "test-hooks")]
#[test]
fn unlock_requires_force_for_other_agent_and_names_previous_holder() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo_file) = setup_proj(&tmp, LEASE_BODY);
    let t0 = "2026-11-15T14:00:00+01:00";

    let lock_out = common::run_todo(
        &home,
        &work,
        &["lock", "--agent", "alice", "--minutes", "30"],
        &[("TODO_TEST_NOW", t0)],
    )
    .unwrap();
    assert!(lock_out.status.success());

    // bob without --force is refused (exit 1, NOT_OWNER).
    let deny = common::run_todo(
        &home,
        &work,
        &["--json", "unlock", "--agent", "bob"],
        &[("TODO_TEST_NOW", t0)],
    )
    .unwrap();
    assert_eq!(deny.status.code(), Some(1));
    let deny_json: serde_json::Value = parse_json(&deny.stdout);
    assert_eq!(deny_json["error"]["code"], "NOT_OWNER");
    assert_eq!(deny_json["error"]["owner"], "alice");

    // bob with --force ends alice's lease and names alice.
    let forced = common::run_todo(
        &home,
        &work,
        &["--json", "unlock", "--agent", "bob", "--force"],
        &[("TODO_TEST_NOW", t0)],
    )
    .unwrap();
    assert!(forced.status.success());
    let forced_json: serde_json::Value = parse_json(&forced.stdout);
    assert_eq!(forced_json["schema"], "todo.unlock.v1");
    assert_eq!(forced_json["data"]["unlocked"], true);
    assert_eq!(forced_json["data"]["forced"], true);
    assert_eq!(forced_json["data"]["previous_holder"], "alice");

    // Same in text mode: verify message names previous holder.
    assert!(
        common::run_todo(
            &home,
            &work,
            &["lock", "--agent", "carol"],
            &[("TODO_TEST_NOW", t0)],
        )
        .unwrap()
        .status
        .success()
    );
    let forced_text = common::run_todo(
        &home,
        &work,
        &["unlock", "--agent", "bob", "--force"],
        &[("TODO_TEST_NOW", t0)],
    )
    .unwrap();
    assert!(forced_text.status.success());
    assert!(stdout_text(&forced_text).contains("carol"));

    // Owner unlocking own lease works without --force.
    assert!(
        common::run_todo(
            &home,
            &work,
            &["lock", "--agent", "dave"],
            &[("TODO_TEST_NOW", t0)],
        )
        .unwrap()
        .status
        .success()
    );
    let own_unlock = common::run_todo(
        &home,
        &work,
        &["unlock", "--agent", "dave"],
        &[("TODO_TEST_NOW", t0)],
    )
    .unwrap();
    assert!(own_unlock.status.success());
    assert!(stdout_text(&own_unlock).contains("dave"));
}

#[cfg(feature = "test-hooks")]
#[test]
fn status_shows_active_write_lock_holder_without_waiting() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo_file) = setup_proj(&tmp, LEASE_BODY);
    let ready = tmp.path().join("status.ready");
    let ready_str = ready.to_string_lossy().into_owned();

    let mut holder = common::spawn_todo(
        &home,
        &work,
        &[
            "__test-append",
            "--agent",
            "lock-owner",
            "--text",
            "x",
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

    // A status that waited for the write lock would wait the whole lock timeout, here 10 s (the
    // holder keeps the lock for 20 s). Status takes well under a second when it does not wait, so
    // the timeout is a bound that does not depend on how fast the machine is.
    let lock_wait = Duration::from_secs(10);
    let lock_wait_ms = lock_wait.as_millis().to_string();
    let start = std::time::Instant::now();
    let status_out = common::run_todo(
        &home,
        &work,
        &["--json", "status"],
        &[("TODO_TEST_LOCK_WAIT_MS", lock_wait_ms.as_str())],
    )
    .unwrap();
    let elapsed = start.elapsed();

    assert!(status_out.status.success());
    assert!(
        elapsed < lock_wait,
        "status took {elapsed:?}, so it waited for the write lock"
    );
    let parsed: serde_json::Value = parse_json(&status_out.stdout);
    assert_eq!(parsed["data"]["write_lock"]["held"], true);
    assert_eq!(
        parsed["data"]["write_lock"]["holder"]["agent"],
        "lock-owner"
    );
    assert_eq!(parsed["data"]["write_lock"]["holder"]["pid"], holder_pid);

    holder.kill().unwrap();
    let _ = holder.wait();
}

#[cfg(feature = "test-hooks")]
#[test]
fn cli_changed_during_write_exits_3_and_leaves_no_tmp() {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo_file) = setup_proj(&tmp, LEASE_BODY);
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
            ("TODO_TEST_BEFORE_RENAME_READY_FILE", &ready_str),
            ("TODO_TEST_BEFORE_RENAME_HOLD_MS", "400"),
        ],
    )
    .unwrap();

    common::wait_for_file(&ready, Duration::from_secs(5)).unwrap();

    // Modify TODO.md externally right before rename.
    let external = "<!-- TEST FIXTURE: invented data, not a real task list -->\n# External\n";
    std::fs::write(&todo_file, external).unwrap();

    let output = writer.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(3));
    let parsed: serde_json::Value = parse_json(&output.stdout);
    assert_eq!(parsed["error"]["code"], "CHANGED_DURING_WRITE");
    assert_eq!(read_text(&todo_file), external);

    let tmp_count = std::fs::read_dir(&work)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
        .count();
    assert_eq!(tmp_count, 0);
}

#[test]
fn linked_worktree_status_works_with_git_removed_from_path() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let main_repo = tmp.path().join("main-repo");
    let wt_dir = tmp.path().join("wt-no-git");
    std::fs::create_dir_all(&main_repo).unwrap();

    git(&main_repo, &["init"]);
    let main_todo = main_repo.join("TODO.md");
    std::fs::write(
        &main_todo,
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n# Main\n",
    )
    .unwrap();
    git(&main_repo, &["add", "."]);
    git(&main_repo, &["commit", "-m", "init"]);
    git(
        &main_repo,
        &["worktree", "add", "-b", "wt", wt_dir.to_str().unwrap()],
    );

    // Run `todo status --json` with PATH cleared so git cannot be found.
    let out = common::run_todo(&home, &wt_dir, &["--json", "status"], &[("PATH", "")]).unwrap();
    assert!(
        out.status.success(),
        "status failed without git on PATH: {}",
        stderr_text(&out)
    );
    let parsed: serde_json::Value = parse_json(&out.stdout);
    let reported = Path::new(parsed["data"]["file"].as_str().unwrap());
    assert_eq!(
        std::fs::canonicalize(reported).unwrap(),
        std::fs::canonicalize(&main_todo).unwrap()
    );
}
