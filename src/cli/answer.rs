//! `todo answer` command: record an answer note and mark a question done (T-031; R-WRITE-11).

use clap::Args as ClapArgs;

use crate::cli::{CommandKind, CommandMeta, Context, resolve_text_arg, run_task_op};
use crate::doc::sanitize_trimmed_value;
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::TaskOp;
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
    let op = TaskOp::Answer {
        verb: "answered",
        text: &answer_text,
    };
    run_task_op(ctx, out, META.name, &args.task, &op)
}
