//! Running a task's `verify` command (T-158; R-VERIFY-1, R-VERIFY-2, NFR-5).
//!
//! todo runs the command through `verify_shell`, with the `verify` line as the last argument and
//! standard input closed, in the folder given by the caller. Standard output and standard error go
//! to one log file in the state folder (`verify/<ID>.log`), so they never reach todo's own output.
//! The command runs outside the write lock (R-LOCK-4): callers run it before they take the lock.

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::json;

use crate::doc::{Status, Task};
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::OpsConfig;
use crate::process_tree::ProcessTree;

/// How many lines of output a failure keeps for the text report.
pub const TAIL_LINES: usize = 10;

/// The start of the note a failed check adds (`docs/format.md` §9).
pub const FAILED_CHECK_NOTE: &str = "verify failed";

/// A check that ran and failed, or was stopped after `verify_timeout_seconds` (R-VERIFY-1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// The exit code of the shell; `None` when it was stopped (timeout) or ended by a signal.
    pub exit_code: Option<i32>,
    /// The last non-empty line of the output, or `timed out after <N> seconds` on a timeout.
    pub last_line: String,
    /// The last lines of the output, for the text report (at most [`TAIL_LINES`]).
    pub tail: Vec<String>,
    /// Whether the check was stopped after `verify_timeout_seconds`.
    pub timed_out: bool,
    /// The file that holds the full output of this run.
    pub log: PathBuf,
}

/// What running a `verify` command came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckOutcome {
    /// The command exited with 0.
    Passed,
    /// The command exited with another code, or was stopped.
    Failed(Failure),
    /// No shell could start the command, so no check happened (`VERIFY_NOT_RUN`).
    NotRun {
        /// Plain-English reason, for the error message.
        reason: String,
    },
}

/// Whether `task` is a done task whose `verify` command has not passed yet (`R-VERIFY-2`): it has a
/// `verify` command and no `verified` line. `verify --wave` runs these, and `check` and `plan` list
/// them; the one rule is here so that they cannot disagree.
#[must_use]
pub fn awaits_verified(task: &Task) -> bool {
    task.status == Status::Done && task.verify.is_some() && task.field_value("verified").is_none()
}

/// The log file of one task's check: `<state folder>/verify/<ID>.log` (`R-VERIFY-1`).
#[must_use]
pub fn log_path(state_dir: &Path, id: &str) -> PathBuf {
    state_dir.join("verify").join(format!("{id}.log"))
}

/// Runs `command` through the configured shell in `work_dir`, saves its output in `log` and
/// reports the outcome (R-VERIFY-1). The log is replaced by each run.
///
/// On a timeout, and when the shell has ended, every process the check started is stopped too
/// (T-172): a check leaves nothing running behind it.
pub fn run_check(
    cfg: &OpsConfig,
    command: &str,
    work_dir: &Path,
    log: &Path,
) -> Result<CheckOutcome> {
    if !work_dir.is_dir() {
        return Ok(CheckOutcome::NotRun {
            reason: format!("the folder {} does not exist", work_dir.display()),
        });
    }
    if let Some(parent) = log.parent() {
        fs::create_dir_all(parent).map_err(|e| log_error(log, &e))?;
    }
    let file = File::create(log).map_err(|e| log_error(log, &e))?;
    let second = file.try_clone().map_err(|e| log_error(log, &e))?;

    let (mut child, tree) = match spawn_shell(cfg, command, work_dir, file, second) {
        Ok(started) => started,
        Err(reason) => return Ok(CheckOutcome::NotRun { reason }),
    };
    let timeout = Duration::from_secs(cfg.verify_timeout_seconds);
    let exited = match wait_for_exit(&mut child, timeout) {
        Ok(exited) => exited,
        Err(e) => {
            tree.stop_all();
            let _ = child.kill();
            let _ = child.wait();
            return Err(log_error(log, &e));
        }
    };
    // Stop what the check started, on a timeout and after a normal end (T-172). On a timeout the
    // shell has not been reaped yet, so the group still exists; after a normal end the group's
    // leader is reaped, and its id is only reused by a new group leader within microseconds.
    tree.stop_all();
    let status = match exited {
        Some(status) => Some(status),
        None => {
            let _ = child.kill();
            let _ = child.wait();
            None
        }
    };

    let text = fs::read(log)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .map_err(|e| log_error(log, &e))?;
    let tail = tail_lines(&text, TAIL_LINES);
    let outcome = match status {
        Some(status) if status.success() => CheckOutcome::Passed,
        Some(status) => CheckOutcome::Failed(Failure {
            exit_code: status.code(),
            last_line: last_line(&text).unwrap_or_else(|| no_output_text(status)),
            tail,
            timed_out: false,
            log: log.to_path_buf(),
        }),
        None => CheckOutcome::Failed(Failure {
            exit_code: None,
            last_line: format!("timed out after {} seconds", cfg.verify_timeout_seconds),
            tail,
            timed_out: true,
            log: log.to_path_buf(),
        }),
    };
    Ok(outcome)
}

/// Starts the shell with the command as its last argument, in the tree that stops everything it
/// starts (T-172). `verify_shell` is used when it is set; otherwise PowerShell 7 (`pwsh`), then
/// Windows PowerShell (`powershell.exe`) on Windows, and `sh -c` elsewhere (R-CFG-2, R-VERIFY-1).
/// `Err` carries the reason when no shell could start.
fn spawn_shell(
    cfg: &OpsConfig,
    command: &str,
    work_dir: &Path,
    stdout: File,
    stderr: File,
) -> std::result::Result<(Child, ProcessTree), String> {
    let shells: Vec<(String, Vec<String>)> = match cfg.verify_shell.split_first() {
        Some((program, args)) => vec![(program.clone(), args.to_vec())],
        None if cfg!(windows) => {
            let args = ["-NoProfile", "-NonInteractive", "-Command"];
            ["pwsh", "powershell.exe"]
                .into_iter()
                .map(|program| {
                    (
                        program.to_string(),
                        args.iter().map(ToString::to_string).collect(),
                    )
                })
                .collect()
        }
        None => vec![("sh".to_string(), vec!["-c".to_string()])],
    };

    let mut tried = Vec::new();
    for (program, args) in &shells {
        let out = stdout
            .try_clone()
            .map_err(|e| format!("could not prepare the output of {program}: {e}"))?;
        let err = stderr
            .try_clone()
            .map_err(|e| format!("could not prepare the output of {program}: {e}"))?;
        let mut cmd = Command::new(program);
        cmd.args(args)
            .arg(command)
            .current_dir(work_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::from(out))
            .stderr(Stdio::from(err));
        ProcessTree::prepare(&mut cmd);
        match cmd.spawn() {
            Ok(mut child) => match ProcessTree::attach(&child) {
                Ok(tree) => return Ok((child, tree)),
                Err(e) => {
                    // Without its tree the check cannot be stopped as a whole, so it does not run.
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "could not set up the process tree of {program}: {e}"
                    ));
                }
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => tried.push(program.clone()),
            Err(e) => return Err(format!("could not start {program}: {e}")),
        }
    }
    Err(format!("no shell could start (tried {})", tried.join(", ")))
}

/// Waits for `child` for at most `timeout`, and returns its status, or `None` when the time is up.
/// The child is not stopped here: the caller stops the tree (T-172).
fn wait_for_exit(child: &mut Child, timeout: Duration) -> io::Result<Option<ExitStatus>> {
    // A timeout too long to add to the clock means no timeout at all.
    let deadline = Instant::now().checked_add(timeout);
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        if let Some(deadline) = deadline
            && Instant::now() >= deadline
        {
            return Ok(None);
        }
        thread::sleep(Duration::from_millis(20));
    }
}

/// The last non-empty line of `text`, without its line break.
#[must_use]
pub fn last_line(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .map(str::to_string)
}

/// The last `count` non-empty lines of `text`, in order.
#[must_use]
pub fn tail_lines(text: &str, count: usize) -> Vec<String> {
    let lines: Vec<String> = text
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .map(str::to_string)
        .collect();
    let start = lines.len().saturating_sub(count);
    lines.into_iter().skip(start).collect()
}

/// How a failed check ended, for a message: its exit code, a timeout or a signal.
#[must_use]
pub fn exit_text(failure: &Failure) -> String {
    match (failure.exit_code, failure.timed_out) {
        (_, true) => "timed out".to_string(),
        (Some(code), false) => format!("exit code {code}"),
        (None, false) => "stopped by a signal".to_string(),
    }
}

fn no_output_text(status: ExitStatus) -> String {
    match status.code() {
        Some(code) => format!("no output (exit code {code})"),
        None => "no output (stopped by a signal)".to_string(),
    }
}

fn log_error(path: &Path, err: &io::Error) -> TodoError {
    TodoError::new(
        ErrorCode::WriteFailed,
        format!(
            "could not write the verify output {}: {err}",
            path.display()
        ),
    )
}

/// The error a failed check gives (`VERIFY_FAILED`, exit 1). Its details carry the task, the exit
/// code, the last line and the path of the log (R-VERIFY-1).
#[must_use]
pub fn failure_error(task_id: &str, failure: &Failure, message: impl Into<String>) -> TodoError {
    TodoError::new(ErrorCode::VerifyFailed, message)
        .with_task(task_id)
        .with_detail("exit_code", json!(failure.exit_code))
        .with_detail("last_line", json!(failure.last_line))
        .with_detail("timed_out", json!(failure.timed_out))
        .with_detail("log", json!(failure.log.display().to_string()))
}

/// The error for a check that could not run (`VERIFY_NOT_RUN`, exit 5); nothing was written.
#[must_use]
pub fn not_run_error(task_id: &str, reason: &str) -> TodoError {
    TodoError::new(
        ErrorCode::VerifyNotRun,
        format!("the verify command of {task_id} could not run: {reason}"),
    )
    .with_task(task_id)
}

/// Whether a note text starts a new count of failed checks (`R-VERIFY-1`): the latest
/// `unblocked`, `reopened` or `stuck` note restarts it.
fn restarts_failed_checks(text: &str) -> bool {
    ["unblocked", "reopened", "stuck"]
        .into_iter()
        .any(|start| text.starts_with(start))
}

/// How many failed checks the task has had since it last went back to work (`R-VERIFY-1`): the
/// `verify failed` notes after its latest `unblocked`, `reopened` or `stuck` note, or all of them.
/// `attempts` is a total and is not used here.
#[must_use]
pub fn failed_checks_since_restart(task: &Task) -> u64 {
    let start = task
        .notes
        .iter()
        .rposition(|note| restarts_failed_checks(&note.text))
        .map_or(0, |index| index + 1);
    let count = task
        .notes
        .iter()
        .skip(start)
        .filter(|note| note.text.starts_with(FAILED_CHECK_NOTE))
        .count();
    u64::try_from(count).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_line_skips_blank_lines_and_trailing_breaks() {
        assert_eq!(last_line("one\r\ntwo\r\n\r\n"), Some("two".to_string()));
        assert_eq!(last_line("  \n\n"), None);
        assert_eq!(last_line(""), None);
    }

    #[test]
    fn tail_keeps_the_last_non_empty_lines_in_order() {
        let text = "a\n\nb\nc\nd\n";
        assert_eq!(tail_lines(text, 2), vec!["c", "d"]);
        assert_eq!(tail_lines(text, 10), vec!["a", "b", "c", "d"]);
        assert!(tail_lines("", 3).is_empty());
    }

    #[test]
    fn log_lives_in_a_verify_folder_of_the_state_folder() {
        let log = log_path(Path::new("state"), "T-012");
        assert_eq!(log, Path::new("state").join("verify").join("T-012.log"));
    }

    #[test]
    fn restart_words_are_the_latest_unblocked_reopened_or_stuck_note() {
        assert!(restarts_failed_checks("unblocked"));
        assert!(restarts_failed_checks("reopened — again"));
        assert!(restarts_failed_checks("stuck — the schema is missing"));
        assert!(!restarts_failed_checks("verify failed — x"));
        assert!(!restarts_failed_checks("blocked — waiting"));
    }

    #[test]
    fn failed_checks_are_counted_after_the_latest_restart_note() {
        let text = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
### T-001 · Title\n\
- status: claimed\n\
- owner: alice\n\
- priority: P1 · difficulty: 2\n\
- note (2026-11-15, alice): verify failed — a\n\
- note (2026-11-15, alice): stuck — x\n\
- note (2026-11-15, alice): unblocked — y\n\
- note (2026-11-15, alice): verify failed — b\n\
- note (2026-11-15, alice): verify failed — c\n";
        let doc = crate::doc::Document::parse_bytes_with_config(
            text.as_bytes(),
            crate::doc::IdConfig::default(),
        )
        .unwrap();
        let task = &doc.tasks[0];
        assert_eq!(failed_checks_since_restart(task), 2);

        let none = "### T-002 · Title\n- status: open\n- priority: P1 · difficulty: 2\n";
        let doc = crate::doc::Document::parse_bytes_with_config(
            none.as_bytes(),
            crate::doc::IdConfig::default(),
        )
        .unwrap();
        assert_eq!(failed_checks_since_restart(&doc.tasks[0]), 0);
    }
}
