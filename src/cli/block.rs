//! `todo block` command: mark a task blocked with a reason note (T-025; R-WRITE-5, R-WRITE-9).

use clap::Args as ClapArgs;

use crate::cli::{CommandKind, CommandMeta, Context, resolve_text_arg, run_task_op};
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::TaskOp;
use crate::output::Output;

/// Metadata for the `block` command.
pub const META: CommandMeta = CommandMeta {
    name: "block",
    summary: "Mark a task blocked with a reason",
    kind: CommandKind::Write,
    details: "Marks an `open` or `claimed` task as `blocked` and appends a `blocked — <reason>` note. Keeps any existing `owner` and `claimed` timestamp. Requires `--note`.",
    examples: &[
        "todo block T-012 --agent agent-1 --note \"waiting on API schema\"",
        "todo block T-012 --agent coordinator --force --note \"paused\"",
    ],
};

/// Arguments for `todo block`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Task ID (e.g. `T-012`, `t12`, or `12`).
    #[arg(value_name = "TASK")]
    pub task: String,

    /// Reason the task is blocked (required).
    #[arg(long, value_name = "TEXT")]
    pub note: String,
}

/// Runs `todo block <task> --note <reason>`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let resolved_note = resolve_text_arg(&args.note)?;
    let reason = resolved_note.trim();
    if reason.is_empty() {
        return Err(TodoError::new(
            ErrorCode::Usage,
            "--note reason cannot be empty for block",
        ));
    }
    run_task_op(ctx, out, META.name, &args.task, &TaskOp::Block { reason })
}
