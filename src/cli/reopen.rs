//! `todo reopen` command: put a done or dropped task back to open (T-030; R-WRITE-10).

use clap::Args as ClapArgs;

use crate::cli::{CommandKind, CommandMeta, Context, resolve_text_arg, run_task_op};
use crate::doc::sanitize_trimmed_value;
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::TaskOp;
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
    let op = TaskOp::Reopen { note: Some(&reason) };
    run_task_op(ctx, out, META.name, &args.task, &op)
}
