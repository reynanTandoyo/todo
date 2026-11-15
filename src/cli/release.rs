//! `todo release` command: release a claimed or blocked task (T-022; R-WRITE-3, R-CLAIM-3).

use clap::Args as ClapArgs;

use crate::cli::{CommandKind, CommandMeta, Context};
use crate::error::Result;
use crate::ops::{build_release_edits, edit_task_and_write, plan_release};
use crate::output::Output;

/// Metadata for the `release` command.
pub const META: CommandMeta = CommandMeta {
    name: "release",
    summary: "Release a claimed task back to open (or clear owner on blocked)",
    kind: CommandKind::Write,
    details: "Gives a task back: a `claimed` task becomes `open`, while a `blocked` task stays `blocked`. Removes the owner and claim time. Only the owner may release unless `--force` is passed.",
    examples: &[
        "todo release T-012 --agent agent-1",
        "todo release T-012 --agent coordinator --force",
    ],
};

/// Arguments for `todo release`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Task ID (e.g. `T-012`, `t12`, or `12`).
    #[arg(value_name = "TASK")]
    pub task: String,
}

/// Runs `todo release <task>`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let cfg = ctx.load_config()?;
    let res = edit_task_and_write(
        ctx,
        &cfg,
        META.name,
        &args.task,
        out,
        |doc, task, _archive, agent, now| {
            let plan = plan_release(task, agent, ctx.force, &cfg)?;
            let (edits, _) = build_release_edits(doc, task, &plan, agent, now);
            Ok((edits, ()))
        },
    )?;

    if out.is_json() {
        out.emit_json_ok(META.name, &res.task);
    } else {
        out.write_line(&format!("{}: release ok", res.task_id));
    }
    Ok(())
}
