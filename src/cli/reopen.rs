//! `todo reopen` command: put a done or dropped task back to open (T-030; R-WRITE-10).

use clap::Args as ClapArgs;

use crate::cli::{CommandKind, CommandMeta, Context, resolve_text_arg};
use crate::doc::{Status, sanitize_trimmed_value};
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::{
    OpsConfig, OwnershipDecision, StatusCommand, TaskEditBuilder, edit_task_and_write,
    forced_note_text, validate_transition,
};
use crate::output::Output;

/// Metadata for the `reopen` command.
pub const META: CommandMeta = CommandMeta {
    name: "reopen",
    summary: "Put a done or dropped task back to open",
    kind: CommandKind::Write,
    details: "Reopens a `done` or `dropped` task, clearing its owner and claim time, and records a required `--note` explaining why.",
    examples: &[
        "todo reopen T-012 --note \"regression found in CI\" --agent agent-1",
    ],
};

/// Arguments for `todo reopen`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Task ID or unique title substring.
    #[arg(value_name = "TASK")]
    pub task: String,

    /// Reason for reopening the task (required).
    #[arg(long, value_name = "TEXT")]
    pub note: String,
}

/// Runs `todo reopen <task> --note <text>`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let raw_note = resolve_text_arg(&args.note)?;
    let reason = sanitize_trimmed_value(&raw_note);
    if reason.is_empty() {
        return Err(TodoError::new(
            ErrorCode::BadValue,
            "--note cannot be empty",
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
            let (_, ownership) =
                validate_transition(task, StatusCommand::Reopen, agent, ctx.force, &id_str)?;

            let mut builder = TaskEditBuilder::new(doc, task);
            builder.set_status(Status::Open);
            builder.set_owner(None);
            builder.remove_claimed();

            let date = OpsConfig::format_note_date(now);
            if let OwnershipDecision::Forced { previous_owner } = ownership {
                builder.add_note(&date, agent, &forced_note_text("reopen", &previous_owner));
            }
            builder.add_note(&date, agent, &format!("reopened \u{2014} {reason}"));

            let (edits, _) = builder.build();
            Ok((edits, ()))
        },
    )?;

    if out.is_json() {
        out.emit_json_ok(META.name, &res.task);
    } else {
        out.write_line(&format!("{}: reopen ok", res.task_id));
    }
    Ok(())
}
