//! CLI integration tests for `todo brief` and for the work order that `next --claim --brief` and
//! `claim --brief` print after a claim: the parts in the order of R-DETAIL-3, the goals above the task
//! (R-MILE-4), the finish and give-up commands, the snapshot of the JSON, and that a brief reads only
//! and takes no lock (R-LOCK-5). Every fixture is invented (TEST-4).

mod common;

use std::path::{Path, PathBuf};
use std::process::Output;

use common::{parse_json, read_todo, run_todo, setup_proj, stderr_text, stdout_text, write_toml};
use serde_json::Value;
use tempfile::TempDir;

/// An invented task list: two milestones above a done dependency and the task the work orders are
/// for, which names two files, one of them with a space, and two steps.
const WORK: &str = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 3 -->
# Brief fixture

## Work

### T-010 · Develop the renderer
- status: open
- owner: —
- priority: P1 · type: milestone
- goal: the test scene renders with lighting

### T-011 · Lighting
- status: open
- owner: —
- priority: P1 · type: milestone
- milestone: T-010
- goal: point and directional lights match the reference images

### T-020 · Parse the config file
- status: done
- owner: alice
- priority: P1 · difficulty: 1
- milestone: T-011
- files: src/config.rs
- why: the loader needs the parsed values
- accept: config tests pass
- steps:
  1. open the parser
- verify: cargo test config
- commit: 1a2b3c4d
- note (2026-11-20, alice): done — the parser reads every key

### T-021 · Diffuse term in the C shader
- status: open
- owner: —
- priority: P1 · difficulty: 1
- milestone: T-011
- files: src/shader.c, tests/shader test.c
- read: src/light.h, docs
- depends: T-020
- why: lit surfaces need the diffuse term
- accept: the shader test passes
- steps:
  1. open src/shader.c
  2. add the diffuse term
- verify: cargo test shader
";

/// Sets up a project folder with `body` as its TODO.md, and returns the home and work folders.
fn project(tmp: &TempDir, body: &str) -> (PathBuf, PathBuf) {
    let (home, work, _file) = setup_proj(tmp, body);
    (home, work)
}

/// Runs `todo` with `args` in `work`.
fn run(home: &Path, work: &Path, args: &[&str]) -> Output {
    run_todo(home, work, args, &[]).expect("run todo")
}

/// Runs `todo` with `args`, which must succeed, and returns its standard output as text.
fn text_of(home: &Path, work: &Path, args: &[&str]) -> String {
    let out = run(home, work, args);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    stdout_text(&out)
}

/// Runs `todo --json` with `args`, which must succeed, and returns the parsed envelope.
fn json_of(home: &Path, work: &Path, args: &[&str]) -> Value {
    let mut all = vec!["--json"];
    all.extend_from_slice(args);
    let out = run(home, work, &all);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    parse_json(&out.stdout)
}

/// Asserts that every needle occurs in `text`, each after the one before it.
fn assert_in_order(text: &str, needles: &[&str]) {
    let mut from = 0;
    for needle in needles {
        let found = text[from..].find(needle);
        assert!(
            found.is_some(),
            "expected {needle:?} after byte {from} in:\n{text}"
        );
        from += found.unwrap_or(0) + needle.len();
    }
}

#[test]
fn brief_prints_the_parts_in_the_order_of_r_detail_3() {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(&tmp, WORK);
    let text = text_of(&home, &work, &["--agent", "worker-1", "brief", "T-021"]);
    assert_in_order(
        &text,
        &[
            "Milestones above T-021, from the top down:",
            "- T-010 · Develop the renderer",
            "  goal: the test scene renders with lighting",
            "- T-011 · Lighting",
            "  goal: point and directional lights match the reference images",
            "T-021 · Diffuse term in the C shader",
            "why: lit surfaces need the diffuse term",
            "Files to read (read):",
            "- src/light.h",
            "- docs",
            "Files you may change (files):",
            "- src/shader.c",
            "- tests/shader test.c",
            "No other file may be changed.",
            "Steps:",
            "1. open src/shader.c",
            "2. add the diffuse term",
            "Verify: cargo test shader",
            "done runs it",
            "Finished dependencies:",
            "- T-020 · Parse the config file",
            "done note: the parser reads every key",
            "commit: 1a2b3c4d",
            "Finish:",
            "git add -- src/shader.c 'tests/shader test.c'",
            "git commit -m 'T-021: Diffuse term in the C shader'",
            "todo done T-021 --commit HEAD --agent worker-1",
            "Give up, if you cannot finish:",
            "todo stuck T-021 --note \"<problem>\" --agent worker-1",
        ],
    );
}

#[test]
fn brief_of_a_bare_task_says_what_is_missing_and_leaves_the_agent_as_a_placeholder() {
    let body = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 3 -->
# Bare fixture

## Work

### T-030 · Pick a colour theme
- status: open
- priority: P2 · difficulty: 3
";
    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(&tmp, body);
    let text = text_of(&home, &work, &["brief", "T-030"]);
    assert!(
        text.starts_with("T-030 · Pick a colour theme\nwhy: -\n"),
        "{text}"
    );
    assert!(
        text.contains("Files to read (read): none listed."),
        "{text}"
    );
    assert!(
        text.contains(
            "Files you may change (files): none listed, so the task may change any file."
        ),
        "{text}"
    );
    assert!(!text.contains("No other file may be changed."), "{text}");
    assert!(text.contains("Steps: none written."), "{text}");
    assert!(
        text.contains("Verify: none; nothing checks this task."),
        "{text}"
    );
    assert!(text.contains("Finished dependencies: none."), "{text}");
    assert!(!text.contains("Milestones above"), "{text}");
    assert_in_order(
        &text,
        &[
            "Finish:",
            "git add -- <the files you changed>",
            "git commit -m 'T-030: Pick a colour theme'",
            "todo done T-030 --commit HEAD --agent <agent>",
            "Give up, if you cannot finish:",
            "todo stuck T-030 --note \"<problem>\" --agent <agent>",
        ],
    );
}

#[test]
fn brief_quotes_titles_for_powershell() {
    let body = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 3 -->
# Quote fixture

## Work

### T-031 · Don't stop the loop
- status: open
- priority: P2 · difficulty: 2
";
    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(&tmp, body);
    let text = text_of(&home, &work, &["brief", "T-031"]);
    assert!(
        text.contains("git commit -m 'T-031: Don''t stop the loop'"),
        "{text}"
    );
}

#[test]
fn brief_says_whether_done_runs_the_check_or_the_wave_does() {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(&tmp, WORK);
    write_toml(&work, "verify_when = \"wave\"\n");
    let text = text_of(&home, &work, &["brief", "T-021"]);
    assert!(
        text.contains(
            "Verify: cargo test shader\n  todo verify --wave runs it after the wave, not done\n"
        ),
        "{text}"
    );

    write_toml(&work, "verify = false\n");
    let text = text_of(&home, &work, &["brief", "T-021"]);
    assert!(
        text.contains("Verify: cargo test shader\n  todo never runs it (verify = false)\n"),
        "{text}"
    );
}

#[test]
fn brief_json_has_the_schema_the_parts_and_matches_its_snapshot() {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(&tmp, WORK);
    let json = json_of(&home, &work, &["--agent", "worker-1", "brief", "T-021"]);
    assert_eq!(json["schema"], "todo.brief.v1", "{json}");
    assert_eq!(json["data"]["id"], "T-021", "{json}");
    assert_eq!(json["data"]["commands"]["agent"], "worker-1", "{json}");
    assert_eq!(
        json["data"]["commands"]["finish"].as_array().map(Vec::len),
        Some(3),
        "{json}"
    );
    insta::assert_json_snapshot!("brief_json", json);
}

#[test]
fn brief_refuses_a_milestone_and_an_unknown_task() {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(&tmp, WORK);

    let milestone = run(&home, &work, &["--json", "brief", "T-010"]);
    assert_eq!(milestone.status.code(), Some(1));
    let json = parse_json(&milestone.stdout);
    assert_eq!(json["error"]["code"], "BAD_VALUE", "{json}");

    let missing = run(&home, &work, &["--json", "brief", "T-999"]);
    assert_eq!(missing.status.code(), Some(1));
    assert_eq!(parse_json(&missing.stdout)["error"]["code"], "NOT_FOUND");
}

#[test]
fn brief_takes_an_id_a_number_or_part_of_a_title_and_gives_the_same_text() {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(&tmp, WORK);
    let by_id = text_of(&home, &work, &["brief", "T-021"]);
    assert_eq!(text_of(&home, &work, &["brief", "21"]), by_id);
    assert_eq!(text_of(&home, &work, &["brief", "Diffuse"]), by_id);
}

#[test]
fn brief_reads_only_and_takes_no_lock() {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(&tmp, WORK);
    let before = read_todo(&work);

    // Another agent holds the manual editing lease (R-LOCK-5): reads still work.
    let lease = run(
        &home,
        &work,
        &["--agent", "alice", "lock", "--minutes", "5"],
    );
    assert!(lease.status.success(), "stderr: {}", stderr_text(&lease));
    let text = text_of(&home, &work, &["brief", "T-021"]);
    assert!(text.contains("Finish:"), "{text}");
    assert_eq!(read_todo(&work), before);
}

#[test]
fn next_claim_brief_prints_the_claim_line_and_then_the_work_order() {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(&tmp, WORK);
    let next = text_of(
        &home,
        &work,
        &["--agent", "worker-1", "next", "--claim", "--brief"],
    );
    let (first, rest) = next.split_once('\n').unwrap_or_default();
    assert_eq!(
        first,
        "T-021 \u{00B7} P1 \u{00B7} claimed \u{00B7} worker-1 \u{00B7} Diffuse term in the C shader [claimed]"
    );
    let brief = text_of(&home, &work, &["--agent", "worker-1", "brief", "T-021"]);
    assert_eq!(rest, brief);
    assert!(read_todo(&work).contains("- owner: worker-1"));
}

#[test]
fn next_brief_without_a_claim_prints_the_work_order_and_claims_nothing() {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(&tmp, WORK);
    let before = read_todo(&work);
    let next = text_of(&home, &work, &["--agent", "worker-1", "next", "--brief"]);
    let (first, rest) = next.split_once('\n').unwrap_or_default();
    assert_eq!(
        first,
        "T-021 \u{00B7} P1 \u{00B7} open \u{00B7} Diffuse term in the C shader"
    );
    let brief = text_of(&home, &work, &["--agent", "worker-1", "brief", "T-021"]);
    assert_eq!(rest, brief);
    assert_eq!(read_todo(&work), before);
}

#[test]
fn claim_brief_prints_the_claim_line_and_then_the_same_work_order_once() {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(&tmp, WORK);
    let claim = text_of(
        &home,
        &work,
        &["--agent", "worker-1", "claim", "T-021", "--brief"],
    );
    let (first, rest) = claim.split_once('\n').unwrap_or_default();
    assert_eq!(first, "T-021: claim ok");
    let brief = text_of(&home, &work, &["--agent", "worker-1", "brief", "T-021"]);
    assert_eq!(rest, brief);
    assert_eq!(claim.matches("Milestones above").count(), 1, "{claim}");
}

#[test]
fn json_carries_the_brief_only_when_it_is_asked_for() {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(&tmp, WORK);

    let plain = json_of(&home, &work, &["next"]);
    assert!(plain["data"].get("brief").is_none(), "{plain}");

    let claimed = json_of(
        &home,
        &work,
        &["--agent", "worker-1", "next", "--claim", "--brief"],
    );
    assert_eq!(claimed["data"]["claimed"], true, "{claimed}");
    assert_eq!(claimed["data"]["brief"]["id"], "T-021", "{claimed}");
    assert_eq!(claimed["data"]["brief"]["commands"]["agent"], "worker-1");

    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(&tmp, WORK);
    let claim = json_of(
        &home,
        &work,
        &["--agent", "worker-1", "claim", "T-021", "--brief"],
    );
    assert_eq!(claim["schema"], "todo.claim.v1", "{claim}");
    assert_eq!(claim["data"]["status"], "claimed", "{claim}");
    assert_eq!(claim["data"]["brief"]["id"], "T-021", "{claim}");
}
