//! CLI tests for the commit check of `done --commit` and of the `done` operations of `apply`
//! (R-SCOPE-2, NFR-5). A commit inside the files of the task finishes it; a commit that changed a file
//! outside them refuses `done` (`OUT_OF_SCOPE`) unless `--force`, which notes the files; a task at the
//! lowest level that names files needs its commit while git is there (`MISSING_FIELD`). `apply`
//! refuses its whole batch in the same cases. Every fixture is invented and written into a temporary
//! folder: with its own git repository, or, for a folder outside any repository, with none.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use serde_json::json;
use tempfile::TempDir;

use common::{parse_json, read_text, run_todo, setup_proj, write_toml};

const MARKER: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->";

/// The clock of the notes that `done` writes.
const NOW: &str = "2026-11-15T14:30:00+01:00";

/// The format line of the fixtures: format 3, the current one (`docs/format.md` §18).
const FORMAT_LINE: &str = "<!-- todo-format: 3 -->";

/// A TODO.md with one open task, T-001, at the level `difficulty` (`1` to `5`, R-LEVEL-1), naming
/// `files` when given.
fn body(difficulty: &str, files: Option<&str>) -> String {
    let mut text = format!(
        "{MARKER}\n{FORMAT_LINE}\n## Queue\n\n### T-001 · Scoped task\n- status: open\n- owner: —\n- priority: P1\n- difficulty: {difficulty}\n"
    );
    if let Some(files) = files {
        text.push_str(&format!("- files: {files}\n"));
    }
    text
}

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

/// A project folder whose TODO.md holds T-001 at `difficulty`, naming `files` when given. No git yet.
fn project(difficulty: &str, files: Option<&str>) -> Proj {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work, todo) = setup_proj(&tmp, &body(difficulty, files));
    Proj {
        _tmp: tmp,
        home,
        work,
        todo,
    }
}

/// Makes the project folder a repository whose first commit holds TODO.md and `src/a.rs`.
fn repo_with_first_commit(p: &Proj) {
    write(&p.work, "src/a.rs", "pub fn a() {}\n");
    git(&p.work, &["init", "-q"]);
    commit(&p.work, &["TODO.md", "src/a.rs"], "T-001: start");
}

/// `todo --json done T-001 --agent alice` plus `extra`, run in `dir`, a folder of the project.
fn done_in(p: &Proj, dir: &Path, extra: &[&str]) -> Output {
    let mut args = vec!["--json", "done", "T-001", "--agent", "alice"];
    args.extend_from_slice(extra);
    run_todo(&p.home, dir, &args, &[("TODO_TEST_NOW", NOW)]).expect("run todo")
}

/// `todo --json done T-001 --agent alice` plus `extra`, run in the project folder.
fn done(p: &Proj, extra: &[&str]) -> Output {
    done_in(p, &p.work, extra)
}

#[test]
fn a_commit_inside_the_files_finishes_the_task() {
    let p = project("1", Some("src/"));
    repo_with_first_commit(&p);
    write(&p.work, "src/b.rs", "pub fn b() {}\n");
    write(
        &p.work,
        "TODO-archive.md",
        &format!("{MARKER}\n{FORMAT_LINE}\n"),
    );
    fs::write(&p.todo, format!("{}\n", read_text(&p.todo))).expect("rewrite TODO.md");
    // TODO.md and the archive file may change in the commit as well (R-SCOPE-2).
    commit(
        &p.work,
        &["src/b.rs", "TODO.md", "TODO-archive.md"],
        "T-001: work",
    );

    let out = done(&p, &["--commit", "HEAD"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(read_text(&p.todo).contains("- status: done\n"));
}

#[test]
fn a_commit_outside_the_files_refuses_done_and_names_the_file() {
    let p = project("1", Some("src/"));
    repo_with_first_commit(&p);
    write(&p.work, "src/b.rs", "pub fn b() {}\n");
    write(&p.work, "other.txt", "outside the task\n");
    commit(&p.work, &["src/b.rs", "other.txt"], "T-001: work");
    let before = fs::read(&p.todo).expect("read TODO.md");

    let out = done(&p, &["--commit", "HEAD"]);
    assert_eq!(out.status.code(), Some(1));
    let json = parse_json(&out.stdout);
    assert_eq!(json["error"]["code"], "OUT_OF_SCOPE", "{json}");
    assert_eq!(
        json["error"]["files_outside"],
        json!(["other.txt"]),
        "{json}"
    );
    assert!(
        json["error"]["message"]
            .as_str()
            .is_some_and(|m| m.contains("other.txt")),
        "{json}"
    );
    assert_eq!(fs::read(&p.todo).expect("read TODO.md"), before);
}

#[test]
fn force_finishes_the_task_and_notes_the_files_outside() {
    let p = project("1", Some("src/"));
    repo_with_first_commit(&p);
    write(&p.work, "src/b.rs", "pub fn b() {}\n");
    write(&p.work, "other.txt", "outside the task\n");
    commit(&p.work, &["src/b.rs", "other.txt"], "T-001: work");

    let out = done(&p, &["--commit", "HEAD", "--force"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = read_text(&p.todo);
    assert!(text.contains("- status: done\n"), "{text}");
    assert!(
        text.contains("- note (2026-11-15, alice): forced done outside files: other.txt\n"),
        "{text}"
    );
}

#[test]
fn a_rename_inside_the_files_passes() {
    let p = project("1", Some("src/"));
    repo_with_first_commit(&p);
    git(&p.work, &["mv", "src/a.rs", "src/c.rs"]);
    git(&p.work, &["commit", "-q", "-m", "T-001: rename"]);

    let out = done(&p, &["--commit", "HEAD"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn a_rename_out_of_the_files_is_refused_for_its_new_path() {
    let p = project("1", Some("src/"));
    repo_with_first_commit(&p);
    write(&p.work, "other/.keep", "");
    git(&p.work, &["mv", "src/a.rs", "other/c.rs"]);
    git(&p.work, &["commit", "-q", "-m", "T-001: move"]);

    // The old path lies inside the files, so only the new path is outside.
    let out = done(&p, &["--commit", "HEAD"]);
    assert_eq!(out.status.code(), Some(1));
    let json = parse_json(&out.stdout);
    assert_eq!(json["error"]["code"], "OUT_OF_SCOPE", "{json}");
    assert_eq!(
        json["error"]["files_outside"],
        json!(["other/c.rs"]),
        "{json}"
    );
}

#[test]
fn a_merge_commit_is_compared_with_its_first_parent() {
    let p = project("1", Some("src/"));
    repo_with_first_commit(&p);
    git(&p.work, &["checkout", "-q", "-b", "side"]);
    write(&p.work, "src/b.rs", "pub fn b() {}\n");
    commit(&p.work, &["src/b.rs"], "T-001: side work");
    git(&p.work, &["checkout", "-q", "-"]);
    write(&p.work, "other.txt", "changed on the main line\n");
    commit(&p.work, &["other.txt"], "T-002: main line");
    git(
        &p.work,
        &["merge", "-q", "--no-ff", "-m", "T-001: merge side", "side"],
    );

    // Against the first parent the merge brings in src/b.rs only; other.txt is already there.
    let out = done(&p, &["--commit", "HEAD"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn a_task_without_files_is_not_checked() {
    let p = project("1", None);
    repo_with_first_commit(&p);
    write(&p.work, "other.txt", "outside any task\n");
    commit(&p.work, &["other.txt"], "T-001: elsewhere");

    let out = done(&p, &["--commit", "HEAD"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn a_task_above_the_lowest_level_needs_no_commit() {
    let p = project("3", Some("src/"));
    repo_with_first_commit(&p);

    let out = done(&p, &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn a_lowest_level_task_with_files_needs_its_commit_while_git_is_there() {
    let p = project("1", Some("src/"));
    repo_with_first_commit(&p);
    let before = fs::read(&p.todo).expect("read TODO.md");

    let out = done(&p, &[]);
    assert_eq!(out.status.code(), Some(1));
    let json = parse_json(&out.stdout);
    assert_eq!(json["error"]["code"], "MISSING_FIELD", "{json}");
    assert!(
        json["error"]["message"]
            .as_str()
            .is_some_and(|m| m.contains("--commit HEAD")),
        "{json}"
    );
    assert_eq!(fs::read(&p.todo).expect("read TODO.md"), before);

    // --force does not override the rule (R-SCOPE-2).
    let forced = done(&p, &["--force"]);
    assert_eq!(forced.status.code(), Some(1));
    assert_eq!(parse_json(&forced.stdout)["error"]["code"], "MISSING_FIELD");
    assert_eq!(fs::read(&p.todo).expect("read TODO.md"), before);
}

#[test]
fn the_target_level_decides_which_task_needs_its_commit() {
    // By default the target is level 1, so a level-2 task needs no commit (R-SCOPE-2, R-DETAIL-2).
    let by_default = project("2", Some("src/"));
    repo_with_first_commit(&by_default);
    let out = done(&by_default, &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // With target_difficulty = "2", the same task is at the target and needs its commit.
    let raised = project("2", Some("src/"));
    write_toml(&raised.work, "target_difficulty = \"2\"\n");
    repo_with_first_commit(&raised);
    let before = fs::read(&raised.todo).expect("read TODO.md");
    let out = done(&raised, &[]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(parse_json(&out.stdout)["error"]["code"], "MISSING_FIELD");
    assert_eq!(fs::read(&raised.todo).expect("read TODO.md"), before);
}

#[test]
fn a_folder_without_git_needs_no_commit_and_checks_nothing() {
    let p = project("1", Some("src/"));

    let out = done(&p, &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn without_git_on_path_the_commit_is_taken_as_given() {
    let p = project("1", Some("src/"));
    repo_with_first_commit(&p);
    write(&p.work, "other.txt", "outside the task\n");
    commit(&p.work, &["other.txt"], "T-001: elsewhere");

    let out = run_todo(
        &p.home,
        &p.work,
        &[
            "--json", "done", "T-001", "--agent", "alice", "--commit", "cafebabe",
        ],
        &[("PATH", ""), ("TODO_TEST_NOW", NOW)],
    )
    .expect("run todo");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(read_text(&p.todo).contains("- commit: cafebabe\n"));
}

#[test]
fn file_names_with_spaces_and_letters_outside_ascii_are_compared_as_written() {
    let p = project("1", Some("src/my notes/, src/café.md"));
    repo_with_first_commit(&p);
    write(&p.work, "src/my notes/first draft.md", "inside\n");
    write(&p.work, "src/café.md", "inside\n");
    commit(
        &p.work,
        &["src/my notes/first draft.md", "src/café.md"],
        "T-001: notes",
    );

    let out = done(&p, &["--commit", "HEAD"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn a_file_outside_with_spaces_and_letters_is_named_as_written() {
    let p = project("1", Some("src/"));
    repo_with_first_commit(&p);
    write(&p.work, "other dir/ü notes.txt", "outside\n");
    commit(&p.work, &["other dir/ü notes.txt"], "T-001: elsewhere");

    let out = done(&p, &["--commit", "HEAD"]);
    assert_eq!(out.status.code(), Some(1));
    let json = parse_json(&out.stdout);
    assert_eq!(
        json["error"]["files_outside"],
        json!(["other dir/ü notes.txt"]),
        "{json}"
    );
}

/// A repository whose root holds a folder `proj` with TODO.md, so TODO.md's folder is not the
/// repository's top. Returns `(tempdir, home, folder of TODO.md)`.
fn nested_project(difficulty: &str, files: &str) -> (TempDir, PathBuf, PathBuf) {
    let tmp = TempDir::new().expect("tempdir");
    let home = tmp.path().join("home");
    let repo = tmp.path().join("repo");
    let proj = repo.join("proj");
    fs::create_dir_all(&home).expect("create home");
    write(&proj, "TODO.md", &body(difficulty, Some(files)));
    write(&proj, "src/a.rs", "pub fn a() {}\n");
    git(&repo, &["init", "-q"]);
    commit(&repo, &["proj/TODO.md", "proj/src/a.rs"], "T-001: start");
    (tmp, home, proj)
}

#[test]
fn paths_are_relative_to_the_folder_of_todo_md_inside_a_larger_repository() {
    let (tmp, home, proj) = nested_project("1", "src/");
    let repo = tmp.path().join("repo");
    write(&proj, "src/b.rs", "pub fn b() {}\n");
    write(&repo, "other/x.txt", "outside the task\n");
    commit(&repo, &["proj/src/b.rs", "other/x.txt"], "T-001: work");

    let out = run_todo(
        &home,
        &proj,
        &[
            "--json", "done", "T-001", "--agent", "alice", "--commit", "HEAD",
        ],
        &[("TODO_TEST_NOW", NOW)],
    )
    .expect("run todo");
    assert_eq!(out.status.code(), Some(1));
    let json = parse_json(&out.stdout);
    assert_eq!(json["error"]["code"], "OUT_OF_SCOPE", "{json}");
    assert_eq!(
        json["error"]["files_outside"],
        json!(["../other/x.txt"]),
        "{json}"
    );
}

#[test]
fn a_commit_inside_the_folder_of_todo_md_passes_inside_a_larger_repository() {
    let (tmp, home, proj) = nested_project("1", "src/");
    let repo = tmp.path().join("repo");
    write(&proj, "src/b.rs", "pub fn b() {}\n");
    commit(&repo, &["proj/src/b.rs"], "T-001: work");

    let out = run_todo(
        &home,
        &proj,
        &[
            "--json", "done", "T-001", "--agent", "alice", "--commit", "HEAD",
        ],
        &[("TODO_TEST_NOW", NOW)],
    )
    .expect("run todo");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn from_a_subfolder_the_files_are_named_from_the_folder_of_todo_md() {
    let p = project("1", Some("src/"));
    repo_with_first_commit(&p);
    write(&p.work, "src/b.rs", "pub fn b() {}\n");
    write(&p.work, "other.txt", "outside the task\n");
    commit(&p.work, &["src/b.rs", "other.txt"], "T-001: work");
    let before = fs::read(&p.todo).expect("read TODO.md");

    // Run in the subfolder `src`: git is asked there, but the paths name the files from the folder
    // of TODO.md, so `other.txt` is named as it is written and not as `../other.txt`.
    let out = done_in(&p, &p.work.join("src"), &["--commit", "HEAD"]);
    assert_eq!(out.status.code(), Some(1));
    let json = parse_json(&out.stdout);
    assert_eq!(json["error"]["code"], "OUT_OF_SCOPE", "{json}");
    assert_eq!(
        json["error"]["files_outside"],
        json!(["other.txt"]),
        "{json}"
    );
    assert_eq!(fs::read(&p.todo).expect("read TODO.md"), before);
}

#[test]
fn from_a_subfolder_a_commit_inside_the_files_finishes_the_task() {
    let p = project("1", Some("src/"));
    repo_with_first_commit(&p);
    write(&p.work, "src/b.rs", "pub fn b() {}\n");
    commit(&p.work, &["src/b.rs"], "T-001: work");

    let out = done_in(&p, &p.work.join("src"), &["--commit", "HEAD"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(read_text(&p.todo).contains("- status: done\n"));
}

#[test]
fn a_commit_name_is_kept_as_given_in_a_folder_outside_any_repository() {
    // No `git init`: the temporary folder lies outside every repository, so git cannot check the name.
    let p = project("3", None);

    let out = done(&p, &["--commit", "cafebabe"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(read_text(&p.todo).contains("- commit: cafebabe\n"));
}

#[test]
fn head_is_refused_in_a_folder_outside_any_repository_and_todo_md_is_left_alone() {
    // HEAD names a commit only inside a repository, so without one it is an unknown commit.
    let p = project("3", None);
    let before = fs::read(&p.todo).expect("read TODO.md");

    let out = done(&p, &["--commit", "HEAD"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(parse_json(&out.stdout)["error"]["code"], "BAD_VALUE");
    assert_eq!(fs::read(&p.todo).expect("read TODO.md"), before);
}

#[test]
fn an_unknown_commit_inside_a_repository_is_refused_and_todo_md_is_left_alone() {
    // Inside a repository git knows every commit, so a name it does not know is an unknown commit.
    let p = project("3", None);
    repo_with_first_commit(&p);
    let before = fs::read(&p.todo).expect("read TODO.md");

    let out = done(&p, &["--commit", "deadbeef12"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(parse_json(&out.stdout)["error"]["code"], "BAD_VALUE");
    assert_eq!(fs::read(&p.todo).expect("read TODO.md"), before);
}

// The `done` operations of `todo apply` get the same check, for the whole batch (T-174).

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

#[test]
fn apply_done_inside_the_files_applies_the_whole_batch() {
    let p = two_tasks(Some("src/"));
    repo_with_first_commit(&p);
    write(&p.work, "src/b.rs", "pub fn b() {}\n");
    commit(&p.work, &["src/b.rs"], "T-001: work");

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
fn apply_done_outside_the_files_refuses_the_batch_and_leaves_the_file_alone() {
    let p = two_tasks(Some("src/"));
    repo_with_first_commit(&p);
    write(&p.work, "src/b.rs", "pub fn b() {}\n");
    write(&p.work, "other.txt", "outside the task\n");
    commit(&p.work, &["src/b.rs", "other.txt"], "T-001: work");
    let before = fs::read(&p.todo).expect("read TODO.md");

    let out = apply(&p, &[NOTE_OP, done_op(Some("HEAD")).as_str()], &[]);
    assert_eq!(out.status.code(), Some(1));
    let json = parse_json(&out.stdout);
    assert_eq!(json["error"]["code"], "OUT_OF_SCOPE", "{json}");
    assert_eq!(
        json["error"]["files_outside"],
        json!(["other.txt"]),
        "{json}"
    );
    // The note before the `done` in the batch is not written either.
    assert_eq!(fs::read(&p.todo).expect("read TODO.md"), before);
}

#[test]
fn apply_done_outside_the_files_with_force_applies_and_notes_the_files() {
    let p = two_tasks(Some("src/"));
    repo_with_first_commit(&p);
    write(&p.work, "src/b.rs", "pub fn b() {}\n");
    write(&p.work, "other.txt", "outside the task\n");
    commit(&p.work, &["src/b.rs", "other.txt"], "T-001: work");

    let out = apply(&p, &[NOTE_OP, done_op(Some("HEAD")).as_str()], &["--force"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = read_text(&p.todo);
    assert!(text.contains("- status: done\n"), "{text}");
    assert!(
        text.contains("- note (2026-11-15, alice): forced done outside files: other.txt\n"),
        "{text}"
    );
}

#[test]
fn apply_done_of_a_lowest_level_task_with_files_needs_its_commit_in_git() {
    let p = two_tasks(Some("src/"));
    repo_with_first_commit(&p);
    let before = fs::read(&p.todo).expect("read TODO.md");

    let out = apply(&p, &[NOTE_OP, done_op(None).as_str()], &[]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(parse_json(&out.stdout)["error"]["code"], "MISSING_FIELD");
    assert_eq!(fs::read(&p.todo).expect("read TODO.md"), before);
}

#[test]
fn apply_done_without_git_needs_no_commit() {
    let p = two_tasks(Some("src/"));

    let out = apply(&p, &[NOTE_OP, done_op(None).as_str()], &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(read_text(&p.todo).contains("- status: done\n"));
}

#[test]
fn apply_done_with_an_unknown_commit_is_refused_as_done_refuses_it() {
    let p = two_tasks(Some("src/"));
    repo_with_first_commit(&p);
    let before = fs::read(&p.todo).expect("read TODO.md");

    let out = apply(&p, &[NOTE_OP, done_op(Some("deadbeef9999")).as_str()], &[]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(parse_json(&out.stdout)["error"]["code"], "BAD_VALUE");
    assert_eq!(fs::read(&p.todo).expect("read TODO.md"), before);
}

/// The `- commit:` line of a TODO.md text, or an empty string when there is none.
fn commit_line(text: &str) -> String {
    text.lines()
        .find(|line| line.starts_with("- commit: "))
        .unwrap_or_default()
        .to_string()
}

#[test]
fn apply_writes_the_commit_the_way_done_writes_it() {
    let p = two_tasks(Some("src/"));
    repo_with_first_commit(&p);
    write(&p.work, "src/b.rs", "pub fn b() {}\n");
    commit(&p.work, &["src/b.rs"], "T-001: work");
    let before = fs::read(&p.todo).expect("read TODO.md");
    let short = git(&p.work, &["rev-parse", "--short", "HEAD"]);

    // `done` writes the commit of HEAD as its short hash (R-WRITE-4).
    let done_out = run_todo(
        &p.home,
        &p.work,
        &[
            "--json", "--agent", "alice", "done", "T-001", "--commit", "HEAD",
        ],
        &[("TODO_TEST_NOW", NOW)],
    )
    .expect("run todo");
    assert!(
        done_out.status.success(),
        "{}",
        String::from_utf8_lossy(&done_out.stderr)
    );
    let by_done = commit_line(&read_text(&p.todo));
    assert_eq!(by_done, format!("- commit: {short}"));

    // `apply` of the same operation, from the same TODO.md, writes the same line.
    fs::write(&p.todo, &before).expect("restore TODO.md");
    let out = apply(&p, &[done_op(Some("HEAD")).as_str()], &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(commit_line(&read_text(&p.todo)), by_done);
}

#[test]
fn apply_keeps_a_full_hash_and_resolves_names_as_done_does() {
    let p = two_tasks(Some("src/"));
    repo_with_first_commit(&p);
    write(&p.work, "src/b.rs", "pub fn b() {}\n");
    commit(&p.work, &["src/b.rs"], "T-001: work");
    let full = git(&p.work, &["rev-parse", "HEAD"]);
    let work_short = git(&p.work, &["rev-parse", "--short", "HEAD"]);
    // A commit outside the task's files comes after the work, so HEAD~1 is the work commit, which
    // holds src/b.rs (R-SCOPE-3).
    write(&p.work, "notes.txt", "outside the task\n");
    commit(&p.work, &["notes.txt"], "T-002: notes");
    let before = fs::read(&p.todo).expect("read TODO.md");

    // A full hash is kept as it was given, as `done` keeps it.
    let out = apply(&p, &[done_op(Some(&full)).as_str()], &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        commit_line(&read_text(&p.todo)),
        format!("- commit: {full}")
    );

    // A name such as HEAD~1 is resolved to the short hash of its commit, the work commit here.
    fs::write(&p.todo, &before).expect("restore TODO.md");
    let out = apply(&p, &[done_op(Some("HEAD~1")).as_str()], &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        commit_line(&read_text(&p.todo)),
        format!("- commit: {work_short}")
    );
}

#[test]
fn apply_without_git_keeps_the_commit_as_given() {
    let p = two_tasks(Some("src/"));
    write(
        &p.work,
        "ops.jsonl",
        &format!("{}\n", done_op(Some("cafebabe"))),
    );

    let out = run_todo(
        &p.home,
        &p.work,
        &["--json", "--agent", "alice", "apply", "ops.jsonl"],
        &[("PATH", ""), ("TODO_TEST_NOW", NOW)],
    )
    .expect("run todo");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(commit_line(&read_text(&p.todo)), "- commit: cafebabe");
}

// The order of the checks of `done` and `apply` (T-158, T-173): the commit scope first, then the
// `verify` command, so a commit outside the task's files runs no command.

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
fn done_refuses_a_commit_outside_the_files_before_the_verify_command_runs() {
    let p = verified_repo_project();
    write(&p.work, "src/b.rs", "pub fn b() {}\n");
    write(&p.work, "other.txt", "outside the task\n");
    commit(&p.work, &["src/b.rs", "other.txt"], "T-001: work");
    let before = fs::read(&p.todo).expect("read TODO.md");

    let out = done(&p, &["--commit", "HEAD"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(parse_json(&out.stdout)["error"]["code"], "OUT_OF_SCOPE");
    assert!(
        !p.work.join("ran.txt").exists(),
        "the verify command ran before the commit scope was checked"
    );
    assert_eq!(fs::read(&p.todo).expect("read TODO.md"), before);
}

#[test]
fn done_runs_the_verify_command_once_the_commit_is_inside_the_files() {
    let p = verified_repo_project();
    write(&p.work, "src/b.rs", "pub fn b() {}\n");
    commit(&p.work, &["src/b.rs"], "T-001: work");

    let out = done(&p, &["--commit", "HEAD"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        p.work.join("ran.txt").exists(),
        "the verify command did not run"
    );
    assert!(read_text(&p.todo).contains("verified: 2026-11-15"));
}

#[test]
fn apply_refuses_a_batch_with_a_commit_outside_the_files_before_the_verify_command_runs() {
    let p = verified_repo_project();
    write(&p.work, "src/b.rs", "pub fn b() {}\n");
    write(&p.work, "other.txt", "outside the task\n");
    commit(&p.work, &["src/b.rs", "other.txt"], "T-001: work");
    let before = fs::read(&p.todo).expect("read TODO.md");

    let out = apply(&p, &[done_op(Some("HEAD")).as_str()], &[]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(parse_json(&out.stdout)["error"]["code"], "OUT_OF_SCOPE");
    assert!(
        !p.work.join("ran.txt").exists(),
        "the verify command ran before the commit scope was checked"
    );
    assert_eq!(fs::read(&p.todo).expect("read TODO.md"), before);
}
