//! `todo align` command: compare tasks with git history (`R-GIT-1`).

use std::collections::{BTreeMap, BTreeSet};
use std::io::{ErrorKind, Write as _};
use std::path::Path;
use std::process::{Command as ProcessCommand, Stdio};

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::cli::{CommandKind, CommandMeta, Context};
use crate::doc::id::strip_prefix_ignore_case;
use crate::doc::{IdConfig, Status};
use crate::error::{ErrorCode, ExitCode, Result, TodoError};
use crate::ops::load_for_read;
use crate::output::Output;

/// Metadata for the `align` command.
pub const META: CommandMeta = CommandMeta {
    name: "align",
    summary: "Compare tasks with git history and report mismatches",
    kind: CommandKind::Read,
    details: "Reads the git history of the repository that holds TODO.md and reports: COMMITTED_NOT_DONE (an open, claimed or blocked task whose ID starts a commit subject), DONE_WITHOUT_COMMIT (a done task with no `commit` field and no commit subject starting with its ID) and COMMIT_MISSING (a done task whose `commit` value is not in the repository). Exits 0 with no findings, 1 with findings, and 5 with error code NO_GIT_HISTORY when git is missing, the folder is not a repository, the clone is shallow or there are no commits. Never changes TODO.md.",
    examples: &["todo align", "todo align --json"],
};

/// Arguments for `todo align`.
#[derive(Debug, ClapArgs)]
pub struct Args {}

/// One mismatch between a task and git history (`R-GIT-1`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    /// Canonical task ID.
    pub task: String,
    /// 1-based line of the task heading in `TODO.md`.
    pub line: usize,
    /// `COMMITTED_NOT_DONE`, `DONE_WITHOUT_COMMIT` or `COMMIT_MISSING`.
    pub kind: &'static str,
    /// Related commits: matching commits, or the missing `commit` values; empty for
    /// `DONE_WITHOUT_COMMIT`.
    pub commits: Vec<String>,
}

#[derive(Serialize)]
struct AlignJson {
    clean: bool,
    count: usize,
    findings: Vec<Finding>,
}

fn no_history(message: impl Into<String>) -> TodoError {
    TodoError::new(ErrorCode::NoGitHistory, message).with_exit_code(ExitCode::CheckError)
}

/// Runs `todo align`.
pub fn run(_args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let cfg = ctx.load_config()?;
    let loaded = load_for_read(ctx, &cfg, false, false, out)?;
    let dir = loaded
        .found
        .path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    let subjects = read_subjects(dir)?;
    let by_task = commits_by_task(&subjects, &loaded.doc.id_config);

    let mut to_verify: Vec<&str> = Vec::new();
    for task in &loaded.doc.tasks {
        if task.status == Status::Done {
            to_verify.extend(
                task.commit
                    .iter()
                    .map(String::as_str)
                    .filter(|c| !c.is_empty()),
            );
        }
    }
    let missing = missing_commits(dir, &to_verify)?;

    let mut findings: Vec<Finding> = Vec::new();
    for task in &loaded.doc.tasks {
        // Questions are answered, not committed (R-GIT-1).
        if task
            .task_type
            .as_deref()
            .is_some_and(|ty| ty.eq_ignore_ascii_case("question"))
        {
            continue;
        }
        let task_id = loaded.doc.id_config.format_id(task.id);
        let line = task.heading_line + 1;
        let matched = by_task.get(&task.id.number());
        match task.status {
            Status::Open | Status::Claimed | Status::Blocked => {
                if let Some(commits) = matched {
                    findings.push(Finding {
                        task: task_id,
                        line,
                        kind: "COMMITTED_NOT_DONE",
                        commits: commits.clone(),
                    });
                }
            }
            Status::Done => {
                let gone: Vec<String> = task
                    .commit
                    .iter()
                    .filter(|c| missing.contains(c.as_str()))
                    .cloned()
                    .collect();
                if !gone.is_empty() {
                    findings.push(Finding {
                        task: task_id,
                        line,
                        kind: "COMMIT_MISSING",
                        commits: gone,
                    });
                } else if task.commit.is_empty() && matched.is_none() {
                    findings.push(Finding {
                        task: task_id,
                        line,
                        kind: "DONE_WITHOUT_COMMIT",
                        commits: Vec::new(),
                    });
                }
            }
            Status::Dropped => {}
        }
    }

    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &AlignJson {
                clean: findings.is_empty(),
                count: findings.len(),
                findings: findings.clone(),
            },
        );
    } else if findings.is_empty() {
        out.write_line(&format!("{}: aligned with git history", loaded.file_label));
    } else {
        for f in &findings {
            let mut text = format!("{}:{}: {} {}", loaded.file_label, f.line, f.task, f.kind);
            if !f.commits.is_empty() {
                text.push_str(&format!(" ({})", f.commits.join(", ")));
            }
            out.write_line(&text);
        }
    }
    if !findings.is_empty() {
        out.set_exit(ExitCode::Refused);
    }
    Ok(())
}

fn git_command(dir: &Path) -> ProcessCommand {
    let mut cmd = ProcessCommand::new("git");
    cmd.current_dir(dir);
    cmd
}

fn spawn_error(e: &std::io::Error) -> TodoError {
    if e.kind() == ErrorKind::NotFound {
        no_history("git is not installed or not on PATH")
    } else {
        no_history(format!("failed to run git: {e}"))
    }
}

/// One `git log` call; returns `(short hash, subject)` for every commit reachable from `HEAD`.
fn read_subjects(dir: &Path) -> Result<Vec<(String, String)>> {
    let shallow = git_command(dir)
        .args(["rev-parse", "--is-shallow-repository"])
        .output()
        .map_err(|e| spawn_error(&e))?;
    if !shallow.status.success() {
        return Err(no_history("not inside a git repository"));
    }
    if String::from_utf8_lossy(&shallow.stdout).trim() == "true" {
        return Err(no_history(
            "the repository is a shallow clone, so its history is incomplete",
        ));
    }
    let log = git_command(dir)
        .args(["log", "--format=%h%x09%s"])
        .output()
        .map_err(|e| spawn_error(&e))?;
    if !log.status.success() {
        return Err(no_history("the repository has no commits"));
    }
    let text = String::from_utf8_lossy(&log.stdout);
    Ok(text
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .map(|(h, s)| (h.to_string(), s.to_string()))
        .collect())
}

/// Maps task numbers to the commits whose subject starts with the task's ID (`T-012:`, `T-012 `).
fn commits_by_task(subjects: &[(String, String)], ids: &IdConfig) -> BTreeMap<u64, Vec<String>> {
    let mut map: BTreeMap<u64, Vec<String>> = BTreeMap::new();
    for (hash, subject) in subjects {
        let Some(rest) = strip_prefix_ignore_case(subject.trim_start(), &ids.prefix)
            .and_then(|r| r.strip_prefix('-'))
        else {
            continue;
        };
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if digits.is_empty() {
            continue;
        }
        if rest[digits.len()..]
            .chars()
            .next()
            .is_some_and(char::is_alphanumeric)
        {
            continue;
        }
        if let Ok(n) = digits.parse::<u64>() {
            map.entry(n).or_default().push(hash.clone());
        }
    }
    map
}

/// Checks all `commit` values with one `git cat-file --batch-check`; returns those that are absent.
fn missing_commits(dir: &Path, values: &[&str]) -> Result<BTreeSet<String>> {
    let mut missing = BTreeSet::new();
    if values.is_empty() {
        return Ok(missing);
    }
    let mut child = git_command(dir)
        .args(["cat-file", "--batch-check"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| spawn_error(&e))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| no_history("could not talk to git"))?;
    let input: String = values.iter().map(|v| format!("{v}^{{commit}}\n")).collect();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(input.as_bytes());
    });
    let output = child
        .wait_with_output()
        .map_err(|e| no_history(format!("failed to run git: {e}")))?;
    let _ = writer.join();
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        if let Some(spec) = line.strip_suffix(" missing")
            && let Some(value) = spec.strip_suffix("^{commit}")
        {
            missing.insert(value.to_string());
        }
    }
    Ok(missing)
}
