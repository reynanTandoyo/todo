//! `todo set` command: edit task fields and title in place (T-027; R-WRITE-8, R-CLAIM-3).

use clap::Args as ClapArgs;

use crate::cli::{CommandKind, CommandMeta, Context};
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::{
    OpsConfig, OwnershipDecision, TaskCommand, TaskEditBuilder, check_ownership,
    edit_task_and_write, forced_note_text,
};
use crate::output::Output;

/// Metadata for the `set` command.
pub const META: CommandMeta = CommandMeta {
    name: "set",
    summary: "Edit fields or title of a task",
    kind: CommandKind::Write,
    details: "Updates task fields (`--title`, `--priority`, `--type`, `--area`, `--depends`, `--files`, `--problem`, `--fix`, `--accept`), arbitrary `--field key=value` pairs, or removes fields with `--unset key`. Edits combined `- priority:` lines in place. Refuses changing or unsetting `status`, `owner`, or `claimed`.",
    examples: &[
        "todo set T-012 --priority P0 --area cli --agent agent-1",
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

    /// New task type (e.g. `feature`, `bug`).
    #[arg(long = "type", value_name = "VALUE")]
    pub task_type: Option<String>,

    /// New area (e.g. `cli`, `docs`).
    #[arg(long, value_name = "VALUE")]
    pub area: Option<String>,

    /// New dependencies (comma-separated task IDs).
    #[arg(long, value_name = "IDS")]
    pub depends: Option<String>,

    /// New `files` field value.
    #[arg(long, value_name = "TEXT")]
    pub files: Option<String>,

    /// New `problem` field value.
    #[arg(long, value_name = "TEXT")]
    pub problem: Option<String>,

    /// New `fix` field value.
    #[arg(long, value_name = "TEXT")]
    pub fix: Option<String>,

    /// New `accept` field value.
    #[arg(long, value_name = "TEXT")]
    pub accept: Option<String>,

    /// Set any field with `key=value` (repeatable).
    #[arg(long = "field", value_name = "KEY=VALUE")]
    pub fields: Vec<String>,

    /// Remove a field by key (repeatable).
    #[arg(long = "unset", value_name = "KEY")]
    pub unset: Vec<String>,
}

/// Runs `todo set <task> ...`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let parsed_fields = parse_field_pairs(&args.fields)?;
    let parsed_unsets = parse_unset_keys(&args.unset)?;

    let has_any = args.title.is_some()
        || args.priority.is_some()
        || args.task_type.is_some()
        || args.area.is_some()
        || args.depends.is_some()
        || args.files.is_some()
        || args.problem.is_some()
        || args.fix.is_some()
        || args.accept.is_some()
        || !parsed_fields.is_empty()
        || !parsed_unsets.is_empty();
    if !has_any {
        return Err(TodoError::new(
            ErrorCode::Usage,
            "no fields specified: pass at least one option such as --priority, --field, or --unset",
        ));
    }

    if let Some(ref t) = args.title
        && t.trim().is_empty()
    {
        return Err(TodoError::new(
            ErrorCode::BadValue,
            "--title cannot be empty",
        ));
    }

    let cfg = ctx.load_config()?;
    let res = edit_task_and_write(
        ctx,
        &cfg,
        META.name,
        &args.task,
        out,
        |doc, task, _archive, agent, now| {
            let id_str = cfg.id_config.format_id(task.id);
            let ownership = check_ownership(task, TaskCommand::Set, agent, ctx.force, &id_str)?;

            let mut builder = TaskEditBuilder::new(doc, task);

            if let Some(ref title) = args.title {
                builder.set_title(title);
            }
            if let Some(ref priority) = args.priority {
                builder.set_field("priority", priority);
            }
            if let Some(ref task_type) = args.task_type {
                builder.set_field("type", task_type);
            }
            if let Some(ref area) = args.area {
                builder.set_field("area", area);
            }
            if let Some(ref depends) = args.depends {
                builder.set_field("depends", depends);
            }
            if let Some(ref files) = args.files {
                builder.set_field("files", files);
            }
            if let Some(ref problem) = args.problem {
                builder.set_field("problem", problem);
            }
            if let Some(ref fix) = args.fix {
                builder.set_field("fix", fix);
            }
            if let Some(ref accept) = args.accept {
                builder.set_field("accept", accept);
            }
            for (key, value) in &parsed_fields {
                if key == "title" {
                    builder.set_title(value);
                } else {
                    builder.set_field(key, value);
                }
            }
            for key in &parsed_unsets {
                builder.unset_field(key);
            }

            if let OwnershipDecision::Forced { previous_owner } = ownership {
                let date = OpsConfig::format_note_date(now);
                builder.add_note(&date, agent, &forced_note_text("set", &previous_owner));
            }

            let (edits, _) = builder.build();
            Ok((edits, ()))
        },
    )?;

    if out.is_json() {
        out.emit_json_ok(META.name, &res.task);
    } else {
        out.write_line(&format!("{}: set ok", res.task_id));
    }
    Ok(())
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
        if key == "title" && v.trim().is_empty() {
            return Err(TodoError::new(
                ErrorCode::BadValue,
                "title cannot be empty",
            ));
        }
        pairs.push((key, v.to_string()));
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
