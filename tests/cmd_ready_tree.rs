//! CLI integration tests for readiness and `next` along the milestone tree (T-102; R-MILE-6,
//! R-READ-3, R-DEP-1, R-DEP-3, R-DEP-4): milestone dependencies and states, `depends` naming a
//! milestone, `check` cycles through milestones, `next --milestone`, the `next` order, ten parallel
//! claims, the ready-to-close output of `done` and `drop`, and `summary`. Every fixture is invented.

mod common;

use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Output;

use serde_json::Value;
use tempfile::TempDir;

use common::{parse_json, read_todo, run_parallel_todos, run_todo, setup_proj, stdout_text};

/// Fixed clock for note dates and claim age (`TODO_TEST_NOW`, test-hooks feature).
const NOW: &str = "2026-11-18T10:00:00+01:00";

const FIXTURE: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->\n";

/// One entry as a `###` block. Milestones get `type: milestone`; `parent` writes the `milestone`
/// field; `extra` is appended verbatim (goal, depends, owner, claimed, notes).
fn entry(
    id: &str,
    title: &str,
    milestone: bool,
    priority: &str,
    status: &str,
    parent: Option<&str>,
    extra: &str,
) -> String {
    let mut s = format!("### {id} · {title}\n- status: {status}\n");
    if milestone {
        s.push_str(&format!("- priority: {priority} · type: milestone\n"));
    } else {
        // Every task has a level (R-LEVEL-1); `next` and `list --ready` hand out only leveled tasks.
        s.push_str(&format!("- priority: {priority} · difficulty: 2\n"));
    }
    if let Some(p) = parent {
        s.push_str(&format!("- milestone: {p}\n"));
    }
    s.push_str(extra);
    s.push('\n');
    s
}

/// The whole file: the fixture header, one section, and the entries in the given order.
fn body(entries: &[String]) -> String {
    let mut s = format!("{FIXTURE}## Now\n\n");
    for e in entries {
        s.push_str(e);
        s.push('\n');
    }
    s
}

struct Proj {
    _tmp: TempDir,
    home: PathBuf,
    work: PathBuf,
}

fn proj(text: &str) -> Proj {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo) = setup_proj(&tmp, text);
    Proj {
        _tmp: tmp,
        home,
        work,
    }
}

/// Runs `todo <args>` with the fixed clock.
fn run(p: &Proj, args: &[&str]) -> Output {
    run_todo(&p.home, &p.work, args, &[("TODO_TEST_NOW", NOW)]).unwrap()
}

/// Runs `todo --json <args>` with the fixed clock.
fn run_json(p: &Proj, args: &[&str]) -> Output {
    let mut all = vec!["--json"];
    all.extend_from_slice(args);
    run(p, &all)
}

/// Runs `args` and asserts exit code 0; returns the output.
fn ok(p: &Proj, args: &[&str]) -> Output {
    let out = run(p, args);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

/// Runs `todo --json <args>` and asserts a refusal with exit 1 and `code`.
fn refused(p: &Proj, args: &[&str], code: &str) {
    let out = run_json(p, args);
    assert_eq!(out.status.code(), Some(1), "exit code for {args:?}");
    assert_eq!(
        parse_json(&out.stdout)["error"]["code"].as_str(),
        Some(code),
        "error code for {args:?}"
    );
}

/// The IDs `list --ready` hands out, sorted.
fn ready_ids(p: &Proj) -> Vec<String> {
    let j = parse_json(&run_json(p, &["list", "--ready"]).stdout);
    let mut ids: Vec<String> = j["data"]["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap().to_string())
        .collect();
    ids.sort();
    ids
}

/// The ID `next` returns with `args` (`None` when nothing is ready).
fn next_id(p: &Proj, args: &[&str]) -> Option<String> {
    let mut all = vec!["next"];
    all.extend_from_slice(args);
    let j = parse_json(&run_json(p, &all).stdout);
    j["data"]["task"]["id"].as_str().map(str::to_string)
}

/// The status line of the entry `id` in the file text.
fn status_of(text: &str, id: &str) -> String {
    let heading = format!("### {id} ·");
    let mut lines = text.lines().skip_while(|l| !l.starts_with(&heading));
    lines.next();
    lines
        .find(|l| l.starts_with("- status: "))
        .map(|l| l["- status: ".len()..].to_string())
        .unwrap_or_default()
}

#[test]
fn a_task_waits_for_the_dependencies_of_every_milestone_above_it() {
    let p = proj(&body(&[
        entry(
            "T-001",
            "Renderer",
            true,
            "P1",
            "open",
            None,
            "- goal: the scene renders\n- depends: T-006\n",
        ),
        entry(
            "T-002",
            "Lighting",
            true,
            "P1",
            "open",
            Some("T-001"),
            "- goal: lights match\n",
        ),
        entry(
            "T-003",
            "Shadow pass",
            false,
            "P1",
            "open",
            Some("T-002"),
            "",
        ),
        entry(
            "T-004",
            "Light probes",
            false,
            "P2",
            "open",
            Some("T-001"),
            "",
        ),
        entry("T-005", "Asset import", false, "P2", "open", None, ""),
        entry("T-006", "Docs", false, "P3", "open", None, ""),
    ]));

    // T-003 is two levels below T-001, which waits for T-006; T-004 waits for it directly.
    assert_eq!(ready_ids(&p), ["T-005", "T-006"]);
    assert_eq!(next_id(&p, &[]).as_deref(), Some("T-005"));

    let done = stdout_text(&ok(&p, &["done", "T-006", "--agent", "planner"]));
    assert!(done.contains("now ready: T-003 Shadow pass"), "{done}");
    assert!(done.contains("now ready: T-004 Light probes"), "{done}");
    assert!(!done.contains("now ready: T-005"), "{done}");
    assert_eq!(ready_ids(&p), ["T-003", "T-004", "T-005"]);
}

#[test]
fn a_blocked_done_or_dropped_milestone_above_hides_its_tasks() {
    let p = proj(&body(&[
        entry(
            "T-001",
            "Assets",
            true,
            "P1",
            "blocked",
            None,
            "- goal: assets arrive\n- note (2026-11-15, planner): waiting for the asset team\n",
        ),
        entry("T-002", "Textures", false, "P1", "open", Some("T-001"), ""),
        entry(
            "T-003",
            "Old scope",
            true,
            "P1",
            "done",
            None,
            "- goal: finished earlier\n",
        ),
        entry("T-004", "Leftover", false, "P1", "open", Some("T-003"), ""),
        entry(
            "T-005",
            "Cut scope",
            true,
            "P1",
            "dropped",
            None,
            "- goal: no longer wanted\n",
        ),
        entry("T-006", "Cut task", false, "P1", "open", Some("T-005"), ""),
        entry("T-007", "Plain task", false, "P2", "open", None, ""),
    ]));

    assert_eq!(ready_ids(&p), ["T-007"]);
    assert_eq!(next_id(&p, &[]).as_deref(), Some("T-007"));

    // Once the blocked milestone is open again, its tasks are handed out.
    ok(&p, &["unblock", "T-001", "--agent", "planner"]);
    assert_eq!(ready_ids(&p), ["T-002", "T-007"]);
}

#[test]
fn depends_may_name_a_milestone_and_waits_for_it_to_be_done_or_dropped() {
    let p = proj(&body(&[
        entry(
            "T-001",
            "Renderer",
            true,
            "P1",
            "open",
            None,
            "- goal: the scene renders\n",
        ),
        entry("T-002", "Done part", false, "P1", "done", Some("T-001"), ""),
        entry(
            "T-003",
            "Needs renderer",
            false,
            "P1",
            "open",
            None,
            "- depends: T-001\n",
        ),
        entry(
            "T-004",
            "Old idea",
            true,
            "P1",
            "open",
            None,
            "- goal: maybe later\n",
        ),
        entry(
            "T-005",
            "Needs old idea",
            false,
            "P1",
            "open",
            None,
            "- depends: T-004\n",
        ),
    ]));

    // Both milestones are still open, so the tasks waiting on them are not ready.
    assert!(ready_ids(&p).is_empty());

    ok(&p, &["done", "T-001", "--agent", "planner"]);
    assert_eq!(ready_ids(&p), ["T-003"]);

    ok(
        &p,
        &["drop", "T-004", "--note", "scope cut", "--agent", "planner"],
    );
    assert_eq!(ready_ids(&p), ["T-003", "T-005"]);
}

#[test]
fn check_reports_dependency_cycles_through_milestones() {
    let p = proj(&body(&[
        // T-002 is in T-001 and depends on it: T-001 is finished only when T-002 is.
        entry(
            "T-001",
            "Loop A",
            true,
            "P1",
            "open",
            None,
            "- goal: loop\n",
        ),
        entry(
            "T-002",
            "Inside A",
            false,
            "P1",
            "open",
            Some("T-001"),
            "- depends: T-001\n",
        ),
        // T-005 is in T-004, which depends on T-005: T-005 waits for its own milestone.
        entry(
            "T-004",
            "Loop B",
            true,
            "P1",
            "open",
            None,
            "- goal: loop\n- depends: T-005\n",
        ),
        entry("T-005", "Inside B", false, "P1", "open", Some("T-004"), ""),
        // No cycle here: T-007 waits for T-006 and nothing waits back.
        entry("T-006", "Base", false, "P2", "open", None, ""),
        entry(
            "T-007",
            "Milestone on base",
            true,
            "P2",
            "open",
            None,
            "- goal: base first\n- depends: T-006\n",
        ),
        entry(
            "T-008",
            "Under base",
            false,
            "P2",
            "open",
            Some("T-007"),
            "",
        ),
    ]));

    let out = run(&p, &["check"]);
    assert_eq!(out.status.code(), Some(1));
    let text = stdout_text(&out);
    assert!(
        text.contains("dependency cycle: T-001 -> T-002 -> T-001"),
        "{text}"
    );
    assert!(text.contains("dependency cycle: T-005 -> T-005"), "{text}");
    assert_eq!(text.matches("dependency cycle:").count(), 2, "{text}");
}

#[test]
fn check_reports_a_loop_of_milestone_parents_once_even_when_depends_repeats_it() {
    // T-001 and T-002 are each other's parent (check_milestones reports that loop once), and
    // T-001 also depends on T-002: the same loop must not be reported a second time as a
    // dependency cycle.
    let p = proj(&body(&[
        entry(
            "T-001",
            "Loop A",
            true,
            "P1",
            "open",
            Some("T-002"),
            "- goal: loop\n- depends: T-002\n",
        ),
        entry(
            "T-002",
            "Loop B",
            true,
            "P1",
            "open",
            Some("T-001"),
            "- goal: loop\n",
        ),
    ]));

    let out = run(&p, &["check"]);
    assert_eq!(out.status.code(), Some(1));
    let text = stdout_text(&out);
    assert_eq!(text.matches("milestone cycle:").count(), 1, "{text}");
    assert!(!text.contains("dependency cycle:"), "{text}");
}

#[test]
fn next_milestone_returns_only_tasks_below_that_milestone_at_any_depth() {
    let p = proj(&body(&[
        entry(
            "T-001",
            "Renderer",
            true,
            "P1",
            "open",
            None,
            "- goal: the scene renders\n",
        ),
        entry(
            "T-002",
            "Lighting",
            true,
            "P1",
            "open",
            Some("T-001"),
            "- goal: lights match\n",
        ),
        entry(
            "T-003",
            "Shadow pass",
            false,
            "P2",
            "open",
            Some("T-002"),
            "",
        ),
        entry(
            "T-004",
            "Light probes",
            false,
            "P2",
            "open",
            Some("T-001"),
            "",
        ),
        entry("T-005", "Urgent elsewhere", false, "P1", "open", None, ""),
    ]));

    assert_eq!(next_id(&p, &[]).as_deref(), Some("T-005"));
    assert_eq!(
        next_id(&p, &["--milestone", "T-001"]).as_deref(),
        Some("T-003")
    );
    assert_eq!(
        next_id(&p, &["--milestone", "T-002"]).as_deref(),
        Some("T-003")
    );
    // A title works as well as an ID.
    assert_eq!(
        next_id(&p, &["--milestone", "Lighting"]).as_deref(),
        Some("T-003")
    );

    // Once T-003 is claimed, nothing below T-002 is left; T-004 is still below T-001.
    ok(&p, &["claim", "T-003", "--agent", "planner"]);
    assert_eq!(next_id(&p, &["--milestone", "T-002"]), None);
    assert_eq!(
        next_id(&p, &["--milestone", "T-001"]).as_deref(),
        Some("T-004")
    );

    refused(&p, &["next", "--milestone", "T-005"], "BAD_VALUE");
    refused(&p, &["next", "--milestone", "T-099"], "NOT_FOUND");
}

#[test]
fn next_prefers_started_milestones_then_file_order_and_summary_uses_the_same_order() {
    let p = proj(&body(&[
        entry(
            "T-001",
            "Early",
            true,
            "P1",
            "open",
            None,
            "- goal: first in the file\n",
        ),
        entry(
            "T-002",
            "Untouched early task",
            false,
            "P1",
            "open",
            Some("T-001"),
            "",
        ),
        entry(
            "T-003",
            "Late",
            true,
            "P1",
            "open",
            None,
            "- goal: work has started\n",
        ),
        entry(
            "T-004",
            "Ready in started",
            false,
            "P1",
            "open",
            Some("T-003"),
            "",
        ),
        entry(
            "T-005",
            "Claimed in started",
            false,
            "P1",
            "claimed",
            Some("T-003"),
            "- owner: bob\n- claimed: 2026-11-18T09:00+01:00\n",
        ),
        entry("T-006", "Top level P1", false, "P1", "open", None, ""),
        entry(
            "T-007",
            "Started but P2",
            false,
            "P2",
            "open",
            Some("T-003"),
            "",
        ),
    ]));

    // Priority first; then the started milestone before untouched ones; then file order.
    assert_eq!(next_id(&p, &[]).as_deref(), Some("T-004"));

    let summary = parse_json(&run_json(&p, &["summary"]).stdout);
    let ready: Vec<&str> = summary["data"]["next_ready"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap())
        .collect();
    assert_eq!(ready, ["T-004", "T-002", "T-006", "T-007"]);
}

#[test]
fn ten_parallel_next_claims_get_ten_different_tasks_along_the_tree() {
    let mut entries = vec![
        entry(
            "T-001",
            "Alpha",
            true,
            "P1",
            "open",
            None,
            "- goal: alpha\n",
        ),
        entry("T-002", "Beta", true, "P1", "open", None, "- goal: beta\n"),
    ];
    for i in 0..4 {
        entries.push(entry(
            &format!("T-{:03}", 3 + i),
            &format!("Alpha task {i}"),
            false,
            "P1",
            "open",
            Some("T-001"),
            "",
        ));
    }
    for i in 0..4 {
        entries.push(entry(
            &format!("T-{:03}", 7 + i),
            &format!("Beta task {i}"),
            false,
            "P1",
            "open",
            Some("T-002"),
            "",
        ));
    }
    entries.push(entry("T-011", "Loose one", false, "P2", "open", None, ""));
    entries.push(entry("T-012", "Loose two", false, "P2", "open", None, ""));

    let tmp = TempDir::new().unwrap();
    let (home, work, _todo) = setup_proj(&tmp, &body(&entries));
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

    let outputs = run_parallel_todos(&home, &work, &invocations).unwrap();
    let mut picked = HashSet::new();
    for out in &outputs {
        assert!(out.status.success(), "{}", stdout_text(out));
        let j: Value = parse_json(&out.stdout);
        let id = j["data"]["task"]["id"].as_str().unwrap().to_string();
        assert!(picked.insert(id.clone()), "duplicate claim of {id}");
    }
    assert_eq!(picked.len(), 10);

    let text = read_todo(&work);
    assert_eq!(text.matches("- status: claimed").count(), 10);
}

#[test]
fn done_names_a_milestone_ready_to_close_and_never_closes_it() {
    let p = proj(&body(&[
        entry(
            "T-001",
            "Renderer",
            true,
            "P1",
            "open",
            None,
            "- goal: the scene renders\n",
        ),
        entry(
            "T-002",
            "Lighting",
            true,
            "P1",
            "open",
            Some("T-001"),
            "- goal: lights match\n",
        ),
        entry(
            "T-003",
            "Shadow pass",
            false,
            "P1",
            "open",
            Some("T-002"),
            "",
        ),
        entry(
            "T-004",
            "Light probes",
            false,
            "P1",
            "open",
            Some("T-001"),
            "",
        ),
        entry(
            "T-005",
            "Empty milestone",
            true,
            "P2",
            "open",
            None,
            "- goal: nothing below yet\n",
        ),
    ]));

    // The last entry under T-002 is done: T-002 is ready to close, T-001 still waits for T-004.
    let first = stdout_text(&ok(&p, &["done", "T-003", "--agent", "planner"]));
    assert!(first.contains("ready to close: T-002 Lighting"), "{first}");
    assert!(first.contains("todo done T-002"), "{first}");
    assert!(!first.contains("ready to close: T-001"), "{first}");

    let second = stdout_text(&ok(&p, &["done", "T-004", "--agent", "planner"]));
    assert!(!second.contains("ready to close"), "{second}");

    // Closing T-002 makes T-001 ready to close, and the JSON says so.
    let third = parse_json(&run_json(&p, &["done", "T-002", "--agent", "planner"]).stdout);
    assert_eq!(third["data"]["ready_to_close"][0]["id"], "T-001");
    assert_eq!(third["data"]["ready_to_close"][0]["title"], "Renderer");

    // Nothing closed T-001 on its own.
    let shown = parse_json(&run_json(&p, &["show", "T-001"]).stdout);
    assert_eq!(shown["data"]["status"], "open");
    assert_eq!(status_of(&read_todo(&p.work), "T-001"), "open");

    // summary lists it, but not the milestone with nothing below it.
    let summary = parse_json(&run_json(&p, &["summary"]).stdout);
    let closable: Vec<&str> = summary["data"]["ready_to_close"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["id"].as_str().unwrap())
        .collect();
    assert_eq!(closable, ["T-001"]);
    let text = stdout_text(&run(&p, &["summary"]));
    assert!(text.contains("ready to close (1):"), "{text}");
    assert!(
        text.contains("T-001 · Renderer — todo done T-001"),
        "{text}"
    );

    // Closing it is the owner's decision; afterwards the summary no longer lists it.
    ok(&p, &["done", "T-001", "--agent", "planner"]);
    let after = parse_json(&run_json(&p, &["summary"]).stdout);
    assert!(after["data"].get("ready_to_close").is_none());
}

#[test]
fn drop_that_finishes_the_last_entry_names_the_milestone_too() {
    let p = proj(&body(&[
        entry(
            "T-010",
            "Scope",
            true,
            "P1",
            "open",
            None,
            "- goal: the cut list\n",
        ),
        entry("T-011", "Keep", false, "P1", "open", Some("T-010"), ""),
        entry("T-012", "Cut", false, "P2", "open", Some("T-010"), ""),
    ]));

    let first = stdout_text(&ok(
        &p,
        &[
            "drop",
            "T-011",
            "--note",
            "not needed",
            "--agent",
            "planner",
        ],
    ));
    assert!(!first.contains("ready to close"), "{first}");

    let second = stdout_text(&ok(
        &p,
        &["drop", "T-012", "--note", "cut too", "--agent", "planner"],
    ));
    assert!(second.contains("ready to close: T-010 Scope"), "{second}");
    assert_eq!(status_of(&read_todo(&p.work), "T-010"), "open");
}
