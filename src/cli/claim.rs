//! `todo claim` command: claim a task or refresh/take over a claim (T-022; R-WRITE-2, R-CLAIM-1..4).

use clap::Args as ClapArgs;

use crate::cli::{
    CommandKind, CommandMeta, Context, edit_one_task, report_task_ok, write_milestone_chain,
};
use crate::error::Result;
use crate::ops::TaskOp;
use crate::output::Output;

/// Metadata for the `claim` command.
pub const META: CommandMeta = CommandMeta {
    name: "claim",
    summary: "Claim a task for an agent",
    kind: CommandKind::Write,
    details: "Sets status to `claimed`, records the agent as `owner`, and writes the local `claimed` timestamp with UTC offset. Claiming a task you already hold refreshes the claim time. When an agent session is known (`--session`, else `TODO_SESSION`, else `CLAUDE_CODE_SESSION_ID`), it is written as `session`, so `todo release --session <ID>` (or the `SessionEnd` hook from `todo hooks install`) can give the claim back when the session ends. Stale claims (past `claim_ttl_hours`, default 24h) can be taken over with an automatic note. Pass `--force` to take over a live claim or claim a `blocked` task. A task whose `files` overlap a task another agent holds is refused (`FILES_BUSY`) unless `--force`, which records the override.",
    examples: &[
        "todo claim T-012 --agent agent-1",
        "todo claim 12 --agent agent-1",
        "todo claim T-012 --agent agent-2 --force",
        "todo claim T-012 --agent agent-1 --session 6f0c1c2e-1111-4222-8333-944455556666",
    ],
};

/// Arguments for `todo claim`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Task ID (e.g. `T-012`, `t12`, or `12`).
    #[arg(value_name = "TASK")]
    pub task: String,
}

/// Runs `todo claim <task>`. The text result also lists the milestones above the task (`R-MILE-4`).
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let cfg = ctx.load_config()?;
    let res = edit_one_task(ctx, out, META.name, &args.task, &TaskOp::Claim, &cfg, None)?;
    report_task_ok(out, META.name, &res);
    if !out.is_json() {
        write_milestone_chain(out, &res.task_id, &res.task.milestones);
    }
    Ok(())
}
