//! `todo claim` command: claim a task or refresh/take over a claim (T-022; R-WRITE-2, R-CLAIM-1..4).

use clap::Args as ClapArgs;

use crate::cli::{CommandKind, CommandMeta, Context};
use crate::error::Result;
use crate::ops::{OpsConfig, build_claim_edits, edit_task_and_write, plan_claim};
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
    let cfg = OpsConfig::default();
    let res = edit_task_and_write(
        ctx,
        &cfg,
        META.name,
        &args.task,
        out,
        |doc, task, _archive, agent, now| {
            let plan = plan_claim(task, agent, ctx.force, now, &cfg)?;
            let (edits, _) = build_claim_edits(doc, task, &plan, now);
            Ok((edits, ()))
        },
    )?;

    if out.is_json() {
        out.emit_json_ok(META.name, &res.task);
    } else {
        out.write_line(&format!("{}: claim ok", res.task_id));
    }
    Ok(())
}
