//! Checks that the JSON examples in `docs/json.md` have the keys that the real commands print
//! (REL-5; T-111, T-112). Each example is compared with the output of its command on a fixture
//! in a temporary folder.

// The helpers panic when the document or an example is wrong, which is how a test fails, as in
// the other test files (AGENTS.md "Test rules").
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

mod common;

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde_json::Value;
use tempfile::TempDir;

use common::{parse_json, run_in, stderr_text};

/// Fixture for the `show` example: milestone `T-011` under `T-010`, with a dependency, a
/// dependent, a question about it and a task below it.
const SHOW_FIXTURE: &str = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 2 -->
# Fixture

## Tasks

### T-003 · Setup
- status: done
- owner: —
- priority: P1 · type: feature
- note (2026-11-15, planner): done

### T-010 · Develop the renderer
- status: open
- owner: —
- priority: P1 · type: milestone
- goal: the test scene renders with lighting at 60 frames per second
- note (2026-11-15, planner): added

### T-011 · Lighting
- status: open
- owner: —
- priority: P1 · type: milestone
- milestone: T-010
- goal: point and directional lights match the reference images
- depends: T-003
- note (2026-11-15, planner): added

### T-012 · C shader
- status: open
- owner: —
- priority: P1 · difficulty: hard · type: feature · area: cli
- milestone: T-011
- why: the renderer has no shading stage yet
- note (2026-11-15, planner): added

### T-020 · Ship lighting
- status: open
- owner: —
- priority: P2 · type: feature
- depends: T-011
- note (2026-11-15, planner): added

## Questions

### T-005 · Are shadows in scope?
- status: open
- owner: —
- type: question
- about: T-011
- note (2026-11-15, planner): asked
";

/// `docs/json.md`, read from this repository.
fn json_doc() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("docs")
        .join("json.md");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The first `json` code block under the heading that starts with `heading`, parsed.
fn json_example(doc: &str, heading: &str) -> Value {
    let mut lines = doc.lines();
    assert!(
        lines.any(|line| line.starts_with(heading)),
        "no heading {heading:?} in docs/json.md"
    );
    let mut in_block = false;
    let mut body = String::new();
    for line in lines {
        if !in_block {
            if line == "```json" {
                in_block = true;
            } else if line.starts_with("## ") || line.starts_with("### ") {
                break;
            }
            continue;
        }
        if line == "```" {
            break;
        }
        body.push_str(line);
        body.push('\n');
    }
    assert!(
        in_block,
        "no json example under {heading:?} in docs/json.md"
    );
    serde_json::from_str(&body)
        .unwrap_or_else(|e| panic!("the example under {heading:?} is not JSON: {e}"))
}

/// Every key path in `value`: `depends`, `dependencies.satisfied`. A list contributes the keys of
/// its items as `[]`, so `milestones[].id` is the `id` of each milestone above the entry.
fn key_paths(value: &Value, prefix: &str, out: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                key_paths(child, &path, out);
                out.insert(path);
            }
        }
        Value::Array(items) => {
            for item in items {
                key_paths(item, &format!("{prefix}[]"), out);
            }
        }
        _ => {}
    }
}

/// Fails when the example and the real output do not have the same key paths.
fn assert_same_keys(example: &Value, real: &Value, command: &str) {
    let mut doc_keys = BTreeSet::new();
    key_paths(example, "", &mut doc_keys);
    let mut real_keys = BTreeSet::new();
    key_paths(real, "", &mut real_keys);
    let only_in_doc: Vec<&String> = doc_keys.difference(&real_keys).collect();
    let only_in_output: Vec<&String> = real_keys.difference(&doc_keys).collect();
    assert!(
        only_in_doc.is_empty() && only_in_output.is_empty(),
        "docs/json.md does not match `{command}`: only in docs {only_in_doc:?}; only in the output {only_in_output:?}"
    );
}

#[test]
fn show_example_in_json_md_has_the_keys_of_show_json() {
    let doc = json_doc();
    let example = json_example(&doc, "### `todo show <task> --json`");

    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("TODO.md"), SHOW_FIXTURE).expect("write TODO.md");
    let out = run_in(dir.path(), &["show", "T-011", "--json"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let real = parse_json(&out.stdout);

    assert_eq!(example["schema"], real["schema"]);
    assert_same_keys(&example["data"], &real["data"], "show <task> --json");
}

#[test]
fn where_example_in_json_md_has_the_keys_of_where_json() {
    let doc = json_doc();
    let example = json_example(&doc, "### `todo where --json`");

    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("TODO.md"),
        "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 2 -->
<!-- todo-tool: 0.6.0 -->
# Fixture

## Tasks

### T-001 · First task
- status: open
- owner: —
- priority: P1
",
    )
    .expect("write TODO.md");
    let out = run_in(dir.path(), &["where", "--json"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let real = parse_json(&out.stdout);

    assert_eq!(example["schema"], real["schema"]);
    assert_same_keys(&example["data"], &real["data"], "where --json");
}
