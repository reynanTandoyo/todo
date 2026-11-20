//! `todo claim` command: claim a task or refresh/take over a claim (T-022; R-WRITE-2, R-CLAIM-1..4).

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::cli::brief::write_brief;
use crate::cli::{
    CommandKind, CommandMeta, Context, edit_one_task, load_read, report_task_ok,
    write_milestone_chain,
};
use crate::error::Result;
use crate::ops::{BriefJson, TaskJson, TaskOp, build_brief, resolve_task_in_doc};
use crate::output::Output;

/// Metadata for the `claim` command.
pub const META: CommandMeta = CommandMeta {
    name: "claim",
    summary: "Claim a task for an agent",
    kind: CommandKind::Write,
    details: "Sets status to `claimed`, records the agent as `owner`, and writes the local `claimed` timestamp with UTC offset. Claiming a task you already hold refreshes the claim time. When an agent session is known (`--session`, else `TODO_SESSION`, else `CLAUDE_CODE_SESSION_ID`), it is written as `session`, so `todo release --session <ID>` (or the `SessionEnd` hook from `todo hooks install`) can give the claim back when the session ends. Stale claims (past `claim_ttl_hours`, default 24h) can be taken over with an automatic note. Pass `--force` to take over a live claim or claim a `blocked` task. A task whose `files` overlap a task another agent holds is refused (`FILES_BUSY`) unless `--force`, which records the override. `--brief` prints the work order after the claim, as `todo brief` does (R-DETAIL-3).",
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

    /// Print the work order of the task after the claim, as `todo brief` does (R-DETAIL-3).
    #[arg(long)]
    pub brief: bool,
}

/// `claim --brief --json`: the claimed task, with its work order under `brief` (`todo.claim.v1`).
#[derive(Serialize)]
struct ClaimBriefJson<'a> {
    #[serde(flatten)]
    task: &'a TaskJson,
    brief: &'a BriefJson,
}

/// Runs `todo claim <task>`. The text result also lists the milestones above the task (`R-MILE-4`),
/// or with `--brief` the whole work order, which starts with those milestones.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let cfg = ctx.load_config()?;
    let res = edit_one_task(ctx, out, META.name, &args.task, &TaskOp::Claim, &cfg, None)?;
    if !args.brief {
        report_task_ok(out, META.name, &res);
        if !out.is_json() {
            write_milestone_chain(out, &res.task_id, &res.task.milestones);
        }
        return Ok(());
    }
    // The work order is read back from the file after the claim, so it is what `todo brief` prints.
    let (_, loaded) = load_read(ctx, out, false, true)?;
    let task = resolve_task_in_doc(&loaded.doc, &res.task_id)?;
    let brief = build_brief(
        &loaded.doc,
        loaded.archive.as_ref(),
        task,
        &cfg,
        ctx.agent.as_deref(),
    );
    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &ClaimBriefJson {
                task: &res.task,
                brief: &brief,
            },
        );
    } else {
        report_task_ok(out, META.name, &res);
        write_brief(out, &brief);
    }
    Ok(())
}
