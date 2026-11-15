//! `todo answer` command: record an answer note and mark a question done (T-031; R-WRITE-11).

use clap::Args as ClapArgs;

use crate::cli::{CommandKind, CommandMeta, Context, resolve_text_arg};
use crate::doc::{Status, sanitize_trimmed_value};
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::{
    OpsConfig, OwnershipDecision, StatusCommand, TaskEditBuilder, edit_task_and_write,
    forced_note_text, validate_transition,
};
use crate::output::Output;

/// Metadata for the `answer` command.
pub const META: CommandMeta = CommandMeta {
    name: "answer",
    summary: "Answer a question and mark it done",
    kind: CommandKind::Write,
    details: "Adds the answer as a note on the question task and marks it `done`.",
    examples: &[
        "todo answer T-012 \"Yes, use exponential backoff capped at 30s\" --agent human",
    ],
};

/// Arguments for `todo answer`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Question task ID or unique title substring.
    #[arg(value_name = "TASK")]
    pub task: String,

    /// Answer text (recorded as a note).
    #[arg(value_name = "TEXT")]
    pub text: String,
}

/// Runs `todo answer <task> <text>`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let raw_text = resolve_text_arg(&args.text)?;
    let answer_text = sanitize_trimmed_value(&raw_text);
    if answer_text.is_empty() {
        return Err(TodoError::new(
            ErrorCode::BadValue,
            "answer text cannot be empty",
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
                validate_transition(task, StatusCommand::Done, agent, ctx.force, &id_str)?;

            let mut builder = TaskEditBuilder::new(doc, task);
            builder.set_status(Status::Done);
            builder.remove_claimed();

            let date = OpsConfig::format_note_date(now);
            if let OwnershipDecision::Forced { previous_owner } = ownership {
                builder.add_note(&date, agent, &forced_note_text("answer", &previous_owner));
            }
            builder.add_note(&date, agent, &format!("answered \u{2014} {answer_text}"));

            let (edits, _) = builder.build();
            Ok((edits, ()))
        },
    )?;

    if out.is_json() {
        out.emit_json_ok(META.name, &res.task);
    } else {
        out.write_line(&format!("{}: answer ok", res.task_id));
    }
    Ok(())
}
