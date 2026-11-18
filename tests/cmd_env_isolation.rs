//! The test helpers run `todo` without the `TODO_*` variables of the environment (TEST-8), so a test
//! never reads or writes the task list, agent or clock of whoever runs the tests (T-120).

mod common;

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use tempfile::TempDir;

use common::{parse_json, read_text, run_in, setup_proj, stderr_text, stdout_text};

/// Set on the child process that runs the checks. The parent test starts that child.
const CHILD_MARKER: &str = "TODO_TEST_ENV_ISOLATION_CHILD";

/// This test's name, which the child runs alone (`--exact`).
const TEST_NAME: &str = "helpers_ignore_the_todo_variables_of_the_environment";

const OUTSIDE_BODY: &str =
    "<!-- TEST FIXTURE: invented data, not a real task list -->\n# Outside\n\n## Now\n";

const WORK_BODY: &str =
    "<!-- TEST FIXTURE: invented data, not a real task list -->\n# Work\n\n## Now\n";

#[test]
fn helpers_ignore_the_todo_variables_of_the_environment() {
    if std::env::var_os(CHILD_MARKER).is_some() {
        check_helpers_against_the_environment();
        return;
    }

    // The checks need TODO_FILE and TODO_AGENT in the environment of the `todo` they run. Setting
    // them here would change the environment of every test in this binary, so the checks run in a
    // child process of this test instead.
    let tmp = TempDir::new().unwrap();
    let outside = tmp.path().join("elsewhere").join("TODO.md");
    fs::create_dir_all(outside.parent().unwrap()).unwrap();
    fs::write(&outside, OUTSIDE_BODY).unwrap();

    let mut command = Command::new(std::env::current_exe().unwrap());
    common::clear_todo_environment(&mut command)
        .args(["--exact", TEST_NAME])
        .env(CHILD_MARKER, "1")
        .env("TODO_FILE", &outside)
        .env("TODO_AGENT", "env-agent");
    let child = command.output().unwrap();
    assert!(
        child.status.success(),
        "the checks failed in the child process:\n{}\n{}",
        stdout_text(&child),
        stderr_text(&child)
    );
}

/// Runs in the child process, where `TODO_FILE` names `outside` and `TODO_AGENT` is `env-agent`.
fn check_helpers_against_the_environment() {
    let outside = PathBuf::from(std::env::var_os("TODO_FILE").unwrap());
    let outside_before = read_text(&outside);
    let tmp = TempDir::new().unwrap();
    let (_home, work, work_todo) = setup_proj(&tmp, WORK_BODY);

    // TODO_AGENT must not reach the command: a write without --agent is refused (exit 2).
    let no_agent = run_in(
        &work,
        &[
            "--json",
            "add",
            "--title",
            "Added without an agent",
            "--priority",
            "P2",
            "--section",
            "Now",
        ],
    );
    assert_eq!(
        no_agent.status.code(),
        Some(2),
        "{}",
        stdout_text(&no_agent)
    );
    assert_eq!(parse_json(&no_agent.stdout)["error"]["code"], "NO_AGENT");

    // TODO_FILE must not reach the command: the task goes to the TODO.md in the working folder.
    let added = run_in(
        &work,
        &[
            "add",
            "--title",
            "Written by the helper test",
            "--priority",
            "P2",
            "--section",
            "Now",
            "--agent",
            "helper-test",
        ],
    );
    assert!(added.status.success(), "{}", stderr_text(&added));
    assert!(read_text(&work_todo).contains("Written by the helper test"));
    assert_eq!(
        read_text(&outside),
        outside_before,
        "the file named by TODO_FILE was changed"
    );
}
