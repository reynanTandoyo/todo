//! CLI tests for the milestone chain shown with each task (T-089; R-MILE-4, TEST-4): `show`,
//! `next`, `claim` and `next --claim` print the milestones above an entry, from the top down, each
//! with its ID, title and goal, and their JSON carries `milestones` in the same order. Entries
//! without a milestone print exactly what they printed before. Every fixture is invented.

mod common;

use std::path::PathBuf;
use std::process::Output;

use serde_json::Value;
use tempfile::TempDir;

use common::{parse_json, read_todo, run_todo, setup_proj, stdout_text};

/// Fixed clock for the write commands (`TODO_TEST_NOW`, test-hooks feature).
const NOW: &str = "2026-11-18T10:00:00+01:00";

const FIXTURE: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->\n";

const RENDERER_GOAL: &str = "the test scene renders with lighting at 60 frames per second";
const LIGHTING_GOAL: &str = "point and directional lights match the reference images";

/// T-001 is a milestone holding the milestone T-002 and the task T-004; T-002 holds the task T-003.
/// T-005 is a task without a milestone.
fn tree() -> String {
    format!(
        "{FIXTURE}## Now\n\n\
### T-001 · Develop the renderer\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: milestone\n\
- goal: {RENDERER_GOAL}\n\
- note (2026-11-15, planner): added\n\n\
### T-002 · Lighting\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: milestone\n\
- milestone: T-001\n\
- goal: {LIGHTING_GOAL}\n\
- note (2026-11-15, planner): added\n\n\
### T-003 · C shader\n\
- status: open\n\
- owner: —\n\
- priority: P1 · difficulty: 2 · type: feature · area: cli\n\
- milestone: T-002\n\
- note (2026-11-15, planner): added\n\n\
### T-004 · GPU compiler\n\
- status: open\n\
- owner: —\n\
- priority: P2 · difficulty: 2\n\
- milestone: T-001\n\
- note (2026-11-15, planner): added\n\n\
### T-005 · Write the README\n\
- status: open\n\
- owner: —\n\
- priority: P2 · difficulty: 2\n\
- note (2026-11-15, planner): added\n"
    )
}

/// The chain above T-003 in text: T-001 and then T-002, from the top down.
fn chain_above_t003() -> String {
    // No `\` line continuations here: they would also drop the two spaces that indent each goal.
    let mut lines = vec!["Milestones above T-003, from the top down:".to_string()];
    lines.push("- T-001 · Develop the renderer".to_string());
    lines.push(format!("  goal: {RENDERER_GOAL}"));
    lines.push("- T-002 · Lighting".to_string());
    lines.push(format!("  goal: {LIGHTING_GOAL}"));
    lines.iter().map(|line| format!("{line}\n")).collect()
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

/// Runs `todo <args>` with the fixed clock.
fn run(p: &Proj, args: &[&str]) -> Output {
    run_todo(&p.home, &p.work, args, &[("TODO_TEST_NOW", NOW)]).unwrap()
}

/// Runs `todo <args>` and returns standard output, asserting exit 0.
fn text(p: &Proj, args: &[&str]) -> String {
    let out = run(p, args);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    stdout_text(&out)
}

/// Runs `todo --json <args>` and returns the parsed envelope, asserting exit 0.
fn json(p: &Proj, args: &[&str]) -> Value {
    let mut all = vec!["--json"];
    all.extend_from_slice(args);
    let out = run(p, &all);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{all:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    parse_json(&out.stdout)
}

#[test]
fn show_prints_the_chain_after_the_block_of_a_task_below_milestones() {
    let p = proj(&tree());
    let expected = format!(
        "### T-003 · C shader\n\
- status: open\n\
- owner: —\n\
- priority: P1 · difficulty: 2 · type: feature · area: cli\n\
- milestone: T-002\n\
- note (2026-11-15, planner): added\n\n\
{}",
        chain_above_t003()
    );
    assert_eq!(text(&p, &["show", "T-003"]), expected);
}

#[test]
fn show_prints_questions_before_the_chain() {
    let body = format!(
        "{}\n### T-006 · Is the shader in scope?\n- status: open\n- owner: —\n- type: question\n- about: T-003\n- note (2026-11-15, agent-1): asked\n",
        tree()
    );
    let p = proj(&body);
    let expected = format!(
        "### T-003 · C shader\n\
- status: open\n\
- owner: —\n\
- priority: P1 · difficulty: 2 · type: feature · area: cli\n\
- milestone: T-002\n\
- note (2026-11-15, planner): added\n\n\
Questions about T-003:\n\
- T-006 · open · Is the shader in scope?\n\n\
{}",
        chain_above_t003()
    );
    assert_eq!(text(&p, &["show", "T-003"]), expected);
}

#[test]
fn show_of_an_entry_without_a_milestone_prints_exactly_the_block() {
    let p = proj(&tree());
    assert_eq!(
        text(&p, &["show", "T-005"]),
        "### T-005 · Write the README\n- status: open\n- owner: —\n- priority: P2 · difficulty: 2\n- note (2026-11-15, planner): added\n"
    );
    assert!(!text(&p, &["show", "T-001"]).contains("Milestones above"));
}

#[test]
fn show_json_lists_the_milestones_top_down_with_id_title_goal_and_status() {
    let p = proj(&tree());

    let below = json(&p, &["show", "T-003"]);
    assert_eq!(
        below["data"]["milestones"],
        serde_json::json!([
            {"id": "T-001", "title": "Develop the renderer", "goal": RENDERER_GOAL, "status": "open"},
            {"id": "T-002", "title": "Lighting", "goal": LIGHTING_GOAL, "status": "open"}
        ])
    );

    let top = json(&p, &["show", "T-005"]);
    assert_eq!(top["data"]["milestones"], serde_json::json!([]));
}

#[test]
fn next_prints_the_chain_after_the_task_line() {
    let p = proj(&tree());
    assert_eq!(
        text(&p, &["next"]),
        format!("T-003 · P1 · open · C shader\n{}", chain_above_t003())
    );

    let next = json(&p, &["next"]);
    assert_eq!(next["data"]["task"]["id"], "T-003");
    assert_eq!(next["data"]["task"]["milestones"][0]["id"], "T-001");
    assert_eq!(next["data"]["task"]["milestones"][1]["id"], "T-002");
}

#[test]
fn next_of_an_entry_without_a_milestone_prints_one_line() {
    let body = format!(
        "{FIXTURE}## Now\n\n### T-001 · Write the README\n- status: open\n- owner: —\n- priority: P2 · difficulty: 2\n- note (2026-11-15, planner): added\n"
    );
    let p = proj(&body);
    assert_eq!(
        text(&p, &["next"]),
        "T-001 · P2 · open · Write the README\n"
    );
}

#[test]
fn next_claim_prints_the_chain_and_claims_the_task() {
    let p = proj(&tree());
    assert_eq!(
        text(&p, &["next", "--claim", "--agent", "planner"]),
        format!(
            "T-003 · P1 · claimed · planner · C shader [claimed]\n{}",
            chain_above_t003()
        )
    );
    assert!(read_todo(&p.work).contains("- status: claimed\n"));

    let q = proj(&tree());
    let claimed = json(&q, &["next", "--claim", "--agent", "planner"]);
    assert_eq!(claimed["data"]["task"]["id"], "T-003");
    assert_eq!(claimed["data"]["task"]["milestones"][0]["id"], "T-001");
    assert_eq!(
        claimed["data"]["task"]["milestones"][1]["goal"],
        LIGHTING_GOAL
    );
}

#[test]
fn claim_prints_the_chain_after_the_result() {
    let p = proj(&tree());
    assert_eq!(
        text(&p, &["claim", "T-004", "--agent", "planner"]),
        format!(
            "T-004: claim ok\nMilestones above T-004, from the top down:\n- T-001 · Develop the renderer\n  goal: {RENDERER_GOAL}\n"
        )
    );
}

#[test]
fn claim_json_carries_the_chain_and_claim_of_a_top_level_task_prints_as_before() {
    let p = proj(&tree());

    let claimed = json(&p, &["claim", "T-004", "--agent", "planner"]);
    assert_eq!(claimed["data"]["id"], "T-004");
    assert_eq!(
        claimed["data"]["milestones"],
        serde_json::json!([
            {"id": "T-001", "title": "Develop the renderer", "goal": RENDERER_GOAL, "status": "open"}
        ])
    );

    assert_eq!(
        text(&p, &["claim", "T-005", "--agent", "planner"]),
        "T-005: claim ok\n"
    );
}

#[test]
fn a_milestone_without_a_goal_prints_a_dash() {
    let body = format!(
        "{FIXTURE}## Now\n\n\
### T-001 · Release notes\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: milestone\n\
- note (2026-11-15, planner): added\n\n\
### T-002 · Write the changelog\n\
- status: open\n\
- owner: —\n\
- priority: P2 · difficulty: 2\n\
- milestone: T-001\n\
- note (2026-11-15, planner): added\n"
    );
    let p = proj(&body);
    assert_eq!(
        text(&p, &["show", "T-002"]),
        "### T-002 · Write the changelog\n- status: open\n- owner: —\n- priority: P2 · difficulty: 2\n- milestone: T-001\n- note (2026-11-15, planner): added\n\nMilestones above T-002, from the top down:\n- T-001 · Release notes\n  goal: -\n"
    );
    let shown = json(&p, &["show", "T-002"]);
    assert!(shown["data"]["milestones"][0]["goal"].is_null());
}
