//! CLI integration tests for `todo waves`: the waves, lanes, longest chain, workers at once, the
//! entries apart, the markers of level and detail, the milestone view and the JSON shape (T-163;
//! R-WAVE-1, R-DEP-1, R-MILE-6, R-SCOPE-1, R-DETAIL-2, R-READ-4, TEST-4).

mod common;

use std::fs;
use std::path::Path;
use std::process::Output;

use common::{parse_json, read_todo, run_todo, setup_proj, stderr_text, stdout_text, write_toml};
use serde_json::Value;
use tempfile::TempDir;

/// The levels these tests use, set in `todo.toml` so that they do not depend on the default
/// difficulties. `1` is the lowest level, which needs complete detail.
const LEVELS: &str = "difficulties = [\"1\", \"2\", \"3\", \"4\", \"5\"]\n";

/// The detail a task at the lowest level needs (`why`, `accept`, `steps`, `verify`).
const DETAIL: &str = "\
- why: the reason for the work
- accept: the result is visible
- steps:
  1. open the file
  2. make the change
- verify: cargo test
";

/// A task block of an invented task list: heading, then the given field lines.
fn task(id: &str, title: &str, fields: &str) -> String {
    format!("### {id} · {title}\n{fields}\n")
}

/// A task list with the format line and a heading; `body` holds the task blocks.
fn doc(body: &str) -> String {
    format!(
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
         <!-- todo-format: 2 -->\n\
         # Waves fixture\n\n\
         ## Work\n\n{body}"
    )
}

/// Runs `todo waves` with `args` in `work` and returns the output.
fn waves(home: &Path, work: &Path, args: &[&str]) -> Output {
    let mut all = vec!["waves"];
    all.extend_from_slice(args);
    run_todo(home, work, &all, &[]).expect("run todo waves")
}

/// Runs `todo waves --json` and returns the parsed envelope.
fn waves_json(home: &Path, work: &Path, args: &[&str]) -> Value {
    let mut all = vec!["waves", "--json"];
    all.extend_from_slice(args);
    let out = run_todo(home, work, &all, &[]).expect("run todo waves --json");
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    parse_json(&out.stdout)
}

/// Sets up a project whose `todo.toml` uses the test levels, with `body` as the task list.
fn project(tmp: &TempDir, body: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let (home, work, _file) = setup_proj(tmp, &doc(body));
    write_toml(&work, LEVELS);
    (home, work)
}

/// The IDs of one lane, in order, as the JSON gives them.
fn lane_ids(lane: &Value) -> Vec<String> {
    lane.as_array()
        .expect("lane is an array")
        .iter()
        .map(|t| t["id"].as_str().expect("id").to_string())
        .collect()
}

/// The lanes of wave `number` as lists of IDs.
fn lanes_of_wave(json: &Value, number: u64) -> Vec<Vec<String>> {
    let wave = json["data"]["waves"]
        .as_array()
        .expect("waves")
        .iter()
        .find(|w| w["wave"] == number)
        .unwrap_or_else(|| panic!("no wave {number}"));
    wave["lanes"]
        .as_array()
        .expect("lanes")
        .iter()
        .map(lane_ids)
        .collect()
}

/// The wave number of task `id`, or `None` when it is not placed.
fn wave_of(json: &Value, id: &str) -> Option<u64> {
    json["data"]["waves"]
        .as_array()
        .expect("waves")
        .iter()
        .find(|w| {
            w["lanes"]
                .as_array()
                .expect("lanes")
                .iter()
                .flat_map(|lane| lane.as_array().expect("lane"))
                .any(|t| t["id"] == id)
        })
        .map(|w| w["wave"].as_u64().expect("wave number"))
}

/// The reason task `id` is listed apart, or `None` when it is not.
fn apart_reason(json: &Value, id: &str) -> Option<String> {
    json["data"]["apart"]
        .as_array()
        .expect("apart")
        .iter()
        .find(|a| a["id"] == id)
        .map(|a| a["reason"].as_str().expect("reason").to_string())
}

/// The `missing` list of task `id` in the waves or in the chain, or `None` when it is not shown.
fn missing_of(json: &Value, id: &str) -> Option<Vec<String>> {
    let placed = json["data"]["waves"]
        .as_array()
        .expect("waves")
        .iter()
        .flat_map(|w| w["lanes"].as_array().expect("lanes").iter())
        .flat_map(|lane| lane.as_array().expect("lane").iter());
    placed.into_iter().find(|t| t["id"] == id).map(|t| {
        t["missing"]
            .as_array()
            .expect("missing")
            .iter()
            .map(|m| m.as_str().expect("text").to_string())
            .collect()
    })
}

/// A task list that covers every kind of entry, for the snapshot and the text output.
const EVERYTHING: &str = "\
### T-001 · Parse the config file
- status: claimed
- owner: alice
- priority: P1
- difficulty: 1
- files: src/config.rs
- claimed: 2026-11-20T09:00+01:00
- why: the loader needs the parsed values
- accept: config tests pass
- steps:
  1. open the parser
  2. add the keys
- verify: cargo test config

### T-002 · Write the loader
- status: open
- priority: P1
- difficulty: 1
- files: src/config.rs, src/loader.rs
- depends: T-001
- why: the program reads the file
- accept: the loader tests pass
- steps:
  1. write the loader
- verify: cargo test loader

### T-003 · Document the keys
- status: open
- priority: P2
- difficulty: 2
- files: src/loader.rs, docs/config.md
- depends: T-001

### T-004 · Render the report
- status: open
- priority: P2
- difficulty: 1
- files: src/report/
- why: the report is a page
- accept: the page shows the counts
- steps:
  1. draw the table
- verify: cargo test report

### T-005 · Pick a colour theme
- status: open
- priority: P3

### T-006 · Wait for the API key
- status: blocked
- owner: —
- priority: P2
- note (2026-11-20, bob): blocked — waiting for the API key

### T-007 · Write the schema
- status: blocked
- owner: —
- priority: P1
- difficulty: 3
- note (2026-11-20, bob): stuck — the schema file does not exist

### T-008 · Wait for the key task
- status: open
- priority: P2
- difficulty: 3
- depends: T-006

### T-009 · Wait for an ID that is not there
- status: open
- priority: P2
- difficulty: 3
- depends: T-404

### T-010 · First of a cycle
- status: open
- priority: P2
- difficulty: 3
- depends: T-011

### T-011 · Second of a cycle
- status: open
- priority: P2
- difficulty: 3
- depends: T-010

### T-012 · Wait for the cycle
- status: open
- priority: P2
- difficulty: 3
- depends: T-011

### T-013 · Finished earlier
- status: done
- priority: P2
- difficulty: 3

### T-014 · Starts after a finished task
- status: open
- priority: P2
- difficulty: 3
- depends: T-013

### T-020 · Ship the report
- type: milestone
- status: open
- priority: P1
- goal: the report is shipped
- depends: T-001

### T-021 · Draw the page
- status: open
- priority: P1
- difficulty: 1
- milestone: T-020
- files: src/a.rs
- why: the page exists
- accept: the page is drawn
- steps:
  1. draw it
- verify: cargo test page

### T-030 · Old plan
- type: milestone
- status: blocked
- priority: P2
- goal: an old plan

### T-031 · Under the old plan
- status: open
- priority: P2
- difficulty: 3
- milestone: T-030

### T-040 · Should the report have a logo?
- type: question
- status: open
- priority: P2
";

#[test]
fn waves_json_matches_snapshot() {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(&tmp, EVERYTHING);
    let json = waves_json(&home, &work, &[]);
    insta::assert_json_snapshot!("waves_json", json);
}

#[test]
fn waves_json_has_the_schema_and_the_counts_of_the_fixture() {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(&tmp, EVERYTHING);
    let json = waves_json(&home, &work, &[]);
    assert_eq!(json["schema"], "todo.waves.v1");
    // Fifteen unfinished tasks: seven placed, eight apart. The milestones, the question and the
    // finished task are not counted.
    assert_eq!(json["data"]["tasks"], 15);
    assert_eq!(json["data"]["wave_count"], 2);
    assert_eq!(json["data"]["max_workers"], 4);
    assert!(json["data"]["milestone"].is_null());
}

#[test]
fn text_output_shows_the_waves_lanes_chain_and_the_apart_tasks() {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(&tmp, EVERYTHING);
    let out = waves(&home, &work, &[]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let text = stdout_text(&out);
    assert!(
        text.contains("up to 4 workers at once"),
        "header missing:\n{text}"
    );
    assert!(text.contains("wave 1: 4 tasks in 4 lanes"), "{text}");
    assert!(
        text.contains("  lane 1: T-001 Parse the config file [claimed by alice]"),
        "{text}"
    );
    // T-002 and T-003 share src/loader.rs, so they share a lane and run one after the other.
    assert!(
        text.contains("  lane 1: T-002 Write the loader then T-003 Document the keys"),
        "{text}"
    );
    assert!(text.contains("T-004 Render the report [not ready: files: folder or pattern]"));
    assert!(text.contains("T-005 Pick a colour theme [not ready: level]"));
    assert!(
        text.contains("longest chain, 2 tasks: T-001 -> T-002"),
        "{text}"
    );
    assert!(text.contains("not in a wave (8):"), "{text}");
    assert!(
        text.contains("  T-006 Wait for the API key: blocked: waiting for the API key"),
        "{text}"
    );
    assert!(
        text.contains("  T-007 Write the schema: given back: the schema file does not exist"),
        "{text}"
    );
}

#[test]
fn wave_one_is_what_is_ready_now_and_a_task_waits_for_the_tasks_it_depends_on() {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(&tmp, EVERYTHING);
    let json = waves_json(&home, &work, &[]);
    assert_eq!(
        wave_of(&json, "T-001"),
        Some(1),
        "claimed tasks are running in wave 1"
    );
    assert_eq!(wave_of(&json, "T-002"), Some(2));
    assert_eq!(wave_of(&json, "T-003"), Some(2));
    assert_eq!(
        wave_of(&json, "T-014"),
        Some(1),
        "a finished dependency does not wait"
    );
    assert_eq!(
        wave_of(&json, "T-021"),
        Some(2),
        "the milestone's dependency is inherited"
    );
}

#[test]
fn tasks_that_share_files_form_one_lane_even_through_another_task() {
    let tmp = TempDir::new().expect("tempdir");
    let body = [
        task(
            "T-001",
            "Alpha",
            "- status: open\n- priority: P1\n- difficulty: 3\n- files: src/x.rs, src/y.rs\n",
        ),
        task(
            "T-002",
            "Beta",
            "- status: open\n- priority: P1\n- difficulty: 3\n- files: src/y.rs, src/z.rs\n",
        ),
        task(
            "T-003",
            "Gamma",
            "- status: open\n- priority: P1\n- difficulty: 3\n- files: src/z.rs\n",
        ),
        task(
            "T-004",
            "Delta, no files",
            "- status: open\n- priority: P1\n- difficulty: 3\n",
        ),
        task(
            "T-005",
            "Epsilon",
            "- status: open\n- priority: P1\n- difficulty: 3\n- files: src/w.rs\n",
        ),
    ]
    .concat();
    let (home, work) = project(&tmp, &body);
    let json = waves_json(&home, &work, &[]);
    assert_eq!(
        lanes_of_wave(&json, 1),
        vec![
            vec!["T-001", "T-002", "T-003"],
            vec!["T-004"],
            vec!["T-005"],
        ]
    );
    assert_eq!(json["data"]["max_workers"], 3);
    assert_eq!(json["data"]["waves"][0]["size"]["tasks"], 5);
    assert_eq!(json["data"]["waves"][0]["size"]["lanes"], 3);
}

#[test]
fn the_longest_chain_gives_the_number_of_waves_task_by_task() {
    let tmp = TempDir::new().expect("tempdir");
    let body = format!(
        "{}{}{}",
        task(
            "T-001",
            "First",
            "- status: open\n- priority: P1\n- difficulty: 3\n"
        ),
        task(
            "T-002",
            "Second",
            "- status: open\n- priority: P1\n- difficulty: 3\n- depends: T-001\n"
        ),
        task(
            "T-003",
            "Third",
            "- status: open\n- priority: P1\n- difficulty: 3\n- depends: T-002\n"
        ),
    );
    let (home, work) = project(&tmp, &body);
    let json = waves_json(&home, &work, &[]);
    assert_eq!(json["data"]["wave_count"], 3);
    assert_eq!(json["data"]["max_workers"], 1);
    let chain: Vec<&str> = json["data"]["longest_chain"]
        .as_array()
        .expect("chain")
        .iter()
        .map(|t| t["id"].as_str().expect("id"))
        .collect();
    assert_eq!(chain, vec!["T-001", "T-002", "T-003"]);
}

#[test]
fn a_dependency_on_a_milestone_waits_for_the_entries_below_it() {
    let tmp = TempDir::new().expect("tempdir");
    let body = format!(
        "{}{}{}{}",
        task(
            "T-100",
            "The milestone",
            "- type: milestone\n- status: open\n- priority: P1\n- goal: the thing\n"
        ),
        task(
            "T-101",
            "Child one",
            "- status: open\n- priority: P1\n- difficulty: 3\n- milestone: T-100\n"
        ),
        task(
            "T-102",
            "Child two",
            "- status: open\n- priority: P1\n- difficulty: 3\n- milestone: T-100\n- depends: T-101\n"
        ),
        task(
            "T-103",
            "Waits for the milestone",
            "- status: open\n- priority: P1\n- difficulty: 3\n- depends: T-100\n"
        ),
    );
    let (home, work) = project(&tmp, &body);
    let json = waves_json(&home, &work, &[]);
    assert_eq!(wave_of(&json, "T-101"), Some(1));
    assert_eq!(wave_of(&json, "T-102"), Some(2));
    assert_eq!(
        wave_of(&json, "T-103"),
        Some(3),
        "the milestone is finished only after the entries below it"
    );
}

#[test]
fn a_milestone_dependency_is_inherited_by_the_tasks_below_it() {
    let tmp = TempDir::new().expect("tempdir");
    let body = format!(
        "{}{}{}",
        task(
            "T-001",
            "Prerequisite",
            "- status: open\n- priority: P1\n- difficulty: 3\n"
        ),
        task(
            "T-100",
            "The milestone",
            "- type: milestone\n- status: open\n- priority: P1\n- goal: the thing\n- depends: T-001\n"
        ),
        task(
            "T-101",
            "Child",
            "- status: open\n- priority: P1\n- difficulty: 3\n- milestone: T-100\n"
        ),
    );
    let (home, work) = project(&tmp, &body);
    let json = waves_json(&home, &work, &[]);
    assert_eq!(wave_of(&json, "T-001"), Some(1));
    assert_eq!(wave_of(&json, "T-101"), Some(2));
}

#[test]
fn finished_dropped_and_archived_dependencies_do_not_make_a_task_wait() {
    let tmp = TempDir::new().expect("tempdir");
    let body = format!(
        "{}{}{}",
        task(
            "T-001",
            "Done before",
            "- status: done\n- priority: P1\n- difficulty: 3\n"
        ),
        task(
            "T-002",
            "Dropped before",
            "- status: dropped\n- priority: P1\n- difficulty: 3\n"
        ),
        task(
            "T-003",
            "Depends on all three",
            "- status: open\n- priority: P1\n- difficulty: 3\n- depends: T-001, T-002, T-900\n"
        ),
    );
    let (home, work) = project(&tmp, &body);
    // T-900 is archived and done, so it does not hold T-003 back either.
    fs::write(
        work.join("TODO-archive.md"),
        doc(&task(
            "T-900",
            "Archived",
            "- status: done\n- priority: P1\n- difficulty: 3\n",
        )),
    )
    .expect("write archive");
    let json = waves_json(&home, &work, &[]);
    assert_eq!(wave_of(&json, "T-003"), Some(1));
    assert_eq!(json["data"]["wave_count"], 1);
}

#[test]
fn blocked_tasks_and_tasks_below_a_blocked_milestone_are_listed_apart() {
    let tmp = TempDir::new().expect("tempdir");
    let body = format!(
        "{}{}{}{}{}",
        task(
            "T-001",
            "Blocked for now",
            "- status: blocked\n- owner: —\n- priority: P1\n- difficulty: 3\n- note (2026-11-20, bob): blocked — no key\n"
        ),
        task(
            "T-002",
            "Given back",
            "- status: blocked\n- owner: —\n- priority: P1\n- difficulty: 3\n- note (2026-11-20, bob): stuck — the file is gone\n"
        ),
        task(
            "T-010",
            "A blocked milestone",
            "- type: milestone\n- status: blocked\n- priority: P1\n- goal: stopped\n"
        ),
        task(
            "T-011",
            "Below it",
            "- status: open\n- priority: P1\n- difficulty: 3\n- milestone: T-010\n"
        ),
        task(
            "T-003",
            "Free to go",
            "- status: open\n- priority: P1\n- difficulty: 3\n"
        ),
    );
    let (home, work) = project(&tmp, &body);
    let json = waves_json(&home, &work, &[]);
    assert_eq!(
        apart_reason(&json, "T-001").as_deref(),
        Some("blocked: no key")
    );
    assert_eq!(
        apart_reason(&json, "T-002").as_deref(),
        Some("given back: the file is gone")
    );
    assert_eq!(
        apart_reason(&json, "T-011").as_deref(),
        Some("below blocked milestone T-010")
    );
    assert_eq!(wave_of(&json, "T-003"), Some(1));
    // The blocked milestone itself is not a task, so it is not listed.
    assert!(apart_reason(&json, "T-010").is_none());
}

#[test]
fn a_task_waiting_on_a_blocked_or_missing_task_is_apart_with_the_reason() {
    let tmp = TempDir::new().expect("tempdir");
    let body = format!(
        "{}{}{}",
        task(
            "T-001",
            "Blocked",
            "- status: blocked\n- owner: —\n- priority: P1\n- difficulty: 3\n- note (2026-11-20, bob): blocked — wait\n"
        ),
        task(
            "T-002",
            "Waits on blocked",
            "- status: open\n- priority: P1\n- difficulty: 3\n- depends: T-001\n"
        ),
        task(
            "T-003",
            "Waits on nothing there",
            "- status: open\n- priority: P1\n- difficulty: 3\n- depends: T-404\n"
        ),
    );
    let (home, work) = project(&tmp, &body);
    let json = waves_json(&home, &work, &[]);
    assert_eq!(
        apart_reason(&json, "T-002").as_deref(),
        Some("waits on T-001")
    );
    assert_eq!(
        apart_reason(&json, "T-003").as_deref(),
        Some("depends on T-404, which does not exist")
    );
}

#[test]
fn a_dependency_cycle_is_apart_and_the_tasks_waiting_on_it_say_so() {
    let tmp = TempDir::new().expect("tempdir");
    let body = format!(
        "{}{}{}",
        task(
            "T-001",
            "Ping",
            "- status: open\n- priority: P1\n- difficulty: 3\n- depends: T-002\n"
        ),
        task(
            "T-002",
            "Pong",
            "- status: open\n- priority: P1\n- difficulty: 3\n- depends: T-001\n"
        ),
        task(
            "T-003",
            "Behind the cycle",
            "- status: open\n- priority: P1\n- difficulty: 3\n- depends: T-002\n"
        ),
    );
    let (home, work) = project(&tmp, &body);
    let json = waves_json(&home, &work, &[]);
    assert_eq!(
        apart_reason(&json, "T-001").as_deref(),
        Some("in a dependency cycle: T-001 -> T-002 -> T-001")
    );
    assert_eq!(
        apart_reason(&json, "T-002").as_deref(),
        Some("in a dependency cycle: T-001 -> T-002 -> T-001")
    );
    assert_eq!(
        apart_reason(&json, "T-003").as_deref(),
        Some("waits on T-002")
    );
}

#[test]
fn a_task_that_waits_on_its_own_milestone_is_in_a_cycle() {
    let tmp = TempDir::new().expect("tempdir");
    let body = format!(
        "{}{}",
        task(
            "T-100",
            "The milestone",
            "- type: milestone\n- status: open\n- priority: P1\n- goal: the thing\n"
        ),
        task(
            "T-101",
            "Child that waits on the milestone",
            "- status: open\n- priority: P1\n- difficulty: 3\n- milestone: T-100\n- depends: T-100\n"
        ),
    );
    let (home, work) = project(&tmp, &body);
    let json = waves_json(&home, &work, &[]);
    assert_eq!(
        apart_reason(&json, "T-101").as_deref(),
        Some("in a dependency cycle: T-100 -> T-101 -> T-100")
    );
}

#[test]
fn a_task_without_a_level_is_placed_and_marked() {
    let tmp = TempDir::new().expect("tempdir");
    let body = task("T-001", "No level", "- status: open\n- priority: P1\n");
    let (home, work) = project(&tmp, &body);
    let json = waves_json(&home, &work, &[]);
    assert_eq!(wave_of(&json, "T-001"), Some(1));
    assert_eq!(missing_of(&json, "T-001"), Some(vec!["level".to_string()]));
}

#[test]
fn at_the_lowest_level_the_detail_rule_marks_what_is_missing() {
    let tmp = TempDir::new().expect("tempdir");
    let body = format!(
        "{}{}{}{}{}",
        // Complete: nothing missing.
        task(
            "T-001",
            "Complete",
            &format!(
                "- status: open\n- priority: P1\n- difficulty: 1\n- files: src/a.rs\n{DETAIL}"
            )
        ),
        // Missing why, steps and verify.
        task(
            "T-002",
            "Thin",
            "- status: open\n- priority: P1\n- difficulty: 1\n- files: src/b.rs\n- accept: done\n"
        ),
        // Four single files: too many.
        task(
            "T-003",
            "Too many files",
            &format!(
                "- status: open\n- priority: P1\n- difficulty: 1\n- files: a.rs, b.rs, c.rs, d.rs\n{DETAIL}"
            )
        ),
        // A pattern, which names more than one file.
        task(
            "T-005",
            "A pattern",
            &format!(
                "- status: open\n- priority: P1\n- difficulty: 1\n- files: src/*.rs\n{DETAIL}"
            )
        ),
        // Above the lowest level, nothing is checked.
        task(
            "T-004",
            "Level two",
            "- status: open\n- priority: P1\n- difficulty: 2\n"
        ),
    );
    let (home, work) = project(&tmp, &body);
    let json = waves_json(&home, &work, &[]);
    assert_eq!(missing_of(&json, "T-001"), Some(vec![]));
    assert_eq!(
        missing_of(&json, "T-002"),
        Some(vec![
            "why".to_string(),
            "steps".to_string(),
            "verify".to_string()
        ])
    );
    assert_eq!(
        missing_of(&json, "T-003"),
        Some(vec!["files: more than 3".to_string()])
    );
    assert_eq!(
        missing_of(&json, "T-005"),
        Some(vec!["files: folder or pattern".to_string()])
    );
    assert_eq!(missing_of(&json, "T-004"), Some(vec![]));
    // Marked tasks are still placed.
    assert_eq!(wave_of(&json, "T-002"), Some(1));
}

#[test]
fn the_milestone_view_shows_its_tasks_with_the_wave_numbers_of_the_whole_file() {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(&tmp, EVERYTHING);
    let json = waves_json(&home, &work, &["T-020"]);
    assert_eq!(json["data"]["milestone"], "T-020");
    // Only T-021 is below T-020; it waits for T-001 through the milestone, so it is in wave 2.
    assert_eq!(json["data"]["tasks"], 1);
    assert_eq!(wave_of(&json, "T-021"), Some(2));
    assert_eq!(wave_of(&json, "T-001"), None, "not below the milestone");
    assert_eq!(json["data"]["wave_count"], 2);
    let chain: Vec<&str> = json["data"]["longest_chain"]
        .as_array()
        .expect("chain")
        .iter()
        .map(|t| t["id"].as_str().expect("id"))
        .collect();
    assert_eq!(
        chain,
        vec!["T-001", "T-021"],
        "the chain that ends in the milestone's task"
    );
}

#[test]
fn the_milestone_argument_must_be_a_milestone_or_exist() {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(&tmp, EVERYTHING);
    let not_a_milestone = waves(&home, &work, &["T-001", "--json"]);
    assert_eq!(not_a_milestone.status.code(), Some(1));
    let json = parse_json(&not_a_milestone.stdout);
    assert_eq!(json["error"]["code"], "BAD_VALUE");

    let missing = waves(&home, &work, &["T-999", "--json"]);
    assert_eq!(missing.status.code(), Some(1));
    let json = parse_json(&missing.stdout);
    assert_eq!(json["error"]["code"], "NOT_FOUND");
}

#[test]
fn a_file_without_unfinished_tasks_has_no_waves() {
    let tmp = TempDir::new().expect("tempdir");
    let body = task(
        "T-001",
        "Finished",
        "- status: done\n- priority: P1\n- difficulty: 1\n",
    );
    let (home, work) = project(&tmp, &body);
    let out = waves(&home, &work, &[]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    assert!(stdout_text(&out).contains("no unfinished task"));
    let json = waves_json(&home, &work, &[]);
    assert_eq!(json["data"]["tasks"], 0);
    assert_eq!(json["data"]["wave_count"], 0);
    assert_eq!(json["data"]["max_workers"], 0);
    assert_eq!(json["data"]["waves"], serde_json::json!([]));
    assert_eq!(json["data"]["longest_chain"], serde_json::json!([]));
}

#[test]
fn waves_reads_only_and_leaves_todo_md_byte_identical() {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(&tmp, EVERYTHING);
    let before = read_todo(&work);
    let out = waves(&home, &work, &[]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    assert_eq!(read_todo(&work), before);
}

/// `todo list --ready --json` as the IDs of the tasks `next` could hand out (T-171).
fn ready_ids(home: &Path, work: &Path) -> Vec<String> {
    let out = run_todo(home, work, &["list", "--ready", "--json"], &[]).expect("run list --ready");
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    parse_json(&out.stdout)["data"]["tasks"]
        .as_array()
        .expect("tasks")
        .iter()
        .map(|t| t["id"].as_str().expect("id").to_string())
        .collect()
}

/// Milestones that hold their tasks back (R-MILE-6): a done one, a dropped one, and an open one
/// nested under the done one. Plus a task free to go, a dependency on the done milestone, and a
/// task that waits on a held-back one.
const HELD_BACK: &str = "\
### T-100 · Done milestone
- type: milestone
- status: done
- priority: P1
- goal: finished

### T-101 · Open child of the done milestone
- status: open
- priority: P1
- difficulty: 3
- milestone: T-100

### T-110 · Dropped milestone
- type: milestone
- status: dropped
- priority: P1
- goal: dropped

### T-111 · Child of the dropped milestone
- status: open
- priority: P1
- difficulty: 3
- milestone: T-110

### T-120 · Open milestone under the done one
- type: milestone
- status: open
- priority: P1
- goal: a part of the work
- milestone: T-100

### T-121 · Nested under the open milestone
- status: open
- priority: P1
- difficulty: 3
- milestone: T-120

### T-130 · Free task
- status: open
- priority: P1
- difficulty: 3

### T-150 · Depends on the done milestone
- status: open
- priority: P1
- difficulty: 3
- depends: T-100

### T-160 · Waits on a held-back task
- status: open
- priority: P1
- difficulty: 3
- depends: T-101
";

#[test]
fn tasks_below_a_done_or_dropped_milestone_are_apart_as_next_holds_them_back() {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(&tmp, HELD_BACK);
    let json = waves_json(&home, &work, &[]);
    assert_eq!(
        apart_reason(&json, "T-101").as_deref(),
        Some("below done milestone T-100")
    );
    assert_eq!(
        apart_reason(&json, "T-111").as_deref(),
        Some("below dropped milestone T-110")
    );
    // The nearest milestone that holds the task back gives the reason, here through an open one.
    assert_eq!(
        apart_reason(&json, "T-121").as_deref(),
        Some("below done milestone T-100")
    );
    assert_eq!(
        apart_reason(&json, "T-160").as_deref(),
        Some("waits on T-101")
    );
    for held in ["T-101", "T-111", "T-121", "T-160"] {
        assert_eq!(wave_of(&json, held), None, "{held} is in no wave");
    }
    assert_eq!(wave_of(&json, "T-130"), Some(1));
    assert_eq!(
        wave_of(&json, "T-150"),
        Some(1),
        "a dependency on a done milestone is satisfied"
    );
    assert_eq!(ready_ids(&home, &work), vec!["T-130", "T-150"]);
}

/// Every task that `next` could hand out is in wave 1, on both the fixture with held-back milestones
/// and the one that covers every kind of entry (T-171: waves and next use one readiness rule).
#[test]
fn every_task_next_can_hand_out_is_placed_in_wave_one() {
    for body in [HELD_BACK, EVERYTHING] {
        let tmp = TempDir::new().expect("tempdir");
        let (home, work) = project(&tmp, body);
        let json = waves_json(&home, &work, &[]);
        let ready = ready_ids(&home, &work);
        assert!(!ready.is_empty(), "the fixture has tasks next can hand out");
        for id in &ready {
            assert_eq!(
                wave_of(&json, id),
                Some(1),
                "{id} is ready, so it is in wave 1"
            );
        }
    }
}
