//! `todo overview` (T-093; R-PROJ-2, TEST-8). The projects are invented and registered with
//! `todo project add` in a temporary folder, which is also the register (`TODO_REGISTER`).

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use serde_json::Value;
use tempfile::TempDir;

use common::{FIXTURE_MARKER, clean_todo_command, parse_json, stderr_text, stdout_text};

/// A healthy project: tasks in every status, a nested milestone, two top-level milestones (one of
/// them done, one dropped), a stale claim, an open and a done question. Its stamp is the running
/// version, so it does not need a migration.
fn renderer_body() -> String {
    let version = env!("CARGO_PKG_VERSION");
    format!(
        "{FIXTURE_MARKER}
<!-- todo-format: 2 -->
<!-- todo-tool: {version} -->
# Renderer

## Tasks

### T-001 · Setup
- status: done
- owner: —
- priority: P1 · type: feature

### T-010 · Develop the renderer
- status: done
- owner: —
- priority: P1 · type: milestone
- goal: the test scene renders

### T-011 · Lighting
- status: open
- owner: —
- priority: P1 · type: milestone
- milestone: T-010
- goal: point lights match the reference images

### T-012 · C shader
- status: claimed
- owner: alice
- claimed: 2020-01-01T00:00+00:00
- priority: P1 · type: feature
- milestone: T-011

### T-013 · Compute pass
- status: blocked
- owner: —
- priority: P2
- note (2026-11-15, planner): blocked: waiting for the key

### T-014 · Ship lighting
- status: open
- owner: —
- priority: P2

### T-015 · Old idea
- status: dropped
- owner: —
- priority: P3

### T-020 · Audio
- status: open
- owner: —
- priority: P2 · type: milestone
- goal: sound plays with the scene

### T-030 · Old plan
- status: dropped
- owner: —
- priority: P3 · type: milestone
- goal: abandoned

## Questions

### T-005 · Are shadows in scope?
- status: open
- owner: —
- type: question

### T-006 · Which key?
- status: done
- owner: —
- type: question
"
    )
}

/// A project from before format 2 and before the tool stamp: it needs a migration.
const LEGACY_BODY: &str = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 1 -->
# Legacy

## Tasks

### T-001 · Old task
- status: open
- owner: —
- priority: P2
";

/// A file with a conflict marker: it cannot be read as tasks.
const BROKEN_BODY: &str = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 2 -->
# Broken

## Tasks

<<<<<<< HEAD
### T-001 · Conflicted
- status: open
- owner: —
- priority: P1
=======
### T-001 · Conflicted
- status: done
- owner: —
- priority: P1
>>>>>>> branch
";

/// A file in a format this todo does not know.
const NEWER_BODY: &str = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 3 -->
<!-- todo-tool: 0.9.0 -->
# Newer

## Tasks
";

/// Creates `<base>/<name>/TODO.md` with `body` and returns the path of that file.
fn project_file(base: &Path, name: &str, body: &str) -> PathBuf {
    let dir = base.join(name);
    fs::create_dir_all(&dir).expect("create project folder");
    let file = dir.join("TODO.md");
    fs::write(&file, body).expect("write TODO.md");
    file
}

/// Runs `todo args` in `dir`, with the register at `register`.
fn todo_in(home: &Path, dir: &Path, register: &Path, args: &[&str]) -> Output {
    clean_todo_command(home, dir)
        .env("TODO_REGISTER", register)
        .args(args)
        .output()
        .expect("run todo")
}

/// Registers `file` under `name` by `todo project add`, run in its folder.
fn register(home: &Path, register_file: &Path, file: &Path, name: &str) {
    let folder = file.parent().expect("folder");
    let out = todo_in(
        home,
        folder,
        register_file,
        &["project", "add", "--name", name],
    );
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
}

/// The five projects of these tests, in the order they are registered. `gone` is registered and
/// then its file is removed.
struct Projects {
    home: PathBuf,
    register: PathBuf,
    renderer: PathBuf,
    gone: PathBuf,
    /// The folder that holds the test data; it is not registered.
    tmp: PathBuf,
    _guard: TempDir,
}

fn set_up() -> Projects {
    let guard = TempDir::new().expect("tempdir");
    let tmp = guard.path().to_path_buf();
    let home = tmp.join("home");
    fs::create_dir_all(&home).expect("create home");
    let register_file = tmp.join("register").join("projects.json");

    let renderer = project_file(&tmp, "renderer", &renderer_body());
    let legacy = project_file(&tmp, "legacy", LEGACY_BODY);
    let gone = project_file(&tmp, "gone", LEGACY_BODY);
    let broken = project_file(&tmp, "broken", BROKEN_BODY);
    let newer = project_file(&tmp, "newer", NEWER_BODY);
    register(&home, &register_file, &renderer, "renderer");
    register(&home, &register_file, &legacy, "legacy");
    register(&home, &register_file, &gone, "gone");
    register(&home, &register_file, &broken, "broken");
    register(&home, &register_file, &newer, "newer");
    fs::remove_file(&gone).expect("remove the gone project's file");

    Projects {
        home,
        register: register_file,
        renderer: renderer.parent().expect("renderer folder").to_path_buf(),
        gone: gone.parent().expect("gone folder").to_path_buf(),
        tmp,
        _guard: guard,
    }
}

/// Runs `todo overview` (with `args`) from `dir` and returns its output.
fn overview(p: &Projects, dir: &Path, args: &[&str]) -> Output {
    todo_in(&p.home, dir, &p.register, args)
}

/// The `projects` of an `overview --json` run.
fn projects_of(out: &Output) -> Vec<Value> {
    let json = parse_json(&out.stdout);
    assert_eq!(json["schema"], "todo.projects.v1", "{json}");
    assert_eq!(json["ok"], true, "{json}");
    json["data"]["projects"]
        .as_array()
        .expect("projects array")
        .clone()
}

fn project_named<'a>(projects: &'a [Value], name: &str) -> &'a Value {
    projects
        .iter()
        .find(|p| p["name"] == name)
        .unwrap_or_else(|| panic!("no project {name} in {projects:?}"))
}

#[test]
fn overview_shows_one_line_per_project_and_each_problem_stops_only_its_own() {
    let p = set_up();
    // From a folder with no TODO.md and no link to the projects: the register is all it reads.
    let out = overview(&p, &p.tmp, &["overview"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    let lines: Vec<String> = stdout_text(&out).lines().map(str::to_owned).collect();
    assert_eq!(lines.len(), 5, "{lines:#?}");

    let renderer = lines
        .iter()
        .find(|l| l.starts_with("renderer | "))
        .expect("renderer line");
    assert!(
        renderer.contains(
            "open 1, claimed 1, blocked 1, done 1, dropped 1 | milestones done 1 of 2 | stale claims 1 | open questions 1 | todo "
        ),
        "{renderer}"
    );
    assert!(!renderer.contains("needs todo migrate"), "{renderer}");

    let legacy = lines
        .iter()
        .find(|l| l.starts_with("legacy | "))
        .expect("legacy line");
    assert!(legacy.contains("no todo version in the file"), "{legacy}");
    assert!(legacy.ends_with("(needs todo migrate --yes)"), "{legacy}");

    let gone = lines
        .iter()
        .find(|l| l.starts_with("gone | "))
        .expect("gone line");
    assert!(gone.ends_with("problem: TODO.md not found"), "{gone}");

    let broken = lines
        .iter()
        .find(|l| l.starts_with("broken | "))
        .expect("broken line");
    assert!(
        broken.ends_with("problem: git conflict marker found"),
        "{broken}"
    );

    let newer = lines
        .iter()
        .find(|l| l.starts_with("newer | "))
        .expect("newer line");
    assert!(
        newer.contains("problem: format version 3 is newer than this todo"),
        "{newer}"
    );
    assert!(
        newer.contains("Update the todo executable first"),
        "{newer}"
    );
}

#[test]
fn overview_json_has_the_figures_and_the_problems_of_each_project() {
    let p = set_up();
    let out = overview(&p, &p.tmp, &["--json", "overview"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    let projects = projects_of(&out);
    assert_eq!(projects.len(), 5);

    let renderer = project_named(&projects, "renderer");
    assert!(renderer["problem"].is_null(), "{renderer}");
    assert_eq!(renderer["format_version"], 2);
    assert_eq!(renderer["tool_stamp"], env!("CARGO_PKG_VERSION"));
    assert_eq!(renderer["needs_migration"], false);
    assert_eq!(
        renderer["tasks"],
        serde_json::json!({"open": 1, "claimed": 1, "blocked": 1, "done": 1, "dropped": 1, "total": 5})
    );
    assert_eq!(
        renderer["milestones"],
        serde_json::json!({"done": 1, "total": 2})
    );
    assert_eq!(renderer["stale_claims"], 1);
    assert_eq!(renderer["open_questions"], 1);

    // The same tasks as `todo where` counts for the same file.
    let file = p.renderer.join("TODO.md");
    let where_out = clean_todo_command(&p.home, &p.tmp)
        .args(["--file"])
        .arg(&file)
        .args(["--json", "where"])
        .output()
        .expect("run where");
    assert_eq!(
        where_out.status.code(),
        Some(0),
        "{}",
        stderr_text(&where_out)
    );
    let counts = &parse_json(&where_out.stdout)["data"]["counts"];
    assert_eq!(counts["open"], renderer["tasks"]["open"]);
    assert_eq!(counts["claimed"], renderer["tasks"]["claimed"]);
    assert_eq!(counts["blocked"], renderer["tasks"]["blocked"]);
    assert_eq!(counts["done"], renderer["tasks"]["done"]);
    assert_eq!(counts["dropped"], renderer["tasks"]["dropped"]);
    assert_eq!(counts["total"], renderer["tasks"]["total"]);
    assert_eq!(counts["questions"], renderer["open_questions"]);

    let legacy = project_named(&projects, "legacy");
    assert_eq!(legacy["format_version"], 1);
    assert!(legacy["tool_stamp"].is_null(), "{legacy}");
    assert_eq!(legacy["needs_migration"], true);
    assert_eq!(legacy["tasks"]["open"], 1);

    let gone = project_named(&projects, "gone");
    assert_eq!(gone["problem"]["code"], "NO_FILE");
    assert_eq!(gone["problem"]["message"], "TODO.md not found");
    assert!(gone["tasks"].is_null(), "{gone}");
    assert!(gone["needs_migration"].is_null(), "{gone}");

    let broken = project_named(&projects, "broken");
    assert_eq!(broken["problem"]["code"], "UNSAFE_FILE");
    assert!(broken["stale_claims"].is_null(), "{broken}");

    let newer = project_named(&projects, "newer");
    assert_eq!(newer["problem"]["code"], "TOOL_TOO_OLD");
    assert!(newer["milestones"].is_null(), "{newer}");
    assert!(newer["open_questions"].is_null(), "{newer}");
}

#[test]
fn overview_writes_nothing_and_reads_only_the_register() {
    let p = set_up();
    // A TODO.md that is not registered, next to the projects: it must not show up.
    let stranger = project_file(&p.tmp, "stranger", &renderer_body());
    let stranger_dir = stranger.parent().expect("folder").to_path_buf();

    let before = snapshot(&p.tmp);
    let out = overview(&p, &stranger_dir, &["--json", "overview"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    let text_out = overview(&p, &p.gone, &["overview"]);
    assert_eq!(
        text_out.status.code(),
        Some(0),
        "{}",
        stderr_text(&text_out)
    );
    let after = snapshot(&p.tmp);
    assert_eq!(before, after, "overview must not write or create any file");

    let names: Vec<String> = projects_of(&out)
        .iter()
        .map(|p| p["name"].as_str().unwrap_or_default().to_owned())
        .collect();
    assert_eq!(names, ["renderer", "legacy", "gone", "broken", "newer"]);
}

/// Every file under `root` with its bytes, sorted, so two snapshots can be compared.
fn snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).expect("read dir") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let bytes = fs::read(&path).expect("read file");
                found.push((path, bytes));
            }
        }
    }
    found.sort();
    found
}

#[test]
fn overview_of_an_empty_or_missing_register_says_so_and_creates_nothing() {
    let tmp = TempDir::new().expect("tempdir");
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).expect("create home");
    let register_file = tmp.path().join("register").join("projects.json");
    let p = Projects {
        home: home.clone(),
        register: register_file.clone(),
        renderer: tmp.path().to_path_buf(),
        gone: tmp.path().to_path_buf(),
        tmp: tmp.path().to_path_buf(),
        _guard: tmp,
    };

    let out = overview(&p, &p.tmp, &["overview"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    assert!(stdout_text(&out).starts_with("No projects are registered in "));
    assert!(!register_file.exists());

    let out = overview(&p, &p.tmp, &["--json", "overview"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    assert!(projects_of(&out).is_empty());
    assert!(!register_file.parent().expect("folder").exists());
}

#[test]
fn overview_refuses_a_register_that_is_not_one_and_keeps_it() {
    let p = set_up();
    fs::write(&p.register, "not json").expect("break the register");

    let out = overview(&p, &p.tmp, &["--json", "overview"]);
    assert_eq!(out.status.code(), Some(4), "{}", stderr_text(&out));
    let json = parse_json(&out.stdout);
    assert_eq!(json["schema"], "todo.projects.v1");
    assert_eq!(json["error"]["code"], "UNSAFE_FILE");
    assert_eq!(fs::read(&p.register).expect("read register"), b"not json");
}
