//! Shared helpers for the integration tests (AGENTS.md "Test rules").
//!
//! Every file in `tests/` that needs these helpers declares `mod common;`. Cargo does not build
//! this folder as a test crate of its own. Helpers panic on setup failures, which is acceptable
//! in test code, so the module-level allow below covers the `unwrap`/`expect` calls (clippy only
//! exempts `#[test]` bodies). Functions a given test file does not use are allowed as dead code.
#![allow(dead_code, clippy::expect_used, clippy::unwrap_used)]

use std::fs;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;
use tempfile::TempDir;

/// First line of every invented fixture (AGENTS.md "Test rules").
pub const FIXTURE_MARKER: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->";

/// A `todo` command with the inherited environment as the test process has it.
#[must_use]
pub fn todo_bin() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_todo"));
    // Claude Code exports its session ID to the commands it runs; tests must not tag claims.
    cmd.env_remove("TODO_SESSION")
        .env_remove("CLAUDE_CODE_SESSION_ID");
    cmd
}

/// Runs `todo` with `args` in `dir` and returns its output. The environment is inherited.
#[must_use]
pub fn run_in(dir: &Path, args: &[&str]) -> Output {
    todo_bin()
        .current_dir(dir)
        .args(args)
        .output()
        .expect("run todo")
}

/// Builds a `todo` command with a clean test environment (TEST-8): no `TODO_FILE` or
/// `TODO_AGENT`, `HOME` and `APPDATA` pointing at `home`, colour off.
#[must_use]
pub fn clean_todo_command(home: &Path, work_dir: &Path) -> Command {
    let mut cmd = todo_bin();
    cmd.current_dir(work_dir)
        .env_remove("TODO_FILE")
        .env_remove("TODO_AGENT")
        .env_remove("TODO_SESSION")
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .env("HOME", home)
        .env("APPDATA", home)
        .env("GIT_CONFIG_GLOBAL", home.join("gitconfig-none"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("NO_COLOR", "1");
    cmd
}

/// `assert_cmd` variant of [`clean_todo_command`], for tests that use `assert_cmd` matchers.
#[must_use]
pub fn assert_todo(home: &Path) -> assert_cmd::Command {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("todo");
    cmd.env_remove("TODO_FILE")
        .env_remove("TODO_AGENT")
        .env_remove("TODO_SESSION")
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .env("HOME", home)
        .env("APPDATA", home)
        .env("GIT_CONFIG_GLOBAL", home.join("gitconfig-none"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("NO_COLOR", "1");
    cmd
}

/// Spawns `todo` in the background with stdout/stderr piped.
pub fn spawn_todo(
    home: &Path,
    work_dir: &Path,
    args: &[&str],
    extra_env: &[(&str, &str)],
) -> io::Result<Child> {
    let mut cmd = clean_todo_command(home, work_dir);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
    for &(k, v) in extra_env {
        cmd.env(k, v);
    }
    cmd.spawn()
}

/// Runs `todo` to completion with a clean environment and returns its `Output`.
pub fn run_todo(
    home: &Path,
    work_dir: &Path,
    args: &[&str],
    extra_env: &[(&str, &str)],
) -> io::Result<Output> {
    let mut cmd = clean_todo_command(home, work_dir);
    cmd.args(args);
    for &(k, v) in extra_env {
        cmd.env(k, v);
    }
    cmd.output()
}

/// Spawns `n` parallel `todo` invocations and waits for all of them to finish.
pub fn run_parallel_todos(
    home: &Path,
    work_dir: &Path,
    invocations: &[Vec<String>],
) -> io::Result<Vec<Output>> {
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

/// Polls until `path` exists on disk or `timeout` elapses.
pub fn wait_for_file(path: &Path, timeout: Duration) -> io::Result<()> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if path.exists() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        format!("timed out waiting for marker file {}", path.display()),
    ))
}

/// Creates `<tmp>/home` and `<tmp>/work` and writes `body` to `<work>/TODO.md`.
/// Returns `(home, work, todo_file)`.
#[must_use]
pub fn setup_proj(tmp: &TempDir, body: &str) -> (PathBuf, PathBuf, PathBuf) {
    let home = tmp.path().join("home");
    let work = tmp.path().join("work");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    let todo_file = write_todo(&work, body);
    (home, work, todo_file)
}

/// Like [`setup_proj`], but copies `tests/fixtures/<name>` as `TODO.md`.
#[must_use]
pub fn setup_fixture_proj(tmp: &TempDir, name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let home = tmp.path().join("home");
    let work = tmp.path().join("work");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    copy_fixture(&work, name);
    let todo_file = work.join("TODO.md");
    (home, work, todo_file)
}

/// Runs `todo` in `dir` with `args`, feeding `stdin_data` on standard input.
#[must_use]
pub fn run_with_stdin(dir: &Path, args: &[&str], stdin_data: &str) -> Output {
    let mut child = todo_bin()
        .current_dir(dir)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn todo");
    child
        .stdin
        .as_mut()
        .expect("todo stdin")
        .write_all(stdin_data.as_bytes())
        .expect("write todo stdin");
    child.wait_with_output().expect("wait for todo")
}

/// Writes `content` to `<dir>/TODO.md` and returns the path.
pub fn write_todo(dir: &Path, content: &str) -> PathBuf {
    let path = dir.join("TODO.md");
    fs::write(&path, content).expect("write TODO.md");
    path
}

/// Reads the UTF-8 text of `path`; panics if it cannot be read.
#[must_use]
pub fn read_text(path: &Path) -> String {
    fs::read_to_string(path).expect("read file")
}

/// Reads `<dir>/TODO.md` as text.
#[must_use]
pub fn read_todo(dir: &Path) -> String {
    read_text(&dir.join("TODO.md"))
}

/// Writes `content` to `<dir>/todo.toml`.
pub fn write_toml(dir: &Path, content: &str) {
    fs::write(dir.join("todo.toml"), content).expect("write todo.toml");
}

/// Bytes of the invented fixture `tests/fixtures/<name>`.
#[must_use]
pub fn fixture(name: &str) -> Vec<u8> {
    fs::read(Path::new("tests/fixtures").join(name)).expect("read fixture")
}

/// Copies `tests/fixtures/<name>` into `<dir>/TODO.md`.
pub fn copy_fixture(dir: &Path, name: &str) {
    fs::write(dir.join("TODO.md"), fixture(name)).expect("copy fixture");
}

/// Copies the clean fixture `tests/fixtures/clean_basic.md` into `<dir>/TODO.md`.
pub fn copy_clean_fixture(dir: &Path) {
    copy_fixture(dir, "clean_basic.md");
}

/// Runs `git` with `args` in `dir`, panicking on failure, and returns trimmed stdout.
/// Sets a fixed author and committer so commits work without a global identity.
pub fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .output()
        .expect("run git");
    assert!(
        out.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Parses `bytes` (usually `Output::stdout`) as JSON; panics if they are not valid JSON.
#[must_use]
pub fn parse_json(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).expect("valid JSON on stdout")
}

/// `Output::stdout` as text (lossy UTF-8).
#[must_use]
pub fn stdout_text(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// `Output::stderr` as text (lossy UTF-8).
#[must_use]
pub fn stderr_text(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}
