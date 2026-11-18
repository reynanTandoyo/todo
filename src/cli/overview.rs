//! `todo overview` command: one line per registered project with its tasks, top-level milestones,
//! stale claims and open questions (T-093; R-PROJ-2).

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::cli::{CommandKind, CommandMeta, Context};
use crate::error::Result;
use crate::lock::current_time;
use crate::output::Output;
use crate::register::overview::{ProjectOverview, overview_of};
use crate::register::{read_register, register_path};

/// The name in the JSON schema of this command's output, `todo.projects.v1`. `todo.overview.v1` is
/// taken by `todo --json`, which lists the commands (`docs/json.md`).
pub const JSON_NAME: &str = "projects";

/// Metadata for the `overview` command.
pub const META: CommandMeta = CommandMeta {
    name: "overview",
    summary: "Show every registered project on one line: tasks, milestones, stale claims, open questions",
    kind: CommandKind::Meta,
    details: "Reads only the project register (see `todo project`), never searches folders, and works from any folder. For each registered project it reads that project's TODO.md and prints one line: name, path, tasks per status, top-level milestones done of total, stale claims, open questions and the todo version stamped in the file. A project that needs a migration is marked, as `todo migrate --yes` brings it up to date. A project whose file is gone, cannot be read or needs a newer todo is listed with its problem, and the others are still shown. It writes nothing.",
    examples: &["todo overview", "todo overview --json"],
};

/// Arguments for `todo overview`.
#[derive(Debug, ClapArgs)]
pub struct Args {}

#[derive(Serialize)]
struct OverviewJson<'a> {
    projects: &'a [ProjectOverview],
    register: String,
}

/// Runs `todo overview`.
pub fn run(_args: Args, _ctx: &Context, out: &mut Output) -> Result<()> {
    let register = register_path()?;
    let projects = read_register(&register)?
        .map(|reg| reg.projects)
        .unwrap_or_default();
    let now = current_time();
    let overviews: Vec<ProjectOverview> = projects
        .iter()
        .map(|project| overview_of(project, now))
        .collect();

    if out.is_json() {
        out.emit_json_ok(
            JSON_NAME,
            &OverviewJson {
                projects: &overviews,
                register: register.display().to_string(),
            },
        );
        return Ok(());
    }
    if overviews.is_empty() {
        out.write_line(&format!(
            "No projects are registered in {}. Run todo project add in a project folder to register it.",
            register.display()
        ));
        return Ok(());
    }
    for overview in &overviews {
        out.write_line(&overview_line(overview));
    }
    Ok(())
}

/// The text line of one project: its problem, or its figures.
fn overview_line(overview: &ProjectOverview) -> String {
    let (Some(tasks), Some(milestones), Some(stale), Some(questions)) = (
        overview.tasks,
        overview.milestones,
        overview.stale_claims,
        overview.open_questions,
    ) else {
        let message = overview
            .problem
            .as_ref()
            .map_or("the project cannot be read", |problem| {
                problem.message.as_str()
            });
        return format!("{} | {} | problem: {message}", overview.name, overview.path);
    };
    let stamp = overview.tool_stamp.as_deref().map_or_else(
        || "no todo version in the file".to_owned(),
        |version| format!("todo {version}"),
    );
    let migrate = if overview.needs_migration == Some(true) {
        " (needs todo migrate --yes)"
    } else {
        ""
    };
    format!(
        "{} | {} | open {}, claimed {}, blocked {}, done {}, dropped {} | milestones done {} of {} | stale claims {} | open questions {} | {stamp}{migrate}",
        overview.name,
        overview.path,
        tasks.open,
        tasks.claimed,
        tasks.blocked,
        tasks.done,
        tasks.dropped,
        milestones.done,
        milestones.total,
        stale,
        questions,
    )
}
