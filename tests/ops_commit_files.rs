//! Tests of `worktree_differences` (R-SCOPE-3): which files inside a folder are not as a commit holds
//! them, so `done --commit` can refuse a commit that was checked before a later change. Every fixture
//! is invented and written into a temporary folder with its own git repository.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use common::FIXTURE_MARKER;
use todo::error::ErrorCode;
use todo::ops::commit_files::worktree_differences;

/// The files that the commit check always allows (`TODO.md`, as `done` passes them).
const ALLOWED: &[&str] = &["TODO.md"];

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

/// A folder that is a repository whose first commit holds TODO.md and `src/a.rs`.
fn repo() -> TempDir {
    let tmp = TempDir::new().expect("tempdir");
    write(tmp.path(), "TODO.md", &format!("{FIXTURE_MARKER}\n"));
    write(tmp.path(), "src/a.rs", "pub fn a() {}\n");
    git(tmp.path(), &["init", "-q"]);
    commit(tmp.path(), &["TODO.md", "src/a.rs"], "T-001: start");
    tmp
}

/// The differences of `dir` against HEAD, with TODO.md always allowed.
fn differences(dir: &Path) -> Vec<String> {
    worktree_differences(dir, "HEAD", ALLOWED).expect("compare the folder with HEAD")
}

/// The expected paths, as strings.
fn listed(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

#[test]
fn a_clean_folder_has_no_differences() {
    let tmp = repo();
    assert_eq!(differences(tmp.path()), listed(&[]));
}

#[test]
fn a_changed_file_is_listed() {
    let tmp = repo();
    write(tmp.path(), "src/a.rs", "pub fn a() { changed(); }\n");
    assert_eq!(differences(tmp.path()), listed(&["src/a.rs"]));
}

#[test]
fn a_new_file_that_was_not_added_is_listed() {
    let tmp = repo();
    write(tmp.path(), "src/new.rs", "pub fn new() {}\n");
    assert_eq!(differences(tmp.path()), listed(&["src/new.rs"]));
}

#[test]
fn a_deleted_file_is_listed() {
    let tmp = repo();
    fs::remove_file(tmp.path().join("src/a.rs")).expect("delete file");
    assert_eq!(differences(tmp.path()), listed(&["src/a.rs"]));
}

#[test]
fn a_changed_todo_md_is_not_listed() {
    let tmp = repo();
    write(
        tmp.path(),
        "TODO.md",
        &format!("{FIXTURE_MARKER}\n## Queue\n"),
    );
    assert_eq!(differences(tmp.path()), listed(&[]));
}

#[test]
fn a_file_that_a_committed_gitignore_names_is_not_listed() {
    let tmp = repo();
    write(tmp.path(), ".gitignore", "scratch.log\n");
    commit(tmp.path(), &[".gitignore"], "T-001: ignore scratch files");
    write(tmp.path(), "scratch.log", "not for git\n");
    // A new file that git does not ignore is still listed in the same folder.
    write(tmp.path(), "src/new.rs", "pub fn new() {}\n");
    assert_eq!(differences(tmp.path()), listed(&["src/new.rs"]));
}

#[test]
fn a_new_file_with_spaces_and_letters_outside_ascii_is_listed_as_written() {
    let tmp = repo();
    write(tmp.path(), "src/my notes/café.md", "draft\n");
    assert_eq!(differences(tmp.path()), listed(&["src/my notes/café.md"]));
}

/// A repository whose root holds the folder `proj`, with TODO.md and `src/a.rs` inside it, and a
/// folder `other` outside it. Returns `(tempdir, folder proj)`.
fn nested_repo() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().expect("tempdir");
    let proj = tmp.path().join("proj");
    write(&proj, "TODO.md", &format!("{FIXTURE_MARKER}\n"));
    write(&proj, "src/a.rs", "pub fn a() {}\n");
    write(tmp.path(), "other/x.txt", "outside the folder\n");
    git(tmp.path(), &["init", "-q"]);
    commit(
        tmp.path(),
        &["proj/TODO.md", "proj/src/a.rs", "other/x.txt"],
        "T-001: start",
    );
    (tmp, proj)
}

#[test]
fn paths_are_relative_to_the_folder_and_changes_outside_it_are_not_listed() {
    let (tmp, proj) = nested_repo();
    write(&proj, "src/a.rs", "pub fn a() { changed(); }\n");
    write(&proj, "src/b.rs", "pub fn b() {}\n");
    write(tmp.path(), "other/x.txt", "changed outside the folder\n");
    assert_eq!(
        worktree_differences(&proj, "HEAD", ALLOWED).expect("compare the folder with HEAD"),
        listed(&["src/a.rs", "src/b.rs"])
    );
}

#[test]
fn the_result_is_sorted() {
    let tmp = repo();
    // The changed file comes from git's diff and the new file from git's list of new files, so the
    // raw output is not in order; the result is.
    write(tmp.path(), "src/a.rs", "pub fn a() { changed(); }\n");
    write(tmp.path(), "src/0.rs", "pub fn zero() {}\n");
    assert_eq!(differences(tmp.path()), listed(&["src/0.rs", "src/a.rs"]));
}

#[test]
fn an_unknown_commit_is_refused_with_bad_value() {
    let tmp = repo();
    let err = worktree_differences(tmp.path(), "deadbeef9999", ALLOWED)
        .expect_err("an unknown commit is refused");
    assert_eq!(err.code, ErrorCode::BadValue);
    assert_eq!(
        err.message,
        "could not compare the folder with git commit 'deadbeef9999'"
    );
}

#[test]
fn a_commit_that_starts_with_a_dash_is_refused_as_unknown() {
    let tmp = repo();
    let err = worktree_differences(tmp.path(), "--output=x", ALLOWED)
        .expect_err("a commit that starts with a dash is refused");
    assert_eq!(err.code, ErrorCode::BadValue);
    assert_eq!(err.message, "unknown git commit '--output=x'");
}
