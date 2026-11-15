//! `todo note` command: append a dated note to any task (T-026; R-WRITE-7, R-IO-6).

use clap::Args as ClapArgs;

use crate::cli::{
    CommandKind, CommandMeta, Context, edit_one_task, report_task_ok, resolve_text_arg,
};
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::TaskOp;
use crate::output::Output;

/// Metadata for the `note` command.
pub const META: CommandMeta = CommandMeta {
    name: "note",
    summary: "Add a dated note to a task",
    kind: CommandKind::Write,
    details: "Appends `- note (YYYY-MM-DD, <agent>): <text>` after the last line of the task block using the local date. Any agent may add a note to a task in any status without `--force`.",
    examples: &[
        "todo note T-012 \"reproduced on Windows 11\" --agent agent-1",
        "todo note T-012 --text \"checked logs\" --agent reviewer",
    ],
};

/// Arguments for `todo note`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Task ID (e.g. `T-012`, `t12`, or `12`).
    #[arg(value_name = "TASK")]
    pub task: String,

    /// Note text (positional form).
    #[arg(value_name = "TEXT", conflicts_with = "text_flag")]
    pub text: Option<String>,

    /// Note text (flag form, alternative to positional `TEXT`).
    #[arg(long = "text", value_name = "TEXT")]
    pub text_flag: Option<String>,
}

/// Runs `todo note <task> <text>`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let raw_text = args.text.or(args.text_flag).ok_or_else(|| {
        TodoError::new(
            ErrorCode::Usage,
            "missing note text: pass <TEXT> or --text <TEXT>",
        )
    })?;
    let resolved_text = resolve_text_arg(&raw_text)?;
    let note_text = resolved_text.trim();
    if note_text.is_empty() {
        return Err(TodoError::new(ErrorCode::Usage, "note text cannot be empty"));
    }

    let (found, cfg) = ctx.load_config_with_found()?;
    let op = TaskOp::Note { text: note_text };
    let res = edit_one_task(ctx, out, META.name, &args.task, &op, &cfg, Some(found))?;
    report_task_ok(out, META.name, &res);
    Ok(())
}
