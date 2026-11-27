//! CLI tests for the check that `done --commit` makes of the task's files as they are on disk
//! (R-SCOPE-3; T-255). A changed or new file inside the task's `files` that the commit does not hold
//! refuses `done` (`UNCOMMITTED_CHANGES`) before the `verify` command runs and again after it passed;
//! `--force` finishes the task and notes the files. A file outside the files, `TODO.md` and a commit
//! that holds the files all pass. Every fixture is invented and written into a temporary folder with
//! its own git repository.

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

/// `todo --json done T-001 --agent alice` plus `extra`, run in the project folder.
fn done(p: &Proj, extra: &[&str]) -> Output {
    let mut args = vec!["--json", "done", "T-001", "--agent", "alice"];
    args.extend_from_slice(extra);
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

/// A project like [`verified_repo_project`], but with the `verify` command `verify`.
fn project_with_verify(verify: &str) -> Proj {
    let tmp = TempDir::new().expect("tempdir");
    let (home, work, todo) = setup_proj(&tmp, &verify_body("src/", verify));
    let p = Proj {
        _tmp: tmp,
        home,
        work,
        todo,
    };
    repo_with_first_commit(&p);
    p
}

/// Asserts that `done` was refused with `UNCOMMITTED_CHANGES` (exit 1) and names `files` in the
/// JSON error; the message says to commit them and to run `done --commit HEAD` again.
fn assert_refused_uncommitted(out: &Output, files: &[&str]) {
    assert_eq!(out.status.code(), Some(1), "{}", parse_json(&out.stdout));
    let json = parse_json(&out.stdout);
    assert_eq!(json["error"]["code"], "UNCOMMITTED_CHANGES", "{json}");
    assert_eq!(json["error"]["files_uncommitted"], json!(files), "{json}");
    assert!(
        json["error"]["message"]
            .as_str()
            .is_some_and(|m| m.contains("--commit HEAD")),
        "{json}"
    );
}

// The refusals: each writes nothing, and a refusal before the command runs no command.

#[test]
fn a_file_changed_after_the_commit_refuses_done() {
    let p = project("1", Some("src/"));
    repo_with_first_commit(&p);
    write(&p.work, "src/a.rs", "pub fn a() { changed }\n");
    let before = fs::read(&p.todo).expect("read TODO.md");

    let out = done(&p, &["--commit", "HEAD"]);
    assert_refused_uncommitted(&out, &["src/a.rs"]);
    assert_eq!(fs::read(&p.todo).expect("read TODO.md"), before);
}

#[test]
fn a_new_file_inside_the_files_that_is_not_added_refuses_done() {
    let p = project("1", Some("src/"));
    repo_with_first_commit(&p);
    write(&p.work, "src/b.rs", "pub fn b() {}\n");
    commit(&p.work, &["src/b.rs"], "T-001: work");
    write(&p.work, "src/new.rs", "pub fn new() {}\n");
    let before = fs::read(&p.todo).expect("read TODO.md");

    let out = done(&p, &["--commit", "HEAD"]);
    assert_refused_uncommitted(&out, &["src/new.rs"]);
    assert_eq!(fs::read(&p.todo).expect("read TODO.md"), before);
}

#[test]
fn a_changed_file_refuses_done_before_the_verify_command_runs() {
    let p = verified_repo_project();
    write(&p.work, "src/a.rs", "pub fn a() { changed }\n");
    let before = fs::read(&p.todo).expect("read TODO.md");

    let out = done(&p, &["--commit", "HEAD"]);
    assert_refused_uncommitted(&out, &["src/a.rs"]);
    assert!(
        !p.work.join("ran.txt").exists(),
        "the verify command ran before the uncommitted changes were checked"
    );
    let text = read_text(&p.todo);
    assert!(!text.contains("attempts"), "{text}");
    assert_eq!(fs::read(&p.todo).expect("read TODO.md"), before);
}

#[test]
fn a_changed_file_refuses_done_in_wave_mode_too() {
    let p = verified_repo_project();
    write_toml(&p.work, "verify_when = \"wave\"\n");
    write(&p.work, "src/a.rs", "pub fn a() { changed }\n");
    let before = fs::read(&p.todo).expect("read TODO.md");

    let out = done(&p, &["--commit", "HEAD"]);
    assert_refused_uncommitted(&out, &["src/a.rs"]);
    assert_eq!(fs::read(&p.todo).expect("read TODO.md"), before);
}

#[test]
fn a_later_commit_changing_the_files_refuses_done_of_an_earlier_commit() {
    let p = project("1", Some("src/"));
    repo_with_first_commit(&p);
    let first = git(&p.work, &["rev-parse", "HEAD"]);
    write(&p.work, "src/a.rs", "pub fn a() { second }\n");
    commit(&p.work, &["src/a.rs"], "T-001: second");
    let before = fs::read(&p.todo).expect("read TODO.md");

    let out = done(&p, &["--commit", &first]);
    assert_refused_uncommitted(&out, &["src/a.rs"]);
    assert_eq!(fs::read(&p.todo).expect("read TODO.md"), before);
}

#[test]
fn a_verify_command_that_changes_a_file_refuses_done_after_it_passed() {
    let change = if cfg!(windows) {
        "Add-Content -Path src/a.rs -Value x"
    } else {
        "echo x >> src/a.rs"
    };
    let p = project_with_verify(change);
    let before = fs::read(&p.todo).expect("read TODO.md");

    let out = done(&p, &["--commit", "HEAD"]);
    assert_refused_uncommitted(&out, &["src/a.rs"]);
    let text = read_text(&p.todo);
    assert!(!text.contains("- status: done"), "{text}");
    assert!(!text.contains("verified"), "{text}");
    assert_eq!(fs::read(&p.todo).expect("read TODO.md"), before);
}

// The successes: a commit that holds the task's files passes, and so does a file that is not one of them.

#[test]
fn a_clean_folder_finishes_the_task() {
    let p = project("1", Some("src/"));
    repo_with_first_commit(&p);

    let out = done(&p, &["--commit", "HEAD"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(read_text(&p.todo).contains("- status: done\n"));
}

#[test]
fn a_changed_file_outside_the_files_does_not_refuse_done() {
    let p = project("1", Some("src/"));
    repo_with_first_commit(&p);
    // other.txt is changed after the commit that holds it, and the commit HEAD does not touch it.
    write(&p.work, "other.txt", "first\n");
    commit(&p.work, &["other.txt"], "T-001: other file");
    write(&p.work, "src/b.rs", "pub fn b() {}\n");
    commit(&p.work, &["src/b.rs"], "T-001: work");
    write(&p.work, "other.txt", "second\n");

    let out = done(&p, &["--commit", "HEAD"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(read_text(&p.todo).contains("- status: done\n"));
}

#[test]
fn force_finishes_the_task_and_notes_the_uncommitted_file() {
    let p = project("1", Some("src/"));
    repo_with_first_commit(&p);
    write(&p.work, "src/a.rs", "pub fn a() { changed }\n");

    let out = done(&p, &["--commit", "HEAD", "--force"]);
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
fn a_task_without_files_is_not_checked() {
    let p = project("1", None);
    repo_with_first_commit(&p);
    write(&p.work, "src/a.rs", "pub fn a() { changed }\n");

    let out = done(&p, &["--commit", "HEAD"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(read_text(&p.todo).contains("- status: done\n"));
}

#[test]
fn without_a_commit_a_task_above_the_target_level_is_not_checked() {
    let p = project("3", Some("src/"));
    repo_with_first_commit(&p);
    write(&p.work, "src/a.rs", "pub fn a() { changed }\n");

    let out = done(&p, &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(read_text(&p.todo).contains("- status: done\n"));
}

#[test]
fn todo_md_changed_under_a_pattern_never_counts() {
    let p = project("3", Some("*.md, src/"));
    repo_with_first_commit(&p);
    let note = run_todo(
        &p.home,
        &p.work,
        &["note", "T-001", "x", "--agent", "alice"],
        &[("TODO_TEST_NOW", NOW)],
    )
    .expect("run todo");
    assert!(
        note.status.success(),
        "{}",
        String::from_utf8_lossy(&note.stderr)
    );

    let out = done(&p, &["--commit", "HEAD"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(read_text(&p.todo).contains("- status: done\n"));
}
