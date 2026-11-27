//! CLI tests for the uncommitted-changes check of the `done` operations of `todo apply` (R-SCOPE-3).
//! A `done` whose commit does not hold the task's files as they are on disk refuses the whole batch
//! (`UNCOMMITTED_CHANGES`) unless `--force`, which notes the files; the check runs before a `verify`
//! command. Every fixture is invented and written into a temporary folder with its own git repository.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use serde_json::json;
use tempfile::TempDir;

use common::{parse_json, read_text, run_todo, setup_proj};

const MARKER: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->";

/// The clock of the notes that `done` writes.
const NOW: &str = "2026-11-15T14:30:00+01:00";

/// The format line of the fixtures: format 3, the current one (`docs/format.md` §18).
const FORMAT_LINE: &str = "<!-- todo-format: 3 -->";

/// Runs git in `dir` as an invented author, with the developer's configuration out of the way.
fn git(dir: &Path, args: &[&str]) -> String {
    let out = common::git_command()
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", dir.join("no-such-gitconfig"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args([
            "-c",
            "user.name=Test User",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .expect("run git");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Writes `text` to `dir/rel`, making the folders it needs.
fn write(dir: &Path, rel: &str, text: &str) {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create folder");
    }
    fs::write(path, text).expect("write file");
}

/// Stages `paths` and commits them with `message`.
fn commit(dir: &Path, paths: &[&str], message: &str) {
    let mut add = vec!["add", "--"];
    add.extend_from_slice(paths);
    git(dir, &add);
    git(dir, &["commit", "-q", "-m", message]);
}

struct Proj {
    _tmp: TempDir,
    home: PathBuf,
    work: PathBuf,
    todo: PathBuf,
}

/// Makes the project folder a repository whose first commit holds TODO.md and `src/a.rs`.
fn repo_with_first_commit(p: &Proj) {
    write(&p.work, "src/a.rs", "pub fn a() {}\n");
    git(&p.work, &["init", "-q"]);
    commit(&p.work, &["TODO.md", "src/a.rs"], "T-001: start");
}

/// A TODO.md with T-001, which names `files` when given, and T-002, which names none.
fn two_tasks_body(files: Option<&str>) -> String {
    let mut text = format!(
        "{MARKER}\n{FORMAT_LINE}\n## Queue\n\n### T-001 · Scoped task\n- status: open\n- owner: —\n- priority: P1\n- difficulty: 1\n"
    );
    if let Some(files) = files {
        text.push_str(&format!("- files: {files}\n"));
    }
    text.push_str(
        "\n### T-002 · Other task\n- status: open\n- owner: —\n- priority: P1\n- difficulty: 1\n",
    );
    text
}

/// A project folder with the two tasks of [`two_tasks_body`].
fn two_tasks(files: Option<&str>) -> Proj {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work, todo) = setup_proj(&tmp, &two_tasks_body(files));
    Proj {
        _tmp: tmp,
        home,
        work,
        todo,
    }
}

/// A note on T-002, which a batch applies along with the `done` of T-001.
const NOTE_OP: &str = r#"{"op":"note","task":"T-002","text":"checked"}"#;

/// The `done` operation of T-001, with its `commit` when given.
fn done_op(commit: Option<&str>) -> String {
    match commit {
        Some(c) => format!(r#"{{"op":"done","task":"T-001","commit":"{c}"}}"#),
        None => r#"{"op":"done","task":"T-001"}"#.to_string(),
    }
}

/// Writes `ops` as JSON lines to `ops.jsonl` in the project folder and runs
/// `todo --json --agent alice [extra] apply ops.jsonl` there.
fn apply(p: &Proj, ops: &[&str], extra: &[&str]) -> Output {
    write(&p.work, "ops.jsonl", &format!("{}\n", ops.join("\n")));
    let mut args = vec!["--json", "--agent", "alice"];
    args.extend_from_slice(extra);
    args.extend_from_slice(&["apply", "ops.jsonl"]);
    run_todo(&p.home, &p.work, &args, &[("TODO_TEST_NOW", NOW)]).expect("run todo")
}

/// A `verify` command that writes `ran.txt` in the folder of TODO.md, so a test can see that it ran.
fn marker_check() -> String {
    if cfg!(windows) {
        "Set-Content -Path ran.txt -Value ok".to_string()
    } else {
        "echo ok > ran.txt".to_string()
    }
}

/// A TODO.md with T-001 at level 1, naming `files`, with the `verify` command `verify`.
fn verify_body(files: &str, verify: &str) -> String {
    format!(
        "{MARKER}\n{FORMAT_LINE}\n## Queue\n\n### T-001 · Scoped task\n- status: open\n- owner: —\n- priority: P1\n- difficulty: 1\n- files: {files}\n- verify: {verify}\n"
    )
}

/// A project with [`verify_body`] and a repository whose first commit is inside the files.
fn verified_repo_project() -> Proj {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work, todo) = setup_proj(&tmp, &verify_body("src/", &marker_check()));
    let p = Proj {
        _tmp: tmp,
        home,
        work,
        todo,
    };
    repo_with_first_commit(&p);
    p
}

#[test]
fn apply_done_with_a_changed_file_inside_the_files_refuses_and_names_it() {
    let p = two_tasks(Some("src/"));
    repo_with_first_commit(&p);
    write(&p.work, "src/a.rs", "pub fn a() { changed() }\n");
    let before = fs::read(&p.todo).expect("read TODO.md");

    let out = apply(&p, &[done_op(Some("HEAD")).as_str()], &[]);
    assert_eq!(out.status.code(), Some(1));
    let json = parse_json(&out.stdout);
    assert_eq!(json["error"]["code"], "UNCOMMITTED_CHANGES", "{json}");
    assert_eq!(
        json["error"]["files_uncommitted"],
        json!(["src/a.rs"]),
        "{json}"
    );
    assert_eq!(fs::read(&p.todo).expect("read TODO.md"), before);
}

#[test]
fn apply_refused_for_uncommitted_changes_writes_none_of_the_batch() {
    let p = two_tasks(Some("src/"));
    repo_with_first_commit(&p);
    write(&p.work, "src/a.rs", "pub fn a() { changed() }\n");
    let before = fs::read(&p.todo).expect("read TODO.md");

    // The note before the `done` in the batch is not written either.
    let out = apply(&p, &[NOTE_OP, done_op(Some("HEAD")).as_str()], &[]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        parse_json(&out.stdout)["error"]["code"],
        "UNCOMMITTED_CHANGES"
    );
    assert_eq!(fs::read(&p.todo).expect("read TODO.md"), before);
}

#[test]
fn apply_done_of_a_clean_folder_applies_the_whole_batch() {
    let p = two_tasks(Some("src/"));
    repo_with_first_commit(&p);

    let out = apply(&p, &[NOTE_OP, done_op(Some("HEAD")).as_str()], &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = read_text(&p.todo);
    assert!(text.contains("- status: done\n"), "{text}");
    assert!(
        text.contains("- note (2026-11-15, alice): checked\n"),
        "{text}"
    );
}

#[test]
fn apply_done_with_force_applies_and_notes_the_uncommitted_file() {
    let p = two_tasks(Some("src/"));
    repo_with_first_commit(&p);
    write(&p.work, "src/a.rs", "pub fn a() { changed() }\n");

    let out = apply(&p, &[NOTE_OP, done_op(Some("HEAD")).as_str()], &["--force"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = read_text(&p.todo);
    assert!(text.contains("- status: done\n"), "{text}");
    assert!(
        text.contains(
            "- note (2026-11-15, alice): forced done with uncommitted changes: src/a.rs\n"
        ),
        "{text}"
    );
}

#[test]
fn apply_refuses_uncommitted_changes_before_the_verify_command_runs() {
    let p = verified_repo_project();
    write(&p.work, "src/a.rs", "pub fn a() { changed() }\n");
    let before = fs::read(&p.todo).expect("read TODO.md");

    let out = apply(&p, &[done_op(Some("HEAD")).as_str()], &[]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        parse_json(&out.stdout)["error"]["code"],
        "UNCOMMITTED_CHANGES"
    );
    assert!(
        !p.work.join("ran.txt").exists(),
        "the verify command ran before the uncommitted changes were checked"
    );
    assert_eq!(fs::read(&p.todo).expect("read TODO.md"), before);
}
