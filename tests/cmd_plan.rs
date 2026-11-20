//! CLI integration tests for `todo plan` (T-149; R-LEVEL-2, R-READ-4, R-LEVEL-3, R-VERIFY-2): the
//! six groups in order, what is left out, the exit code, the hardest level first, the target level,
//! the milestone view, `verify = false`, the read-only run, the JSON snapshot, and the planner's line
//! of `todo summary`.

mod common;

use std::fs;
use std::path::Path;
use std::process::Output;

use common::{parse_json, run_todo, setup_proj, stderr_text, stdout_text, write_toml};
use serde_json::{Value, json};
use tempfile::TempDir;

/// A task block of the invented task list: heading, then the field lines.
fn task(id: &str, fields: &str) -> String {
    format!("### {id} · Task {id}\n{fields}\n")
}

/// The fields of an open task at `level` that has all of its detail.
const READY: &str = "- why: the reason\n- accept: the result is visible\n- steps:\n  1. one\n- verify: cargo test\n- files: src/a.rs\n";

/// A task list with the format line and a `## Work` section holding `body`.
fn doc(body: &str) -> String {
    format!(
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
         <!-- todo-format: 3 -->\n\
         # Plan fixture\n\n\
         ## Work\n\n{body}"
    )
}

/// Runs `todo` with `args` in `work` and returns the output.
fn todo(home: &Path, work: &Path, args: &[&str]) -> Output {
    run_todo(home, work, args, &[]).expect("run todo")
}

/// Runs `todo --json plan` with `args` and returns the envelope. Exit 1 is a list with work in it, so
/// it is still an answer; any other exit code fails the test.
fn plan_json(home: &Path, work: &Path, args: &[&str]) -> Value {
    let mut all = vec!["--json", "plan"];
    all.extend_from_slice(args);
    let out = todo(home, work, &all);
    assert!(
        matches!(out.status.code(), Some(0 | 1)),
        "{args:?}: {}",
        stderr_text(&out)
    );
    parse_json(&out.stdout)
}

/// The IDs of one group of the plan's `data`, in order.
fn ids(json: &Value, group: &str) -> Vec<String> {
    json["data"][group]
        .as_array()
        .unwrap_or_else(|| panic!("{group} is a list in {json}"))
        .iter()
        .map(|item| item["id"].as_str().expect("id").to_string())
        .collect()
}

/// A task list with one task in each of the six groups and entries the plan leaves out.
fn all_groups() -> String {
    [
        // Group 2: above the target level (the lowest level is the target).
        task("T-001", "- status: open\n- priority: P1\n- difficulty: 4\n- why: big\n"),
        // Group 3: at the target level, the detail is incomplete.
        task("T-002", "- status: open\n- priority: P1\n- difficulty: 1\n- why: small\n"),
        // Group 1: given back, with its problem and attempts.
        task(
            "T-003",
            "- status: blocked\n- priority: P2\n- difficulty: 2\n- attempts: 2\n\
             - note (2026-11-15, alice): stuck — the build breaks\n",
        ),
        // Group 4: without a difficulty.
        task("T-004", "- status: open\n- priority: P2\n"),
        // Group 5: kept above the target level on purpose.
        task(
            "T-005",
            "- status: open\n- priority: P1\n- difficulty: 3\n- final: the design is open\n",
        ),
        // Group 6: done with a verify command and no verified line.
        task(
            "T-006",
            "- status: done\n- owner: alice\n- priority: P2\n- difficulty: 1\n- verify: cargo test\n",
        ),
        // Left out: claimed, dropped, done and verified, a milestone, an idea, a question, and a
        // ready task.
        task(
            "T-007",
            "- status: claimed\n- owner: bob\n- claimed: 2026-11-15T18:00+01:00\n- priority: P1\n- difficulty: 5\n",
        ),
        task("T-008", "- status: dropped\n- priority: P3\n"),
        task(
            "T-009",
            "- status: done\n- owner: alice\n- priority: P2\n- difficulty: 1\n- verify: cargo test\n- verified: 2026-11-15\n",
        ),
        task(
            "T-010",
            "- type: milestone\n- status: open\n- priority: P1\n- goal: the goal\n",
        ),
        task("T-011", "- type: idea\n- status: open\n- owner: —\n"),
        task(
            "T-012",
            "- type: question\n- status: open\n- owner: —\n- about: T-001\n",
        ),
        task(
            "T-013",
            &format!("- status: open\n- priority: P1\n- difficulty: 1\n{READY}"),
        ),
    ]
    .concat()
}

#[test]
fn plan_lists_the_six_groups_in_order_and_leaves_out_the_rest() {
    let tmp = TempDir::new().expect("temp dir");
    let (home, work, _) = setup_proj(&tmp, &doc(&all_groups()));

    let json = plan_json(&home, &work, &[]);
    assert_eq!(json["schema"], "todo.plan.v1");
    assert_eq!(ids(&json, "given_back"), vec!["T-003"]);
    assert_eq!(ids(&json, "to_split"), vec!["T-001"]);
    assert_eq!(ids(&json, "to_detail"), vec!["T-002"]);
    assert_eq!(ids(&json, "without_level"), vec!["T-004"]);
    assert_eq!(ids(&json, "kept_on_purpose"), vec!["T-005"]);
    assert_eq!(ids(&json, "unverified"), vec!["T-006"]);
    assert_eq!(json["data"]["clean"], false);

    // The entries of each group carry what the planner needs (R-LEVEL-2, R-STUCK-1, R-DETAIL-2).
    assert_eq!(json["data"]["given_back"][0]["problem"], "the build breaks");
    assert_eq!(json["data"]["given_back"][0]["attempts"], "2");
    assert_eq!(
        json["data"]["to_detail"][0]["missing"],
        json!(["accept", "steps", "verify", "files"])
    );
    assert_eq!(
        json["data"]["kept_on_purpose"][0]["reason"],
        "the design is open"
    );
    assert_eq!(json["data"]["unverified"][0]["verify"], "cargo test");

    // The text output names the groups in the same order.
    let text = stdout_text(&todo(&home, &work, &["plan"]));
    let headings = [
        "given back to the planner (1):",
        "to split, hardest first (1):",
        "to detail (1):",
        "without a level (1):",
        "kept above the target level on purpose (1), counted apart:",
        "done, not verified (1), counted apart:",
    ];
    let mut at = 0;
    for heading in headings {
        let found = text[at..]
            .find(heading)
            .unwrap_or_else(|| panic!("{heading} in order:\n{text}"));
        at += found + heading.len();
    }
    for left_out in [
        "T-007", "T-008", "T-009", "T-010", "T-011", "T-012", "T-013",
    ] {
        assert!(!text.contains(left_out), "{left_out} is left out:\n{text}");
        assert!(
            ids(&json, "given_back")
                .iter()
                .chain(&ids(&json, "to_split"))
                .chain(&ids(&json, "to_detail"))
                .all(|id| id != left_out),
            "{left_out}"
        );
    }
}

#[test]
fn exit_code_is_zero_only_when_the_first_four_groups_are_empty() {
    let tmp = TempDir::new().expect("temp dir");
    // Only the groups counted apart: exit 0, and both are still listed.
    let body = [
        task(
            "T-001",
            "- status: open\n- priority: P1\n- difficulty: 3\n- final: kept on purpose\n",
        ),
        task(
            "T-002",
            "- status: done\n- owner: alice\n- priority: P2\n- difficulty: 1\n- verify: cargo test\n",
        ),
    ]
    .concat();
    let (home, work, _) = setup_proj(&tmp, &doc(&body));
    let out = todo(&home, &work, &["plan"]);
    assert_eq!(out.status.code(), Some(0), "{}", stdout_text(&out));
    let text = stdout_text(&out);
    assert!(
        text.contains("kept above the target level on purpose (1)"),
        "{text}"
    );
    assert!(text.contains("done, not verified (1)"), "{text}");

    // A task that a worker gave back makes the exit 1.
    let body = task(
        "T-003",
        "- status: blocked\n- priority: P2\n- difficulty: 2\n- note (2026-11-15, alice): stuck — stuck\n",
    );
    fs::write(work.join("TODO.md"), doc(&body)).expect("rewrite TODO.md");
    let out = todo(&home, &work, &["plan"]);
    assert_eq!(out.status.code(), Some(1), "{}", stdout_text(&out));
}

#[test]
fn an_empty_list_says_so_and_exits_zero() {
    let tmp = TempDir::new().expect("temp dir");
    let body = task(
        "T-001",
        &format!("- status: open\n- priority: P1\n- difficulty: 1\n{READY}"),
    );
    let (home, work, _) = setup_proj(&tmp, &doc(&body));
    let out = todo(&home, &work, &["plan"]);
    assert_eq!(out.status.code(), Some(0), "{}", stdout_text(&out));
    assert!(
        stdout_text(&out).contains("nothing for the planner: every task can go to a worker"),
        "{}",
        stdout_text(&out)
    );
    let json = plan_json(&home, &work, &[]);
    assert_eq!(json["data"]["clean"], true);
}

#[test]
fn to_split_lists_the_hardest_level_first_and_keeps_file_order_for_equal_levels() {
    let tmp = TempDir::new().expect("temp dir");
    let body = [
        task("T-041", "- status: open\n- priority: P1\n- difficulty: 3\n"),
        task("T-042", "- status: open\n- priority: P1\n- difficulty: 5\n"),
        task("T-043", "- status: open\n- priority: P1\n- difficulty: 4\n"),
        task("T-044", "- status: open\n- priority: P1\n- difficulty: 3\n"),
    ]
    .concat();
    let (home, work, _) = setup_proj(&tmp, &doc(&body));
    let json = plan_json(&home, &work, &[]);
    assert_eq!(
        ids(&json, "to_split"),
        vec!["T-042", "T-043", "T-041", "T-044"]
    );
}

#[test]
fn a_configured_target_moves_tasks_between_the_groups() {
    // T-053 is above the default target (the lowest level), so it is to split; with the target at
    // level 2, level 2 is at the target and needs its detail, and level 3 is still above it.
    let tmp = TempDir::new().expect("temp dir");
    let body = [
        task("T-051", "- status: open\n- priority: P1\n- difficulty: 2\n"),
        task("T-052", "- status: open\n- priority: P1\n- difficulty: 1\n"),
        task("T-053", "- status: open\n- priority: P1\n- difficulty: 3\n"),
    ]
    .concat();
    let (home, work, _) = setup_proj(&tmp, &doc(&body));

    let json = plan_json(&home, &work, &[]);
    assert_eq!(ids(&json, "to_split"), vec!["T-053", "T-051"]);
    assert_eq!(ids(&json, "to_detail"), vec!["T-052"]);

    write_toml(&work, "target_difficulty = \"2\"\n");
    let json = plan_json(&home, &work, &[]);
    assert_eq!(ids(&json, "to_split"), vec!["T-053"]);
    assert_eq!(ids(&json, "to_detail"), vec!["T-051", "T-052"]);
}

#[test]
fn a_milestone_limits_the_list_and_a_task_that_is_not_one_is_refused() {
    let tmp = TempDir::new().expect("temp dir");
    let body = [
        task(
            "T-020",
            "- type: milestone\n- status: open\n- priority: P1\n- goal: the report\n",
        ),
        task(
            "T-021",
            "- status: open\n- priority: P1\n- difficulty: 3\n- milestone: T-020\n",
        ),
        task("T-022", "- status: open\n- priority: P1\n- difficulty: 4\n"),
    ]
    .concat();
    let (home, work, _) = setup_proj(&tmp, &doc(&body));

    let json = plan_json(&home, &work, &["T-020"]);
    assert_eq!(json["data"]["milestone"], "T-020");
    assert_eq!(ids(&json, "to_split"), vec!["T-021"]);

    let refused = todo(&home, &work, &["--json", "plan", "T-021"]);
    assert_eq!(refused.status.code(), Some(1));
    assert_eq!(parse_json(&refused.stdout)["error"]["code"], "BAD_VALUE");
}

#[test]
fn verify_false_leaves_the_unverified_group_out() {
    // R-VERIFY-2: nothing is listed while `verify = false`, since no check can pass then.
    let tmp = TempDir::new().expect("temp dir");
    let body = task(
        "T-061",
        "- status: done\n- owner: alice\n- priority: P2\n- difficulty: 1\n- verify: cargo test\n",
    );
    let (home, work, _) = setup_proj(&tmp, &doc(&body));
    assert_eq!(
        ids(&plan_json(&home, &work, &[]), "unverified"),
        vec!["T-061"]
    );

    write_toml(&work, "verify = false\n");
    let json = plan_json(&home, &work, &[]);
    assert_eq!(ids(&json, "unverified"), Vec::<String>::new());
    assert_eq!(json["data"]["clean"], true);
}

#[test]
fn plan_reads_only_and_the_file_stays_as_it_was() {
    let tmp = TempDir::new().expect("temp dir");
    let (home, work, file) = setup_proj(&tmp, &doc(&all_groups()));
    let before = fs::read(&file).expect("read TODO.md");

    let _ = todo(&home, &work, &["plan"]);
    let _ = todo(&home, &work, &["--json", "plan"]);
    assert_eq!(fs::read(&file).expect("read TODO.md"), before);
}

#[test]
fn plan_json_matches_snapshot() {
    let tmp = TempDir::new().expect("temp dir");
    let (home, work, _) = setup_proj(&tmp, &doc(&all_groups()));
    let json = plan_json(&home, &work, &[]);
    insta::assert_json_snapshot!("plan_json", json);
}

#[test]
fn summary_shows_the_planners_counts_from_the_same_list_as_plan() {
    let tmp = TempDir::new().expect("temp dir");
    let (home, work, _) = setup_proj(&tmp, &doc(&all_groups()));

    let out = todo(&home, &work, &["summary"]);
    let text = stdout_text(&out);
    let line = "planner: to split 1 · to detail 1 · without a level 1 · kept above the target level on purpose: 1";
    assert!(text.contains(line), "{text}");
    assert!(
        text.lines()
            .position(|l| l == line)
            .is_some_and(|at| at < 40),
        "the planner line is within the 40 lines: {text}"
    );

    let summary = parse_json(&todo(&home, &work, &["--json", "summary"]).stdout);
    assert_eq!(summary["data"]["to_split"], 1);
    assert_eq!(summary["data"]["to_detail"], 1);
    assert_eq!(summary["data"]["without_level"], 1);
    assert_eq!(summary["data"]["kept_on_purpose"], 1);
    // The given-back task is counted on its own line, not in the planner's counts.
    assert_eq!(summary["data"]["given_back"][0]["id"], "T-003");
}

#[test]
fn the_unverified_group_is_the_same_rule_check_warns_with() {
    let tmp = TempDir::new().expect("temp dir");
    let (home, work, _) = setup_proj(&tmp, &doc(&all_groups()));
    let check = todo(&home, &work, &["check"]);
    let report = format!("{}{}", stdout_text(&check), stderr_text(&check));
    assert!(
        report.contains("T-006: done, but its verify command has not passed"),
        "{report}"
    );
    assert!(!report.contains("T-009: done, but"), "{report}");
}
