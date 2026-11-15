//! Reusable process-spawning helpers for multi-process locking tests (TEST-5, TEST-8).
#![allow(dead_code)]

use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

/// Path to the compiled `todo` binary built by Cargo.
#[must_use]
pub fn todo_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_todo"))
}

/// Builds a `Command` targeting the `todo` binary with a clean test environment (TEST-8).
#[must_use]
pub fn clean_todo_command(home: &Path, work_dir: &Path) -> Command {
    let mut cmd = Command::new(todo_bin());
    cmd.current_dir(work_dir)
        .env_remove("TODO_FILE")
        .env_remove("TODO_AGENT")
        .env("HOME", home)
        .env("APPDATA", home)
        .env("NO_COLOR", "1");
    cmd
}

/// Spawns `todo` in the background with stdout/stderr piped.
pub fn spawn_todo(
    home: &Path,
    work_dir: &Path,
    args: &[&str],
    extra_env: &[(&str, &str)],
) -> std::io::Result<Child> {
    let mut cmd = clean_todo_command(home, work_dir);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
    for &(k, v) in extra_env {
        cmd.env(k, v);
    }
    cmd.spawn()
}

/// Runs `todo` to completion and returns its `Output`.
pub fn run_todo(
    home: &Path,
    work_dir: &Path,
    args: &[&str],
    extra_env: &[(&str, &str)],
) -> std::io::Result<Output> {
    let mut cmd = clean_todo_command(home, work_dir);
    cmd.args(args);
    for &(k, v) in extra_env {
        cmd.env(k, v);
    }
    cmd.output()
}

/// Polls until `path` exists on disk or `timeout` elapses.
pub fn wait_for_file(path: &Path, timeout: Duration) -> std::io::Result<()> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if path.exists() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::TimedOut,
        format!("timed out waiting for marker file {}", path.display()),
    ))
}

/// Spawns `n` parallel `todo` invocations and waits for all of them to finish.
pub fn run_parallel_todos(
    home: &Path,
    work_dir: &Path,
    invocations: &[Vec<String>],
) -> std::io::Result<Vec<Output>> {
    let mut children = Vec::with_capacity(invocations.len());
    for args in invocations {
        let mut cmd = clean_todo_command(home, work_dir);
        cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
        children.push(cmd.spawn()?);
    }
    let mut outputs = Vec::with_capacity(children.len());
    for child in children {
        outputs.push(child.wait_with_output()?);
    }
    Ok(outputs)
}
