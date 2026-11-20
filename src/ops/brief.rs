//! The work order of a task (`R-DETAIL-3`): what a worker reads before it starts, in one text.
//!
//! `todo brief`, `todo next --claim --brief` and `todo claim --brief` print what [`build_brief`]
//! returns, so the text is the same after a claim as before it. It reads the task and the entries it
//! names, and changes nothing (`R-LOCK-5`).

use serde::Serialize;

use crate::cli::help::update::powershell_quote;
use crate::doc::{Document, Status, Task};
use crate::ops::config::{OpsConfig, VerifyWhen};
use crate::ops::read::{MilestoneRefJson, milestone_chain_json};
use crate::ops::scope::raw_entries;

/// The work order of one task, as `todo brief --json` returns it (`todo.brief.v1`).
#[derive(Debug, Clone, Serialize)]
pub struct BriefJson {
    /// Canonical task ID.
    pub id: String,
    /// Task title.
    pub title: String,
    /// The milestones above the task, from the top down, with their goals (`R-MILE-4`).
    pub milestones: Vec<MilestoneRefJson>,
    /// `- why:` value: what the task is for; `null` when absent.
    pub why: Option<String>,
    /// The paths the worker should look at but not change, from `read` (`R-DETAIL-1`).
    pub read: Vec<String>,
    /// The paths, folders and patterns the task may change, from `files` (`R-SCOPE-1`); empty when no
    /// `files` are listed, which means the task may change any file.
    pub files: Vec<String>,
    /// The numbered steps, without their numbers (`R-DETAIL-1`); empty when none are written.
    pub steps: Vec<String>,
    /// The check and when todo runs it; `null` when the task has no `verify` line (`R-VERIFY-1`).
    pub verify: Option<VerifyJson>,
    /// The finished dependencies, in the order of `depends`, each with its done note and commits.
    pub finished_dependencies: Vec<FinishedDependencyJson>,
    /// The commands that finish the task, and the one that gives it back (`R-DETAIL-3`).
    pub commands: CommandsJson,
}

/// The check of a task and when todo runs it (`R-VERIFY-1`, `R-VERIFY-2`).
#[derive(Debug, Clone, Serialize)]
pub struct VerifyJson {
    /// The command line, as written.
    pub command: String,
    /// `done` when `done` runs it, `wave` when `todo verify --wave` runs it after the wave, and
    /// `never` when `verify = false` turns running off.
    pub runs: &'static str,
}

/// A dependency that is done: its title, the text of its latest done note and its commits.
#[derive(Debug, Clone, Serialize)]
pub struct FinishedDependencyJson {
    /// Canonical ID of the dependency.
    pub id: String,
    /// Its title.
    pub title: String,
    /// The text of its latest `done — <note>`; `null` when it was finished without a note.
    pub note: Option<String>,
    /// The commit hashes from its `- commit:` field.
    pub commit: Vec<String>,
}

/// The commands that end the work: finishing it, and giving it back.
#[derive(Debug, Clone, Serialize)]
pub struct CommandsJson {
    /// The agent name the commands carry; `null` when no agent is set, and a placeholder is printed.
    pub agent: Option<String>,
    /// The lines that commit only the listed files and then finish the task, in order.
    pub finish: Vec<String>,
    /// The command that gives the task back to the planner with the problem.
    pub give_up: String,
}

/// Builds the work order of `task` (`R-DETAIL-3`). `archive` answers for dependencies that were
/// archived; `agent` fills in the agent name of the commands.
#[must_use]
pub fn build_brief(
    doc: &Document,
    archive: Option<&Document>,
    task: &Task,
    cfg: &OpsConfig,
    agent: Option<&str>,
) -> BriefJson {
    let id = cfg.id_config.format_id(task.id);
    let files: Vec<String> = task
        .files
        .as_deref()
        .map(|value| raw_entries(value).map(str::to_string).collect())
        .unwrap_or_default();
    let commands = commands_of(&id, &task.title, &files, agent);
    BriefJson {
        id,
        title: task.title.clone(),
        milestones: milestone_chain_json(doc, task, cfg),
        why: task
            .why
            .as_deref()
            .map(str::trim)
            .filter(|why| !why.is_empty())
            .map(str::to_string),
        read: task.read_paths(),
        files,
        steps: task.steps.clone(),
        verify: verify_of(task, cfg),
        finished_dependencies: finished_dependencies(doc, archive, task, cfg),
        commands,
    }
}

/// The check of `task`, with when todo runs it (`R-VERIFY-2`); `None` without a `verify` line.
fn verify_of(task: &Task, cfg: &OpsConfig) -> Option<VerifyJson> {
    let command = task
        .verify
        .as_deref()
        .map(str::trim)
        .filter(|command| !command.is_empty())?;
    let runs = if !cfg.verify_enabled {
        "never"
    } else if cfg.verify_when == VerifyWhen::Wave {
        "wave"
    } else {
        "done"
    };
    Some(VerifyJson {
        command: command.to_string(),
        runs,
    })
}

/// The dependencies of `task` that are done, in the order of `depends` (`R-DETAIL-3`).
fn finished_dependencies(
    doc: &Document,
    archive: Option<&Document>,
    task: &Task,
    cfg: &OpsConfig,
) -> Vec<FinishedDependencyJson> {
    task.depends
        .iter()
        .filter_map(|dep| dep.id)
        .filter_map(|id| {
            doc.task_by_id(id)
                .or_else(|| archive.and_then(|a| a.task_by_id(id)))
        })
        .filter(|dep| dep.status == Status::Done)
        .map(|dep| FinishedDependencyJson {
            id: cfg.id_config.format_id(dep.id),
            title: dep.title.clone(),
            note: done_note(dep),
            commit: dep.commit.clone(),
        })
        .collect()
}

/// The text of the latest `done` note of `task` (`done — <note>`, `docs/format.md` §9); `None` when
/// the latest one has no text, or there is none.
fn done_note(task: &Task) -> Option<String> {
    task.notes
        .iter()
        .rev()
        .find_map(|note| {
            if note.text == "done" {
                Some(String::new())
            } else {
                note.text
                    .strip_prefix("done \u{2014} ")
                    .map(|text| text.trim().to_string())
            }
        })
        .filter(|text| !text.is_empty())
}

/// The commands that finish the task and give it back, with the agent name filled in (`R-DETAIL-3`).
fn commands_of(id: &str, title: &str, files: &[String], agent: Option<&str>) -> CommandsJson {
    let agent_word = agent.map_or_else(|| "<agent>".to_string(), powershell_word);
    let add = if files.is_empty() {
        "git add -- <the files you changed>".to_string()
    } else {
        let listed: Vec<String> = files.iter().map(|file| powershell_word(file)).collect();
        format!("git add -- {}", listed.join(" "))
    };
    let message = powershell_word(&format!("{id}: {title}"));
    CommandsJson {
        agent: agent.map(str::to_string),
        finish: vec![
            add,
            format!("git commit -m {message}"),
            format!("todo done {id} --commit HEAD --agent {agent_word}"),
        ],
        give_up: format!("todo stuck {id} --note \"<problem>\" --agent {agent_word}"),
    }
}

/// One word as PowerShell reads it (`AGENTS.md`: examples are in PowerShell). Plain when it holds only
/// letters, digits and `- _ . / \ : = +`, as the work order of R-DETAIL-3 prints paths; otherwise it
/// is quoted by [`powershell_quote`], the quoting `todo help update` uses.
fn powershell_word(text: &str) -> String {
    let plain = !text.is_empty()
        && text.chars().all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | '\\' | ':' | '=' | '+')
        });
    if plain {
        text.to_string()
    } else {
        powershell_quote(text)
    }
}

#[cfg(test)]
mod tests {
    use super::powershell_word;

    #[test]
    fn plain_words_stay_as_they_are() {
        assert_eq!(powershell_word("T-021"), "T-021");
        assert_eq!(powershell_word("src/shader.c"), "src/shader.c");
        assert_eq!(powershell_word("worker-1"), "worker-1");
    }

    #[test]
    fn words_with_spaces_or_quotes_are_single_quoted_and_doubled() {
        assert_eq!(
            powershell_word("T-021: diffuse term"),
            "'T-021: diffuse term'"
        );
        assert_eq!(powershell_word("Don't stop"), "'Don''t stop'");
        assert_eq!(powershell_word("a\u{2019}b"), "'a\u{2019}\u{2019}b'");
        assert_eq!(powershell_word("$x `y"), "'$x `y'");
    }
}
