//! CLI integration tests for the ready rule of tasks at or below the target level (T-153; R-DETAIL-2,
//! R-READ-3, R-CHECK-1, R-CFG-2, R-KIND-2): `next`, `list --ready`, `show --json`, `check`, and the
//! `target_difficulty` and `level1_max_files` settings in `todo.toml`.

mod common;

use std::fs;
use std::path::Path;
use std::process::Output;

use common::{parse_json, run_todo, setup_proj, stderr_text, write_toml};
use serde_json::{Value, json};
use tempfile::TempDir;

/// The parts of the detail a task at the lowest level needs, each with its field name.
const PARTS: [(&str, &str); 5] = [
    ("why", "- why: the reason for the work\n"),
    ("accept", "- accept: the result is visible\n"),
    (
        "steps",
        "- steps:\n  1. open the file\n  2. make the change\n",
    ),
    ("verify", "- verify: cargo test\n"),
    ("files", "- files: src/a.rs\n"),
];

/// The field lines of an open task at `level` that has all of its detail, except the parts in
/// `skip`.
fn open_task(level: &str, skip: &[&str]) -> String {
    let mut fields = format!("- status: open\n- priority: P1\n- difficulty: {level}\n");
    for (name, text) in PARTS {
        if !skip.contains(&name) {
            fields.push_str(text);
        }
    }
    fields
}

/// A task block of the invented task list: heading, then the field lines.
fn task(id: &str, fields: &str) -> String {
    format!("### {id} · Task {id}\n{fields}\n")
}

/// A question block of the invented task list about `about`, open or with the given status.
fn question(id: &str, status: &str, about: &str) -> String {
    format!("### {id} · Question {id}\n- status: {status}\n- type: question\n- about: {about}\n\n")
}

/// A task list with the format line and a `## Work` section holding `body`.
fn doc(body: &str) -> String {
    format!(
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
         <!-- todo-format: 3 -->\n\
         # Detail fixture\n\n\
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

/// What `show --json` lists as missing for task `id`.
fn missing_of(home: &Path, work: &Path, id: &str) -> Vec<String> {
    data(home, work, &["show", id])["missing"]
        .as_array()
        .expect("missing is a list")
        .iter()
        .map(|item| item.as_str().expect("a text item").to_string())
        .collect()
}

/// Whether `show --json` says task `id` is ready for `next`.
fn ready_of(home: &Path, work: &Path, id: &str) -> bool {
    data(home, work, &["show", id])["ready"]
        .as_bool()
        .expect("ready is a bool")
}

/// The ID of the task `next` hands out, or `None` when it hands out nothing.
fn next_id(home: &Path, work: &Path) -> Option<String> {
    data(home, work, &["next"])["task"]["id"]
        .as_str()
        .map(str::to_string)
}

#[test]
fn a_level1_task_with_all_its_detail_is_handed_out_and_show_says_so() {
    let tmp = TempDir::new().expect("temp dir");
    let (home, work, _) = setup_proj(&tmp, &doc(&task("T-001", &open_task("1", &[]))));

    assert_eq!(next_id(&home, &work).as_deref(), Some("T-001"));
    assert!(ready_of(&home, &work, "T-001"));
    assert_eq!(missing_of(&home, &work, "T-001"), Vec::<String>::new());
}

#[test]
fn each_missing_part_keeps_a_level1_task_out_of_next_and_is_listed_by_show() {
    let tmp = TempDir::new().expect("temp dir");
    let body = [
        task("T-001", &open_task("1", &["why"])),
        task("T-002", &open_task("1", &["accept"])),
        task("T-003", &open_task("1", &["steps"])),
        task("T-004", &open_task("1", &["verify"])),
        task("T-005", &open_task("1", &["files"])),
        task("T-006", &open_task("1", &[])),
    ]
    .concat();
    let (home, work, _) = setup_proj(&tmp, &doc(&body));

    for (id, part) in [
        ("T-001", "why"),
        ("T-002", "accept"),
        ("T-003", "steps"),
        ("T-004", "verify"),
        ("T-005", "files"),
    ] {
        assert_eq!(missing_of(&home, &work, id), vec![part], "{id}");
        assert!(!ready_of(&home, &work, id), "{id} is not ready");
    }
    assert_eq!(next_id(&home, &work).as_deref(), Some("T-006"));

    let listed = data(&home, &work, &["list", "--ready"]);
    let ids: Vec<&str> = listed["tasks"]
        .as_array()
        .expect("tasks")
        .iter()
        .filter_map(|t| t["id"].as_str())
        .collect();
    assert_eq!(ids, vec!["T-006"]);

    // A claim by ID still works, whatever the detail says (R-DETAIL-2).
    let claim = todo(&home, &work, &["--agent", "alice", "claim", "T-001"]);
    assert!(claim.status.success(), "{}", stderr_text(&claim));
}

#[test]
fn the_file_limit_and_folders_and_patterns_are_checked_with_level1_max_files() {
    let tmp = TempDir::new().expect("temp dir");
    let with_files = |files: &str| format!("{}- files: {files}\n", open_task("1", &["files"]));
    let body = [
        task(
            "T-001",
            &with_files("src/a.rs, src/b.rs, src/c.rs, src/d.rs"),
        ),
        task("T-002", &with_files("src/")),
        task("T-003", &with_files("src/*.rs")),
        task("T-004", &with_files("src/a.rs, src/b.rs, src/c.rs")),
    ]
    .concat();
    let (home, work, _) = setup_proj(&tmp, &doc(&body));

    // The default limit is 3 files.
    assert_eq!(
        missing_of(&home, &work, "T-001"),
        vec!["files: more than 3"]
    );
    assert_eq!(
        missing_of(&home, &work, "T-002"),
        vec!["files: folder or pattern"]
    );
    assert_eq!(
        missing_of(&home, &work, "T-003"),
        vec!["files: folder or pattern"]
    );
    assert!(ready_of(&home, &work, "T-004"));

    // `level1_max_files` in todo.toml replaces the limit.
    write_toml(&work, "level1_max_files = 2\n");
    assert_eq!(
        missing_of(&home, &work, "T-004"),
        vec!["files: more than 2"]
    );
    assert!(!ready_of(&home, &work, "T-004"));
}

#[test]
fn read_paths_must_exist_in_the_folder_of_todo_md() {
    let tmp = TempDir::new().expect("temp dir");
    let body = [
        task(
            "T-001",
            &format!("{}- read: docs/notes.md:3, src\n", open_task("1", &[])),
        ),
        task(
            "T-002",
            &format!("{}- read: docs/missing.md\n", open_task("1", &[])),
        ),
    ]
    .concat();
    let (home, work, _) = setup_proj(&tmp, &doc(&body));
    // A file and a folder next to TODO.md are found; a `:<line>` suffix is not part of the path, and
    // the other path is not there.
    fs::create_dir_all(work.join("docs")).expect("create docs");
    fs::write(work.join("docs").join("notes.md"), "notes\n").expect("write notes");
    fs::create_dir_all(work.join("src")).expect("create src");

    assert!(ready_of(&home, &work, "T-001"));
    assert_eq!(
        missing_of(&home, &work, "T-002"),
        vec!["read: docs/missing.md does not exist"]
    );
    assert!(!ready_of(&home, &work, "T-002"));
}

#[test]
fn an_open_question_about_a_task_keeps_it_out_until_the_question_is_answered() {
    let tmp = TempDir::new().expect("temp dir");
    let body = [
        task("T-001", &open_task("1", &[])),
        task("T-002", &open_task("1", &[])),
        task("T-003", &open_task("1", &[])),
        question("T-005", "open", "T-001"),
        // A question that is done or dropped does not count (R-KIND-2).
        question("T-006", "done", "T-002"),
        question("T-007", "dropped", "T-003"),
    ]
    .concat();
    let (home, work, _) = setup_proj(&tmp, &doc(&body));

    assert_eq!(
        missing_of(&home, &work, "T-001"),
        vec!["open question T-005"]
    );
    assert!(!ready_of(&home, &work, "T-001"));
    assert!(ready_of(&home, &work, "T-002"));
    assert!(ready_of(&home, &work, "T-003"));
    assert_eq!(next_id(&home, &work).as_deref(), Some("T-002"));

    let answer = todo(
        &home,
        &work,
        &["--agent", "human", "answer", "T-005", "yes"],
    );
    assert!(answer.status.success(), "{}", stderr_text(&answer));
    assert!(ready_of(&home, &work, "T-001"));
    assert_eq!(missing_of(&home, &work, "T-001"), Vec::<String>::new());
}

#[test]
fn tasks_above_the_target_level_are_not_affected_and_the_target_can_be_raised() {
    let tmp = TempDir::new().expect("temp dir");
    let body = [
        // Level 2 without any detail: above the lowest level, so it is handed out.
        task(
            "T-001",
            &open_task("2", &["why", "accept", "steps", "verify", "files"]),
        ),
        // Level 1 without its why: not handed out.
        task("T-002", &open_task("1", &["why"])),
        // Level 3 without any detail: above the target, so it is not affected either.
        task(
            "T-003",
            &open_task("3", &["why", "accept", "steps", "verify", "files"]),
        ),
    ]
    .concat();
    let (home, work, _) = setup_proj(&tmp, &doc(&body));

    assert!(ready_of(&home, &work, "T-001"));
    assert_eq!(missing_of(&home, &work, "T-001"), Vec::<String>::new());
    assert_eq!(missing_of(&home, &work, "T-002"), vec!["why"]);
    assert_eq!(next_id(&home, &work).as_deref(), Some("T-001"));

    // With the target at level 2, level 2 needs its detail too; level 3 is still above it.
    write_toml(&work, "target_difficulty = \"2\"\n");
    assert_eq!(
        missing_of(&home, &work, "T-001"),
        vec!["why", "accept", "steps", "verify", "files"]
    );
    assert!(!ready_of(&home, &work, "T-001"));
    assert_eq!(missing_of(&home, &work, "T-003"), Vec::<String>::new());
    assert!(ready_of(&home, &work, "T-003"));
    assert_eq!(next_id(&home, &work).as_deref(), Some("T-003"));
}

#[test]
fn check_warns_about_unfinished_detail_gaps_and_keeps_the_exit_code() {
    let tmp = TempDir::new().expect("temp dir");
    let finished = "- status: done\n- priority: P1\n- difficulty: 1\n- owner: alice\n";
    let body = [
        task("T-001", &open_task("1", &["verify"])),
        task("T-002", finished),
    ]
    .concat();
    let (home, work, _) = setup_proj(&tmp, &doc(&body));

    let out = todo(&home, &work, &["check"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    let stderr = stderr_text(&out);
    assert!(
        stderr.contains("T-001: not ready, its detail is incomplete: verify"),
        "{stderr}"
    );
    // A done task is not handed out, so it is not reported (R-CHECK-1).
    assert!(!stderr.contains("T-002"), "{stderr}");
}

#[test]
fn answering_a_question_makes_the_task_it_is_about_newly_ready() {
    let tmp = TempDir::new().expect("temp dir");
    let body = [
        task("T-001", &open_task("1", &[])),
        question("T-005", "open", "T-001"),
    ]
    .concat();
    let (home, work, _) = setup_proj(&tmp, &doc(&body));
    assert!(!ready_of(&home, &work, "T-001"));

    // `done` names the task that the question has just let go (R-DEP-4).
    let done = data(&home, &work, &["--agent", "human", "done", "T-005"]);
    let newly: Vec<&str> = done["newly_ready"]
        .as_array()
        .expect("newly_ready")
        .iter()
        .filter_map(|t| t["id"].as_str())
        .collect();
    assert_eq!(newly, vec!["T-001"]);
}

#[test]
fn waves_list_the_same_missing_parts_as_show() {
    // `waves` reads the same list as `show`, `next` and `check` (R-WAVE-1, R-DETAIL-2).
    let tmp = TempDir::new().expect("temp dir");
    let body = [
        task(
            "T-001",
            &format!("{}- read: docs/none.md\n", open_task("1", &[])),
        ),
        question("T-005", "open", "T-001"),
    ]
    .concat();
    let (home, work, _) = setup_proj(&tmp, &doc(&body));

    let waves = data(&home, &work, &["waves"]);
    let entry = waves["waves"]
        .as_array()
        .expect("waves")
        .iter()
        .flat_map(|wave| wave["lanes"].as_array().expect("lanes").iter())
        .flat_map(|lane| lane.as_array().expect("lane").iter())
        .find(|t| t["id"] == "T-001")
        .expect("T-001 is placed in a wave");
    assert_eq!(
        entry["missing"],
        json!(["read: docs/none.md does not exist", "open question T-005"])
    );
}
