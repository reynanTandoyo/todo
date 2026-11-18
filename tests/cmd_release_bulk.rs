//! Tests for `todo release --stale` and `todo release --all` (T-033; R-CLAIM-6).

mod common;

use common::parse_json;

use std::fs;

use chrono::Local;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn release_stale_releases_only_stale_claims_in_one_step() {
    let dir = TempDir::new().unwrap();
    let fresh_ts = Local::now().fixed_offset().format("%Y-%m-%dT%H:%M%:z");
    let content = format!(
        "<!-- todo-format: 1 -->\n\n## Tasks\n\n\
         ### T-001 · Stale claim\n- status: claimed\n- owner: alice\n- claimed: 2020-01-01T00:00+00:00\n- priority: P1\n\n\
         ### T-002 · Fresh claim\n- status: claimed\n- owner: bob\n- claimed: {fresh_ts}\n- priority: P1\n\n\
         ### T-003 · Open task\n- status: open\n- owner: —\n- priority: P2\n"
    );
    fs::write(dir.path().join("TODO.md"), content).unwrap();

    let out = common::assert_todo_bin()
        .current_dir(dir.path())
        .args(["--json", "--agent", "coordinator", "release", "--stale"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let json: Value = parse_json(&out.stdout);
    assert_eq!(json["data"]["count"], 1);
    assert_eq!(json["data"]["released"][0]["id"], "T-001");
    assert_eq!(json["data"]["released"][0]["status"], "open");

    // T-002 must still be claimed by bob.
    let show = common::assert_todo_bin()
        .current_dir(dir.path())
        .args(["--json", "show", "T-002"])
        .output()
        .unwrap();
    let show_json: Value = parse_json(&show.stdout);
    assert_eq!(show_json["data"]["status"], "claimed");
    assert_eq!(show_json["data"]["owner"], "bob");
}

#[test]
fn release_all_releases_own_claims_and_requires_force_for_another_agent() {
    let dir = TempDir::new().unwrap();
    let fresh_ts = Local::now().fixed_offset().format("%Y-%m-%dT%H:%M%:z");
    let content = format!(
        "<!-- todo-format: 1 -->\n\n## Tasks\n\n\
         ### T-001 · Alice one\n- status: claimed\n- owner: alice\n- claimed: {fresh_ts}\n- priority: P1\n\n\
         ### T-002 · Alice two\n- status: claimed\n- owner: alice\n- claimed: {fresh_ts}\n- priority: P1\n\n\
         ### T-003 · Bob one\n- status: claimed\n- owner: bob\n- claimed: {fresh_ts}\n- priority: P1\n"
    );
    fs::write(dir.path().join("TODO.md"), content).unwrap();

    // Alice releases all of her own claims without --force.
    let own = common::assert_todo_bin()
        .current_dir(dir.path())
        .args(["--json", "--agent", "alice", "release", "--all"])
        .output()
        .unwrap();
    assert_eq!(own.status.code(), Some(0));
    let own_json: Value = parse_json(&own.stdout);
    assert_eq!(own_json["data"]["count"], 2);

    // Coordinator trying to release Bob's claims without --force is refused with NOT_OWNER.
    let denied = common::assert_todo_bin()
        .current_dir(dir.path())
        .args([
            "--json",
            "--agent",
            "coordinator",
            "release",
            "--all",
            "--owner",
            "bob",
        ])
        .output()
        .unwrap();
    assert_eq!(denied.status.code(), Some(1));
    let denied_json: Value = parse_json(&denied.stdout);
    assert_eq!(denied_json["error"]["code"], "NOT_OWNER");

    // With --force, coordinator releases Bob's claims.
    let forced = common::assert_todo_bin()
        .current_dir(dir.path())
        .args([
            "--json",
            "--agent",
            "coordinator",
            "--force",
            "release",
            "--all",
            "--owner",
            "bob",
        ])
        .output()
        .unwrap();
    assert_eq!(forced.status.code(), Some(0));
    let forced_json: Value = parse_json(&forced.stdout);
    assert_eq!(forced_json["data"]["count"], 1);
    assert_eq!(forced_json["data"]["released"][0]["id"], "T-003");
    assert_eq!(forced_json["data"]["released"][0]["status"], "open");
}
