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

use common::{parse_json, run_in, stderr_text, todo_bin};

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

/// Fixture for the `next` example: one open task, which `next` hands out.
const NEXT_FIXTURE: &str = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 2 -->
# Fixture

## Tasks

### T-001 · Write the parser
- status: open
- owner: —
- priority: P1 · difficulty: easy · type: feature · area: cli
- why: the reader needs it first
- note (2026-11-15, planner): added
";

#[test]
fn next_example_in_json_md_has_the_keys_of_next_json() {
    let doc = json_doc();
    let example = json_example(&doc, "### `todo next --json`");

    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("TODO.md"), NEXT_FIXTURE).expect("write TODO.md");
    let out = run_in(dir.path(), &["next", "--json"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let real = parse_json(&out.stdout);

    assert_eq!(example["schema"], real["schema"]);
    assert_same_keys(&example["data"], &real["data"], "next --json");
    assert_same_keys(
        &example["data"]["task"],
        &real["data"]["task"],
        "next --json task",
    );
}

#[test]
fn next_json_has_a_null_task_when_nothing_is_ready() {
    // The example says so in json.md: with no ready task, `task` is null and both flags are false.
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("TODO.md"),
        "<!-- todo-format: 2 -->\n\n## Tasks\n",
    )
    .expect("write TODO.md");
    let out = run_in(dir.path(), &["next", "--json"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let real = parse_json(&out.stdout);

    assert!(real["data"]["task"].is_null(), "{real}");
    assert_eq!(real["data"]["claimed"], false);
    assert_eq!(real["data"]["already_claimed"], false);
}

/// Fixture for the `tree` example: milestone `T-010` with a task below it, a question about the task
/// and an idea about the milestone, which `tree` lists for the whole file (R-KIND-3).
const TREE_FIXTURE: &str = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 2 -->
# Fixture

## Tasks

### T-010 · Develop the renderer
- status: open
- owner: —
- priority: P1 · type: milestone
- goal: the test scene renders with lighting at 60 frames per second
- note (2026-11-15, planner): added

### T-012 · C shader
- status: open
- owner: —
- priority: P1 · difficulty: hard · type: feature · area: cli
- milestone: T-010
- why: the renderer has no shading stage yet
- note (2026-11-15, planner): added

## Questions

### T-004 · Should the shader support shadows?
- status: open
- owner: —
- type: question
- about: T-012
- note (2026-11-15, planner): asked

## Ideas

### T-005 · Hot-reload shaders
- status: open
- owner: —
- type: idea
- about: T-010
- note (2026-11-15, planner): added
";

#[test]
fn tree_example_in_json_md_has_the_keys_of_tree_json() {
    let doc = json_doc();
    let example = json_example(&doc, "### `todo tree --json`");

    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("TODO.md"), TREE_FIXTURE).expect("write TODO.md");
    let out = run_in(dir.path(), &["tree", "--json"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let real = parse_json(&out.stdout);

    assert_eq!(example["schema"], real["schema"]);
    assert_same_keys(&example["data"], &real["data"], "tree --json");
}

/// Fixture for the `summary` example: an open milestone with a claimed task below it, a blocked
/// task, an open task, a question and an idea.
const SUMMARY_FIXTURE: &str = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 2 -->
# Fixture

## Now

### T-001 · Develop the renderer
- status: open
- owner: —
- priority: P1 · type: milestone
- goal: the test scene renders with lighting at 60 frames per second
- note (2026-11-15, planner): added

### T-002 · C shader
- status: claimed
- owner: alice
- claimed: 2026-11-15T18:00+01:00
- priority: P1 · difficulty: hard · type: feature
- milestone: T-001
- note (2026-11-15, planner): added

### T-003 · GPU compiler
- status: open
- owner: —
- priority: P2 · type: feature
- milestone: T-001
- note (2026-11-15, planner): added

### T-004 · Lighting pass
- status: blocked
- owner: —
- priority: P2 · type: feature
- note (2026-11-15, planner): added
- note (2026-11-15, planner): blocked — waiting on key

## Questions

### T-007 · Ship today?
- status: open
- owner: —
- type: question
- note (2026-11-15, planner): asked

## Ideas

### T-008 · Hot-reload shaders
- status: open
- owner: —
- type: idea
- note (2026-11-15, planner): added
";

#[test]
fn summary_example_in_json_md_has_the_keys_of_summary_json() {
    let doc = json_doc();
    let example = json_example(&doc, "### `todo summary --json`");

    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("TODO.md"), SUMMARY_FIXTURE).expect("write TODO.md");
    let out = run_in(dir.path(), &["summary", "--json"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let real = parse_json(&out.stdout);

    assert_eq!(example["schema"], real["schema"]);
    assert_same_keys(&example["data"], &real["data"], "summary --json");
}

/// Fixture for the `list` example: one open task.
const LIST_FIXTURE: &str = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 2 -->
# Fixture

## Tasks

### T-001 · First task
- status: open
- owner: —
- priority: P1
- note (2026-11-15, planner): added
";

#[test]
fn list_example_in_json_md_has_the_keys_of_list_json() {
    let doc = json_doc();
    let example = json_example(&doc, "### `todo list --json`");

    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("TODO.md"), LIST_FIXTURE).expect("write TODO.md");
    let out = run_in(dir.path(), &["list", "--json"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let real = parse_json(&out.stdout);

    assert_eq!(example["schema"], real["schema"]);
    assert_same_keys(&example["data"], &real["data"], "list --json");
    assert_same_keys(
        &example["data"]["tasks"][0],
        &real["data"]["tasks"][0],
        "list --json task",
    );
}

#[test]
fn migrate_example_in_json_md_has_the_keys_of_migrate_json() {
    let doc = json_doc();
    let example = json_example(&doc, "### `todo migrate --json`");

    // A format 1 file, and project files that are installed but out of date: the rules block,
    // the skill and one hook event, so every kind of `project` row appears.
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("TODO.md"),
        "<!-- todo-format: 1 -->\n\n## Tasks\n\n### T-001 · First task\n- status: open\n- owner: —\n- priority: P1\n",
    )
    .expect("write TODO.md");
    fs::write(
        dir.path().join("AGENTS.md"),
        "<!-- todo:begin -->\nOld rules.\n<!-- todo:end -->\n",
    )
    .expect("write AGENTS.md");
    let install = run_in(dir.path(), &["skill", "install", "--yes"]);
    assert!(
        install.status.success(),
        "stderr: {}",
        stderr_text(&install)
    );
    let skill = dir
        .path()
        .join(".claude")
        .join("skills")
        .join("todo")
        .join("SKILL.md");
    fs::write(&skill, "Old skill.\n").expect("make the skill out of date");
    let hooks = run_in(dir.path(), &["hooks", "install", "--yes"]);
    assert!(hooks.status.success(), "stderr: {}", stderr_text(&hooks));
    let settings = dir.path().join(".claude").join("settings.json");
    let text = fs::read_to_string(&settings).expect("read settings");
    fs::write(
        &settings,
        text.replacen("todo summary", "todo summary --old", 1),
    )
    .expect("make a hook out of date");

    let out = run_in(
        dir.path(),
        &["migrate", "--yes", "--agent", "docs-test", "--json"],
    );
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let real = parse_json(&out.stdout);

    assert_eq!(example["schema"], real["schema"]);
    assert_same_keys(&example["data"], &real["data"], "migrate --json");
}

#[test]
fn help_update_example_in_json_md_has_the_keys_of_help_update_json() {
    let doc = json_doc();
    let example = json_example(&doc, "### `todo help update --json`");

    let dir = TempDir::new().expect("tempdir");
    let out = run_in(dir.path(), &["help", "update", "--json"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let real = parse_json(&out.stdout);

    assert_eq!(example["schema"], real["schema"]);
    assert_same_keys(&example["data"], &real["data"], "help update --json");
    assert_same_keys(
        &example["data"]["windows"]["steps"][0],
        &real["data"]["windows"]["steps"][0],
        "help update --json windows step",
    );
}

/// A readable `TODO.md` for the project examples (REL-5, T-093).
const PROJECT_FIXTURE: &str = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 2 -->
<!-- todo-tool: 0.6.0 -->
# Fixture

## Tasks

### T-001 · First task
- status: open
- owner: —
- priority: P1
";

/// Writes `PROJECT_FIXTURE` as `<dir>/renderer/TODO.md` and returns the folder.
fn project_folder(dir: &Path) -> std::path::PathBuf {
    let folder = dir.join("renderer");
    fs::create_dir_all(&folder).expect("create project folder");
    fs::write(folder.join("TODO.md"), PROJECT_FIXTURE).expect("write TODO.md");
    folder
}

/// `todo` in `folder` with the register at `register`.
fn project_todo(folder: &Path, register: &Path, args: &[&str]) -> std::process::Output {
    todo_bin()
        .current_dir(folder)
        .env("TODO_REGISTER", register)
        .args(args)
        .output()
        .expect("run todo")
}

#[test]
fn project_add_example_in_json_md_has_the_keys_of_project_add_json() {
    let doc = json_doc();
    let example = json_example(&doc, "### `todo project add --json`");

    let dir = TempDir::new().expect("tempdir");
    let folder = project_folder(dir.path());
    let register = dir.path().join("register").join("projects.json");
    let out = project_todo(&folder, &register, &["--json", "project", "add"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let real = parse_json(&out.stdout);

    assert_eq!(example["schema"], real["schema"]);
    assert_same_keys(&example["data"], &real["data"], "project add --json");
}

#[test]
fn project_remove_example_in_json_md_has_the_keys_of_project_remove_json() {
    let doc = json_doc();
    let example = json_example(&doc, "### `todo project remove <name|path> --json`");

    let dir = TempDir::new().expect("tempdir");
    let folder = project_folder(dir.path());
    let register = dir.path().join("register").join("projects.json");
    let added = project_todo(&folder, &register, &["project", "add"]);
    assert!(added.status.success(), "stderr: {}", stderr_text(&added));
    let out = project_todo(
        &folder,
        &register,
        &["--json", "project", "remove", "renderer"],
    );
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let real = parse_json(&out.stdout);

    assert_eq!(example["schema"], real["schema"]);
    assert_same_keys(&example["data"], &real["data"], "project remove --json");
}

#[test]
fn project_list_example_in_json_md_has_the_keys_of_project_list_json() {
    let doc = json_doc();
    let example = json_example(&doc, "### `todo project list --json`");

    let dir = TempDir::new().expect("tempdir");
    let folder = project_folder(dir.path());
    let register = dir.path().join("register").join("projects.json");
    let added = project_todo(&folder, &register, &["project", "add"]);
    assert!(added.status.success(), "stderr: {}", stderr_text(&added));
    let out = project_todo(&folder, &register, &["--json", "project", "list"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let real = parse_json(&out.stdout);

    assert_eq!(example["schema"], real["schema"]);
    assert_same_keys(&example["data"], &real["data"], "project list --json");
    assert_same_keys(
        &example["data"]["projects"][0],
        &real["data"]["projects"][0],
        "project list --json project",
    );
}

#[test]
fn overview_example_in_json_md_has_the_keys_of_overview_json() {
    let doc = json_doc();
    let example = json_example(&doc, "### `todo overview --json`");

    let dir = TempDir::new().expect("tempdir");
    let folder = project_folder(dir.path());
    let register = dir.path().join("register").join("projects.json");
    let added = project_todo(&folder, &register, &["project", "add"]);
    assert!(added.status.success(), "stderr: {}", stderr_text(&added));
    // A second project whose file is gone, which shows the problem keys.
    let archive = dir.path().join("archiv");
    fs::create_dir_all(&archive).expect("create archive folder");
    fs::write(archive.join("TODO.md"), PROJECT_FIXTURE).expect("write TODO.md");
    let added = project_todo(&archive, &register, &["project", "add", "--name", "archiv"]);
    assert!(added.status.success(), "stderr: {}", stderr_text(&added));
    fs::remove_file(archive.join("TODO.md")).expect("remove TODO.md");

    let out = project_todo(&folder, &register, &["--json", "overview"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let real = parse_json(&out.stdout);

    assert_eq!(example["schema"], real["schema"]);
    assert_same_keys(&example["data"], &real["data"], "overview --json");
    assert_same_keys(
        &example["data"]["projects"][0],
        &real["data"]["projects"][0],
        "overview --json project",
    );
    assert_same_keys(
        &example["data"]["projects"][1],
        &real["data"]["projects"][1],
        "overview --json problem project",
    );
}

#[test]
fn report_example_in_json_md_has_the_keys_of_report_json() {
    let doc = json_doc();
    let example = json_example(&doc, "### `todo report --html --json`");

    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("TODO.md"), SHOW_FIXTURE).expect("write TODO.md");
    let out = run_in(dir.path(), &["--json", "report", "--html"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let real = parse_json(&out.stdout);

    assert_eq!(example["schema"], real["schema"]);
    assert_same_keys(&example["data"], &real["data"], "report --html --json");
}
