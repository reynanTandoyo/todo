//! `todo unblock` command: return a blocked task to claimed or open (T-025; R-WRITE-5, R-WRITE-9).

use clap::Args as ClapArgs;

use crate::cli::{CommandKind, CommandMeta, Context, run_task_op};
use crate::error::Result;
use crate::ops::TaskOp;
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
    let op = TaskOp::Unblock {
        note: args.note.as_deref(),
    };
    run_task_op(ctx, out, META.name, &args.task, &op)
}
