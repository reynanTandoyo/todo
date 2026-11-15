//! CLI integration tests for T-004 and T-005 (TEST-8).

use std::process::{Command, Stdio};

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use tempfile::TempDir;

fn todo_cmd(home: &TempDir) -> assert_cmd::Command {
    let mut cmd = cargo_bin_cmd!("todo");
    cmd.env_remove("TODO_FILE")
        .env_remove("TODO_AGENT")
        .env("HOME", home.path())
        .env("APPDATA", home.path())
        .env("NO_COLOR", "1");
    cmd
}

#[test]
fn no_args_prints_overview_and_exits_zero() {
    let home = TempDir::new().unwrap();
    todo_cmd(&home)
        .assert()
        .success()
        .stdout(predicate::str::contains("todo "))
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains("help"));
}

#[test]
fn version_flag_prints_version() {
    let home = TempDir::new().unwrap();
    todo_cmd(&home)
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")))
        .stdout(predicate::str::contains(env!("TODO_GIT_COMMIT")));

    let json_out = todo_cmd(&home)
        .args(["--version", "--json"])
        .output()
        .unwrap();
    assert!(json_out.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&json_out.stdout).unwrap();
    assert_eq!(parsed["schema"], "todo.version.v1");
    assert_eq!(parsed["data"]["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(parsed["data"]["commit"], env!("TODO_GIT_COMMIT"));
}

#[test]
fn help_lists_every_registered_command() {
    let home = TempDir::new().unwrap();
    let output = todo_cmd(&home).arg("help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    for meta in todo::cli::COMMANDS {
        assert!(
            stdout.contains(meta.name),
            "help output missing registered command `{}`:\n{stdout}",
            meta.name
        );
    }
}

#[test]
fn help_command_explains_with_examples() {
    let home = TempDir::new().unwrap();
    todo_cmd(&home)
        .args(["help", "help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Examples:"))
        .stdout(predicate::str::contains("todo help"));
}

#[test]
fn unknown_help_topic_exits_2() {
    let home = TempDir::new().unwrap();
    todo_cmd(&home)
        .args(["help", "nonexistent"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("unknown help topic"));
}

#[test]
fn invalid_agent_flag_and_env_exit_2() {
    let home = TempDir::new().unwrap();
    todo_cmd(&home)
        .args(["--agent", "bad name!", "help"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("invalid agent name"));

    todo_cmd(&home)
        .env("TODO_AGENT", "bad/agent")
        .arg("help")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("invalid agent name"));
}

#[test]
fn valid_global_options_are_accepted() {
    let home = TempDir::new().unwrap();
    todo_cmd(&home)
        .args([
            "--file",
            "custom/TODO.md",
            "--agent",
            "agent-1_ok.2",
            "--dry-run",
            "--yes",
            "--force",
            "help",
        ])
        .assert()
        .success();
}

#[test]
fn json_overview_and_error_envelopes_match_snapshots() {
    let home = TempDir::new().unwrap();

    let ok_out = todo_cmd(&home).arg("--json").output().unwrap();
    assert!(ok_out.status.success());
    let ok_json: serde_json::Value = serde_json::from_slice(&ok_out.stdout).unwrap();
    insta::assert_json_snapshot!("cli_overview_json", ok_json);

    let err_out = todo_cmd(&home)
        .args(["--json", "help", "nonexistent"])
        .output()
        .unwrap();
    assert_eq!(err_out.status.code(), Some(2));
    assert!(err_out.stderr.is_empty());
    let err_json: serde_json::Value = serde_json::from_slice(&err_out.stdout).unwrap();
    insta::assert_json_snapshot!("cli_help_unknown_json", err_json);
}

#[test]
fn closed_pipe_exits_zero() {
    let home = TempDir::new().unwrap();
    let bin = assert_cmd::cargo::cargo_bin!("todo");
    let mut child = Command::new(bin)
        .arg("--help")
        .env_remove("TODO_FILE")
        .env_remove("TODO_AGENT")
        .env("HOME", home.path())
        .env("APPDATA", home.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    // Close the read end of stdout immediately.
    drop(child.stdout.take());
    let status = child.wait().unwrap();
    assert_eq!(status.code(), Some(0));
}

#[test]
fn help_agents_prints_builtin_rules_text() {
    let home = TempDir::new().unwrap();
    let out = todo_cmd(&home).args(["help", "agents"]).output().unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        todo::agents::RULES_TEXT
    );

    let json_out = todo_cmd(&home)
        .args(["--json", "help", "agents"])
        .output()
        .unwrap();
    assert!(json_out.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&json_out.stdout).unwrap();
    assert_eq!(parsed["schema"], "todo.help.v1");
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["topic"], "agents");
    assert_eq!(parsed["data"]["rules"], todo::agents::RULES_TEXT);
}
