//! `todo drop` command: mark a task dropped with a reason note (T-025; R-WRITE-6, R-WRITE-9).

use clap::Args as ClapArgs;

use crate::cli::{CommandKind, CommandMeta, Context};
use crate::doc::Status;
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::{
    OpsConfig, OwnershipDecision, StatusCommand, TaskEditBuilder, edit_task_and_write,
    forced_note_text, validate_transition,
};
use crate::output::Output;

/// Metadata for the `drop` command.
pub const META: CommandMeta = CommandMeta {
    name: "drop",
    summary: "Mark a task dropped with a reason",
    kind: CommandKind::Write,
    details: "Marks an `open`, `claimed`, or `blocked` task as `dropped`, removes any `claimed` timestamp, and appends a `dropped — <reason>` note. Requires `--note`.",
    examples: &[
        "todo drop T-012 --agent agent-1 --note \"superseded by T-020\"",
        "todo drop T-012 --agent coordinator --force --note \"out of scope\"",
    ],
};

/// Arguments for `todo drop`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Task ID (e.g. `T-012`, `t12`, or `12`).
    #[arg(value_name = "TASK")]
    pub task: String,

    /// Reason the task is dropped (required).
    #[arg(long, value_name = "TEXT")]
    pub note: String,
}

/// Runs `todo drop <task> --note <reason>`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let reason = args.note.trim();
    if reason.is_empty() {
        return Err(TodoError::new(
            ErrorCode::Usage,
            "--note reason cannot be empty for drop",
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
                validate_transition(task, StatusCommand::Drop, agent, ctx.force, &id_str)?;

            let mut builder = TaskEditBuilder::new(doc, task);
            builder.set_status(Status::Dropped);
            builder.remove_claimed();

            let date = OpsConfig::format_note_date(now);
            if let OwnershipDecision::Forced { previous_owner } = ownership {
                builder.add_note(&date, agent, &forced_note_text("drop", &previous_owner));
            }
            builder.add_note(&date, agent, &format!("dropped \u{2014} {reason}"));

            let (edits, _) = builder.build();
            Ok((edits, ()))
        },
    )?;

    if out.is_json() {
        out.emit_json_ok(META.name, &res.task);
    } else {
        out.write_line(&format!("{}: drop ok", res.task_id));
    }
    Ok(())
}
