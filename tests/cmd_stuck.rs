//! CLI integration tests for `todo stuck`, which gives a claimed task back to the planner with the
//! problem (T-159; R-STUCK-1, R-WRITE-9, R-CLAIM-3, TEST-4).

mod common;

use std::path::Path;
use std::process::Output;

use common::{parse_json, read_text, setup_proj, stderr_text};
use tempfile::TempDir;

/// The clock of every command in these tests (the `TODO_TEST_NOW` hook), so note dates are fixed.
const NOW: &str = "2026-11-15T12:00:00+01:00";

/// Runs `todo` in `work` at the test clock.
fn todo(home: &Path, work: &Path, args: &[&str]) -> Output {
    common::run_todo(home, work, args, &[("TODO_TEST_NOW", NOW)]).unwrap()
}

/// The file text without the `todo-format` and `todo-tool` stamp lines that a write adds at the
/// top, so the expected text does not depend on the tool version.
fn without_stamps(text: &str) -> String {
    text.split_inclusive('\n')
        .filter(|line| {
            !line.starts_with("<!-- todo-format:") && !line.starts_with("<!-- todo-tool:")
        })
        .collect()
}

/// Two tasks: T-001 claimed by bob with a session and a level, T-002 open.
const CLAIMED_BY_BOB: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
## Now\n\n\
### T-001 · Build the parser\n\
- status: claimed\n\
- owner: bob\n\
- claimed: 2026-11-15T10:00+01:00\n\
- session: sess-bob\n\
- priority: P1 · difficulty: 4 · type: feature · area: core\n\
- note (2026-11-15, bob): added\n\
\n\
### T-002 · Write the docs\n\
- status: open\n\
- owner: —\n\
- priority: P2 · difficulty: 2\n\
- note (2026-11-15, bob): added\n";

#[test]
fn stuck_gives_a_claimed_task_back_in_one_write_and_keeps_its_level() {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo_file) = setup_proj(&tmp, CLAIMED_BY_BOB);

    let out = todo(
        &home,
        &work,
        &[
            "stuck",
            "T-001",
            "--agent",
            "bob",
            "--note",
            "the schema file does not exist",
        ],
    );
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));

    // Blocked with no owner and no claim or session; one attempt; the level and T-002 unchanged.
    assert_eq!(
        without_stamps(&read_text(&todo_file)),
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
## Now\n\n\
### T-001 · Build the parser\n\
- status: blocked\n\
- owner: —\n\
- priority: P1 · difficulty: 4 · type: feature · area: core\n\
- attempts: 1\n\
- note (2026-11-15, bob): added\n\
- note (2026-11-15, bob): stuck — the schema file does not exist\n\
\n\
### T-002 · Write the docs\n\
- status: open\n\
- owner: —\n\
- priority: P2 · difficulty: 2\n\
- note (2026-11-15, bob): added\n"
    );
}

#[test]
fn stuck_by_another_agent_needs_force_and_the_forced_change_says_so() {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo_file) = setup_proj(&tmp, CLAIMED_BY_BOB);
    let before = std::fs::read(&todo_file).unwrap();

    // Alice does not own the claim: NOT_OWNER (exit 1), file byte-identical.
    let refused = common::run_todo(
        &home,
        &work,
        &[
            "--json", "stuck", "T-001", "--agent", "alice", "--note", "halt",
        ],
        &[("TODO_TEST_NOW", NOW)],
    )
    .unwrap();
    assert_eq!(refused.status.code(), Some(1));
    let j: serde_json::Value = parse_json(&refused.stdout);
    assert_eq!(j["error"]["code"], "NOT_OWNER");
    assert_eq!(std::fs::read(&todo_file).unwrap(), before);

    // With --force the change goes through and the forced note comes first.
    let forced = todo(
        &home,
        &work,
        &[
            "stuck",
            "T-001",
            "--agent",
            "alice",
            "--force",
            "--note",
            "the worker stopped answering",
        ],
    );
    assert!(forced.status.success(), "stderr: {}", stderr_text(&forced));
    let text = read_text(&todo_file);
    assert!(text.contains(
        "- status: blocked\n\
- owner: —\n\
- priority: P1 · difficulty: 4 · type: feature · area: core\n\
- attempts: 1\n\
- note (2026-11-15, bob): added\n\
- note (2026-11-15, alice): forced stuck (owner was bob)\n\
- note (2026-11-15, alice): stuck \u{2014} the worker stopped answering\n"
    ));
}

#[test]
fn stuck_refuses_every_other_status_and_leaves_the_file_alone() {
    let body = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
### T-001 · Open task\n\
- status: open\n\
- owner: —\n\
\n\
### T-002 · Blocked task\n\
- status: blocked\n\
- owner: —\n\
- note (2026-11-15, bob): blocked \u{2014} waiting for the review\n\
\n\
### T-003 · Done task\n\
- status: done\n\
- owner: bob\n\
\n\
### T-004 · Dropped task\n\
- status: dropped\n\
- owner: —\n\
\n\
### T-005 · Milestone\n\
- status: open\n\
- owner: —\n\
- type: milestone\n\
- goal: The second version works\n";
    let tmp = TempDir::new().unwrap();
    let (home, work, todo_file) = setup_proj(&tmp, body);
    let before = std::fs::read(&todo_file).unwrap();

    // R-WRITE-9: only a claimed task can be given back (BAD_TRANSITION, exit 1).
    for task in ["T-001", "T-002", "T-003", "T-004", "T-005"] {
        let out = common::run_todo(
            &home,
            &work,
            &[
                "--json", "stuck", task, "--agent", "bob", "--note", "problem",
            ],
            &[("TODO_TEST_NOW", NOW)],
        )
        .unwrap();
        assert_eq!(out.status.code(), Some(1), "stuck on {task}");
        let j: serde_json::Value = parse_json(&out.stdout);
        assert_eq!(j["error"]["code"], "BAD_TRANSITION", "stuck on {task}");
        assert_eq!(
            std::fs::read(&todo_file).unwrap(),
            before,
            "stuck on {task}"
        );
    }
}

#[test]
fn stuck_needs_a_problem_and_a_whole_number_of_attempts() {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo_file) = setup_proj(&tmp, CLAIMED_BY_BOB);
    let before = std::fs::read(&todo_file).unwrap();

    // Missing --note and an empty one: exit 2 (USAGE), file untouched.
    let missing = common::run_todo(
        &home,
        &work,
        &["stuck", "T-001", "--agent", "bob"],
        &[("TODO_TEST_NOW", NOW)],
    )
    .unwrap();
    assert_eq!(missing.status.code(), Some(2));
    let empty = common::run_todo(
        &home,
        &work,
        &["stuck", "T-001", "--agent", "bob", "--note", "   "],
        &[("TODO_TEST_NOW", NOW)],
    )
    .unwrap();
    assert_eq!(empty.status.code(), Some(2));
    assert_eq!(std::fs::read(&todo_file).unwrap(), before);

    // An attempts value that is not a whole number is refused, not reset: BAD_VALUE, exit 1.
    let text = CLAIMED_BY_BOB.replace(
        "- session: sess-bob\n",
        "- session: sess-bob\n- attempts: many\n",
    );
    let tmp2 = TempDir::new().unwrap();
    let (home2, work2, todo_file2) = setup_proj(&tmp2, &text);
    let before2 = std::fs::read(&todo_file2).unwrap();
    let bad = common::run_todo(
        &home2,
        &work2,
        &[
            "--json", "stuck", "T-001", "--agent", "bob", "--note", "problem",
        ],
        &[("TODO_TEST_NOW", NOW)],
    )
    .unwrap();
    assert_eq!(bad.status.code(), Some(1));
    let j: serde_json::Value = parse_json(&bad.stdout);
    assert_eq!(j["error"]["code"], "BAD_VALUE");
    assert_eq!(std::fs::read(&todo_file2).unwrap(), before2);
}

#[test]
fn each_failure_raises_attempts_and_a_task_given_back_waits_for_the_planner() {
    let tmp = TempDir::new().unwrap();
    let body = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
### T-001 · Build the parser\n\
- status: open\n\
- owner: —\n\
- priority: P1 · difficulty: 4\n";
    let (home, work, todo_file) = setup_proj(&tmp, body);

    // First attempt: claim, then give it back.
    assert!(
        todo(&home, &work, &["claim", "T-001", "--agent", "bob"])
            .status
            .success()
    );
    assert!(
        todo(
            &home,
            &work,
            &["stuck", "T-001", "--agent", "bob", "--note", "no schema"]
        )
        .status
        .success()
    );

    // `next` never hands out a blocked task, so the worker gets nothing.
    let next = todo(&home, &work, &["--json", "next", "--agent", "carol"]);
    assert!(next.status.success(), "stderr: {}", stderr_text(&next));
    let j_next: serde_json::Value = parse_json(&next.stdout);
    assert!(j_next["data"]["task"].is_null(), "{j_next}");

    // The planner unblocks it (no owner, so it is open again); bob takes it once more and fails.
    assert!(
        todo(
            &home,
            &work,
            &[
                "unblock",
                "T-001",
                "--agent",
                "planner",
                "--note",
                "schema added"
            ]
        )
        .status
        .success()
    );
    assert!(
        todo(&home, &work, &["claim", "T-001", "--agent", "bob"])
            .status
            .success()
    );
    assert!(
        todo(
            &home,
            &work,
            &[
                "stuck",
                "T-001",
                "--agent",
                "bob",
                "--note",
                "still no schema"
            ]
        )
        .status
        .success()
    );

    assert_eq!(
        without_stamps(&read_text(&todo_file)),
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
### T-001 · Build the parser\n\
- status: blocked\n\
- owner: —\n\
- priority: P1 · difficulty: 4\n\
- attempts: 2\n\
- note (2026-11-15, bob): stuck \u{2014} no schema\n\
- note (2026-11-15, planner): unblocked \u{2014} schema added\n\
- note (2026-11-15, bob): stuck \u{2014} still no schema\n"
    );
}

#[test]
fn summary_counts_tasks_given_back_with_their_problem() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo_file) = setup_proj(&tmp, CLAIMED_BY_BOB);
    assert!(
        todo(
            &home,
            &work,
            &[
                "stuck",
                "T-001",
                "--agent",
                "bob",
                "--note",
                "the schema file does not exist",
            ],
        )
        .status
        .success()
    );
    // A task blocked for another reason is blocked, but not given back.
    assert!(
        todo(
            &home,
            &work,
            &[
                "block",
                "T-002",
                "--agent",
                "alice",
                "--note",
                "waiting for the review"
            ],
        )
        .status
        .success()
    );

    let text = todo(&home, &work, &["summary"]);
    assert!(text.status.success(), "stderr: {}", stderr_text(&text));
    let stdout = String::from_utf8(text.stdout).unwrap();
    assert!(
        stdout.contains(
            "given back to the planner (1):\n  T-001 \u{00b7} Build the parser \u{2014} the schema file does not exist\n"
        ),
        "{stdout}"
    );
    assert!(stdout.contains("blocked (2):"), "{stdout}");

    let json = todo(&home, &work, &["--json", "summary"]);
    let j: serde_json::Value = parse_json(&json.stdout);
    assert_eq!(j["data"]["counts"]["blocked"], 2, "{j}");
    let given_back = j["data"]["given_back"].as_array().unwrap();
    assert_eq!(given_back.len(), 1, "{j}");
    assert_eq!(given_back[0]["id"], "T-001");
    assert_eq!(given_back[0]["reason"], "the schema file does not exist");
}

#[test]
fn check_accepts_a_task_given_back_and_the_json_shows_it_blocked_without_owner() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo_file) = setup_proj(&tmp, CLAIMED_BY_BOB);

    let out = todo(
        &home,
        &work,
        &[
            "--json",
            "stuck",
            "T-001",
            "--agent",
            "bob",
            "--note",
            "no schema",
        ],
    );
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let j: serde_json::Value = parse_json(&out.stdout);
    assert_eq!(j["schema"], "todo.stuck.v1");
    assert_eq!(j["data"]["status"], "blocked");
    assert!(j["data"]["owner"].is_null(), "{j}");
    assert!(j["data"]["claimed"].is_null(), "{j}");

    // The stuck note is the block reason, so `check` has nothing to say about the task.
    let check = todo(&home, &work, &["check"]);
    assert!(
        check.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&check.stdout)
    );
    assert!(
        !String::from_utf8_lossy(&check.stdout).contains("blocked task has no reason note"),
        "check flagged a given-back task"
    );
}
