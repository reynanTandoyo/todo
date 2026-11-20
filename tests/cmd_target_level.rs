//! CLI tests for the settings `target_difficulty` and `level1_max_files` in `todo.toml` (T-154;
//! R-LEVEL-2, R-DETAIL-2, R-CFG-2). The detail rule is seen through `todo waves --json`, which lists
//! what each task at or below the target level lacks. Every fixture is invented (TEST-4).

mod common;

use std::path::Path;

use common::{parse_json, run_todo, setup_proj, write_toml};
use serde_json::Value;
use tempfile::TempDir;

/// Invented tasks: a bare level-1 task, a bare level-2 task, a level-1 task with three files and a
/// level-1 task with four files, each with the rest of the detail a task at the lowest level needs.
const LEVELS: &str = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 3 -->
# Target fixture

## Work

### T-001 · Bare level one
- status: open
- priority: P1 · difficulty: 1

### T-002 · Bare level two
- status: open
- priority: P1 · difficulty: 2

### T-003 · Level one with three files
- status: open
- priority: P1 · difficulty: 1
- files: src/a.rs, src/b.rs, src/c.rs
- why: the parts are small
- accept: the parts build
- steps:
  1. split the file
- verify: cargo test parts

### T-004 · Level one with four files
- status: open
- priority: P1 · difficulty: 1
- files: src/a.rs, src/b.rs, src/c.rs, src/d.rs
- why: the parts are small
- accept: the parts build
- steps:
  1. split the file
- verify: cargo test parts
";

/// Sets up a project with the invented tasks, and `settings` as its `todo.toml` (when not empty).
fn project(tmp: &TempDir, body: &str, settings: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let (home, work, _file) = setup_proj(tmp, body);
    if !settings.is_empty() {
        write_toml(&work, settings);
    }
    (home, work)
}

/// Runs `todo --json waves` and returns the parsed envelope.
fn waves(home: &Path, work: &Path) -> Value {
    let out = run_todo(home, work, &["--json", "waves"], &[]).expect("run todo waves");
    parse_json(&out.stdout)
}

/// What the task `id` lacks at the detail rule, as `todo waves --json` lists it: `None` when it is
/// not in a wave.
fn missing(json: &Value, id: &str) -> Option<Vec<String>> {
    let waves = json["data"]["waves"].as_array()?;
    for wave in waves {
        for lane in wave["lanes"].as_array()? {
            for task in lane.as_array()? {
                if task["id"] == id {
                    return task["missing"].as_array().map(|items| {
                        items
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect()
                    });
                }
            }
        }
    }
    None
}

#[test]
fn target_difficulty_defaults_to_the_lowest_configured_level() {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(&tmp, LEVELS, "");
    let json = waves(&home, &work);
    assert_eq!(
        missing(&json, "T-001"),
        Some(vec![
            "why".to_string(),
            "accept".to_string(),
            "steps".to_string(),
            "verify".to_string(),
            "files".to_string(),
        ])
    );
    // Level 2 is above the default target, so its missing detail is not checked.
    assert_eq!(missing(&json, "T-002"), Some(Vec::new()));
}

#[test]
fn target_difficulty_moves_the_detail_rule_up_to_the_named_level() {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(&tmp, LEVELS, "target_difficulty = \"2\"\n");
    let json = waves(&home, &work);
    assert_eq!(
        missing(&json, "T-002"),
        Some(vec![
            "why".to_string(),
            "accept".to_string(),
            "steps".to_string(),
            "verify".to_string(),
            "files".to_string(),
        ])
    );
    // A complete level-1 task still has nothing missing.
    assert_eq!(missing(&json, "T-003"), Some(Vec::new()));
}

#[test]
fn target_difficulty_is_the_first_of_a_configured_list_when_none_is_set() {
    let body = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 3 -->
# Named levels

## Work

### T-001 · Bare easy task
- status: open
- priority: P1 · difficulty: easy

### T-002 · Bare medium task
- status: open
- priority: P1 · difficulty: medium
";
    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(
        &tmp,
        body,
        "difficulties = [\"easy\", \"medium\", \"hard\"]\n",
    );
    let json = waves(&home, &work);
    assert!(
        missing(&json, "T-001").is_some_and(|m| !m.is_empty()),
        "{json}"
    );
    assert_eq!(missing(&json, "T-002"), Some(Vec::new()), "{json}");
}

#[test]
fn target_difficulty_must_be_one_of_the_configured_difficulties() {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(&tmp, LEVELS, "target_difficulty = \"9\"\n");
    let out = run_todo(&home, &work, &["--json", "waves"], &[]).expect("run todo waves");
    assert_eq!(out.status.code(), Some(1));
    let json = parse_json(&out.stdout);
    assert_eq!(json["error"]["code"], "BAD_VALUE", "{json}");
    assert!(
        json["error"]["message"]
            .as_str()
            .is_some_and(|m| m.contains("target_difficulty")),
        "{json}"
    );
}

#[test]
fn level1_max_files_sets_how_many_files_a_lowest_level_task_may_name() {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(&tmp, LEVELS, "");
    let json = waves(&home, &work);
    // The default is 3: three files pass, four do not.
    assert_eq!(missing(&json, "T-003"), Some(Vec::new()), "{json}");
    assert_eq!(
        missing(&json, "T-004"),
        Some(vec!["files: more than 3".to_string()]),
        "{json}"
    );

    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(&tmp, LEVELS, "level1_max_files = 2\n");
    let json = waves(&home, &work);
    assert_eq!(
        missing(&json, "T-003"),
        Some(vec!["files: more than 2".to_string()]),
        "{json}"
    );
}

#[test]
fn level1_max_files_must_be_at_least_one() {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work) = project(&tmp, LEVELS, "level1_max_files = 0\n");
    let out = run_todo(&home, &work, &["--json", "waves"], &[]).expect("run todo waves");
    assert_eq!(out.status.code(), Some(1));
    let json = parse_json(&out.stdout);
    assert_eq!(json["error"]["code"], "BAD_VALUE", "{json}");
}
