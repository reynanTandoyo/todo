//! `todo claim` command: claim a task or refresh/take over a claim (T-022; R-WRITE-2, R-CLAIM-1..4).

use clap::Args as ClapArgs;

use crate::cli::{CommandKind, CommandMeta, Context, run_task_op};
use crate::error::Result;
use crate::ops::TaskOp;
use crate::output::Output;

/// Metadata for the `claim` command.
pub const META: CommandMeta = CommandMeta {
    name: "claim",
    summary: "Claim a task for an agent",
    kind: CommandKind::Write,
    details: "Sets status to `claimed`, records the agent as `owner`, and writes the local `claimed` timestamp with UTC offset. Claiming a task you already hold refreshes the claim time. Stale claims (past `claim_ttl_hours`, default 24h) can be taken over with an automatic note. Pass `--force` to take over a live claim or claim a `blocked` task.",
    examples: &[
        "todo claim T-012 --agent agent-1",
        "todo claim 12 --agent agent-1",
        "todo claim T-012 --agent agent-2 --force",
    ],
};

/// Arguments for `todo claim`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Task ID (e.g. `T-012`, `t12`, or `12`).
    #[arg(value_name = "TASK")]
    pub task: String,
}

/// Runs `todo claim <task>`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    run_task_op(ctx, out, META.name, &args.task, &TaskOp::Claim)
}
