//! `todo unblock` command: return a blocked task to claimed or open (T-025; R-WRITE-5, R-WRITE-9).

use clap::Args as ClapArgs;

use crate::cli::{CommandKind, CommandMeta, Context};
use crate::doc::Status;
use crate::error::Result;
use crate::ops::{
    OpsConfig, OwnershipDecision, StatusCommand, TaskEditBuilder, edit_task_and_write,
    forced_note_text, validate_transition,
};
use crate::output::Output;

/// Metadata for the `unblock` command.
pub const META: CommandMeta = CommandMeta {
    name: "unblock",
    summary: "Unblock a blocked task back to claimed or open",
    kind: CommandKind::Write,
    details: "Unblocks a `blocked` task: returns it to `claimed` (refreshing the `claimed` timestamp) if it has an `owner`, or to `open` otherwise. Appends an `unblocked` note.",
    examples: &[
        "todo unblock T-012 --agent agent-1",
        "todo unblock T-012 --agent agent-1 --note \"schema merged\"",
    ],
};

/// Arguments for `todo unblock`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Task ID (e.g. `T-012`, `t12`, or `12`).
    #[arg(value_name = "TASK")]
    pub task: String,

    /// Optional note explaining why the task is unblocked.
    #[arg(long, value_name = "TEXT")]
    pub note: Option<String>,
}

/// Runs `todo unblock <task>`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let cfg = OpsConfig::default();
    let res = edit_task_and_write(
        ctx,
        &cfg,
        META.name,
        &args.task,
        out,
        |doc, task, _archive, agent, now| {
            let id_str = cfg.id_config.format_id(task.id);
            let (next_status, ownership) =
                validate_transition(task, StatusCommand::Unblock, agent, ctx.force, &id_str)?;

            let mut builder = TaskEditBuilder::new(doc, task);
            builder.set_status(next_status);
            if next_status == Status::Claimed {
                builder.set_claimed(&OpsConfig::format_claimed_timestamp(now));
            } else {
                builder.remove_claimed();
            }

            let date = OpsConfig::format_note_date(now);
            if let OwnershipDecision::Forced { previous_owner } = ownership {
                builder.add_note(&date, agent, &forced_note_text("unblock", &previous_owner));
            }
            let note_text = match args.note.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                Some(text) => format!("unblocked \u{2014} {text}"),
                None => "unblocked".to_string(),
            };
            builder.add_note(&date, agent, &note_text);

            let (edits, _) = builder.build();
            Ok((edits, ()))
        },
    )?;

    if out.is_json() {
        out.emit_json_ok(META.name, &res.task);
    } else {
        out.write_line(&format!("{}: unblock ok", res.task_id));
    }
    Ok(())
}
