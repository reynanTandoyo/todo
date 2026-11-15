//! CLI integration tests for `todo next` including `--claim` (T-023; R-READ-3, R-DEP-1, TEST-5).

mod common;

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;
use todo::doc::{Document, Status, TaskId};

fn setup_proj(tmp: &TempDir, body: &str) -> std::io::Result<(PathBuf, PathBuf, PathBuf)> {
    let home = tmp.path().join("home");
    let work = tmp.path().join("work");
    fs::create_dir_all(&home)?;
    fs::create_dir_all(&work)?;
    let todo_file = work.join("TODO.md");
    fs::write(&todo_file, body)?;
    Ok((home, work, todo_file))
}

#[cfg(feature = "test-hooks")]
#[test]
fn next_respects_priority_dependencies_questions_stale_and_filters() {
    let tmp = TempDir::new().unwrap();
    let body = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
## 1. Backend\n\n\
### T-001 · Blocking open dependency\n\
- status: open\n\
- priority: P2\n\
- type: task\n\
\n\
### T-002 · High prio waiting on T-001\n\
- status: open\n\
- priority: P0\n\
- depends: T-001\n\
\n\
### T-003 · Design question should never be picked\n\
- status: open\n\
- priority: P0\n\
- type: question\n\
\n\
### T-004 · Live claimed P0\n\
- status: claimed\n\
- owner: bob\n\
- claimed: 2026-11-15T13:00+01:00\n\
- priority: P0\n\
\n\
### T-005 · Missing dependency\n\
- status: open\n\
- priority: P0\n\
- depends: T-999\n\
\n\
### T-006 · Unblocked P1 ready\n\
- status: open\n\
- priority: P1 · type: bug · area: api\n\
- flavor: hot\n\
\n\
## 2. Frontend\n\n\
### T-007 · Archived dependency satisfied P1\n\
- status: open\n\
- priority: P1 · type: feat · area: ui\n\
- depends: T-090\n\
\n\
## 3. Maintenance\n\n\
### T-008 · Stale claimed P0\n\
- status: claimed\n\
- owner: sleepy\n\
- claimed: 2026-11-14T10:00+01:00\n\
- priority: P0\n";

    let archive = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
## Done\n\n\
### T-090 · Old finished prereq\n\
- status: done\n\
- owner: alice\n";

    let (home, work, _) = setup_proj(&tmp, body).unwrap();
    fs::write(work.join("TODO-archive.md"), archive).unwrap();
    let now = "2026-11-15T14:00:00+01:00";

    // Unfiltered: stale P0 T-008 wins over ready P1s, never T-002/T-003/T-004/T-005.
    let top =
        common::run_todo(&home, &work, &["--json", "next"], &[("TODO_TEST_NOW", now)]).unwrap();
    assert!(top.status.success());
    let j: serde_json::Value = serde_json::from_slice(&top.stdout).unwrap();
    assert_eq!(j["data"]["task"]["id"], "T-008");
    assert_eq!(j["data"]["claimed"], false);
    assert_eq!(j["data"]["already_claimed"], false);

    // Filter `--area api`: skips T-008 and T-007, picks T-006.
    let api = common::run_todo(
        &home,
        &work,
        &["--json", "next", "--area", "api", "--type", "bug"],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    let j_api: serde_json::Value = serde_json::from_slice(&api.stdout).unwrap();
    assert_eq!(j_api["data"]["task"]["id"], "T-006");

    // Filter `--section Frontend` checks archive deps too -> picks T-007.
    let fe = common::run_todo(
        &home,
        &work,
        &["--json", "next", "--section", "frontend"],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    let j_fe: serde_json::Value = serde_json::from_slice(&fe.stdout).unwrap();
    assert_eq!(j_fe["data"]["task"]["id"], "T-007");

    // Filter `--where flavor=hot`: picks T-006.
    let wh = common::run_todo(
        &home,
        &work,
        &["--json", "next", "--where", "flavor=hot"],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    let j_wh: serde_json::Value = serde_json::from_slice(&wh.stdout).unwrap();
    assert_eq!(j_wh["data"]["task"]["id"], "T-006");
}

#[cfg(feature = "test-hooks")]
#[test]
fn next_returns_own_claim_first_unless_new_and_claims_atomically() {
    let tmp = TempDir::new().unwrap();
    let body = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
## Work\n\n\
### T-001 · P0 ready open task\n\
- status: open\n\
- priority: P0\n\
\n\
### T-002 · P1 open task earlier\n\
- status: open\n\
- priority: P1\n\
\n\
### T-003 · P2 already mine\n\
- status: claimed\n\
- owner: alice\n\
- claimed: 2026-11-15T13:30+01:00\n\
- priority: P2\n\
\n\
### T-004 · Unprioritized ready\n\
- status: open\n";

    let (home, work, todo_file) = setup_proj(&tmp, body).unwrap();
    let now = "2026-11-15T14:00:00+01:00";

    // Read-only `next --agent alice` returns alice's own T-003 even though P0 T-001 is open.
    let own = common::run_todo(
        &home,
        &work,
        &["--json", "next", "--agent", "alice"],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    let j_own: serde_json::Value = serde_json::from_slice(&own.stdout).unwrap();
    assert_eq!(j_own["data"]["task"]["id"], "T-003");
    assert_eq!(j_own["data"]["already_claimed"], true);
    assert_eq!(j_own["data"]["claimed"], false);

    // Text output shows task summary and already claimed note.
    let own_txt = common::run_todo(
        &home,
        &work,
        &["next", "--agent", "alice"],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    let txt = String::from_utf8_lossy(&own_txt.stdout);
    assert!(txt.contains("T-003"), "txt: {txt}");
    assert!(txt.contains("already claimed by you"), "txt: {txt}");

    // `next --claim` without agent fails with exit 2 (NO_AGENT) and leaves file untouched.
    let no_agent = common::run_todo(
        &home,
        &work,
        &["--json", "next", "--claim"],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert_eq!(no_agent.status.code(), Some(2));
    assert_eq!(fs::read_to_string(&todo_file).unwrap(), body);

    // `next --claim --new --agent alice` skips T-003 and claims highest-priority P0 T-001.
    let claim_new = common::run_todo(
        &home,
        &work,
        &["--json", "next", "--claim", "--new", "--agent", "alice"],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert!(claim_new.status.success());
    let j_new: serde_json::Value = serde_json::from_slice(&claim_new.stdout).unwrap();
    assert_eq!(j_new["data"]["task"]["id"], "T-001");
    assert_eq!(j_new["data"]["claimed"], true);
    assert_eq!(j_new["data"]["already_claimed"], false);
    assert_eq!(j_new["data"]["task"]["status"], "claimed");
    assert_eq!(j_new["data"]["task"]["owner"], "alice");

    // Verify file on disk has T-001 claimed and T-004 still untouched until picked next.
    let after1 = Document::parse_bytes(&fs::read(&todo_file).unwrap()).unwrap();
    assert_eq!(
        after1.task_by_id(TaskId(1)).unwrap().status,
        Status::Claimed
    );
    assert_eq!(after1.task_by_id(TaskId(4)).unwrap().status, Status::Open);

    // Filter matching nothing returns exit 0 with task: null.
    let none = common::run_todo(
        &home,
        &work,
        &["--json", "next", "--claim", "--area", "nonexistent"],
        &[("TODO_TEST_NOW", now), ("TODO_AGENT", "alice")],
    )
    .unwrap();
    assert!(none.status.success());
    let j_none: serde_json::Value = serde_json::from_slice(&none.stdout).unwrap();
    assert!(j_none["data"]["task"].is_null());
    assert_eq!(j_none["data"]["claimed"], false);
}

#[test]
fn ten_parallel_next_claim_get_ten_different_tasks() {
    let tmp = TempDir::new().unwrap();
    let mut body =
        String::from("<!-- TEST FIXTURE: invented data, not a real task list -->\n## Queue\n\n");
    for i in 1..=12 {
        let prio = if i <= 4 {
            "P0"
        } else if i <= 8 {
            "P1"
        } else {
            "P2"
        };
        body.push_str(&format!(
            "### T-{i:03} · Parallel work item {i}\n- status: open\n- owner: —\n- priority: {prio}\n\n"
        ));
    }
    let (home, work, todo_file) = setup_proj(&tmp, &body).unwrap();

    let invocations: Vec<Vec<String>> = (0..10)
        .map(|i| {
            vec![
                "--json".to_string(),
                "next".to_string(),
                "--claim".to_string(),
                "--agent".to_string(),
                format!("worker-{i:02}"),
            ]
        })
        .collect();

    let outputs = common::run_parallel_todos(&home, &work, &invocations).unwrap();
    let mut picked_ids = HashSet::new();
    let mut picked_owners = HashSet::new();

    for (i, out) in outputs.iter().enumerate() {
        assert!(
            out.status.success(),
            "worker-{i:02} failed ({:?}): stdout={} stderr={}",
            out.status.code(),
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let j: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(j["ok"], true);
        assert_eq!(j["data"]["claimed"], true);
        let id = j["data"]["task"]["id"].as_str().unwrap().to_string();
        let owner = j["data"]["task"]["owner"].as_str().unwrap().to_string();
        assert!(picked_ids.insert(id.clone()), "duplicate id claimed: {id}");
        assert!(
            picked_owners.insert(owner.clone()),
            "duplicate owner reported: {owner}"
        );
    }

    assert_eq!(picked_ids.len(), 10);
    assert_eq!(picked_owners.len(), 10);

    let doc = Document::parse_bytes(&fs::read(&todo_file).unwrap()).unwrap();
    let claimed_count = doc
        .tasks
        .iter()
        .filter(|t| t.status == Status::Claimed)
        .count();
    let open_count = doc
        .tasks
        .iter()
        .filter(|t| t.status == Status::Open)
        .count();
    assert_eq!(claimed_count, 10);
    assert_eq!(open_count, 2);
}
