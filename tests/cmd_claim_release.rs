//! CLI integration tests for `todo claim` and `todo release` (T-022; TEST-4, TEST-5).

mod common;

use std::fs;

use tempfile::TempDir;

use common::{parse_json, read_text, setup_proj, stderr_text, stdout_text};
use todo::doc::{Document, Status, TaskId};

#[cfg(feature = "test-hooks")]
#[test]
fn claim_open_refresh_stale_takeover_and_force_cases() {
    let tmp = TempDir::new().unwrap();
    let initial = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
## Now\n\n\
### t-1 - Lenient open task\n\
- status: open\n\
- owner: —\n\
- priority: P1\n\
\n\
### T-002 · Stale without timestamp\n\
- status: claimed\n\
- owner: ghost-bot\n";
    let (home, work, todo_file) = setup_proj(&tmp, initial);

    // Missing agent -> NO_AGENT exit 2 and file untouched.
    let no_agent = common::run_todo(&home, &work, &["--json", "claim", "T-001"], &[]).unwrap();
    assert_eq!(no_agent.status.code(), Some(2));
    assert_eq!(read_text(&todo_file), initial);

    // 1. Claim open task as alice at 14:00+01:00; heading on T-001 stays lenient (not rewritten!).
    let t0 = "2026-11-15T14:00:00+01:00";
    let c1 = common::run_todo(
        &home,
        &work,
        &["claim", "1", "--agent", "alice"],
        &[("TODO_TEST_NOW", t0)],
    )
    .unwrap();
    assert!(c1.status.success(), "stderr: {}", stderr_text(&c1));
    assert_eq!(stdout_text(&c1).trim(), "T-001: claim ok");

    let after1 = read_text(&todo_file);
    assert!(
        after1.contains("### t-1 - Lenient open task\n"),
        "untouched heading changed:\n{after1}"
    );
    assert!(after1.contains(
        "- status: claimed\n- owner: alice\n- claimed: 2026-11-15T14:00+01:00\n- priority: P1\n"
    ));

    // 2. Claim again by alice at 15:30+01:00 refreshes claim time (JSON output returns post-edit task).
    let t1 = "2026-11-15T15:30:00+01:00";
    let c2 = common::run_todo(
        &home,
        &work,
        &["--json", "claim", "T-001", "--agent", "ALICE"],
        &[("TODO_TEST_NOW", t1)],
    )
    .unwrap();
    assert!(c2.status.success());
    let j2: serde_json::Value = parse_json(&c2.stdout);
    assert_eq!(j2["schema"], "todo.claim.v1");
    assert_eq!(j2["data"]["status"], "claimed");
    assert_eq!(j2["data"]["owner"], "alice");
    assert_eq!(j2["data"]["claimed"], "2026-11-15T15:30+01:00");

    // 3. Live claim by another agent (bob) is refused with ALREADY_CLAIMED (exit 1), file byte-identical.
    let before_refusal = fs::read(&todo_file).unwrap();
    let t2 = "2026-11-15T18:00:00+01:00";
    let c3 = common::run_todo(
        &home,
        &work,
        &["--json", "claim", "T-001", "--agent", "bob"],
        &[("TODO_TEST_NOW", t2)],
    )
    .unwrap();
    assert_eq!(c3.status.code(), Some(1));
    let j3: serde_json::Value = parse_json(&c3.stdout);
    assert_eq!(j3["error"]["code"], "ALREADY_CLAIMED");
    assert_eq!(j3["error"]["owner"], "alice");
    assert_eq!(j3["error"]["age_hours"], 2);
    assert_eq!(fs::read(&todo_file).unwrap(), before_refusal);

    // 4. Forced claim by bob succeeds and records forced note.
    let c4 = common::run_todo(
        &home,
        &work,
        &["claim", "T-001", "--agent", "bob", "--force"],
        &[("TODO_TEST_NOW", t2)],
    )
    .unwrap();
    assert!(c4.status.success());
    let after4 = read_text(&todo_file);
    assert!(after4.contains("- owner: bob\n- claimed: 2026-11-15T18:00+01:00\n"));
    assert!(after4.contains("- note (2026-11-15, bob): forced claim (owner was alice)"));

    // 5. Stale claim takeover (25 hours later) by carol succeeds without --force and adds takeover note.
    let t_stale = "2026-11-16T19:30:00+01:00";
    let c5 = common::run_todo(
        &home,
        &work,
        &["claim", "T-001", "--agent", "carol"],
        &[("TODO_TEST_NOW", t_stale)],
    )
    .unwrap();
    assert!(c5.status.success());
    let after5 = read_text(&todo_file);
    assert!(after5.contains("- owner: carol\n- claimed: 2026-11-16T19:30+01:00\n"));
    assert!(after5.contains("- note (2026-11-16, carol): took over from bob (claim was 25 h old)"));

    // 6. Claimed task without valid claim time (T-002) counts as stale and can be taken over.
    let c6 = common::run_todo(
        &home,
        &work,
        &["claim", "T-002", "--agent", "carol"],
        &[("TODO_TEST_NOW", t_stale)],
    )
    .unwrap();
    assert!(c6.status.success());
    let after6 = read_text(&todo_file);
    assert!(after6.contains("took over from ghost-bot (claim had no valid timestamp)"));
}

#[cfg(feature = "test-hooks")]
#[test]
fn claim_and_release_transition_table_and_refusals() {
    let tmp = TempDir::new().unwrap();
    let initial = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
## Tasks\n\n\
### T-001 · Open\n- status: open\n- owner: —\n\n\
### T-002 · Claimed\n- status: claimed\n- owner: alice\n- claimed: 2026-11-15T14:00+01:00\n\n\
### T-003 · Blocked owned\n- status: blocked\n- owner: alice\n- claimed: 2026-11-15T14:00+01:00\n\n\
### T-004 · Done\n- status: done\n- owner: alice\n\n\
### T-005 · Dropped\n- status: dropped\n- owner: —\n";
    let (home, work, todo_file) = setup_proj(&tmp, initial);
    let now = "2026-11-15T15:00:00+01:00";

    // Refused transitions must leave TODO.md byte-identical.
    let refused_cases: &[(&[&str], &str)] = &[
        (
            &["--json", "release", "T-001", "--agent", "alice"],
            "BAD_TRANSITION",
        ),
        (
            &["--json", "claim", "T-003", "--agent", "alice"],
            "BAD_TRANSITION",
        ),
        (
            &["--json", "claim", "T-004", "--agent", "alice", "--force"],
            "BAD_TRANSITION",
        ),
        (
            &["--json", "release", "T-004", "--agent", "alice", "--force"],
            "BAD_TRANSITION",
        ),
        (
            &["--json", "claim", "T-005", "--agent", "alice", "--force"],
            "BAD_TRANSITION",
        ),
        (
            &["--json", "release", "T-005", "--agent", "alice", "--force"],
            "BAD_TRANSITION",
        ),
        (
            &["--json", "release", "T-002", "--agent", "bob"],
            "NOT_OWNER",
        ),
        (
            &["--json", "release", "T-003", "--agent", "bob"],
            "NOT_OWNER",
        ),
        (
            &["--json", "claim", "T-999", "--agent", "alice"],
            "NOT_FOUND",
        ),
    ];

    for (args, expected_code) in refused_cases {
        let out = common::run_todo(&home, &work, args, &[("TODO_TEST_NOW", now)]).unwrap();
        assert_eq!(out.status.code(), Some(1), "args: {args:?}");
        let j: serde_json::Value = parse_json(&out.stdout);
        assert_eq!(
            j["error"]["code"], *expected_code,
            "args: {args:?}, body: {j}"
        );
        assert_eq!(read_text(&todo_file), initial);
    }

    // Release T-002 as owner (alice) -> status: open, owner: —, claimed removed.
    let r2 = common::run_todo(
        &home,
        &work,
        &["release", "T-002", "--agent", "alice"],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert!(r2.status.success());
    assert_eq!(stdout_text(&r2).trim(), "T-002: release ok");

    // Release T-003 (blocked, owned by alice) as bob --force -> stays blocked, owner: —, claimed removed, forced note added.
    let r3 = common::run_todo(
        &home,
        &work,
        &["--json", "release", "T-003", "--agent", "bob", "--force"],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert!(r3.status.success());
    let j3: serde_json::Value = parse_json(&r3.stdout);
    assert_eq!(j3["data"]["status"], "blocked");
    assert!(j3["data"]["owner"].is_null());
    assert!(j3["data"]["claimed"].is_null());

    // Claim blocked T-003 with --force -> becomes claimed.
    let c3 = common::run_todo(
        &home,
        &work,
        &["claim", "T-003", "--agent", "carol", "--force"],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert!(c3.status.success());

    let doc = Document::parse_bytes(&fs::read(&todo_file).unwrap()).unwrap();
    let t2 = doc.task_by_id(TaskId(2)).unwrap();
    assert_eq!(t2.status, Status::Open);
    assert_eq!(t2.owner, None);
    assert_eq!(t2.claimed, None);

    let t3 = doc.task_by_id(TaskId(3)).unwrap();
    assert_eq!(t3.status, Status::Claimed);
    assert_eq!(t3.owner.as_deref(), Some("carol"));
    assert_eq!(t3.notes.len(), 2);
    assert_eq!(t3.notes[0].text, "forced release (owner was alice)");
    assert_eq!(t3.notes[1].text, "forced claim (owner was —)");
}

#[test]
fn claim_refuses_unsafe_files_and_duplicate_archive_ids_without_writing() {
    let tmp = TempDir::new().unwrap();
    let unsafe_body = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
### T-001 · Repeated field\n- status: open\n- priority: P1\n- priority: P2\n\n\
### T-002 · Clean task\n- status: open\n";
    let (home, work, todo_file) = setup_proj(&tmp, unsafe_body);

    // Task with repeated field is refused with UNSAFE_FILE (exit 4).
    let bad = common::run_todo(
        &home,
        &work,
        &["--json", "claim", "T-001", "--agent", "alice"],
        &[],
    )
    .unwrap();
    assert_eq!(bad.status.code(), Some(4));
    let jb: serde_json::Value = parse_json(&bad.stdout);
    assert_eq!(jb["error"]["code"], "UNSAFE_FILE");
    assert_eq!(read_text(&todo_file), unsafe_body);

    // Duplicate ID in TODO-archive.md also refuses writing T-002.
    fs::write(
        work.join("TODO-archive.md"),
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n### T-002 · Archived dup\n- status: done\n",
    )
    .unwrap();
    let dup = common::run_todo(
        &home,
        &work,
        &["--json", "claim", "T-002", "--agent", "alice"],
        &[],
    )
    .unwrap();
    assert_eq!(dup.status.code(), Some(4));
    let jd: serde_json::Value = parse_json(&dup.stdout);
    assert_eq!(jd["error"]["code"], "UNSAFE_FILE");
    assert_eq!(read_text(&todo_file), unsafe_body);
}

#[test]
fn twenty_parallel_claims_of_one_task_have_exactly_one_winner() {
    let tmp = TempDir::new().unwrap();
    let body = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
## Race\n\n\
### T-001 · Contested task\n\
- status: open\n\
- owner: —\n\
- priority: P0\n";
    let (home, work, todo_file) = setup_proj(&tmp, body);

    let invocations: Vec<Vec<String>> = (0..20)
        .map(|i| {
            vec![
                "--json".to_string(),
                "claim".to_string(),
                "T-001".to_string(),
                "--agent".to_string(),
                format!("racer-{i:02}"),
            ]
        })
        .collect();

    let outputs = common::run_parallel_todos(&home, &work, &invocations).unwrap();
    let mut winners = Vec::new();
    let mut losers = 0usize;

    for (i, out) in outputs.iter().enumerate() {
        let j: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
            panic!(
                "invalid json for racer-{i:02} (status {:?}): {e}\nstdout: {}\nstderr: {}",
                out.status.code(),
                stdout_text(out),
                stderr_text(out)
            )
        });
        if out.status.success() {
            assert_eq!(j["ok"], true);
            winners.push(j["data"]["owner"].as_str().unwrap().to_string());
        } else {
            assert_eq!(
                out.status.code(),
                Some(1),
                "unexpected exit for racer-{i:02}: {j}"
            );
            assert_eq!(j["error"]["code"], "ALREADY_CLAIMED");
            losers += 1;
        }
    }

    assert_eq!(winners.len(), 1, "expected 1 winner, got {winners:?}");
    assert_eq!(losers, 19);

    let doc = Document::parse_bytes(&fs::read(&todo_file).unwrap()).unwrap();
    let t1 = doc.task_by_id(TaskId(1)).unwrap();
    assert_eq!(t1.status, Status::Claimed);
    assert_eq!(t1.owner.as_deref(), Some(winners[0].as_str()));
}
