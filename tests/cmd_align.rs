mod common;

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use common::{git, parse_json, read_todo, run_in, stdout_text};

const HEAD: &str = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 1 -->
# Align Tasks

## Tasks
";

fn init_repo(dir: &Path) {
    git(dir, &["init", "-q"]);
}

fn commit(dir: &Path, subject: &str) -> String {
    git(
        dir,
        &[
            "-c",
            "user.name=x",
            "-c",
            "user.email=x@example.invalid",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            subject,
        ],
    );
    git(dir, &["rev-parse", "--short", "HEAD"])
}

fn task(id: &str, status: &str, extra: &str) -> String {
    format!("\n### {id} · Task {id}\n- status: {status}\n{extra}")
}

#[test]
fn reports_all_three_kinds_and_never_writes() {
    let dir = TempDir::new().expect("tempdir");
    init_repo(dir.path());
    let real = commit(dir.path(), "T-001: first thing");
    commit(dir.path(), "T-002: work on an open task");
    commit(dir.path(), "unrelated change");

    let mut body = HEAD.to_string();
    body.push_str(&task("T-001", "done", &format!("- commit: {real}\n")));
    body.push_str(&task("T-002", "open", ""));
    body.push_str(&task("T-003", "done", ""));
    body.push_str(&task("T-004", "done", "- commit: deadbeef\n"));
    body.push_str(&task("T-005", "open", ""));
    body.push_str(&task("T-006", "dropped", ""));
    fs::write(dir.path().join("TODO.md"), &body).expect("write");

    let text = run_in(dir.path(), &["align"]);
    assert_eq!(text.status.code(), Some(1));
    let stdout = stdout_text(&text);
    assert!(stdout.contains("T-002 COMMITTED_NOT_DONE"), "{stdout}");
    assert!(stdout.contains("T-003 DONE_WITHOUT_COMMIT"), "{stdout}");
    assert!(
        stdout.contains("T-004 COMMIT_MISSING (deadbeef)"),
        "{stdout}"
    );
    assert!(!stdout.contains("T-001"), "{stdout}");
    assert!(!stdout.contains("T-005"), "{stdout}");
    assert!(!stdout.contains("T-006"), "{stdout}");

    let json_out = run_in(dir.path(), &["align", "--json"]);
    assert_eq!(json_out.status.code(), Some(1));
    let json = parse_json(&json_out.stdout);
    assert_eq!(json["schema"], "todo.align.v1");
    assert_eq!(json["ok"], true);
    let findings = json["data"]["findings"].as_array().expect("findings");
    assert_eq!(findings.len(), 3);
    assert_eq!(findings[0]["task"], "T-002");
    assert_eq!(findings[0]["kind"], "COMMITTED_NOT_DONE");
    assert_eq!(findings[0]["commits"].as_array().map(Vec::len), Some(1));
    assert_eq!(findings[0]["line"], 11);
    assert_eq!(findings[1]["kind"], "DONE_WITHOUT_COMMIT");
    assert_eq!(findings[1]["commits"].as_array().map(Vec::len), Some(0));
    assert_eq!(findings[2]["kind"], "COMMIT_MISSING");
    assert_eq!(findings[2]["commits"][0], "deadbeef");

    assert_eq!(read_todo(dir.path()), body, "align must not change TODO.md");
}

#[test]
fn clean_exits_zero() {
    let dir = TempDir::new().expect("tempdir");
    init_repo(dir.path());
    let real = commit(dir.path(), "T-001: first");
    let mut body = HEAD.to_string();
    body.push_str(&task("T-001", "done", &format!("- commit: {real}\n")));
    body.push_str(&task("T-002", "open", ""));
    fs::write(dir.path().join("TODO.md"), body).expect("write");

    let out = run_in(dir.path(), &["align"]);
    assert_eq!(out.status.code(), Some(0), "{}", stdout_text(&out));
    let json_out = run_in(dir.path(), &["align", "--json"]);
    assert_eq!(json_out.status.code(), Some(0));
    let json = parse_json(&json_out.stdout);
    assert_eq!(json["data"]["findings"].as_array().map(Vec::len), Some(0));
}

#[test]
fn answered_questions_are_ignored() {
    let dir = TempDir::new().expect("tempdir");
    init_repo(dir.path());
    commit(dir.path(), "unrelated change");
    let mut body = HEAD.to_string();
    body.push_str(&task("T-001", "done", "- type: question\n"));
    fs::write(dir.path().join("TODO.md"), body).expect("write");
    let out = run_in(dir.path(), &["align"]);
    assert_eq!(out.status.code(), Some(0), "{}", stdout_text(&out));
}

#[test]
fn done_task_matched_by_subject_only_is_fine() {
    let dir = TempDir::new().expect("tempdir");
    init_repo(dir.path());
    commit(dir.path(), "T-007: shipped");
    let mut body = HEAD.to_string();
    body.push_str(&task("T-007", "done", ""));
    fs::write(dir.path().join("TODO.md"), body).expect("write");
    let out = run_in(dir.path(), &["align"]);
    assert_eq!(out.status.code(), Some(0), "{}", stdout_text(&out));
}

#[test]
fn not_a_repo_is_no_git_history() {
    let dir = TempDir::new().expect("tempdir");
    let mut body = HEAD.to_string();
    body.push_str(&task("T-001", "open", ""));
    fs::write(dir.path().join("TODO.md"), body).expect("write");

    let out = run_in(dir.path(), &["align", "--json"]);
    assert_eq!(out.status.code(), Some(5));
    let json = parse_json(&out.stdout);
    assert_eq!(json["ok"], false);
    assert_eq!(json["error"]["code"], "NO_GIT_HISTORY");
}

#[test]
fn repo_without_commits_is_no_git_history() {
    let dir = TempDir::new().expect("tempdir");
    init_repo(dir.path());
    let mut body = HEAD.to_string();
    body.push_str(&task("T-001", "open", ""));
    fs::write(dir.path().join("TODO.md"), body).expect("write");

    let out = run_in(dir.path(), &["align"]);
    assert_eq!(out.status.code(), Some(5));
}

#[test]
fn shallow_clone_is_no_git_history() {
    let origin = TempDir::new().expect("tempdir");
    init_repo(origin.path());
    commit(origin.path(), "one");
    commit(origin.path(), "two");

    let holder = TempDir::new().expect("tempdir");
    let clone = holder.path().join("clone");
    let url = format!(
        "file://{}",
        origin.path().display().to_string().replace('\\', "/")
    );
    git(
        holder.path(),
        &["clone", "-q", "--depth", "1", &url, "clone"],
    );
    let mut body = HEAD.to_string();
    body.push_str(&task("T-001", "open", ""));
    fs::write(clone.join("TODO.md"), body).expect("write");

    let out = run_in(&clone, &["align", "--json"]);
    assert_eq!(out.status.code(), Some(5), "{}", stdout_text(&out));
    let json = parse_json(&out.stdout);
    assert_eq!(json["error"]["code"], "NO_GIT_HISTORY");
}
