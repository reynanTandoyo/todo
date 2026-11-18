//! `todo schema [command...]`: a machine-readable description of this build (T-073; ideas I-4).
//!
//! Commands, arguments and options come from the clap definitions at run time, so they cannot
//! drift from what the program accepts. Error and exit codes come from [`ErrorCode::ALL`] and
//! [`ExitCode::ALL`], and the task fields come from the same [`TaskJson`] that `show --json` uses.

use std::collections::{BTreeMap, BTreeSet};

use clap::{Arg, ArgAction, Args as ClapArgs, Command as ClapCommand, CommandFactory};
use serde::Serialize;
use serde_json::{Value, json};

use crate::cli::{Cli, CommandKind, CommandMeta, Context};
use crate::error::{ErrorCode, ExitCode, Result, TodoError};
use crate::ops::TaskJson;
use crate::output::{Output, schema_name};

/// Metadata for the `schema` command.
pub const META: CommandMeta = CommandMeta {
    name: "schema",
    summary: "Describe commands, options, JSON shapes and exit codes as JSON",
    kind: CommandKind::Meta,
    details: "Prints a JSON description of this build: its version, the JSON envelope, every exit code and error code with its meaning, the task fields that `show --json` returns, and every command with its arguments and options. Each command names the schema its `--json` output uses, `todo.<command>.v1` (subcommands use the schema of their parent, such as `todo.hooks.v1`). Give a command name to describe only that command; for a subcommand, give each word, such as `hooks install`. Without `--json` the description is pretty-printed; with `--json` it is the `data` of a `todo.schema.v1` envelope.",
    examples: &[
        "todo schema",
        "todo schema show",
        "todo schema hooks install",
        "todo schema --json",
    ],
};

/// Arguments for `todo schema`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Command to describe, one word per argument (e.g. `show`, or `hooks install`). Omit for all.
    #[arg(value_name = "COMMAND", num_args = 0..)]
    pub command: Vec<String>,
}

#[derive(Serialize)]
struct SchemaJson {
    tool: &'static str,
    version: &'static str,
    commit: &'static str,
    envelope: Value,
    exit_codes: Vec<ExitCodeJson>,
    error_codes: Vec<ErrorCodeJson>,
    task: TaskFieldsJson,
    global_options: Vec<ArgumentJson>,
    commands: Vec<CommandJson>,
}

#[derive(Serialize)]
struct ExitCodeJson {
    code: i32,
    meaning: &'static str,
}

#[derive(Serialize)]
struct ErrorCodeJson {
    code: &'static str,
    exit_code: i32,
    meaning: &'static str,
}

#[derive(Serialize)]
struct TaskFieldsJson {
    fields: Vec<String>,
    example: TaskJson,
}

#[derive(Serialize)]
struct CommandJson {
    name: String,
    schema: String,
    summary: String,
    subcommands: Vec<String>,
    arguments: Vec<ArgumentJson>,
    options: Vec<ArgumentJson>,
}

#[derive(Serialize)]
struct ArgumentJson {
    /// Positional name (the argument's ID), or the option as typed (`--agent`, `-x`).
    name: String,
    /// Placeholder shown for the value, or `null` for a flag without a value.
    value_name: Option<String>,
    required: bool,
    /// `true` when the option may be given more than once (or a positional takes many values).
    repeatable: bool,
    help: String,
}

/// Runs `todo schema [command...]`.
pub fn run(args: Args, _ctx: &Context, out: &mut Output) -> Result<()> {
    let root = Cli::command();
    let globals = global_ids(&root);
    if args.command.is_empty() {
        let description = SchemaJson {
            tool: "todo",
            version: env!("CARGO_PKG_VERSION"),
            commit: env!("TODO_GIT_COMMIT"),
            envelope: envelope_shape(),
            exit_codes: ExitCode::ALL
                .iter()
                .map(|code| ExitCodeJson {
                    code: code.as_i32(),
                    meaning: code.meaning(),
                })
                .collect(),
            error_codes: ErrorCode::ALL
                .iter()
                .map(|code| ErrorCodeJson {
                    code: code.as_str(),
                    exit_code: code.default_exit_code().as_i32(),
                    meaning: code.meaning(),
                })
                .collect(),
            task: task_fields(),
            global_options: root
                .get_arguments()
                .filter(|arg| arg.is_global_set() && !is_builtin(arg) && !arg.is_hide_set())
                .map(describe_arg)
                .collect(),
            commands: all_commands(&root, &globals),
        };
        return emit(out, &description);
    }

    let cmd = find_command(&root, &args.command).ok_or_else(|| {
        TodoError::new(
            ErrorCode::Usage,
            format!(
                "unknown command: {} (run `todo schema` for the list)",
                args.command.join(" ")
            ),
        )
    })?;
    emit(out, &describe_command(cmd, &args.command, &globals))
}

/// Prints `data` as the `todo.schema.v1` envelope with `--json`, or pretty-printed otherwise.
fn emit<T: Serialize>(out: &mut Output, data: &T) -> Result<()> {
    if out.is_json() {
        out.emit_json_ok(META.name, data);
        return Ok(());
    }
    let text = serde_json::to_string_pretty(data).map_err(|e| {
        TodoError::new(
            ErrorCode::BadValue,
            format!("could not format the schema: {e}"),
        )
    })?;
    out.write_line(&text);
    Ok(())
}

/// The JSON envelope shape of R-OUT-1, described with placeholders for the values.
fn envelope_shape() -> Value {
    json!({
        "success": {
            "schema": "todo.<command>.v1",
            "ok": true,
            "data": "the command's result (see the command's entry)",
            "warnings": ["message"],
            "dry_run": "true, only with --dry-run on a write command",
            "diff": "unified diff, only with --dry-run on a write command"
        },
        "error": {
            "schema": "todo.<command>.v1 (todo.cli.v1 when the command line itself is invalid)",
            "ok": false,
            "error": {
                "code": "one of error_codes",
                "message": "plain-English message",
                "task": "optional: task ID",
                "owner": "optional: owning agent",
                "candidates": "optional: candidate IDs or section names",
                "line": "optional: 1-based line number",
                "file": "optional: file path"
            },
            "warnings": ["message"]
        }
    })
}

/// The fields of a task as `show --json` returns them, with an invented example.
fn task_fields() -> TaskFieldsJson {
    let example = TaskJson {
        id: "T-001".to_string(),
        title: "Example task".to_string(),
        section: Some("Tasks".to_string()),
        archived: false,
        status: "claimed".to_string(),
        owner: Some("agent-1".to_string()),
        claimed: Some("2026-11-15T18:00+01:00".to_string()),
        stale: false,
        priority: Some("P1".to_string()),
        task_type: Some("feature".to_string()),
        area: Some("cli".to_string()),
        depends: vec!["T-000".to_string()],
        dependencies: Vec::new(),
        dependents: Vec::new(),
        questions: Vec::new(),
        ready: false,
        files: None,
        problem: None,
        fix: None,
        accept: Some("the check that proves it is done".to_string()),
        commit: Vec::new(),
        fields: BTreeMap::new(),
        notes: Vec::new(),
        line_start: 1,
        line_end: 4,
    };
    let fields = serde_json::to_value(&example)
        .ok()
        .and_then(|value| value.as_object().map(|map| map.keys().cloned().collect()))
        .unwrap_or_default();
    TaskFieldsJson { fields, example }
}

/// Names of the options declared `global = true` on the root command.
fn global_ids(root: &ClapCommand) -> BTreeSet<String> {
    root.get_arguments()
        .filter(|arg| arg.is_global_set())
        .map(|arg| arg.get_id().as_str().to_string())
        .collect()
}

/// clap's built-in `--help` and `--version` are not part of the description.
fn is_builtin(arg: &Arg) -> bool {
    matches!(arg.get_id().as_str(), "help" | "version")
}

fn describe_arg(arg: &Arg) -> ArgumentJson {
    let name = if arg.is_positional() {
        arg.get_id().as_str().to_string()
    } else if let Some(long) = arg.get_long() {
        format!("--{long}")
    } else {
        arg.get_short()
            .map_or_else(|| arg.get_id().as_str().to_string(), |short| format!("-{short}"))
    };
    let value_name = arg.get_action().takes_values().then(|| {
        arg.get_value_names()
            .and_then(|names| names.first())
            .map_or_else(|| arg.get_id().as_str().to_uppercase(), ToString::to_string)
    });
    ArgumentJson {
        name,
        value_name,
        required: arg.is_required_set(),
        repeatable: matches!(arg.get_action(), ArgAction::Append),
        help: arg.get_help().map(ToString::to_string).unwrap_or_default(),
    }
}

/// The arguments (`positional == true`) or options of `cmd`, without hidden and global ones.
fn command_args(cmd: &ClapCommand, globals: &BTreeSet<String>, positional: bool) -> Vec<ArgumentJson> {
    cmd.get_arguments()
        .filter(|arg| {
            !arg.is_hide_set()
                && !is_builtin(arg)
                && !globals.contains(arg.get_id().as_str())
                && arg.is_positional() == positional
        })
        .map(describe_arg)
        .collect()
}

/// Describes one command; `path` holds its name words, such as `["hooks", "install"]`.
fn describe_command(cmd: &ClapCommand, path: &[String], globals: &BTreeSet<String>) -> CommandJson {
    let top_level = path.first().map_or("", String::as_str);
    CommandJson {
        name: path.join(" "),
        schema: schema_name(top_level),
        summary: cmd.get_about().map(ToString::to_string).unwrap_or_default(),
        subcommands: visible_subcommands(cmd)
            .map(|sub| sub.get_name().to_string())
            .collect(),
        arguments: command_args(cmd, globals, true),
        options: command_args(cmd, globals, false),
    }
}

/// Every visible command below `cmd`, depth first (hidden commands are not listed).
fn collect_commands(
    cmd: &ClapCommand,
    path: &[String],
    globals: &BTreeSet<String>,
    out: &mut Vec<CommandJson>,
) {
    for sub in visible_subcommands(cmd) {
        let mut sub_path = path.to_vec();
        sub_path.push(sub.get_name().to_string());
        out.push(describe_command(sub, &sub_path, globals));
        collect_commands(sub, &sub_path, globals, out);
    }
}

fn all_commands(root: &ClapCommand, globals: &BTreeSet<String>) -> Vec<CommandJson> {
    let mut commands = Vec::new();
    collect_commands(root, &[], globals, &mut commands);
    commands
}

fn visible_subcommands(cmd: &ClapCommand) -> impl Iterator<Item = &ClapCommand> {
    cmd.get_subcommands().filter(|sub| !sub.is_hide_set())
}

/// Finds the visible command named by `words` (one word per level), if any.
fn find_command<'a>(root: &'a ClapCommand, words: &[String]) -> Option<&'a ClapCommand> {
    let mut current = root;
    for word in words {
        current = visible_subcommands(current).find(|sub| sub.get_name() == word.as_str())?;
    }
    Some(current)
}
