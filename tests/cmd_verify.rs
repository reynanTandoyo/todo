//! CLI tests for the `verify` command and the check that `done` runs (T-158; R-VERIFY-1, R-VERIFY-2,
//! R-WRITE-4, R-WRITE-8, R-WRITE-10, R-CHECK-1, NFR-5, NFR-6).
//!
//! The checks are tiny invented commands that pass, fail or outlive a short timeout. Each runs on the
//! default shell of the platform, so the same test passes on Windows and Linux.

mod common;

use std::fs;
use std::path::Path;
use std::process::Output;
use std::time::Instant;

use tempfile::TempDir;

use common::{
    FIXTURE_MARKER, git, parse_json, read_text, run_todo, setup_proj, stderr_text, stdout_text,
    write_toml,
};

/// The clock of every command in these tests (the `TODO_TEST_NOW` hook), so note dates are fixed.
const NOW: &str = "2026-11-15T12:00:00+01:00";

/// Runs `todo` in `work` at the test clock.
fn todo(home: &Path, work: &Path, args: &[&str]) -> Output {
    run_todo(home, work, args, &[("TODO_TEST_NOW", NOW)]).unwrap()
}

/// The file text without the stamp lines that a write adds at the top, so the expected text does
/// not depend on the tool version.
fn without_stamps(text: &str) -> String {
    text.split_inclusive('\n')
        .filter(|line| {
            !line.starts_with("<!-- todo-format:") && !line.starts_with("<!-- todo-tool:")
        })
        .collect()
}

/// A check that passes on this platform's default shell.
fn pass_check() -> &'static str {
    "exit 0"
}

/// A check that prints `message` and fails with exit code 1.
fn fail_check(message: &str) -> String {
    if cfg!(windows) {
        format!("Write-Output \"{message}\"; exit 1")
    } else {
        format!("echo \"{message}\"; exit 1")
    }
}

/// A check that prints a line and passes.
fn chatty_check() -> String {
    if cfg!(windows) {
        "Write-Output \"chatter\"; exit 0".to_string()
    } else {
        "echo chatter; exit 0".to_string()
    }
}

/// A check that runs for 30 seconds, longer than any timeout the tests set.
fn slow_check() -> &'static str {
    if cfg!(windows) {
        "Start-Sleep -Seconds 30"
    } else {
        "exec sleep 30"
    }
}

/// A check that writes `file` in its folder and passes.
fn marker_check(file: &str) -> String {
    if cfg!(windows) {
        format!("Set-Content -Path {file} -Value ok")
    } else {
        format!("echo ok > {file}")
    }
}

/// One task block: the heading, then one `- field` line per entry.
fn block(id: &str, title: &str, fields: &[&str]) -> String {
    let mut text = format!("### {id} · {title}\n");
    for field in fields {
        text.push_str("- ");
        text.push_str(field);
        text.push('\n');
    }
    text.push('\n');
    text
}

/// A TODO.md with the fixture marker and the given blocks, under one `##` section.
fn file(blocks: &[String]) -> String {
    let mut text = format!("{FIXTURE_MARKER}\n# Fixture\n\n## Now\n\n");
    for b in blocks {
        text.push_str(b);
    }
    text
}

/// T-001 claimed by alice, with `verify` as its check and a note that says it was added.
fn claimed_with_verify(verify: &str) -> String {
    let verify = format!("verify: {verify}");
    file(&[block(
        "T-001",
        "Build the parser",
        &[
            "status: claimed",
            "owner: alice",
            "claimed: 2026-11-15T10:00+01:00",
            "priority: P1 · difficulty: 2",
            &verify,
            "note (2026-11-15, alice): added",
        ],
    )])
}

/// Parses the `--json` error object of a failed command and checks its code and exit code.
fn assert_json_error(out: &Output, code: &str, exit: i32) -> serde_json::Value {
    assert_eq!(
        out.status.code(),
        Some(exit),
        "exit code; stdout: {}; stderr: {}",
        stdout_text(out),
        stderr_text(out)
    );
    let json = parse_json(&out.stdout);
    assert_eq!(json["ok"], false, "{json}");
    assert_eq!(json["error"]["code"], code, "{json}");
    json
}

// ---------------------------------------------------------------------------------------------
// `done` runs the check first (mode `done`)
// ---------------------------------------------------------------------------------------------

#[test]
fn done_runs_the_check_first_and_records_verified_when_it_passes() {
    let tmp = TempDir::new().unwrap();
    let verify = format!("verify: {}", pass_check());
    let body = file(&[block(
        "T-001",
        "Build the parser",
        &[
            "status: claimed",
            "owner: alice",
            "claimed: 2026-11-15T10:00+01:00",
            "priority: P1 · difficulty: 2",
            &verify,
            "note (2026-11-15, alice): added",
        ],
    )]);
    let (home, work, todo_file) = setup_proj(&tmp, &body);

    let out = todo(
        &home,
        &work,
        &[
            "done",
            "T-001",
            "--agent",
            "alice",
            "--note",
            "all checks pass",
        ],
    );
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));

    // Done and owned by alice, claim removed, `verified` after `verify` (format §10).
    assert_eq!(
        without_stamps(&read_text(&todo_file)),
        file(&[block(
            "T-001",
            "Build the parser",
            &[
                "status: done",
                "owner: alice",
                "priority: P1 · difficulty: 2",
                &verify,
                "verified: 2026-11-15",
                "note (2026-11-15, alice): added",
                "note (2026-11-15, alice): done — all checks pass",
            ],
        )])
    );
}

#[test]
fn done_refused_when_the_check_fails_keeps_the_task_claimed_and_counts_the_attempt() {
    let tmp = TempDir::new().unwrap();
    let verify = format!("verify: {}", fail_check("tests failed"));
    let body = claimed_with_verify(&fail_check("tests failed"));
    let (home, work, todo_file) = setup_proj(&tmp, &body);

    let out = todo(
        &home,
        &work,
        &["--json", "done", "T-001", "--agent", "alice"],
    );
    let json = assert_json_error(&out, "VERIFY_FAILED", 1);
    assert_eq!(json["error"]["task"], "T-001");
    assert_eq!(json["error"]["exit_code"], 1);
    assert_eq!(json["error"]["last_line"], "tests failed");
    assert_eq!(json["error"]["timed_out"], false);

    // The output of the check is kept in full in the state folder (R-VERIFY-1).
    let log = json["error"]["log"].as_str().unwrap();
    assert!(read_text(Path::new(log)).contains("tests failed"));

    // Still claimed by alice; one attempt; the note names the last line; not verified.
    assert_eq!(
        without_stamps(&read_text(&todo_file)),
        file(&[block(
            "T-001",
            "Build the parser",
            &[
                "status: claimed",
                "owner: alice",
                "claimed: 2026-11-15T10:00+01:00",
                "priority: P1 · difficulty: 2",
                &verify,
                "attempts: 1",
                "note (2026-11-15, alice): added",
                "note (2026-11-15, alice): verify failed — tests failed",
            ],
        )])
    );
}

#[test]
fn after_verify_attempts_failures_done_gives_the_task_back_as_stuck() {
    let tmp = TempDir::new().unwrap();
    let body = claimed_with_verify(&fail_check("tests failed"));
    let (home, work, todo_file) = setup_proj(&tmp, &body);

    // The default `verify_attempts` is 3: the first two failures leave the task claimed.
    for _ in 0..2 {
        let out = todo(&home, &work, &["done", "T-001", "--agent", "alice"]);
        assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr_text(&out));
        assert!(read_text(&todo_file).contains("- status: claimed"));
    }

    let out = todo(
        &home,
        &work,
        &["--json", "done", "T-001", "--agent", "alice"],
    );
    let json = assert_json_error(&out, "VERIFY_FAILED", 1);
    assert!(
        json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("given back to the planner as stuck"),
        "{json}"
    );

    let text = without_stamps(&read_text(&todo_file));
    assert!(text.contains("- status: blocked\n- owner: —\n"), "{text}");
    assert!(text.contains("- attempts: 3\n"), "{text}");
    assert!(
        text.contains("- note (2026-11-15, alice): stuck — verify failed 3 times: tests failed\n"),
        "{text}"
    );
    assert!(!text.contains("- claimed:"), "{text}");
}

#[test]
fn the_count_of_failed_checks_restarts_when_the_task_goes_back_to_work() {
    let tmp = TempDir::new().unwrap();
    let body = claimed_with_verify(&fail_check("tests failed"));
    let (home, work, todo_file) = setup_proj(&tmp, &body);
    write_toml(&work, "verify_attempts = 2\n");

    // Two failures give the task back as stuck.
    for _ in 0..2 {
        let out = todo(&home, &work, &["done", "T-001", "--agent", "alice"]);
        assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr_text(&out));
    }
    assert!(read_text(&todo_file).contains("- status: blocked"));

    // Back to work: `unblock` restarts the count, so the next failure only leaves it claimed.
    let out = todo(
        &home,
        &work,
        &[
            "unblock",
            "T-001",
            "--agent",
            "planner",
            "--note",
            "try again",
        ],
    );
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let out = todo(&home, &work, &["claim", "T-001", "--agent", "alice"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));

    let out = todo(
        &home,
        &work,
        &["--json", "done", "T-001", "--agent", "alice"],
    );
    let json = assert_json_error(&out, "VERIFY_FAILED", 1);
    assert!(
        json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("stays claimed"),
        "{json}"
    );
    let text = read_text(&todo_file);
    assert!(text.contains("- status: claimed"), "{text}");
    assert!(
        text.contains("- attempts: 3"),
        "attempts is a total: {text}"
    );
}

#[test]
fn a_check_that_outlives_verify_timeout_seconds_counts_as_failed() {
    let tmp = TempDir::new().unwrap();
    let body = claimed_with_verify(slow_check());
    let (home, work, todo_file) = setup_proj(&tmp, &body);
    write_toml(&work, "verify_timeout_seconds = 1\n");

    let started = Instant::now();
    let out = todo(
        &home,
        &work,
        &["--json", "done", "T-001", "--agent", "alice"],
    );
    let elapsed = started.elapsed();

    let json = assert_json_error(&out, "VERIFY_FAILED", 1);
    assert_eq!(json["error"]["timed_out"], true, "{json}");
    assert_eq!(
        json["error"]["last_line"], "timed out after 1 seconds",
        "{json}"
    );
    assert!(
        elapsed.as_secs() < 25,
        "the check was stopped, not waited for: {elapsed:?}"
    );
    assert!(
        read_text(&todo_file)
            .contains("- note (2026-11-15, alice): verify failed — timed out after 1 seconds\n"),
        "the timeout is a failed check in the file"
    );
}

#[test]
fn a_check_that_cannot_start_is_verify_not_run_and_writes_nothing() {
    let tmp = TempDir::new().unwrap();
    let body = claimed_with_verify(&fail_check("never run"));
    let (home, work, todo_file) = setup_proj(&tmp, &body);
    write_toml(
        &work,
        "verify_shell = [\"todo-test-no-such-shell\", \"-c\"]\n",
    );
    let before = fs::read(&todo_file).unwrap();

    let out = todo(
        &home,
        &work,
        &["--json", "done", "T-001", "--agent", "alice"],
    );
    assert_json_error(&out, "VERIFY_NOT_RUN", 5);
    assert_eq!(fs::read(&todo_file).unwrap(), before, "nothing is written");
}

#[test]
fn verify_false_finishes_done_without_running_or_recording_a_check() {
    let tmp = TempDir::new().unwrap();
    let verify = format!("verify: {}", fail_check("would fail"));
    let body = claimed_with_verify(&fail_check("would fail"));
    let (home, work, todo_file) = setup_proj(&tmp, &body);
    write_toml(&work, "verify = false\n");

    // The check would fail, so a success shows that nothing ran.
    let out = todo(&home, &work, &["done", "T-001", "--agent", "alice"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));

    assert_eq!(
        without_stamps(&read_text(&todo_file)),
        file(&[block(
            "T-001",
            "Build the parser",
            &[
                "status: done",
                "owner: alice",
                "priority: P1 · difficulty: 2",
                &verify,
                "note (2026-11-15, alice): added",
                "note (2026-11-15, alice): done",
            ],
        )])
    );
}

#[test]
fn verify_when_wave_makes_done_run_nothing_and_leave_verified_out() {
    let tmp = TempDir::new().unwrap();
    let body = claimed_with_verify(&fail_check("checked later"));
    let (home, work, todo_file) = setup_proj(&tmp, &body);
    write_toml(&work, "verify_when = \"wave\"\n");

    let out = todo(&home, &work, &["done", "T-001", "--agent", "alice"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));

    let text = without_stamps(&read_text(&todo_file));
    assert!(text.contains("- status: done\n"), "{text}");
    assert!(!text.contains("verified:"), "{text}");
    assert!(!text.contains("attempts:"), "{text}");
}

#[test]
fn done_of_a_task_without_verify_behaves_as_before() {
    let tmp = TempDir::new().unwrap();
    let body = file(&[block(
        "T-001",
        "Plain task",
        &[
            "status: claimed",
            "owner: alice",
            "claimed: 2026-11-15T10:00+01:00",
            "priority: P1 · difficulty: 2",
            "note (2026-11-15, alice): added",
        ],
    )]);
    let (home, work, todo_file) = setup_proj(&tmp, &body);

    let out = todo(&home, &work, &["done", "T-001", "--agent", "alice"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    assert_eq!(
        without_stamps(&read_text(&todo_file)),
        file(&[block(
            "T-001",
            "Plain task",
            &[
                "status: done",
                "owner: alice",
                "priority: P1 · difficulty: 2",
                "note (2026-11-15, alice): added",
                "note (2026-11-15, alice): done",
            ],
        )])
    );
}

#[test]
fn a_refused_done_is_refused_before_any_check_runs() {
    let tmp = TempDir::new().unwrap();
    let verify = format!("verify: {}", marker_check("ran.txt"));
    let body = file(&[block(
        "T-001",
        "Already finished",
        &[
            "status: done",
            "owner: alice",
            "priority: P1 · difficulty: 2",
            &verify,
            "note (2026-11-15, alice): added",
        ],
    )]);
    let (home, work, _todo_file) = setup_proj(&tmp, &body);

    // Done cannot be done again (R-WRITE-9), so the check must not run.
    let out = todo(&home, &work, &["done", "T-001", "--agent", "alice"]);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr_text(&out));
    assert!(
        !work.join("ran.txt").exists(),
        "the check ran for a refused change"
    );
}

#[test]
fn no_verify_needs_force_and_then_leaves_a_forced_note() {
    let tmp = TempDir::new().unwrap();
    let body = claimed_with_verify(&fail_check("would fail"));
    let (home, work, todo_file) = setup_proj(&tmp, &body);

    let out = todo(
        &home,
        &work,
        &["done", "T-001", "--agent", "alice", "--no-verify"],
    );
    assert_eq!(out.status.code(), Some(2), "stderr: {}", stderr_text(&out));
    assert!(
        stderr_text(&out).contains("needs --force"),
        "{}",
        stderr_text(&out)
    );
    assert_eq!(
        read_text(&todo_file),
        body,
        "a refused done changes nothing"
    );

    let out = todo(
        &home,
        &work,
        &[
            "done",
            "T-001",
            "--agent",
            "alice",
            "--force",
            "--no-verify",
        ],
    );
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let text = without_stamps(&read_text(&todo_file));
    assert!(text.contains("- status: done\n"), "{text}");
    assert!(!text.contains("verified:"), "{text}");
    assert!(
        text.contains("- note (2026-11-15, alice): forced done without verify\n"),
        "{text}"
    );
}

#[test]
fn the_output_of_a_passing_check_never_breaks_json() {
    let tmp = TempDir::new().unwrap();
    let body = claimed_with_verify(&chatty_check());
    let (home, work, _todo_file) = setup_proj(&tmp, &body);

    let out = todo(
        &home,
        &work,
        &["--json", "done", "T-001", "--agent", "alice"],
    );
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let json = parse_json(&out.stdout);
    assert_eq!(json["schema"], "todo.done.v1");
    assert_eq!(json["ok"], true);
}

#[test]
fn a_check_runs_in_the_folder_of_todo_md() {
    let tmp = TempDir::new().unwrap();
    let body = claimed_with_verify(&marker_check("ran.txt"));
    let (home, work, _todo_file) = setup_proj(&tmp, &body);

    let out = todo(&home, &work, &["done", "T-001", "--agent", "alice"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    assert!(
        work.join("ran.txt").exists(),
        "the check ran in the folder of TODO.md"
    );
}

#[test]
fn a_check_in_a_linked_worktree_runs_in_the_matching_folder_of_that_worktree() {
    let tmp = TempDir::new().unwrap();
    let main_repo = tmp.path().join("main-repo");
    let wt = tmp.path().join("wt-feature");
    let home = tmp.path().join("home");
    fs::create_dir_all(&main_repo).unwrap();
    fs::create_dir_all(&home).unwrap();

    git(&main_repo, &["init"]);
    let verify = format!("verify: {}", marker_check("ran.txt"));
    fs::write(
        main_repo.join("TODO.md"),
        claimed_with_verify(&marker_check("ran.txt")),
    )
    .unwrap();
    git(&main_repo, &["add", "."]);
    git(&main_repo, &["commit", "-m", "initial"]);
    git(
        &main_repo,
        &["worktree", "add", "-b", "feature", wt.to_str().unwrap()],
    );
    assert!(read_text(&main_repo.join("TODO.md")).contains(&verify));

    // Run from the worktree: the file is the main checkout's, the check runs in the worktree.
    let out = todo(&home, &wt, &["done", "T-001", "--agent", "alice"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    assert!(
        wt.join("ran.txt").exists(),
        "the check runs in the worktree"
    );
    assert!(
        !main_repo.join("ran.txt").exists(),
        "not in the main checkout"
    );
    assert!(read_text(&main_repo.join("TODO.md")).contains("- status: done\n"));
}

#[test]
fn a_reopened_task_loses_verified_and_set_cannot_write_attempts_or_verified() {
    let tmp = TempDir::new().unwrap();
    let verify = format!("verify: {}", pass_check());
    let body = file(&[block(
        "T-001",
        "Finished",
        &[
            "status: done",
            "owner: alice",
            "priority: P1 · difficulty: 2",
            &verify,
            "verified: 2026-11-01",
            "attempts: 1",
            "note (2026-11-01, alice): added",
        ],
    )]);
    let (home, work, todo_file) = setup_proj(&tmp, &body);

    for field in ["attempts=5", "verified=2026-11-15"] {
        let out = todo(
            &home,
            &work,
            &["set", "T-001", "--field", field, "--agent", "alice"],
        );
        assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr_text(&out));
        assert!(
            stderr_text(&out).contains("cannot change"),
            "{}",
            stderr_text(&out)
        );
    }
    let out = todo(
        &home,
        &work,
        &["set", "T-001", "--unset", "verified", "--agent", "alice"],
    );
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr_text(&out));
    assert_eq!(read_text(&todo_file), body, "a refused set changes nothing");

    let out = todo(
        &home,
        &work,
        &[
            "reopen",
            "T-001",
            "--agent",
            "alice",
            "--note",
            "check again",
        ],
    );
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let text = without_stamps(&read_text(&todo_file));
    assert!(!text.contains("verified:"), "{text}");
    assert!(text.contains("- attempts: 1\n"), "attempts stays: {text}");
}

// ---------------------------------------------------------------------------------------------
// `todo verify <task>`
// ---------------------------------------------------------------------------------------------

#[test]
fn verify_task_runs_the_check_and_changes_nothing() {
    let tmp = TempDir::new().unwrap();
    let body = claimed_with_verify(&marker_check("ran.txt"));
    let (home, work, todo_file) = setup_proj(&tmp, &body);

    let out = todo(&home, &work, &["verify", "T-001", "--agent", "planner"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    assert!(
        stdout_text(&out).contains("T-001: verify passed"),
        "{}",
        stdout_text(&out)
    );
    assert!(work.join("ran.txt").exists());
    assert_eq!(read_text(&todo_file), body, "TODO.md is not changed");

    let out = todo(
        &home,
        &work,
        &["--json", "verify", "T-001", "--agent", "planner"],
    );
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let json = parse_json(&out.stdout);
    assert_eq!(json["schema"], "todo.verify.v1");
    assert_eq!(json["data"]["task"], "T-001");
    assert_eq!(json["data"]["passed"], true);
    assert_eq!(read_text(&todo_file), body);
}

#[test]
fn verify_task_that_fails_exits_1_shows_the_last_lines_and_changes_nothing() {
    let tmp = TempDir::new().unwrap();
    let body = claimed_with_verify(&fail_check("tests failed"));
    let (home, work, todo_file) = setup_proj(&tmp, &body);

    let out = todo(&home, &work, &["verify", "T-001", "--agent", "planner"]);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr_text(&out));
    assert!(
        stderr_text(&out).contains("verify of T-001 failed"),
        "{}",
        stderr_text(&out)
    );
    assert!(
        stderr_text(&out).contains("tests failed"),
        "{}",
        stderr_text(&out)
    );
    assert_eq!(read_text(&todo_file), body);

    let out = todo(
        &home,
        &work,
        &["--json", "verify", "T-001", "--agent", "planner"],
    );
    assert_json_error(&out, "VERIFY_FAILED", 1);
}

#[test]
fn verify_task_without_a_verify_line_has_nothing_to_run() {
    let tmp = TempDir::new().unwrap();
    let body = file(&[block(
        "T-001",
        "No check",
        &["status: open", "priority: P1 · difficulty: 2"],
    )]);
    let (home, work, _todo_file) = setup_proj(&tmp, &body);

    let out = todo(
        &home,
        &work,
        &["--json", "verify", "T-001", "--agent", "planner"],
    );
    assert_json_error(&out, "MISSING_FIELD", 1);
}

#[test]
fn verify_task_with_verify_false_is_verify_not_run() {
    let tmp = TempDir::new().unwrap();
    let body = claimed_with_verify(&fail_check("never"));
    let (home, work, _todo_file) = setup_proj(&tmp, &body);
    write_toml(&work, "verify = false\n");

    let out = todo(
        &home,
        &work,
        &["--json", "verify", "T-001", "--agent", "planner"],
    );
    assert_json_error(&out, "VERIFY_NOT_RUN", 5);
}

// ---------------------------------------------------------------------------------------------
// `todo verify --wave`
// ---------------------------------------------------------------------------------------------

/// A wave in mode `wave`: T-001 done and not yet checked (passes), T-002 done and not yet checked
/// (fails), T-003 already verified, T-004 open, T-005 done without a check.
fn wave_body() -> String {
    let pass = format!("verify: {}", pass_check());
    let fail = format!("verify: {}", fail_check("two failed"));
    file(&[
        block(
            "T-001",
            "Passes",
            &[
                "status: done",
                "owner: alice",
                "priority: P1 · difficulty: 2",
                &pass,
                "note (2026-11-15, alice): added",
            ],
        ),
        block(
            "T-002",
            "Fails",
            &[
                "status: done",
                "owner: alice",
                "priority: P1 · difficulty: 2",
                &fail,
                "note (2026-11-15, alice): added",
            ],
        ),
        block(
            "T-003",
            "Checked before",
            &[
                "status: done",
                "owner: alice",
                "priority: P1 · difficulty: 2",
                &pass,
                "verified: 2026-11-01",
                "note (2026-11-01, alice): added",
            ],
        ),
        block(
            "T-004",
            "Still open",
            &["status: open", "priority: P1 · difficulty: 2", &fail],
        ),
        block(
            "T-005",
            "No check",
            &[
                "status: done",
                "owner: alice",
                "priority: P1 · difficulty: 2",
                "note (2026-11-15, alice): added",
            ],
        ),
    ])
}

#[test]
fn wave_checks_each_done_task_once_and_records_the_results_in_file_order() {
    let tmp = TempDir::new().unwrap();
    let body = wave_body();
    let (home, work, todo_file) = setup_proj(&tmp, &body);
    write_toml(&work, "verify_when = \"wave\"\n");

    let out = todo(&home, &work, &["verify", "--wave", "--agent", "planner"]);
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr_text(&out));
    let shown = stdout_text(&out);
    assert!(shown.contains("passed: T-001"), "{shown}");
    assert!(shown.contains("failed: T-002: two failed"), "{shown}");
    assert!(
        shown.contains("given back (blocked, as stuck): T-002"),
        "{shown}"
    );

    let verify = format!("verify: {}", pass_check());
    let fail = format!("verify: {}", fail_check("two failed"));
    assert_eq!(
        without_stamps(&read_text(&todo_file)),
        file(&[
            block(
                "T-001",
                "Passes",
                &[
                    "status: done",
                    "owner: alice",
                    "priority: P1 · difficulty: 2",
                    &verify,
                    "verified: 2026-11-15",
                    "note (2026-11-15, alice): added",
                ],
            ),
            block(
                "T-002",
                "Fails",
                &[
                    "status: blocked",
                    "owner: —",
                    "priority: P1 · difficulty: 2",
                    &fail,
                    "attempts: 1",
                    "note (2026-11-15, alice): added",
                    "note (2026-11-15, planner): stuck — verify failed after done: two failed",
                ],
            ),
            block(
                "T-003",
                "Checked before",
                &[
                    "status: done",
                    "owner: alice",
                    "priority: P1 · difficulty: 2",
                    &verify,
                    "verified: 2026-11-01",
                    "note (2026-11-01, alice): added",
                ],
            ),
            block(
                "T-004",
                "Still open",
                &["status: open", "priority: P1 · difficulty: 2", &fail]
            ),
            block(
                "T-005",
                "No check",
                &[
                    "status: done",
                    "owner: alice",
                    "priority: P1 · difficulty: 2",
                    "note (2026-11-15, alice): added"
                ],
            ),
        ])
    );
}

#[test]
fn wave_json_lists_passed_failed_and_given_back() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo_file) = setup_proj(&tmp, &wave_body());
    write_toml(&work, "verify_when = \"wave\"\n");

    let out = todo(
        &home,
        &work,
        &["--json", "verify", "--wave", "--agent", "planner"],
    );
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr_text(&out));
    let json = parse_json(&out.stdout);
    assert_eq!(json["schema"], "todo.verify.v1");
    assert_eq!(json["ok"], true);
    assert_eq!(json["data"]["checked"], 2);
    assert_eq!(json["data"]["passed"], serde_json::json!(["T-001"]));
    assert_eq!(json["data"]["failed"][0]["task"], "T-002");
    assert_eq!(json["data"]["failed"][0]["last_line"], "two failed");
    assert_eq!(json["data"]["failed"][0]["exit_code"], 1);
    assert_eq!(json["data"]["given_back"], serde_json::json!(["T-002"]));
}

#[test]
fn wave_exits_0_when_every_check_passes() {
    let tmp = TempDir::new().unwrap();
    let verify = format!("verify: {}", pass_check());
    let body = file(&[block(
        "T-001",
        "Passes",
        &[
            "status: done",
            "owner: alice",
            "priority: P1 · difficulty: 2",
            &verify,
        ],
    )]);
    let (home, work, todo_file) = setup_proj(&tmp, &body);
    write_toml(&work, "verify_when = \"wave\"\n");

    let out = todo(&home, &work, &["verify", "--wave", "--agent", "planner"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    assert!(read_text(&todo_file).contains("- verified: 2026-11-15\n"));
}

#[test]
fn wave_refuses_while_a_task_is_claimed_unless_forced() {
    let tmp = TempDir::new().unwrap();
    let verify = format!("verify: {}", pass_check());
    let body = file(&[
        block(
            "T-001",
            "Passes",
            &[
                "status: done",
                "owner: alice",
                "priority: P1 · difficulty: 2",
                &verify,
            ],
        ),
        block(
            "T-009",
            "Being edited",
            &[
                "status: claimed",
                "owner: bob",
                "claimed: 2026-11-15T10:00+01:00",
                "priority: P1 · difficulty: 2",
            ],
        ),
    ]);
    let (home, work, todo_file) = setup_proj(&tmp, &body);
    write_toml(&work, "verify_when = \"wave\"\n");

    let out = todo(
        &home,
        &work,
        &["--json", "verify", "--wave", "--agent", "planner"],
    );
    let json = assert_json_error(&out, "ALREADY_CLAIMED", 1);
    assert_eq!(json["error"]["candidates"], serde_json::json!(["T-009"]));
    assert_eq!(read_text(&todo_file), body, "nothing is checked or written");

    let out = todo(
        &home,
        &work,
        &["verify", "--wave", "--agent", "planner", "--force"],
    );
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    assert!(read_text(&todo_file).contains("- verified: 2026-11-15\n"));
}

#[test]
fn wave_with_verify_false_is_verify_not_run() {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo_file) = setup_proj(&tmp, &wave_body());
    write_toml(&work, "verify = false\n");
    let before = read_text(&todo_file);

    let out = todo(
        &home,
        &work,
        &["--json", "verify", "--wave", "--agent", "planner"],
    );
    assert_json_error(&out, "VERIFY_NOT_RUN", 5);
    assert_eq!(read_text(&todo_file), before);
}

#[test]
fn wave_with_nothing_to_check_says_so_and_exits_0() {
    let tmp = TempDir::new().unwrap();
    let body = file(&[block(
        "T-001",
        "Open",
        &["status: open", "priority: P1 · difficulty: 2"],
    )]);
    let (home, work, todo_file) = setup_proj(&tmp, &body);

    let out = todo(&home, &work, &["verify", "--wave", "--agent", "planner"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    assert!(
        stdout_text(&out).contains("nothing to check"),
        "{}",
        stdout_text(&out)
    );
    assert_eq!(read_text(&todo_file), body);
}

#[test]
fn verify_needs_a_task_or_wave_and_not_both() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo_file) = setup_proj(&tmp, &wave_body());

    let out = todo(&home, &work, &["verify", "--agent", "planner"]);
    assert_eq!(out.status.code(), Some(2), "stderr: {}", stderr_text(&out));
    let out = todo(
        &home,
        &work,
        &["verify", "T-001", "--wave", "--agent", "planner"],
    );
    assert_eq!(out.status.code(), Some(2), "stderr: {}", stderr_text(&out));
}

// ---------------------------------------------------------------------------------------------
// `check` warns about done tasks whose check has not passed (warnings only)
// ---------------------------------------------------------------------------------------------

#[test]
fn check_warns_about_done_tasks_without_a_passed_check_and_keeps_exit_0() {
    let tmp = TempDir::new().unwrap();
    let verify = format!("verify: {}", pass_check());
    let body = file(&[block(
        "T-001",
        "Finished, not checked",
        &[
            "status: done",
            "owner: alice",
            "priority: P1 · difficulty: 2",
            &verify,
        ],
    )]);
    let (home, work, _todo_file) = setup_proj(&tmp, &body);

    let out = todo(&home, &work, &["--json", "check"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let json = parse_json(&out.stdout);
    let warnings = json["warnings"].as_array().unwrap();
    assert!(
        warnings
            .iter()
            .any(|w| w.as_str().unwrap().contains("T-001")
                && w.as_str()
                    .unwrap()
                    .contains("verify command has not passed")),
        "{json}"
    );

    // With verify = false no check can pass, so nothing is warned about.
    write_toml(&work, "verify = false\n");
    let out = todo(&home, &work, &["--json", "check"]);
    let json = parse_json(&out.stdout);
    assert_eq!(json["warnings"], serde_json::json!([]), "{json}");
}

#[test]
fn wave_check_that_outlives_the_timeout_is_given_back_with_the_timeout_as_its_problem() {
    let tmp = TempDir::new().unwrap();
    let verify = format!("verify: {}", slow_check());
    let body = file(&[block(
        "T-001",
        "Too slow",
        &[
            "status: done",
            "owner: alice",
            "priority: P1 · difficulty: 2",
            &verify,
        ],
    )]);
    let (home, work, todo_file) = setup_proj(&tmp, &body);
    write_toml(
        &work,
        "verify_when = \"wave\"\nverify_timeout_seconds = 1\n",
    );

    let started = Instant::now();
    let out = todo(
        &home,
        &work,
        &["--json", "verify", "--wave", "--agent", "planner"],
    );
    assert!(started.elapsed().as_secs() < 25, "the check was stopped");
    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr_text(&out));
    let json = parse_json(&out.stdout);
    assert_eq!(json["data"]["failed"][0]["task"], "T-001", "{json}");
    assert_eq!(
        json["data"]["given_back"],
        serde_json::json!(["T-001"]),
        "{json}"
    );

    let text = without_stamps(&read_text(&todo_file));
    assert!(text.contains("- status: blocked\n"), "{text}");
    assert!(
        text.contains("- note (2026-11-15, planner): stuck — verify failed after done: timed out after 1 seconds\n"),
        "{text}"
    );
}
