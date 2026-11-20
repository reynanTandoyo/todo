//! `todo set` command: edit task fields and title in place (T-027; R-WRITE-8, R-CLAIM-3).

use clap::Args as ClapArgs;

use crate::cli::{
    CommandKind, CommandMeta, Context, resolve_opt_text_arg, resolve_text_arg, run_task_op,
};
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::{SetChange, TaskOp};
use crate::output::Output;

/// Metadata for the `set` command.
pub const META: CommandMeta = CommandMeta {
    name: "set",
    summary: "Edit fields or title of a task",
    kind: CommandKind::Write,
    details: "Updates task fields (`--title`, `--priority`, `--difficulty`, `--type`, `--area`, `--milestone`, `--goal`, `--why`, `--depends`, `--files`, `--read`, `--problem`, `--fix`, `--steps`, `--accept`, `--verify`), arbitrary `--field key=value` pairs, or removes fields with `--unset key`. `--steps` takes one step per line and replaces the whole list. `--milestone` moves the entry, with everything below it, under another milestone; `--unset milestone` makes it top-level. Edits combined `- priority:` lines in place. Refuses changing or unsetting `status`, `owner`, or `claimed`.",
    examples: &[
        "todo set T-012 --priority P0 --area cli --agent agent-1",
        "todo set T-012 --verify \"cargo test\" --steps @steps.txt --agent agent-1",
        "todo set T-012 --milestone T-010 --agent agent-1",
        "todo set T-012 --field reqs=R-WRITE-8 --unset fix --agent agent-1",
        "todo set T-012 --title \"New title\" --agent coordinator --force",
    ],
};

/// Arguments for `todo set`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Task ID (e.g. `T-012`, `t12`, or `12`).
    #[arg(value_name = "TASK")]
    pub task: String,

    /// New task title.
    #[arg(long, value_name = "TEXT")]
    pub title: Option<String>,

    /// New priority (e.g. `P0`, `P1`).
    #[arg(long, value_name = "VALUE")]
    pub priority: Option<String>,

    /// New difficulty: one of the configured `difficulties` (e.g. `easy`, `hard`).
    #[arg(long, value_name = "VALUE")]
    pub difficulty: Option<String>,

    /// New task type (e.g. `feature`, `bug`).
    #[arg(long = "type", value_name = "VALUE")]
    pub task_type: Option<String>,

    /// New area (e.g. `cli`, `docs`).
    #[arg(long, value_name = "VALUE")]
    pub area: Option<String>,

    /// New parent milestone: the ID or title of a milestone (remove it with `--unset milestone`).
    #[arg(long, value_name = "TASK")]
    pub milestone: Option<String>,

    /// New goal of a milestone.
    #[arg(long, value_name = "TEXT")]
    pub goal: Option<String>,

    /// New `why` field value: what the task is for.
    #[arg(long, value_name = "TEXT")]
    pub why: Option<String>,

    /// New dependencies (comma-separated task IDs).
    #[arg(long, value_name = "IDS")]
    pub depends: Option<String>,

    /// New `files` field value.
    #[arg(long, value_name = "TEXT")]
    pub files: Option<String>,

    /// New `read` field value: paths to look at but not change, separated by commas.
    #[arg(long, value_name = "TEXT")]
    pub read: Option<String>,

    /// New `problem` field value.
    #[arg(long, value_name = "TEXT")]
    pub problem: Option<String>,

    /// New `fix` field value.
    #[arg(long, value_name = "TEXT")]
    pub fix: Option<String>,

    /// New `steps` list: one step per line of the text, replacing the whole list.
    #[arg(long, value_name = "TEXT")]
    pub steps: Option<String>,

    /// New `accept` field value.
    #[arg(long, value_name = "TEXT")]
    pub accept: Option<String>,

    /// New `verify` field value: one command line that checks the task.
    #[arg(long, value_name = "TEXT")]
    pub verify: Option<String>,

    /// Set any field with `key=value` (repeatable).
    #[arg(long = "field", value_name = "KEY=VALUE")]
    pub fields: Vec<String>,

    /// Remove a field by key (repeatable); `--unset milestone` makes the entry top-level.
    #[arg(long = "unset", value_name = "KEY")]
    pub unset: Vec<String>,
}

/// Runs `todo set <task> ...`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let parsed_fields = parse_field_pairs(&args.fields)?;
    let parsed_unsets = parse_unset_keys(&args.unset)?;

    let has_any = args.title.is_some()
        || args.priority.is_some()
        || args.difficulty.is_some()
        || args.task_type.is_some()
        || args.area.is_some()
        || args.milestone.is_some()
        || args.goal.is_some()
        || args.why.is_some()
        || args.depends.is_some()
        || args.files.is_some()
        || args.read.is_some()
        || args.problem.is_some()
        || args.fix.is_some()
        || args.steps.is_some()
        || args.accept.is_some()
        || args.verify.is_some()
        || !parsed_fields.is_empty()
        || !parsed_unsets.is_empty();
    if !has_any {
        return Err(TodoError::new(
            ErrorCode::Usage,
            "no fields specified: pass at least one option such as --priority, --field, or --unset",
        ));
    }

    let resolved_title = resolve_opt_text_arg(args.title.as_deref())?;
    let resolved_goal = resolve_opt_text_arg(args.goal.as_deref())?;
    let resolved_why = resolve_opt_text_arg(args.why.as_deref())?;
    let resolved_files = resolve_opt_text_arg(args.files.as_deref())?;
    let resolved_read = resolve_opt_text_arg(args.read.as_deref())?;
    let resolved_problem = resolve_opt_text_arg(args.problem.as_deref())?;
    let resolved_fix = resolve_opt_text_arg(args.fix.as_deref())?;
    let resolved_steps = resolve_opt_text_arg(args.steps.as_deref())?;
    let resolved_accept = resolve_opt_text_arg(args.accept.as_deref())?;
    let resolved_verify = resolve_opt_text_arg(args.verify.as_deref())?;

    if let Some(ref t) = resolved_title
        && t.trim().is_empty()
    {
        return Err(TodoError::new(
            ErrorCode::BadValue,
            "--title cannot be empty",
        ));
    }

    // Same order as before: named options, then `--field` pairs, then `--unset`.
    let mut changes: Vec<SetChange<'_>> = Vec::new();
    if let Some(ref title) = resolved_title {
        changes.push(SetChange::Title(title));
    }
    let named = [
        ("priority", args.priority.as_ref()),
        ("difficulty", args.difficulty.as_ref()),
        ("type", args.task_type.as_ref()),
        ("area", args.area.as_ref()),
        ("milestone", args.milestone.as_ref()),
        ("goal", resolved_goal.as_ref()),
        ("why", resolved_why.as_ref()),
        ("depends", args.depends.as_ref()),
        ("files", resolved_files.as_ref()),
        ("read", resolved_read.as_ref()),
        ("problem", resolved_problem.as_ref()),
        ("fix", resolved_fix.as_ref()),
        ("steps", resolved_steps.as_ref()),
        ("accept", resolved_accept.as_ref()),
        ("verify", resolved_verify.as_ref()),
    ];
    for (key, value) in named {
        if let Some(value) = value {
            changes.push(SetChange::Field(key, value));
        }
    }
    for (key, value) in &parsed_fields {
        changes.push(if key == "title" {
            SetChange::Title(value)
        } else {
            SetChange::Field(key, value)
        });
    }
    changes.extend(parsed_unsets.iter().map(|k| SetChange::Unset(k)));

    run_task_op(
        ctx,
        out,
        META.name,
        &args.task,
        &TaskOp::Set { changes: &changes },
    )
}

fn parse_field_pairs(raw: &[String]) -> Result<Vec<(String, String)>> {
    let mut pairs = Vec::with_capacity(raw.len());
    for item in raw {
        let Some((k, v)) = item.split_once('=') else {
            return Err(TodoError::new(
                ErrorCode::Usage,
                format!("invalid --field {item:?}: expected KEY=VALUE"),
            ));
        };
        let key = k.trim().to_ascii_lowercase();
        if key.is_empty() {
            return Err(TodoError::new(
                ErrorCode::Usage,
                format!("invalid --field {item:?}: key cannot be empty"),
            ));
        }
        check_protected_key(&key, false)?;
        let resolved_v = resolve_text_arg(v)?;
        if key == "title" && resolved_v.trim().is_empty() {
            return Err(TodoError::new(
                ErrorCode::BadValue,
                "title cannot be empty",
            ));
        }
        pairs.push((key, resolved_v));
    }
    Ok(pairs)
}

fn parse_unset_keys(raw: &[String]) -> Result<Vec<String>> {
    let mut keys = Vec::with_capacity(raw.len());
    for item in raw {
        let key = item.trim().to_ascii_lowercase();
        if key.is_empty() {
            return Err(TodoError::new(
                ErrorCode::Usage,
                "invalid --unset: key cannot be empty",
            ));
        }
        check_protected_key(&key, true)?;
        keys.push(key);
    }
    Ok(keys)
}

fn check_protected_key(key: &str, is_unset: bool) -> Result<()> {
    if matches!(key, "status" | "owner" | "claimed") {
        return Err(TodoError::new(
            ErrorCode::BadValue,
            format!("cannot change `{key}` with set; use status commands instead"),
        ));
    }
    if key == "note" {
        return Err(TodoError::new(
            ErrorCode::BadValue,
            "notes are append-only; use `todo note` instead",
        ));
    }
    if is_unset && key == "title" {
        return Err(TodoError::new(
            ErrorCode::BadValue,
            "cannot unset `title`; a task must have a title",
        ));
    }
    Ok(())
}
