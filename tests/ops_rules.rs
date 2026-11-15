//! Unit-style integration tests for `todo::ops` rules, transitions, and canonical edits (T-022; R-WRITE-9, R-CLAIM-3, R-IO-6).

use chrono::DateTime;
use todo::doc::{Document, Status, TaskId};
use todo::error::{ErrorCode, ExitCode};
use todo::ops::{
    OpsConfig, OwnershipDecision, StatusCommand, TaskCommand, TaskEditBuilder, check_ownership,
    plan_claim, plan_release, validate_transition,
};

#[test]
fn transition_table_covers_all_statuses_and_commands() {
    let fixture = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
### T-001 · Open task\n- status: open\n- owner: \u{2014}\n\
### T-002 · Claimed task\n- status: claimed\n- owner: alice\n- claimed: 2026-11-15T10:00+01:00\n\
### T-003 · Blocked unowned\n- status: blocked\n- owner: \u{2014}\n\
### T-004 · Blocked owned\n- status: blocked\n- owner: alice\n- claimed: 2026-11-15T10:00+01:00\n\
### T-005 · Done task\n- status: done\n- owner: alice\n\
### T-006 · Dropped task\n- status: dropped\n"
        .as_bytes();
    let doc = Document::parse_bytes(fixture).unwrap();

    let t1 = doc.task_by_id(TaskId(1)).unwrap();
    let t2 = doc.task_by_id(TaskId(2)).unwrap();
    let t3 = doc.task_by_id(TaskId(3)).unwrap();
    let t4 = doc.task_by_id(TaskId(4)).unwrap();
    let t5 = doc.task_by_id(TaskId(5)).unwrap();
    let t6 = doc.task_by_id(TaskId(6)).unwrap();

    // Open
    assert_eq!(
        validate_transition(t1, StatusCommand::Claim, "bob", false, "T-001")
            .unwrap()
            .0,
        Status::Claimed
    );
    assert_eq!(
        validate_transition(t1, StatusCommand::Block, "bob", false, "T-001")
            .unwrap()
            .0,
        Status::Blocked
    );
    assert_eq!(
        validate_transition(t1, StatusCommand::Done, "bob", false, "T-001")
            .unwrap()
            .0,
        Status::Done
    );
    assert_eq!(
        validate_transition(t1, StatusCommand::Drop, "bob", false, "T-001")
            .unwrap()
            .0,
        Status::Dropped
    );
    for cmd in [
        StatusCommand::Release,
        StatusCommand::Unblock,
        StatusCommand::Reopen,
    ] {
        let err = validate_transition(t1, cmd, "bob", false, "T-001").unwrap_err();
        assert_eq!(err.code, ErrorCode::BadTransition);
        assert_eq!(err.exit_code, ExitCode::Refused);
        assert!(err.message.contains("open"));
        assert!(err.message.contains("claim"));
    }

    // Claimed
    assert_eq!(
        validate_transition(t2, StatusCommand::Release, "alice", false, "T-002").unwrap(),
        (Status::Open, OwnershipDecision::Allowed)
    );
    let err_not_owner =
        validate_transition(t2, StatusCommand::Release, "bob", false, "T-002").unwrap_err();
    assert_eq!(err_not_owner.code, ErrorCode::NotOwner);
    assert_eq!(
        validate_transition(t2, StatusCommand::Release, "bob", true, "T-002").unwrap(),
        (
            Status::Open,
            OwnershipDecision::Forced {
                previous_owner: "alice".to_string()
            }
        )
    );
    for cmd in [StatusCommand::Unblock, StatusCommand::Reopen] {
        let err = validate_transition(t2, cmd, "alice", false, "T-002").unwrap_err();
        assert_eq!(err.code, ErrorCode::BadTransition);
    }

    // Blocked
    assert_eq!(
        validate_transition(t3, StatusCommand::Unblock, "bob", false, "T-003")
            .unwrap()
            .0,
        Status::Open
    );
    assert_eq!(
        validate_transition(t4, StatusCommand::Unblock, "alice", false, "T-004")
            .unwrap()
            .0,
        Status::Claimed
    );
    assert_eq!(
        validate_transition(t4, StatusCommand::Release, "alice", false, "T-004")
            .unwrap()
            .0,
        Status::Blocked
    );
    for cmd in [StatusCommand::Claim, StatusCommand::Done] {
        let err = validate_transition(t3, cmd, "bob", false, "T-003").unwrap_err();
        assert_eq!(err.code, ErrorCode::BadTransition);
        assert!(err.message.contains("--force"));
        assert!(validate_transition(t3, cmd, "bob", true, "T-003").is_ok());
    }
    for cmd in [StatusCommand::Block, StatusCommand::Reopen] {
        let err = validate_transition(t3, cmd, "bob", true, "T-003").unwrap_err();
        assert_eq!(err.code, ErrorCode::BadTransition);
    }

    // Done & Dropped
    for task in [t5, t6] {
        let id = if task.id == TaskId(5) {
            "T-005"
        } else {
            "T-006"
        };
        assert_eq!(
            validate_transition(task, StatusCommand::Reopen, "bob", false, id)
                .unwrap()
                .0,
            Status::Open
        );
        for cmd in [
            StatusCommand::Claim,
            StatusCommand::Release,
            StatusCommand::Block,
            StatusCommand::Unblock,
            StatusCommand::Done,
            StatusCommand::Drop,
        ] {
            let err = validate_transition(task, cmd, "bob", true, id).unwrap_err();
            assert_eq!(err.code, ErrorCode::BadTransition);
            assert!(err.message.contains("reopen"));
        }
    }
}

#[test]
fn note_command_bypasses_ownership_while_other_commands_require_force() {
    let fixture = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
### T-001 · Claimed task\n- status: claimed\n- owner: alice\n- claimed: 2026-11-15T10:00+01:00\n"
        .as_bytes();
    let doc = Document::parse_bytes(fixture).unwrap();
    let t1 = doc.task_by_id(TaskId(1)).unwrap();

    assert_eq!(
        check_ownership(t1, TaskCommand::Note, "bob", false, "T-001").unwrap(),
        OwnershipDecision::Allowed
    );
    let err = check_ownership(t1, TaskCommand::Set, "bob", false, "T-001").unwrap_err();
    assert_eq!(err.code, ErrorCode::NotOwner);
    assert_eq!(
        check_ownership(t1, TaskCommand::Set, "bob", true, "T-001").unwrap(),
        OwnershipDecision::Forced {
            previous_owner: "alice".to_string()
        }
    );
}

#[test]
fn stale_claim_compares_in_utc_across_timezones() {
    // Claimed at 2026-11-15T14:00-05:00 == 19:00Z.
    // Checked at 2026-11-16T20:00+02:00 == 18:00Z -> only 23 hours old (NOT stale)!
    // Checked at 2026-11-16T22:00+02:00 == 20:00Z -> 25 hours old (STALE)!
    let fixture = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
### T-001 · Cross-tz claim\n- status: claimed\n- owner: alice\n- claimed: 2026-11-15T14:00-05:00\n"
        .as_bytes();
    let doc = Document::parse_bytes(fixture).unwrap();
    let t1 = doc.task_by_id(TaskId(1)).unwrap();
    let cfg = OpsConfig::default();

    let not_stale_now = DateTime::parse_from_rfc3339("2026-11-16T20:00:00+02:00").unwrap();
    assert!(!cfg.is_claim_stale(t1, not_stale_now));
    let err = plan_claim(t1, "bob", false, not_stale_now, &cfg).unwrap_err();
    assert_eq!(err.code, ErrorCode::AlreadyClaimed);
    assert_eq!(err.details.owner.as_deref(), Some("alice"));

    let stale_now = DateTime::parse_from_rfc3339("2026-11-16T22:00:00+02:00").unwrap();
    assert!(cfg.is_claim_stale(t1, stale_now));
    let plan = plan_claim(t1, "bob", false, stale_now, &cfg).unwrap();
    assert_eq!(plan.new_owner, "bob");
    assert_eq!(
        plan.auto_note.as_deref(),
        Some("took over from alice (claim was 25 h old)")
    );
}

#[test]
fn task_edit_builder_inserts_fields_in_canonical_order_and_edits_combined_priority() {
    let fixture = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
### T-001 · Bare task\n- priority: P1 · type: bug\nProse stays.\n"
        .as_bytes();
    let doc = Document::parse_bytes(fixture).unwrap();
    let t1 = doc.task_by_id(TaskId(1)).unwrap();

    let mut builder = TaskEditBuilder::new(&doc, t1);
    builder.set_status(Status::Claimed);
    builder.set_owner(Some("alice"));
    builder.set_claimed("2026-11-15T14:03+01:00");
    builder.set_field("area", "cli");
    builder.add_note("2026-11-15", "alice", "line1\nline2");
    let (edits, intent) = builder.build();

    let applied = doc.apply_and_verify(&edits, &intent).unwrap();
    let text = String::from_utf8(applied.bytes).unwrap();
    assert_eq!(
        text,
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
### T-001 · Bare task\n\
- status: claimed\n\
- owner: alice\n\
- claimed: 2026-11-15T14:03+01:00\n\
- priority: P1 · type: bug · area: cli\n\
Prose stays.\n\
- note (2026-11-15, alice): line1 line2\n"
    );

    // Also verify release plan on unowned blocked task is a no-op.
    let blocked_doc = Document::parse_bytes(
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n### T-002 · Blocked\n- status: blocked\n- owner: \u{2014}\n"
            .as_bytes(),
    )
    .unwrap();
    let t2 = blocked_doc.task_by_id(TaskId(2)).unwrap();
    let rplan = plan_release(t2, "alice", false, &OpsConfig::default()).unwrap();
    assert_eq!(rplan.new_status, Status::Blocked);
}
