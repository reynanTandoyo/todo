//! `todo drop` command: mark a task dropped with a reason note (T-025; R-WRITE-6, R-WRITE-9).

use clap::Args as ClapArgs;

use crate::cli::{
    CommandKind, CommandMeta, Context, edit_one_task, report_task_with_ready, resolve_text_arg,
};
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::TaskOp;
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
    let resolved_note = resolve_text_arg(&args.note)?;
    let reason = resolved_note.trim();
    if reason.is_empty() {
        return Err(TodoError::new(
            ErrorCode::Usage,
            "--note reason cannot be empty for drop",
        ));
    }

    let cfg = ctx.load_config()?;
    let op = TaskOp::Drop { reason };
    let res = edit_one_task(ctx, out, META.name, &args.task, &op, &cfg, None)?;
    report_task_with_ready(out, META.name, &res, false);
    Ok(())
}
