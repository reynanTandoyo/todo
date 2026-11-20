//! `todo stuck` command: give a claimed task back to the planner with the problem (R-STUCK-1, T-159).

use clap::Args as ClapArgs;

use crate::cli::{CommandKind, CommandMeta, Context, resolve_text_arg, run_task_op};
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::TaskOp;
use crate::output::Output;

/// Metadata for the `stuck` command.
pub const META: CommandMeta = CommandMeta {
    name: "stuck",
    summary: "Give a claimed task back to the planner with the problem",
    kind: CommandKind::Write,
    details: "Gives a `claimed` task back to the planner: it becomes `blocked`, loses its owner and its claim, gets the note `stuck — <problem>` and one more `attempts`. The level is not changed. Requires `--note`. Only the owner may do it; another agent needs `--force`. `next` never hands out a blocked task, so the task waits until the planner runs `unblock` or `split` on it.",
    examples: &[
        "todo stuck T-012 --agent agent-1 --note \"the schema file does not exist\"",
        "todo stuck T-012 --agent coordinator --force --note \"worker stopped responding\"",
    ],
};

/// Arguments for `todo stuck`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Task ID (e.g. `T-012`, `t12`, or `12`).
    #[arg(value_name = "TASK")]
    pub task: String,

    /// The problem that stops the task (required).
    #[arg(long, value_name = "TEXT")]
    pub note: String,
}

/// Runs `todo stuck <task> --note <problem>`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let resolved_note = resolve_text_arg(&args.note)?;
    let problem = resolved_note.trim();
    if problem.is_empty() {
        return Err(TodoError::new(
            ErrorCode::Usage,
            "--note problem cannot be empty for stuck",
        ));
    }
    run_task_op(
        ctx,
        out,
        META.name,
        &args.task,
        &TaskOp::Stuck { reason: problem },
    )
}
