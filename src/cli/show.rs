//! `todo show` command: display a single task block and its dependencies (T-021; R-READ-2, R-DEP-2).

use clap::Args as ClapArgs;

use crate::cli::{CommandKind, CommandMeta, Context, load_read, write_milestone_chain};
use crate::error::{ErrorCode, Result, TodoError};
use crate::lock::current_time;
use crate::ops::{
    build_task_json, file_label, milestone_chain_json, questions_about, resolve_task_by_title,
};
use crate::output::Output;

/// Metadata for the `show` command.
pub const META: CommandMeta = CommandMeta {
    name: "show",
    summary: "Show full details of a task",
    kind: CommandKind::Read,
    details: "Prints the task block exactly as it appears in TODO.md (or TODO-archive.md if archived), then the questions about it with their status, then the milestones above it from the top down with their goals. With `--json`, returns all known and unknown fields, notes, line numbers, dependencies with their statuses, tasks waiting on this task, the questions about it (`questions`), the milestones above it (`milestones`), whether `next` could hand it out now (`ready`) and what the detail rule lacks at the target level (`missing`, R-DETAIL-2). Works for milestones too.",
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
    let (cfg, loaded) = load_read(ctx, out, false, true)?;

    let (primary_doc, secondary_doc, task, archived) =
        if let Some(task_id) = cfg.id_config.parse_user_id(&args.task) {
            let id_str = cfg.id_config.format_id(task_id);
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
                return Err(TodoError::new(
                    ErrorCode::NotFound,
                    format!("task {id_str} not found"),
                )
                .with_task(id_str));
            }
        } else {
            let t = resolve_task_by_title(&loaded.doc, &args.task)?;
            (&loaded.doc, loaded.archive.as_ref(), t, false)
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

    let questions = questions_about(primary_doc, secondary_doc, archived, task.id, &cfg);
    if !questions.is_empty() {
        out.write_line("");
        out.write_line(&format!(
            "Questions about {}:",
            cfg.id_config.format_id(task.id)
        ));
        for question in &questions {
            let archived_note = if question.archived { " (archived)" } else { "" };
            out.write_line(&format!(
                "- {} · {} · {}{archived_note}",
                question.id, question.status, question.title
            ));
        }
    }

    let milestones = milestone_chain_json(primary_doc, task, &cfg);
    if !milestones.is_empty() {
        out.write_line("");
        write_milestone_chain(out, &cfg.id_config.format_id(task.id), &milestones);
    }
    Ok(())
}
