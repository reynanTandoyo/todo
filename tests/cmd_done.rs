//! CLI integration tests for `todo done` (T-024; R-WRITE-4, NFR-5, TEST-4).

mod common;

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use common::{git, parse_json, read_text, stderr_text, stdout_text};

use common::setup_proj;

fn init_git_repo_with_commit(dir: &Path) -> String {
    git(dir, &["init"]);
    git(dir, &["config", "user.email", "test@example.com"]);
    git(dir, &["config", "user.name", "Test User"]);
    git(dir, &["add", "TODO.md"]);
    git(dir, &["commit", "-m", "initial"]);
    git(dir, &["rev-parse", "--short", "HEAD"])
}

#[test]
fn done_transitions_ownership_notes_and_accept_reminder() {
    let tmp = TempDir::new().unwrap();
    let initial = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
## Now\n\n\
### T-001 · Open task\n\
- status: open\n\
- owner: —\n\
- priority: P1\n\
- accept: all unit tests pass\n\
\n\
### T-002 · Claimed by alice\n\
- status: claimed\n\
- owner: alice\n\
- claimed: 2026-11-15T10:00+01:00\n\
- priority: P0\n\
- accept: zero warnings\n\
\n\
### T-003 · Blocked task\n\
- status: blocked\n\
- owner: alice\n\
- claimed: 2026-11-15T10:00+01:00\n\
\n\
### T-004 · Already done\n\
- status: done\n\
- owner: alice\n";
    let (home, work, todo_file) = setup_proj(&tmp, initial);
    let now = "2026-11-15T14:30:00+01:00";

    // 1. Open task -> done with --note; prints accept line.
    let out1 = common::run_todo(
        &home,
        &work,
        &["done", "T-001", "--agent", "bob", "--note", "implemented"],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert!(out1.status.success(), "stderr: {}", stderr_text(&out1));
    let stdout1 = stdout_text(&out1);
    assert!(stdout1.contains("T-001: done ok"), "stdout: {stdout1}");
    assert!(
        stdout1.contains("accept: all unit tests pass"),
        "stdout: {stdout1}"
    );
    let after1 = read_text(&todo_file);
    assert!(after1.contains(
        "### T-001 · Open task\n\
- status: done\n\
- owner: bob\n\
- priority: P1\n\
- accept: all unit tests pass\n\
- note (2026-11-15, bob): done \u{2014} implemented\n"
    ));

    // 2. Claimed by alice: bob without --force is refused (NOT_OWNER, exit 1), file byte-identical.
    let before_refusal = fs::read(&todo_file).unwrap();
    let out_not_owner = common::run_todo(
        &home,
        &work,
        &["--json", "done", "T-002", "--agent", "bob"],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert_eq!(out_not_owner.status.code(), Some(1));
    let j_no: serde_json::Value = parse_json(&out_not_owner.stdout);
    assert_eq!(j_no["error"]["code"], "NOT_OWNER");
    assert_eq!(fs::read(&todo_file).unwrap(), before_refusal);

    // 3. Claimed by alice: alice completes it without --note -> `done`, claimed removed.
    let out2 = common::run_todo(
        &home,
        &work,
        &["done", "2", "--agent", "alice"],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert!(out2.status.success());
    let stdout2 = stdout_text(&out2);
    assert!(stdout2.contains("accept: zero warnings"));
    let after2 = read_text(&todo_file);
    assert!(after2.contains(
        "### T-002 · Claimed by alice\n\
- status: done\n\
- owner: alice\n\
- priority: P0\n\
- accept: zero warnings\n\
- note (2026-11-15, alice): done\n"
    ));
    assert!(!after2.contains("2026-11-15T10:00+01:00\n- priority: P0"));

    // 4. Blocked task: refused without --force (BAD_TRANSITION), allowed with --force + forced note.
    let before_blocked = fs::read(&todo_file).unwrap();
    let out_blocked = common::run_todo(
        &home,
        &work,
        &["--json", "done", "T-003", "--agent", "alice"],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert_eq!(out_blocked.status.code(), Some(1));
    let j_bl: serde_json::Value = parse_json(&out_blocked.stdout);
    assert_eq!(j_bl["error"]["code"], "BAD_TRANSITION");
    assert_eq!(fs::read(&todo_file).unwrap(), before_blocked);

    let out_force = common::run_todo(
        &home,
        &work,
        &["--json", "done", "T-003", "--agent", "bob", "--force"],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert!(out_force.status.success());
    let j_force: serde_json::Value = parse_json(&out_force.stdout);
    assert_eq!(j_force["schema"], "todo.done.v1");
    assert_eq!(j_force["data"]["status"], "done");
    assert_eq!(j_force["data"]["owner"], "bob");
    let after3 = read_text(&todo_file);
    assert!(after3.contains("- note (2026-11-15, bob): forced done (owner was alice)\n"));
    assert!(after3.contains("- note (2026-11-15, bob): done\n"));

    // 5. Already done -> BAD_TRANSITION even with --force, file byte-identical.
    let before_done = fs::read(&todo_file).unwrap();
    let out_done = common::run_todo(
        &home,
        &work,
        &["--json", "done", "T-004", "--agent", "alice", "--force"],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert_eq!(out_done.status.code(), Some(1));
    let j_done: serde_json::Value = parse_json(&out_done.stdout);
    assert_eq!(j_done["error"]["code"], "BAD_TRANSITION");
    assert_eq!(fs::read(&todo_file).unwrap(), before_done);
}

#[test]
fn done_commit_resolves_head_rejects_unknown_and_accepts_when_git_missing() {
    let tmp = TempDir::new().unwrap();
    let initial = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
## Now\n\n\
### T-001 · First task\n\
- status: open\n\
- owner: —\n\
\n\
### T-002 · Second task\n\
- status: open\n\
- owner: —\n";
    let (home, work, todo_file) = setup_proj(&tmp, initial);
    let head_short = init_git_repo_with_commit(&work);
    let now = "2026-11-15T14:30:00+01:00";

    // Unknown commit is refused when git is available, leaving file byte-identical.
    let before_bad = fs::read(&todo_file).unwrap();
    let bad_commit = common::run_todo(
        &home,
        &work,
        &[
            "--json",
            "done",
            "T-001",
            "--agent",
            "alice",
            "--commit",
            "deadbeef9999",
        ],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert_eq!(bad_commit.status.code(), Some(1));
    let j_bad: serde_json::Value = parse_json(&bad_commit.stdout);
    assert_eq!(j_bad["error"]["code"], "BAD_VALUE");
    assert_eq!(fs::read(&todo_file).unwrap(), before_bad);

    // HEAD is resolved to the current commit hash.
    let good_head = common::run_todo(
        &home,
        &work,
        &[
            "done", "T-001", "--agent", "alice", "--commit", "HEAD", "--note", "shipped",
        ],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert!(
        good_head.status.success(),
        "stderr: {}",
        stderr_text(&good_head)
    );
    let after1 = read_text(&todo_file);
    assert!(
        after1.contains(&format!("- commit: {head_short}\n")),
        "missing resolved commit {head_short}:\n{after1}"
    );

    // When git is not on PATH, an unchecked commit hash is accepted.
    let without_git = common::run_todo(
        &home,
        &work,
        &["done", "T-002", "--agent", "alice", "--commit", "cafebabe"],
        &[("PATH", ""), ("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert!(
        without_git.status.success(),
        "stderr: {}",
        stderr_text(&without_git)
    );
    let after2 = read_text(&todo_file);
    assert!(after2.contains("- commit: cafebabe\n"));
}

#[test]
fn done_lists_tasks_that_became_ready() {
    let dir = TempDir::new().unwrap();
    let home = dir.path().join("home");
    let work = dir.path().join("work");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&work).unwrap();
    let todo_file = work.join("TODO.md");
    fs::write(
        &todo_file,
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
         <!-- todo-format: 1 -->\n\n\
         ## Tasks\n\n\
         ### T-001 · First\n\
         - status: open\n\
         - priority: P1\n\n\
         ### T-002 · Second\n\
         - status: open\n\
         - priority: P1\n\n\
         ### T-003 · Needs T-001 only\n\
         - status: open\n\
         - priority: P1\n\
         - depends: T-001\n\n\
         ### T-004 · Needs T-001 and T-002\n\
         - status: open\n\
         - priority: P1\n\
         - depends: T-001, T-002\n",
    )
    .unwrap();

    let now = "2026-11-15T10:00:00+01:00";
    let res = common::run_todo(
        &home,
        &work,
        &["done", "T-001", "--agent", "alice"],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert!(res.status.success());
    let stdout = String::from_utf8(res.stdout).unwrap();
    assert!(
        stdout.contains("now ready: T-003 Needs T-001 only"),
        "{stdout}"
    );
    assert!(!stdout.contains("T-004"), "{stdout}");

    let res2 = common::run_todo(
        &home,
        &work,
        &["--json", "done", "T-002", "--agent", "alice"],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert!(res2.status.success());
    let json: serde_json::Value = parse_json(&res2.stdout);
    let ready = json["data"]["newly_ready"].as_array().unwrap();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0]["id"], "T-004");
}
