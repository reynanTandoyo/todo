//! CLI integration tests for a task whose difficulty is set but is not one of the configured levels
//! (T-181; R-LEVEL-1, R-READ-3, R-LEVEL-2, R-WAVE-1, R-CHECK-1): `next`, `list --ready`, `show --json`,
//! `plan`, `waves` and `summary` all hold it back as they hold back a task without a level.

mod common;

use std::path::Path;
use std::process::Output;

use common::{parse_json, run_todo, setup_proj, stderr_text, stdout_text, write_toml};
use serde_json::Value;
use tempfile::TempDir;

/// The fields of an open task at `level` that has all of its detail, so that only its level can hold
/// it back.
fn ready_fields(level: Option<&str>) -> String {
    let level = level.map_or(String::new(), |l| format!("- difficulty: {l}\n"));
    format!(
        "- status: open\n- priority: P1\n{level}- why: the reason\n- accept: the result is visible\n\
         - steps:\n  1. one\n- verify: cargo test\n- files: src/a.rs\n"
    )
}

/// A task block of the invented task list: heading, then the field lines.
fn task(id: &str, fields: &str) -> String {
    format!("### {id} · Task {id}\n{fields}\n")
}

/// A task list with the format line and a `## Work` section holding `body`.
fn doc(body: &str) -> String {
    format!(
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
         <!-- todo-format: 3 -->\n\
         # Level fixture\n\n\
         ## Work\n\n{body}"
    )
}

/// Runs `todo` with `args` in `work` and returns the output.
fn todo(home: &Path, work: &Path, args: &[&str]) -> Output {
    run_todo(home, work, args, &[]).expect("run todo")
}

/// Runs `todo --json` with `args`, which must succeed, and returns the `data` of the envelope.
fn data(home: &Path, work: &Path, args: &[&str]) -> Value {
    let mut all = vec!["--json"];
    all.extend_from_slice(args);
    let out = todo(home, work, &all);
    assert!(out.status.success(), "{args:?}: {}", stderr_text(&out));
    parse_json(&out.stdout)["data"].clone()
}

/// The ID of the task `next` hands out, or `None`.
fn next_id(home: &Path, work: &Path, args: &[&str]) -> Option<String> {
    let mut all = vec!["next"];
    all.extend_from_slice(args);
    data(home, work, &all)["task"]["id"]
        .as_str()
        .map(str::to_string)
}

/// Whether `show --json` says task `id` is ready for `next`.
fn ready_of(home: &Path, work: &Path, id: &str) -> bool {
    data(home, work, &["show", id])["ready"]
        .as_bool()
        .expect("ready is a bool")
}

/// The IDs of the entries `plan` lists in `group`, in order.
fn plan_ids(home: &Path, work: &Path, group: &str) -> Vec<String> {
    plan_data(home, work)[group]
        .as_array()
        .expect("group is a list")
        .iter()
        .map(|item| item["id"].as_str().expect("id").to_string())
        .collect()
}

/// The fixture: T-001 at level 9 (not configured), T-002 at level 1, T-003 with no level, and each
/// of them has all of its detail.
fn three_tasks() -> String {
    [
        task("T-001", &ready_fields(Some("9"))),
        task("T-002", &ready_fields(Some("1"))),
        task("T-003", &ready_fields(None)),
    ]
    .concat()
}

#[test]
fn a_task_whose_level_is_not_configured_is_not_handed_out_like_one_without_a_level() {
    let tmp = TempDir::new().expect("temp dir");
    let (home, work, _) = setup_proj(&tmp, &doc(&three_tasks()));

    // Only the task at a configured level is ready: the other two have no usable level.
    assert_eq!(next_id(&home, &work, &[]).as_deref(), Some("T-002"));
    let listed = data(&home, &work, &["list", "--ready"]);
    let ids: Vec<&str> = listed["tasks"]
        .as_array()
        .expect("tasks")
        .iter()
        .filter_map(|t| t["id"].as_str())
        .collect();
    assert_eq!(ids, vec!["T-002"]);
    assert!(!ready_of(&home, &work, "T-001"));
    assert!(!ready_of(&home, &work, "T-003"));
    assert!(ready_of(&home, &work, "T-002"));

    // The same holds for `next --claim` and for a task the agent already holds: an unusable level
    // is not handed out even to its own claim.
    let claim = todo(&home, &work, &["--agent", "alice", "claim", "T-001"]);
    assert!(claim.status.success(), "{}", stderr_text(&claim));
    assert_eq!(
        next_id(&home, &work, &["--agent", "alice", "--new"]).as_deref(),
        Some("T-002")
    );
    assert_eq!(
        next_id(&home, &work, &["--agent", "alice"]).as_deref(),
        Some("T-002")
    );
}

#[test]
fn plan_lists_a_task_with_an_unconfigured_level_in_the_without_a_level_group_with_its_reason() {
    let tmp = TempDir::new().expect("temp dir");
    let (home, work, _) = setup_proj(&tmp, &doc(&three_tasks()));

    assert_eq!(
        plan_ids(&home, &work, "without_level"),
        vec!["T-001", "T-003"]
    );
    let json = plan_data(&home, &work);
    assert_eq!(json["clean"], false);
    let reason = json["without_level"][0]["reason"].as_str().expect("reason");
    assert!(
        reason.starts_with("level 9 is not configured (configured: 1, 2, 3, 4, 5)"),
        "{reason}"
    );
    assert!(
        json["without_level"][1]["reason"]
            .as_str()
            .is_some_and(|r| r.starts_with("no level: give it one")),
        "{json}"
    );

    let text = stdout_text(&todo(&home, &work, &["plan"]));
    assert!(
        text.contains("T-001 · Task T-001 — level 9 is not configured"),
        "{text}"
    );
    // The task is in group 4 only, not in the detail group, although its detail is complete.
    assert!(plan_ids(&home, &work, "to_detail").is_empty());
}

#[test]
fn waves_holds_the_task_back_with_the_same_reason_and_summary_counts_it() {
    let tmp = TempDir::new().expect("temp dir");
    let (home, work, _) = setup_proj(&tmp, &doc(&three_tasks()));

    let waves = data(&home, &work, &["waves"]);
    let apart: Vec<(String, String)> = waves["apart"]
        .as_array()
        .expect("apart")
        .iter()
        .map(|a| {
            (
                a["id"].as_str().expect("id").to_string(),
                a["reason"].as_str().expect("reason").to_string(),
            )
        })
        .collect();
    let t001 = apart
        .iter()
        .find(|(id, _)| id == "T-001")
        .expect("T-001 is apart");
    assert!(
        t001.1.starts_with("level 9 is not configured"),
        "{}",
        t001.1
    );

    let summary = data(&home, &work, &["summary"]);
    assert_eq!(summary["without_level"], 2);
    assert_eq!(summary["next_ready"][0]["id"], "T-002");
}

#[test]
fn check_reports_the_value_and_a_configured_difficulty_makes_the_task_usable() {
    let tmp = TempDir::new().expect("temp dir");
    let body = [
        task("T-001", &ready_fields(Some("3"))),
        task("T-002", &ready_fields(Some("hard"))),
    ]
    .concat();
    let (home, work, _) = setup_proj(&tmp, &doc(&body));
    // Only `easy` and `hard` are configured: `3` is not, and `hard` is (in another case, too).
    write_toml(&work, "difficulties = [\"easy\", \"hard\"]\n");

    let check = todo(&home, &work, &["check"]);
    let report = format!("{}{}", stdout_text(&check), stderr_text(&check));
    assert!(
        report.contains("difficulty \"3\" is not in configured difficulties"),
        "{report}"
    );
    assert!(!ready_of(&home, &work, "T-001"));
    assert_eq!(next_id(&home, &work, &[]).as_deref(), Some("T-002"));

    // Configuring `3` makes the task usable, and its level is then a level like any other.
    write_toml(&work, "difficulties = [\"3\", \"hard\"]\n");
    assert!(ready_of(&home, &work, "T-001"));
    assert_eq!(next_id(&home, &work, &[]).as_deref(), Some("T-001"));

    // The configured spelling is matched ignoring case.
    write_toml(&work, "difficulties = [\"easy\", \"HARD\"]\n");
    assert!(ready_of(&home, &work, "T-002"));
}

/// The `data` of `todo --json plan`. Exit 1 is a list with work in it, which is still an answer.
fn plan_data(home: &Path, work: &Path) -> Value {
    let out = todo(home, work, &["--json", "plan"]);
    assert!(
        matches!(out.status.code(), Some(0 | 1)),
        "{}",
        stderr_text(&out)
    );
    parse_json(&out.stdout)["data"].clone()
}
