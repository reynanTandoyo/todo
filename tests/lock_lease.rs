//! Tests for manual leases, lock/unlock/status CLI commands, and git-free worktree execution (T-015, T-011; R-LOCK-5, R-LOCK-6, TEST-5, TEST-6, TEST-7).

mod lock_support;

use std::path::Path;
use std::process::Command;
#[cfg(feature = "test-hooks")]
use std::time::Duration;

use tempfile::TempDir;

fn setup_proj(
    tmp: &TempDir,
) -> std::io::Result<(std::path::PathBuf, std::path::PathBuf, std::path::PathBuf)> {
    let home = tmp.path().join("home");
    let work = tmp.path().join("work");
    std::fs::create_dir_all(&home)?;
    std::fs::create_dir_all(&work)?;
    let todo_file = work.join("TODO.md");
    std::fs::write(
        &todo_file,
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n# Lease tests\n",
    )?;
    Ok((home, work, todo_file))
}

fn run_git(dir: &Path, args: &[&str]) -> std::io::Result<()> {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .output()?;
    if !out.status.success() {
        return Err(std::io::Error::other(format!(
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    Ok(())
}

#[test]
fn status_works_before_any_state_exists_without_creating_dot_todo() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo_file) = setup_proj(&tmp).unwrap();

    let out = lock_support::run_todo(&home, &work, &["--json", "status"], &[]).unwrap();
    assert!(out.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(parsed["schema"], "todo.status.v1");
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["write_lock"]["held"], false);
    assert!(parsed["data"]["lease"].is_null());
    assert!(!work.join(".todo").exists());

    let text_out = lock_support::run_todo(&home, &work, &["status"], &[]).unwrap();
    assert!(text_out.status.success());
    let stdout = String::from_utf8_lossy(&text_out.stdout);
    assert!(stdout.contains("Write lock: free"));
    assert!(stdout.contains("Lease: none"));
}

#[test]
fn lock_requires_agent_and_valid_minutes() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo_file) = setup_proj(&tmp).unwrap();

    let no_agent = lock_support::run_todo(&home, &work, &["--json", "lock"], &[]).unwrap();
    assert_eq!(no_agent.status.code(), Some(2));
    let parsed: serde_json::Value = serde_json::from_slice(&no_agent.stdout).unwrap();
    assert_eq!(parsed["error"]["code"], "NO_AGENT");

    let zero_min = lock_support::run_todo(
        &home,
        &work,
        &["--json", "lock", "--agent", "alice", "--minutes", "0"],
        &[],
    )
    .unwrap();
    assert_eq!(zero_min.status.code(), Some(1));
    let parsed: serde_json::Value = serde_json::from_slice(&zero_min.stdout).unwrap();
    assert_eq!(parsed["error"]["code"], "BAD_VALUE");

    let too_long = lock_support::run_todo(
        &home,
        &work,
        &["--json", "lock", "--agent", "alice", "--minutes", "61"],
        &[],
    )
    .unwrap();
    assert_eq!(too_long.status.code(), Some(1));
    let parsed: serde_json::Value = serde_json::from_slice(&too_long.stdout).unwrap();
    assert_eq!(parsed["error"]["code"], "BAD_VALUE");
}

#[cfg(feature = "test-hooks")]
#[test]
fn lease_blocks_other_agents_allows_holder_and_expires() {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo_file) = setup_proj(&tmp).unwrap();
    let t0 = "2026-11-15T14:00:00+01:00";

    // Acquire 10-minute default lease as alice at t0.
    let lock_out = lock_support::run_todo(
        &home,
        &work,
        &["--json", "lock", "--agent", "alice"],
        &[("TODO_TEST_NOW", t0)],
    )
    .unwrap();
    assert!(lock_out.status.success());
    let lock_json: serde_json::Value = serde_json::from_slice(&lock_out.stdout).unwrap();
    assert_eq!(lock_json["schema"], "todo.lock.v1");
    assert_eq!(lock_json["data"]["agent"], "alice");
    assert_eq!(lock_json["data"]["minutes"], 10);
    let expires_at = lock_json["data"]["expires_at"].as_str().unwrap().to_owned();

    // `todo status` shows the active lease (both JSON and text).
    let status_out = lock_support::run_todo(
        &home,
        &work,
        &["--json", "status"],
        &[("TODO_TEST_NOW", "2026-11-15T14:02:00+01:00")],
    )
    .unwrap();
    assert!(status_out.status.success());
    let status_json: serde_json::Value = serde_json::from_slice(&status_out.stdout).unwrap();
    assert_eq!(status_json["data"]["lease"]["agent"], "alice");
    assert_eq!(status_json["data"]["lease"]["remaining_seconds"], 480);

    let status_text = lock_support::run_todo(
        &home,
        &work,
        &["status"],
        &[("TODO_TEST_NOW", "2026-11-15T14:02:00+01:00")],
    )
    .unwrap();
    assert!(status_text.status.success());
    let st = String::from_utf8_lossy(&status_text.stdout);
    assert!(st.contains("Lease: held by alice"), "status:\n{st}");

    // Holder (alice, case-insensitive) can write normally.
    let holder_write = lock_support::run_todo(
        &home,
        &work,
        &["__test-append", "--agent", "ALICE", "--text", "by-alice"],
        &[("TODO_TEST_NOW", "2026-11-15T14:05:00+01:00")],
    )
    .unwrap();
    assert!(holder_write.status.success());
    assert!(
        std::fs::read_to_string(&todo_file)
            .unwrap()
            .contains("by-alice")
    );

    // Another agent (bob) waits and exits 3 with LEASE_HELD naming alice and the expiry time.
    let bob_write = lock_support::run_todo(
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
    let bob_json: serde_json::Value = serde_json::from_slice(&bob_write.stdout).unwrap();
    assert_eq!(bob_json["error"]["code"], "LEASE_HELD");
    assert_eq!(bob_json["error"]["owner"], "alice");
    assert_eq!(bob_json["error"]["expires_at"], expires_at);
    assert!(
        bob_json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("alice")
    );
    assert!(
        !std::fs::read_to_string(&todo_file)
            .unwrap()
            .contains("by-bob-blocked")
    );

    // After expiry (t0 + 11 minutes), status shows no lease and bob can write.
    let expired_status = lock_support::run_todo(
        &home,
        &work,
        &["--json", "status"],
        &[("TODO_TEST_NOW", "2026-11-15T14:11:00+01:00")],
    )
    .unwrap();
    let exp_json: serde_json::Value = serde_json::from_slice(&expired_status.stdout).unwrap();
    assert!(exp_json["data"]["lease"].is_null());

    let bob_after_expiry = lock_support::run_todo(
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
    assert!(
        std::fs::read_to_string(&todo_file)
            .unwrap()
            .contains("by-bob-after-expiry")
    );
}

#[cfg(feature = "test-hooks")]
#[test]
fn unlock_requires_force_for_other_agent_and_names_previous_holder() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo_file) = setup_proj(&tmp).unwrap();
    let t0 = "2026-11-15T14:00:00+01:00";

    let lock_out = lock_support::run_todo(
        &home,
        &work,
        &["lock", "--agent", "alice", "--minutes", "30"],
        &[("TODO_TEST_NOW", t0)],
    )
    .unwrap();
    assert!(lock_out.status.success());

    // bob without --force is refused (exit 1, NOT_OWNER).
    let deny = lock_support::run_todo(
        &home,
        &work,
        &["--json", "unlock", "--agent", "bob"],
        &[("TODO_TEST_NOW", t0)],
    )
    .unwrap();
    assert_eq!(deny.status.code(), Some(1));
    let deny_json: serde_json::Value = serde_json::from_slice(&deny.stdout).unwrap();
    assert_eq!(deny_json["error"]["code"], "NOT_OWNER");
    assert_eq!(deny_json["error"]["owner"], "alice");

    // bob with --force ends alice's lease and names alice.
    let forced = lock_support::run_todo(
        &home,
        &work,
        &["--json", "unlock", "--agent", "bob", "--force"],
        &[("TODO_TEST_NOW", t0)],
    )
    .unwrap();
    assert!(forced.status.success());
    let forced_json: serde_json::Value = serde_json::from_slice(&forced.stdout).unwrap();
    assert_eq!(forced_json["schema"], "todo.unlock.v1");
    assert_eq!(forced_json["data"]["unlocked"], true);
    assert_eq!(forced_json["data"]["forced"], true);
    assert_eq!(forced_json["data"]["previous_holder"], "alice");

    // Same in text mode: verify message names previous holder.
    assert!(
        lock_support::run_todo(
            &home,
            &work,
            &["lock", "--agent", "carol"],
            &[("TODO_TEST_NOW", t0)],
        )
        .unwrap()
        .status
        .success()
    );
    let forced_text = lock_support::run_todo(
        &home,
        &work,
        &["unlock", "--agent", "bob", "--force"],
        &[("TODO_TEST_NOW", t0)],
    )
    .unwrap();
    assert!(forced_text.status.success());
    assert!(String::from_utf8_lossy(&forced_text.stdout).contains("carol"));

    // Owner unlocking own lease works without --force.
    assert!(
        lock_support::run_todo(
            &home,
            &work,
            &["lock", "--agent", "dave"],
            &[("TODO_TEST_NOW", t0)],
        )
        .unwrap()
        .status
        .success()
    );
    let own_unlock = lock_support::run_todo(
        &home,
        &work,
        &["unlock", "--agent", "dave"],
        &[("TODO_TEST_NOW", t0)],
    )
    .unwrap();
    assert!(own_unlock.status.success());
    assert!(String::from_utf8_lossy(&own_unlock.stdout).contains("dave"));
}

#[cfg(feature = "test-hooks")]
#[test]
fn status_shows_active_write_lock_holder_without_waiting() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo_file) = setup_proj(&tmp).unwrap();
    let ready = tmp.path().join("status.ready");
    let ready_str = ready.to_string_lossy().into_owned();

    let mut holder = lock_support::spawn_todo(
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

    lock_support::wait_for_file(&ready, Duration::from_secs(5)).unwrap();

    let start = std::time::Instant::now();
    let status_out = lock_support::run_todo(&home, &work, &["--json", "status"], &[]).unwrap();
    let elapsed = start.elapsed();

    assert!(status_out.status.success());
    assert!(elapsed < Duration::from_millis(500));
    let parsed: serde_json::Value = serde_json::from_slice(&status_out.stdout).unwrap();
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
    let (home, work, todo_file) = setup_proj(&tmp).unwrap();
    let ready = tmp.path().join("before-rename.ready");
    let ready_str = ready.to_string_lossy().into_owned();

    let writer = lock_support::spawn_todo(
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

    lock_support::wait_for_file(&ready, Duration::from_secs(5)).unwrap();

    // Modify TODO.md externally right before rename.
    let external = "<!-- TEST FIXTURE: invented data, not a real task list -->\n# External\n";
    std::fs::write(&todo_file, external).unwrap();

    let output = writer.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(3));
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["error"]["code"], "CHANGED_DURING_WRITE");
    assert_eq!(std::fs::read_to_string(&todo_file).unwrap(), external);

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

    run_git(&main_repo, &["init"]).unwrap();
    let main_todo = main_repo.join("TODO.md");
    std::fs::write(
        &main_todo,
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n# Main\n",
    )
    .unwrap();
    run_git(&main_repo, &["add", "."]).unwrap();
    run_git(&main_repo, &["commit", "-m", "init"]).unwrap();
    run_git(
        &main_repo,
        &["worktree", "add", "-b", "wt", wt_dir.to_str().unwrap()],
    )
    .unwrap();

    // Run `todo status --json` with PATH cleared so git cannot be found.
    let out =
        lock_support::run_todo(&home, &wt_dir, &["--json", "status"], &[("PATH", "")]).unwrap();
    assert!(
        out.status.success(),
        "status failed without git on PATH: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let reported = Path::new(parsed["data"]["file"].as_str().unwrap());
    assert_eq!(
        std::fs::canonicalize(reported).unwrap(),
        std::fs::canonicalize(&main_todo).unwrap()
    );
}
