//! CLI integration tests for `todo block`, `todo unblock`, and `todo drop` (T-025; R-WRITE-5, R-WRITE-6, TEST-4).

mod common;

use std::fs;

use tempfile::TempDir;

use common::{parse_json, read_text, setup_proj};

#[test]
fn block_unblock_and_drop_require_reason_and_follow_transition_table() {
    let tmp = TempDir::new().unwrap();
    let initial = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
## Now\n\n\
### T-001 · Open task\n\
- status: open\n\
- owner: —\n\
\n\
### T-002 · Claimed by alice\n\
- status: claimed\n\
- owner: alice\n\
- claimed: 2026-11-15T10:00+01:00\n\
\n\
### T-003 · Open to drop\n\
- status: open\n\
- owner: —\n\
\n\
### T-004 · Done task\n\
- status: done\n\
- owner: alice\n\
\n\
### T-005 · Dropped task\n\
- status: dropped\n\
- owner: —\n";
    let (home, work, todo_file) = setup_proj(&tmp, initial);
    let t1 = "2026-11-15T12:00:00+01:00";
    let t2 = "2026-11-15T15:45:00+01:00";

    // Missing --note on block and drop -> exit 2 (Usage), file untouched.
    let before = fs::read(&todo_file).unwrap();
    let missing_block_note =
        common::run_todo(&home, &work, &["block", "T-001", "--agent", "bob"], &[]).unwrap();
    assert_eq!(missing_block_note.status.code(), Some(2));
    let empty_drop_note = common::run_todo(
        &home,
        &work,
        &["drop", "T-001", "--agent", "bob", "--note", "   "],
        &[],
    )
    .unwrap();
    assert_eq!(empty_drop_note.status.code(), Some(2));
    assert_eq!(fs::read(&todo_file).unwrap(), before);

    // Block open task (T-001) -> blocked (no owner) + note; then unblock -> open.
    let b1 = common::run_todo(
        &home,
        &work,
        &["block", "T-001", "--agent", "bob", "--note", "waiting"],
        &[("TODO_TEST_NOW", t1)],
    )
    .unwrap();
    assert!(b1.status.success());
    let after_b1 = read_text(&todo_file);
    assert!(after_b1.contains(
        "### T-001 · Open task\n\
- status: blocked\n\
- owner: —\n\
- note (2026-11-15, bob): blocked \u{2014} waiting\n"
    ));

    // Block an already blocked task -> BAD_TRANSITION.
    let before_reblock = fs::read(&todo_file).unwrap();
    let reblock = common::run_todo(
        &home,
        &work,
        &["--json", "block", "T-001", "--agent", "bob", "--note", "x"],
        &[("TODO_TEST_NOW", t1)],
    )
    .unwrap();
    assert_eq!(reblock.status.code(), Some(1));
    let j_rb: serde_json::Value = parse_json(&reblock.stdout);
    assert_eq!(j_rb["error"]["code"], "BAD_TRANSITION");
    assert_eq!(fs::read(&todo_file).unwrap(), before_reblock);

    // Unblock unowned T-001 -> open.
    let u1 = common::run_todo(
        &home,
        &work,
        &["unblock", "T-001", "--agent", "bob"],
        &[("TODO_TEST_NOW", t2)],
    )
    .unwrap();
    assert!(u1.status.success());
    let after_u1 = read_text(&todo_file);
    assert!(after_u1.contains(
        "### T-001 · Open task\n\
- status: open\n\
- owner: —\n\
- note (2026-11-15, bob): blocked \u{2014} waiting\n\
- note (2026-11-15, bob): unblocked\n"
    ));

    // Unblock open task -> BAD_TRANSITION.
    let before_u_open = fs::read(&todo_file).unwrap();
    let u_open = common::run_todo(
        &home,
        &work,
        &["--json", "unblock", "T-001", "--agent", "bob"],
        &[("TODO_TEST_NOW", t2)],
    )
    .unwrap();
    assert_eq!(u_open.status.code(), Some(1));
    assert_eq!(fs::read(&todo_file).unwrap(), before_u_open);

    // Claimed by alice (T-002): bob cannot block without --force (NOT_OWNER).
    let block_other = common::run_todo(
        &home,
        &work,
        &[
            "--json", "block", "T-002", "--agent", "bob", "--note", "halt",
        ],
        &[("TODO_TEST_NOW", t1)],
    )
    .unwrap();
    assert_eq!(block_other.status.code(), Some(1));
    let j_bo: serde_json::Value = parse_json(&block_other.stdout);
    assert_eq!(j_bo["error"]["code"], "NOT_OWNER");

    // Alice blocks T-002 (keeps owner & claimed), then unblocks -> claimed with refreshed time!
    let b2 = common::run_todo(
        &home,
        &work,
        &["block", "T-002", "--agent", "alice", "--note", "need info"],
        &[("TODO_TEST_NOW", t1)],
    )
    .unwrap();
    assert!(b2.status.success());
    let u2 = common::run_todo(
        &home,
        &work,
        &[
            "--json", "unblock", "T-002", "--agent", "alice", "--note", "got info",
        ],
        &[("TODO_TEST_NOW", t2)],
    )
    .unwrap();
    assert!(u2.status.success());
    let j_u2: serde_json::Value = parse_json(&u2.stdout);
    assert_eq!(j_u2["schema"], "todo.unblock.v1");
    assert_eq!(j_u2["data"]["status"], "claimed");
    assert_eq!(j_u2["data"]["owner"], "alice");
    assert_eq!(j_u2["data"]["claimed"], "2026-11-15T15:45+01:00");

    // Unblock claimed task -> BAD_TRANSITION.
    let u_claimed = common::run_todo(
        &home,
        &work,
        &["--json", "unblock", "T-002", "--agent", "alice"],
        &[("TODO_TEST_NOW", t2)],
    )
    .unwrap();
    assert_eq!(u_claimed.status.code(), Some(1));

    // Drop claimed T-002 by bob without --force -> NOT_OWNER; with --force -> dropped, claimed removed.
    let drop_no = common::run_todo(
        &home,
        &work,
        &[
            "--json", "drop", "T-002", "--agent", "bob", "--note", "obsolete",
        ],
        &[("TODO_TEST_NOW", t2)],
    )
    .unwrap();
    assert_eq!(drop_no.status.code(), Some(1));

    let drop_force = common::run_todo(
        &home,
        &work,
        &[
            "drop", "T-002", "--agent", "bob", "--force", "--note", "obsolete",
        ],
        &[("TODO_TEST_NOW", t2)],
    )
    .unwrap();
    assert!(drop_force.status.success());
    let after_drop2 = read_text(&todo_file);
    assert!(after_drop2.contains("- status: dropped\n- owner: alice\n- note (2026-11-15, alice):"));
    assert!(!after_drop2.contains("2026-11-15T15:45+01:00"));
    assert!(after_drop2.contains("- note (2026-11-15, bob): forced drop (owner was alice)\n"));
    assert!(after_drop2.contains("- note (2026-11-15, bob): dropped \u{2014} obsolete\n"));

    // Drop open T-003 -> dropped.
    let drop_open = common::run_todo(
        &home,
        &work,
        &["drop", "T-003", "--agent", "bob", "--note", "wontfix"],
        &[("TODO_TEST_NOW", t2)],
    )
    .unwrap();
    assert!(drop_open.status.success());

    // Block, unblock, drop on done (T-004) and dropped (T-005) -> BAD_TRANSITION, byte-identical.
    let before_terminal = fs::read(&todo_file).unwrap();
    for id in ["T-004", "T-005"] {
        for args in [
            vec!["--json", "block", id, "--agent", "alice", "--note", "x"],
            vec!["--json", "unblock", id, "--agent", "alice"],
            vec!["--json", "drop", id, "--agent", "alice", "--note", "x"],
        ] {
            let res = common::run_todo(&home, &work, &args, &[("TODO_TEST_NOW", t2)]).unwrap();
            assert_eq!(res.status.code(), Some(1), "args: {args:?}");
            let j: serde_json::Value = parse_json(&res.stdout);
            assert_eq!(j["error"]["code"], "BAD_TRANSITION");
        }
    }
    assert_eq!(fs::read(&todo_file).unwrap(), before_terminal);
}

#[test]
fn drop_lists_tasks_that_became_ready() {
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
         ### T-002 · Waiting on T-001\n\
         - status: open\n\
         - priority: P1 · difficulty: 2\n\
         - depends: T-001\n",
    )
    .unwrap();

    let now = "2026-11-15T10:00:00+01:00";
    let res = common::run_todo(
        &home,
        &work,
        &["drop", "T-001", "--agent", "alice", "--note", "unneeded"],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert!(res.status.success());
    let stdout = String::from_utf8(res.stdout).unwrap();
    assert!(
        stdout.contains("now ready: T-002 Waiting on T-001"),
        "{stdout}"
    );
}

#[cfg(feature = "test-hooks")]
#[test]
fn drop_takes_a_blocked_task_without_owner() {
    let tmp = TempDir::new().unwrap();
    let initial = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
## Now\n\n\
### T-001 · Blocked task\n\
- status: blocked\n\
- owner: —\n";
    let (home, work, todo_file) = setup_proj(&tmp, initial);

    // R-WRITE-9: blocked -> dropped is allowed for any agent when the task has no owner.
    let res = common::run_todo(
        &home,
        &work,
        &["drop", "T-001", "--agent", "alice", "--note", "not needed"],
        &[("TODO_TEST_NOW", "2026-11-15T12:00:00+01:00")],
    )
    .unwrap();
    assert!(
        res.status.success(),
        "stderr: {}",
        common::stderr_text(&res)
    );
    let after = read_text(&todo_file);
    assert!(
        after.contains(
            "### T-001 · Blocked task\n\
- status: dropped\n\
- owner: —\n\
- note (2026-11-15, alice): dropped \u{2014} not needed\n"
        ),
        "{after}"
    );
    assert!(!after.contains("- status: blocked"), "{after}");
}
