//! `todo project` command: add, remove and list the projects in the per-user project register
//! (T-093; R-PROJ-1).

use std::path::{Path, PathBuf};

use clap::{Args as ClapArgs, Subcommand};
use serde::Serialize;

use crate::cli::{CommandKind, CommandMeta, Context, emit_find_notices};
use crate::error::{ErrorCode, Result, TodoError};
use crate::find::{
    FindOptions, FoundBy, FoundTodo, TODO_FILENAME, WorktreeMode, find_local_worktree_toml,
    find_todo, fs_path,
};
use crate::lock::current_time;
use crate::ops::OpsConfig;
use crate::output::Output;
use crate::register::{
    Project, default_name, locked_update, not_found, path_text, read_register, register_path,
    validate_name,
};
use crate::state::canonicalize_file_or_parent;

/// Metadata for the `project` command.
pub const META: CommandMeta = CommandMeta {
    name: "project",
    summary: "Add, remove or list the projects in the per-user project register",
    kind: CommandKind::Meta,
    details: "Registers projects, so that `todo overview` can show them together without searching the PC. `project add [<path>] [--name N]` registers the TODO.md found from that folder (default: the current folder), searched for as the other commands search (`--file` and `TODO_FILE` do not apply to `add`); the name defaults to the name of the folder that holds the file, and the same file or the same name twice is refused. `project remove <name|path>` removes a project by its name, or else by the path of its TODO.md or folder. `project list` lists the registered projects. The register is one JSON file: %LOCALAPPDATA%\\todo\\projects.json on Windows, $XDG_DATA_HOME/todo/projects.json (or ~/.local/share/todo/projects.json) on Linux; TODO_REGISTER names another file. No agent name is needed and no TODO.md is changed.",
    examples: &[
        "todo project add",
        "todo project add \"C:\\Users\\Example\\Projekte\\Grafik Engine\" --name renderer",
        "todo project list",
        "todo project list --json",
        "todo project remove renderer",
    ],
};

/// Arguments for `todo project`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Subcommand (`add`, `remove` or `list`).
    #[command(subcommand)]
    pub action: ProjectAction,
}

/// Subcommands under `todo project`.
#[derive(Debug, Subcommand)]
pub enum ProjectAction {
    /// Register the TODO.md found from a folder (default: the current folder).
    Add {
        /// Folder to search from, as other commands search for TODO.md (default: the current folder).
        path: Option<PathBuf>,
        /// Name of the project (default: the name of the folder that holds TODO.md).
        #[arg(long, value_name = "NAME")]
        name: Option<String>,
    },
    /// Remove a project by its name, or by the path of its TODO.md or folder.
    Remove {
        /// The project's name, or the path of its TODO.md or of its folder.
        project: String,
    },
    /// List the registered projects.
    List,
}

#[derive(Serialize)]
struct AddJson<'a> {
    project: &'a Project,
    register: String,
}

#[derive(Serialize)]
struct RemoveJson<'a> {
    removed: &'a Project,
    register: String,
}

#[derive(Serialize)]
struct ListJson<'a> {
    projects: &'a [Project],
    register: String,
}

/// Runs `todo project`.
pub fn run(args: Args, _ctx: &Context, out: &mut Output) -> Result<()> {
    let register = register_path()?;
    match args.action {
        ProjectAction::Add { path, name } => add(
            path.as_deref().unwrap_or(Path::new(".")),
            name.as_deref(),
            &register,
            out,
        ),
        ProjectAction::Remove { project } => remove(&project, &register, out),
        ProjectAction::List => list(&register, out),
    }
}

fn add(folder: &Path, name: Option<&str>, register: &Path, out: &mut Output) -> Result<()> {
    let found = locate_todo(folder, out)?;
    let todo_path = canonicalize_file_or_parent(&found.path)?;
    let name = match name {
        Some(raw) => validate_name(raw)?,
        None => default_name(&todo_path)?,
    };
    let project = Project {
        name,
        path: path_text(&todo_path)?,
        added: OpsConfig::format_claimed_timestamp(current_time()),
    };

    let saved = project.clone();
    locked_update(register, |reg| reg.add(project))?;

    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &AddJson {
                project: &saved,
                register: register.display().to_string(),
            },
        );
    } else {
        out.write_line(&format!("registered {}: {}", saved.name, saved.path));
    }
    Ok(())
}

fn remove(selector: &str, register: &Path, out: &mut Output) -> Result<()> {
    // Nothing is registered before the register exists: refuse without creating its folder or lock.
    if !std::fs::metadata(fs_path(register).as_ref()).is_ok_and(|m| m.is_file()) {
        return Err(not_found(selector));
    }
    let path = selector_path(selector);
    let removed = locked_update(register, |reg| reg.remove(selector, path.as_deref()))?;

    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &RemoveJson {
                removed: &removed,
                register: register.display().to_string(),
            },
        );
    } else {
        out.write_line(&format!("removed {}: {}", removed.name, removed.path));
    }
    Ok(())
}

fn list(register: &Path, out: &mut Output) -> Result<()> {
    let projects = read_register(register)?
        .map(|reg| reg.projects)
        .unwrap_or_default();

    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &ListJson {
                projects: &projects,
                register: register.display().to_string(),
            },
        );
        return Ok(());
    }
    if projects.is_empty() {
        out.write_line(&format!(
            "No projects are registered in {}. Run todo project add in a project folder to register it.",
            register.display()
        ));
        return Ok(());
    }
    for project in &projects {
        out.write_line(&format!(
            "{} | {} | added {}",
            project.name, project.path, project.added
        ));
    }
    Ok(())
}

/// Finds the TODO.md of `folder` as the other commands find theirs (R-FIND-3 to R-FIND-5), and
/// honours `worktree = "local"` (R-FIND-5). `--file` and `TODO_FILE` do not apply: a project is the
/// file of the folder it is registered from.
fn locate_todo(folder: &Path, out: &mut Output) -> Result<FoundTodo> {
    if !std::fs::metadata(fs_path(folder).as_ref()).is_ok_and(|m| m.is_dir()) {
        return Err(TodoError::new(
            ErrorCode::NoFile,
            format!("{} is not a folder", folder.display()),
        ));
    }

    let mut worktree = WorktreeMode::Shared;
    if let Some(local_toml) = find_local_worktree_toml(Some(folder))
        && OpsConfig::load_from_file(&local_toml)?.worktree == WorktreeMode::Local
    {
        worktree = WorktreeMode::Local;
    }
    let mut found = find_from(folder, worktree)?;
    if worktree == WorktreeMode::Shared
        && found.found_by == FoundBy::WorktreeMain
        && OpsConfig::load_next_to(&found.path)?.worktree == WorktreeMode::Local
    {
        found = find_from(folder, WorktreeMode::Local)?;
    }
    emit_find_notices(&found, out);
    Ok(found)
}

fn find_from(folder: &Path, worktree: WorktreeMode) -> Result<FoundTodo> {
    find_todo(&FindOptions {
        explicit_file: None,
        explicit_source: None,
        start_dir: Some(folder),
        worktree,
    })
}

/// The canonical path of the TODO.md that a `remove` argument names, when it names a file or a
/// folder. A folder names its TODO.md. Names that are not paths give `None`, or a path that matches
/// nothing, and are matched by name instead.
fn selector_path(selector: &str) -> Option<String> {
    let given = Path::new(selector);
    let is_folder = std::fs::metadata(fs_path(given).as_ref()).is_ok_and(|m| m.is_dir());
    let todo = if is_folder {
        given.join(TODO_FILENAME)
    } else {
        given.to_path_buf()
    };
    let canonical = canonicalize_file_or_parent(&todo).ok()?;
    canonical.to_str().map(str::to_owned)
}
