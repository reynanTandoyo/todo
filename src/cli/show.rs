//! `todo show` command: display a single task block and its dependencies (T-021; R-READ-2, R-DEP-2).

use clap::Args as ClapArgs;

use crate::cli::{CommandKind, CommandMeta, Context};
use crate::error::{ErrorCode, Result, TodoError};
use crate::lock::current_time;
use crate::ops::{OpsConfig, build_task_json, file_label, load_for_read};
use crate::output::Output;

/// Metadata for the `show` command.
pub const META: CommandMeta = CommandMeta {
    name: "show",
    summary: "Show full details of a task",
    kind: CommandKind::Read,
    details: "Prints the task block exactly as it appears in TODO.md (or TODO-archive.md if archived). With `--json`, returns all known and unknown fields, notes, line numbers, dependencies with their statuses, and tasks waiting on this task.",
    examples: &[
        "todo show T-001",
        "todo show 1",
        "todo show t-1 --json",
    ],
};

/// Arguments for `todo show`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Task ID (e.g. `T-001`, `t1`, or `1`).
    #[arg(value_name = "TASK")]
    pub task: String,
}

/// Runs `todo show <task>`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let cfg = OpsConfig::default();
    let Some(task_id) = cfg.id_config.parse_user_id(&args.task) else {
        return Err(TodoError::new(
            ErrorCode::NotFound,
            format!("task '{}' not found", args.task),
        ));
    };

    let loaded = load_for_read(ctx, &cfg, false, true, out)?;
    let id_str = cfg.id_config.format_id(task_id);

    let (primary_doc, secondary_doc, task, archived) =
        if let Some(t) = loaded.doc.task_by_id(task_id) {
            (&loaded.doc, loaded.archive.as_ref(), t, false)
        } else if let Some(ref arch_doc) = loaded.archive
            && let Some(t) = arch_doc.task_by_id(task_id)
        {
            let arch_label = file_label(&cfg.archive_path(&loaded.found.path));
            for problem in &arch_doc.problems {
                out.warn(problem.format_with_path(&arch_label));
            }
            (arch_doc, Some(&loaded.doc), t, true)
        } else {
            return Err(
                TodoError::new(ErrorCode::NotFound, format!("task {id_str} not found"))
                    .with_task(id_str),
            );
        };

    if out.is_json() {
        let now = current_time();
        let json = build_task_json(primary_doc, task, secondary_doc, archived, now, &cfg);
        out.emit_json_ok(META.name, &json);
        return Ok(());
    }

    for line in &primary_doc.line_doc.lines[task.block_range.clone()] {
        out.write_line(&line.text);
    }
    Ok(())
}
