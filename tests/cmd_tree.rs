//! CLI integration tests for the milestone tree views (T-088; R-MILE-3, R-KIND-3, R-READ-4):
//! `todo tree` in text and nested JSON, `list --milestone` and `list --kind`, and the milestones
//! shown by `summary`. Every fixture is invented.

mod common;

use std::fs;
use std::path::PathBuf;
use std::process::Output;

use serde_json::Value;
use tempfile::TempDir;

use common::{parse_json, run_todo, setup_proj, stdout_text};

const FIXTURE: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->\n";

/// Milestone T-001 holds the milestone T-002, the open task T-004, and the question T-008 about it.
/// T-002 holds the done task T-003 and the dropped task T-005. T-006 is a top-level claimed task;
/// T-007 is a done top-level task; T-009 is an idea.
fn tree_fixture() -> String {
    format!(
        "{FIXTURE}## Now\n\n\
### T-001 · Develop the renderer\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: milestone\n\
- goal: the test scene renders with lighting at 60 frames per second\n\n\
### T-002 · Lighting\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: milestone\n\
- milestone: T-001\n\
- goal: point and directional lights match the reference images\n\n\
### T-003 · C shader\n\
- status: done\n\
- owner: —\n\
- priority: P1 · type: feature · area: cli\n\
- milestone: T-002\n\n\
### T-004 · GPU compiler\n\
- status: open\n\
- owner: —\n\
- priority: P2 · difficulty: hard · area: gpu\n\
- milestone: T-001\n\n\
### T-005 · Old shader\n\
- status: dropped\n\
- owner: —\n\
- priority: P2\n\
- milestone: T-002\n\n\
### T-006 · Write the docs\n\
- status: claimed\n\
- owner: alice\n\
- priority: P2 · area: docs\n\
- claimed: 2026-11-15T18:00+01:00\n\n\
### T-007 · Done loose task\n\
- status: done\n\
- owner: —\n\
- priority: P3\n\n\
### T-008 · Renderer question\n\
- status: open\n\
- owner: —\n\
- priority: P2 · type: question\n\
- about: T-001\n\n\
### T-009 · Try a new idea\n\
- status: open\n\
- owner: —\n\
- type: idea\n"
    )
}

struct Proj {
    _tmp: TempDir,
    home: PathBuf,
    work: PathBuf,
}

fn proj(body: &str) -> Proj {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo) = setup_proj(&tmp, body);
    Proj {
        _tmp: tmp,
        home,
        work,
    }
}

/// Runs `todo <args>` in the project.
fn run(p: &Proj, args: &[&str]) -> Output {
    run_todo(&p.home, &p.work, args, &[]).unwrap()
}

/// Runs `todo --json <args>` and returns the parsed envelope.
fn run_json(p: &Proj, args: &[&str]) -> Value {
    let mut all = vec!["--json"];
    all.extend_from_slice(args);
    let out = run(p, &all);
    assert!(
        out.status.success(),
        "{args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    parse_json(&out.stdout)["data"].clone()
}

/// The IDs of the `tasks` in a `list --json` result, in order.
fn listed_ids(data: &Value) -> Vec<String> {
    data["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap().to_string())
        .collect()
}

/// Asserts that `args` is refused with exit 1 and `code`, in `--json` mode.
fn assert_refused(p: &Proj, args: &[&str], code: &str) {
    let mut all = vec!["--json"];
    all.extend_from_slice(args);
    let out = run(p, &all);
    assert_eq!(out.status.code(), Some(1), "{args:?} should be refused");
    assert_eq!(
        parse_json(&out.stdout)["error"]["code"].as_str(),
        Some(code),
        "{args:?} error code"
    );
}

#[test]
fn tree_prints_milestones_with_goal_progress_and_areas_indented_by_depth() {
    let p = proj(&tree_fixture());
    let out = run(&p, &["tree"]);
    assert_eq!(out.status.code(), Some(0), "{}", stdout_text(&out));
    let expected = "\
T-001    P1  -     open     Develop the renderer  1/2 done
  goal: the test scene renders with lighting at 60 frames per second
  T-002  P1  -     open     Lighting  1/1 done
    goal: point and directional lights match the reference images
  T-004  P2  hard  open     GPU compiler  [gpu]
T-006    P2  -     claimed  Write the docs  [docs]
2 milestones, 2 tasks

questions (1):
  T-008 [open] · Renderer question (about T-001)
ideas (1):
  T-009 [open] · Try a new idea
";
    assert_eq!(stdout_text(&out), expected);
}

#[test]
fn tree_all_shows_done_and_dropped_entries_without_changing_progress() {
    let p = proj(&tree_fixture());
    let out = stdout_text(&run(&p, &["tree", "--all"]));
    assert!(out.contains("T-003  P1  -     done"), "{out}");
    assert!(out.contains("T-005"), "{out}");
    assert!(out.contains("T-007"), "{out}");
    let milestone_line = out
        .lines()
        .find(|l| l.starts_with("T-001"))
        .unwrap_or_default();
    assert!(
        milestone_line.ends_with("Develop the renderer  1/2 done"),
        "T-001 progress must stay 1/2 with --all: {out}"
    );
    // T-001 and T-002 are milestones; the tasks are T-003, T-004, T-005, T-006 and T-007.
    assert!(out.contains("2 milestones, 5 tasks"), "{out}");
}

#[test]
fn tree_with_a_milestone_shows_that_milestone_and_only_what_is_below_it() {
    let p = proj(&tree_fixture());
    let out = run(&p, &["tree", "T-002"]);
    assert_eq!(out.status.code(), Some(0));
    let expected = "\
T-002  P1  open  Lighting  1/1 done
  goal: point and directional lights match the reference images
1 milestone, 0 tasks
";
    assert_eq!(stdout_text(&out), expected);
}

#[test]
fn tree_area_shows_only_matching_tasks_with_the_milestones_above_them() {
    let p = proj(&tree_fixture());
    let out = run(&p, &["tree", "--area", "gpu"]);
    assert_eq!(out.status.code(), Some(0));
    let expected = "\
T-001    P1  -     open  Develop the renderer  1/2 done
  goal: the test scene renders with lighting at 60 frames per second
  T-004  P2  hard  open  GPU compiler  [gpu]
1 milestone, 1 task
";
    assert_eq!(stdout_text(&out), expected);
}

#[test]
fn tree_area_list_with_all_brings_done_tasks_in_those_areas_under_their_milestones() {
    let p = proj(&tree_fixture());
    let out = stdout_text(&run(&p, &["tree", "--area", "cli, docs", "--all"]));
    assert!(out.contains("T-001"), "{out}");
    assert!(out.contains("T-002"), "{out}");
    assert!(out.contains("T-003"), "{out}");
    assert!(out.contains("T-006"), "{out}");
    assert!(!out.contains("T-004"), "{out}");
    assert!(!out.contains("questions ("), "{out}");
}

#[test]
fn tree_json_is_nested_with_progress_and_matches_its_snapshot() {
    let p = proj(&tree_fixture());
    let data = run_json(&p, &["tree"]);
    let nodes = data["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 2, "top-level milestone then top-level task");
    assert_eq!(nodes[0]["id"], "T-001");
    assert_eq!(nodes[0]["kind"], "milestone");
    assert_eq!(nodes[0]["progress"]["done"], 1);
    assert_eq!(nodes[0]["progress"]["total"], 2);
    assert_eq!(nodes[0]["children"][0]["id"], "T-002");
    assert_eq!(
        nodes[0]["children"][0]["children"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(nodes[0]["children"][1]["id"], "T-004");
    assert_eq!(nodes[0]["children"][1]["kind"], "task");
    assert!(nodes[0]["children"][1]["progress"].is_null());
    assert_eq!(nodes[1]["id"], "T-006");
    assert_eq!(data["questions"][0]["about"], "T-001");
    assert_eq!(data["ideas"][0]["id"], "T-009");
    insta::assert_json_snapshot!("tree_json", data);
}

/// An invented `TODO-archive.md`: the finished milestone T-020 with T-021 done and T-022 dropped.
const ARCHIVE: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
## Archive\n\n\
### T-020 · Old pipeline\n\
- status: done\n\
- owner: —\n\
- priority: P2 · type: milestone\n\
- goal: the old pipeline ships\n\n\
### T-021 · Parser\n\
- status: done\n\
- owner: —\n\
- priority: P2\n\
- milestone: T-020\n\n\
### T-022 · Retired feature\n\
- status: dropped\n\
- owner: —\n\
- priority: P2\n\
- milestone: T-020\n";

#[test]
fn tree_all_also_shows_the_entries_archived_in_todo_archive_md_marked_archived() {
    let p = proj(&tree_fixture());
    fs::write(p.work.join("TODO-archive.md"), ARCHIVE).unwrap();

    let plain = stdout_text(&run(&p, &["tree"]));
    assert!(!plain.contains("T-020"), "{plain}");

    let out = stdout_text(&run(&p, &["tree", "--all"]));
    let milestone_line = out
        .lines()
        .find(|l| l.starts_with("T-020"))
        .unwrap_or_default();
    assert!(
        milestone_line.ends_with("Old pipeline  1/1 done  (archived)"),
        "{out}"
    );
    let task_line = out
        .lines()
        .find(|l| l.trim_start().starts_with("T-021"))
        .unwrap_or_default();
    assert!(task_line.ends_with("Parser  (archived)"), "{out}");
    assert!(out.contains("3 milestones, 7 tasks"), "{out}");
}

#[test]
fn tree_finds_an_archived_milestone_only_with_all_and_list_archive_reads_its_tree() {
    let p = proj(&tree_fixture());
    fs::write(p.work.join("TODO-archive.md"), ARCHIVE).unwrap();

    assert_refused(&p, &["tree", "T-020"], "NOT_FOUND");
    let out = run(&p, &["tree", "T-020", "--all"]);
    assert_eq!(out.status.code(), Some(0), "{}", stdout_text(&out));
    let text = stdout_text(&out);
    assert!(
        text.lines()
            .next()
            .unwrap_or_default()
            .ends_with("Old pipeline  1/1 done  (archived)"),
        "{text}"
    );
    assert!(text.contains("1 milestone, 2 tasks"), "{text}");

    assert_refused(
        &p,
        &["list", "--milestone", "T-020", "--status", "all"],
        "NOT_FOUND",
    );
    assert_eq!(
        listed_ids(&run_json(
            &p,
            &[
                "list",
                "--archive",
                "--milestone",
                "T-020",
                "--status",
                "all"
            ]
        )),
        vec!["T-021", "T-022"]
    );
}

#[test]
fn tree_refuses_a_task_that_is_not_a_milestone_an_unknown_one_and_an_empty_area_list() {
    let p = proj(&tree_fixture());
    assert_refused(&p, &["tree", "T-006"], "BAD_VALUE");
    assert_refused(&p, &["tree", "T-999"], "NOT_FOUND");
    assert_refused(&p, &["tree", "--area", " , "], "BAD_VALUE");
}

#[test]
fn tree_terminates_on_a_milestone_cycle_and_shows_each_entry_once() {
    let body = format!(
        "{FIXTURE}## Now\n\n\
### T-001 · Alpha\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: milestone\n\
- goal: a cycle\n\
- milestone: T-002\n\n\
### T-002 · Beta\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: milestone\n\
- goal: the other half of the cycle\n\
- milestone: T-001\n\n\
### T-003 · Gamma\n\
- status: open\n\
- owner: —\n\
- priority: P2\n\
- milestone: T-001\n"
    );
    let p = proj(&body);
    let out = run(&p, &["tree"]);
    assert_eq!(out.status.code(), Some(0), "{}", stdout_text(&out));
    let text = stdout_text(&out);
    for id in ["T-001", "T-002", "T-003"] {
        assert_eq!(
            text.matches(id).count(),
            1,
            "{id} should appear once:\n{text}"
        );
    }
}

#[test]
fn list_milestone_lists_everything_below_at_any_depth_and_refuses_a_task() {
    let p = proj(&tree_fixture());
    assert_eq!(
        listed_ids(&run_json(&p, &["list", "--milestone", "T-001"])),
        vec!["T-002", "T-004"]
    );
    assert_eq!(
        listed_ids(&run_json(
            &p,
            &["list", "--milestone", "T-001", "--status", "all"]
        )),
        vec!["T-002", "T-003", "T-004", "T-005"]
    );
    assert_eq!(
        listed_ids(&run_json(
            &p,
            &["list", "--milestone", "Lighting", "--status", "all"]
        )),
        vec!["T-003", "T-005"]
    );
    assert_refused(&p, &["list", "--milestone", "T-006"], "BAD_VALUE");
    assert_refused(&p, &["list", "--milestone", "T-999"], "NOT_FOUND");
}

#[test]
fn list_kind_filters_by_kind_with_a_comma_separated_list_and_refuses_an_unknown_kind() {
    let p = proj(&tree_fixture());
    assert_eq!(
        listed_ids(&run_json(
            &p,
            &["list", "--kind", "milestone", "--status", "all"]
        )),
        vec!["T-001", "T-002"]
    );
    assert_eq!(
        listed_ids(&run_json(&p, &["list", "--kind", "question"])),
        vec!["T-008"]
    );
    assert_eq!(
        listed_ids(&run_json(&p, &["list", "--kind", "IDEA"])),
        vec!["T-009"]
    );
    assert_eq!(
        listed_ids(&run_json(&p, &["list", "--kind", "task"])),
        vec!["T-004", "T-006"]
    );
    assert_eq!(
        listed_ids(&run_json(&p, &["list", "--kind", "task,milestone"])),
        vec!["T-001", "T-002", "T-004", "T-006"]
    );
    assert_refused(&p, &["list", "--kind", "epic"], "BAD_VALUE");
}

#[test]
fn summary_lists_top_level_milestones_that_are_not_finished_with_their_progress() {
    let p = proj(&tree_fixture());
    let out = run(&p, &["summary"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(
        stdout_text(&out)
            .contains("milestones (1):\n  T-001 [P1] · Develop the renderer · 1/2 done, open\n"),
        "{}",
        stdout_text(&out)
    );

    let body = format!(
        "{FIXTURE}## Now\n\n\
### T-011 · Open top\n- status: open\n- owner: —\n- priority: P1 · type: milestone\n- goal: a\n\n\
### T-012 · Blocked top\n- status: blocked\n- owner: —\n- priority: P1 · type: milestone\n- goal: b\n\n\
### T-013 · Done top\n- status: done\n- owner: —\n- priority: P1 · type: milestone\n- goal: c\n\n\
### T-014 · Dropped top\n- status: dropped\n- owner: —\n- priority: P1 · type: milestone\n- goal: d\n\n\
### T-015 · Open top with a done task\n- status: open\n- owner: —\n- priority: P1 · type: milestone\n- goal: e\n\n\
### T-016 · Done task below\n- status: done\n- owner: —\n- priority: P1\n- milestone: T-015\n"
    );
    let p = proj(&body);
    let data = run_json(&p, &["summary"]);
    let ids: Vec<&str> = data["milestones"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["T-011", "T-012", "T-015"]);
    assert_eq!(data["milestones"][2]["progress"]["done"], 1);
    assert_eq!(data["milestones"][2]["progress"]["total"], 1);
}

#[test]
fn summary_with_sixty_milestones_and_a_full_file_stays_within_40_lines() {
    let mut content = format!("{FIXTURE}## Tasks\n\n");
    for i in 1..=60 {
        content.push_str(&format!(
            "### T-{i:03} · Milestone {i}\n- status: open\n- owner: —\n- priority: P1 · type: milestone\n- goal: goal {i}\n\n"
        ));
    }
    for i in 61..=120 {
        let extra = match i % 3 {
            0 => "- owner: alice\n- claimed: 2026-11-15T18:00+01:00\n",
            1 => "- note (2026-11-15, agent-1): blocked — waiting\n",
            _ => "",
        };
        let status = match i % 3 {
            0 => "claimed",
            1 => "blocked",
            _ => "open",
        };
        content.push_str(&format!(
            "### T-{i:03} · Task {i}\n- status: {status}\n- priority: P2\n- milestone: T-001\n{extra}\n"
        ));
    }
    for i in 121..=130 {
        content.push_str(&format!(
            "### T-{i:03} · Question {i}\n- status: open\n- owner: —\n- priority: P2 · type: question\n\n"
        ));
    }
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("TODO.md"), content).unwrap();
    let out = run_todo(dir.path(), dir.path(), &["summary"], &[]).unwrap();
    assert_eq!(out.status.code(), Some(0));
    let text = stdout_text(&out);
    assert!(
        text.lines().count() <= 40,
        "summary has {} lines:\n{text}",
        text.lines().count()
    );
    assert!(text.contains("milestones (60):"), "{text}");
    assert!(text.contains("... and 55 more milestones"), "{text}");
}
