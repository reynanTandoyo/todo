//! `todo promote` command: turn an idea into an ordinary task (R-KIND-1).

use clap::Args as ClapArgs;

use crate::cli::add::find_section;
use crate::cli::move_cmd::section_move_edits;
use crate::cli::{CommandKind, CommandMeta, Context};
use crate::doc::{Document, LineEdit, Task, TaskChangeIntent, sanitize_trimmed_value};
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::milestone;
use crate::ops::{
    DocWritePlan, OpEnv, TaskOp, build_task_json, check_promotable, check_task_writable,
    edit_doc_and_write, resolve_task_in_doc,
};
use crate::output::Output;

/// Metadata for the `promote` command.
pub const META: CommandMeta = CommandMeta {
    name: "promote",
    summary: "Turn an idea into a task",
    kind: CommandKind::Write,
    details: "Turns an idea (`type: idea`) into an ordinary task: removes `type: idea`, sets `--priority` (required) and `--milestone` (a milestone, format.md §17), keeps `about` and adds the note `promoted from idea`. The task needs a level like any task: `--difficulty` sets it, and an idea that has one keeps it; without either, a refusal with MISSING_FIELD leaves the file as it was (R-LEVEL-1). `--section` also moves the task to the end of that section, as `move` does. Refuses an entry that is not an idea.",
    examples: &[
        "todo promote T-020 --priority P2 --difficulty 2 --agent agent-1",
        "todo promote T-020 --priority P1 --difficulty 3 --milestone T-011 --section Backlog --agent agent-1",
    ],
};

/// Arguments for `todo promote`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Idea task ID or unique title substring.
    #[arg(value_name = "IDEA")]
    pub idea: String,

    /// Priority for the new task (one of the configured priorities, e.g. `P1`).
    #[arg(long, value_name = "P")]
    pub priority: String,

    /// Parent milestone (task ID of a milestone).
    #[arg(long, value_name = "TASK")]
    pub milestone: Option<String>,

    /// Level for the new task: one of the configured `difficulties` (`1` to `5` by default).
    #[arg(long, value_name = "LEVEL")]
    pub difficulty: Option<String>,

    /// Section to move the task to, at its end (exact or unique substring match).
    #[arg(long, value_name = "NAME")]
    pub section: Option<String>,
}

/// Runs `todo promote <idea> --priority P [--difficulty D] [--milestone <task>] [--section S]`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let cfg = ctx.load_config()?;
    let clean_priority = sanitize_trimmed_value(&args.priority);
    let Some(priority) = cfg
        .priorities
        .iter()
        .find(|p| p.eq_ignore_ascii_case(&clean_priority))
    else {
        return Err(TodoError::new(
            ErrorCode::BadValue,
            format!(
                "invalid priority '{clean_priority}'; expected one of: {}",
                cfg.priorities.join(", ")
            ),
        ));
    };
    let priority = priority.clone();
    // A level given on the command line is checked before anything is read or written (BAD_VALUE).
    let difficulty = match args.difficulty.as_deref() {
        Some(raw) => Some(cfg.canonical_difficulty(raw)?),
        None => None,
    };

    let cfg_clone = cfg.clone();
    let res = edit_doc_and_write(
        ctx,
        &cfg,
        META.name,
        out,
        |doc, archive, label, agent, now| {
            let task = resolve_task_in_doc(doc, &args.idea)?;
            check_task_writable(doc, task, label, archive)?;
            let id_str = cfg.id_config.format_id(task.id);
            check_promotable(task, &id_str)?;

            // The parent rules are the ones `add` and `set` apply (R-MILE-1): NOT_FOUND when missing,
            // BAD_VALUE when it is not a milestone.
            let milestone = match args.milestone.as_deref() {
                Some(input) => Some(milestone::check_parent(doc, &cfg, Some(task), input)?),
                None => None,
            };

            let target = match args.section.as_deref() {
                Some(query) => Some(find_section(doc, query)?),
                None => None,
            };

            // The promoted task is a task, so it needs a level as `add` requires one (R-LEVEL-1):
            // the one given here, or the one the idea already has. Refused before anything is written.
            if difficulty.is_none() && task.difficulty.is_none() && cfg.requires_field("difficulty") {
                return Err(TodoError::new(
                    ErrorCode::MissingField,
                    "missing required field `difficulty`; pass --difficulty <level>",
                )
                .with_detail("field", serde_json::Value::String("difficulty".to_string())));
            }

            // Field edits first (the same edits every single-task command uses), then the move, if
            // any, of the block as those edits leave it.
            let env = OpEnv {
                cfg: &cfg,
                agent,
                force: ctx.force,
                now,
                session: ctx.session.as_deref(),
            };
            let field_edits = TaskOp::Promote {
                priority: &priority,
                milestone: milestone.as_deref(),
                difficulty: difficulty.as_deref(),
            }
            .build_edits(doc, task, &env)?;

            let edits = match target {
                Some(idx) if task.section != Some(idx) => {
                    let block = promoted_block(doc, task, &field_edits)?;
                    section_move_edits(doc, task, idx, block)
                }
                _ => field_edits,
            };

            let task_id = task.id;
            let arch_owned = archive.cloned();
            let cfg_owned = cfg_clone.clone();
            let err_id = id_str.clone();

            Ok(DocWritePlan::Edits {
                edits,
                intent: TaskChangeIntent::modify_one(task_id),
                build_value: Box::new(move |new_doc| {
                    let promoted = new_doc.task_by_id(task_id).ok_or_else(|| {
                        TodoError::new(
                            ErrorCode::WriteFailed,
                            format!("promoted task {err_id} missing after verification"),
                        )
                    })?;
                    let task_json = build_task_json(
                        new_doc,
                        promoted,
                        arch_owned.as_ref(),
                        false,
                        now,
                        &cfg_owned,
                    );
                    Ok((err_id, task_json))
                }),
            })
        },
    )?;

    if out.is_json() {
        out.emit_json_ok(META.name, &res.value.1);
    } else {
        out.write_line(&format!("{}: promote ok", res.value.0));
    }
    Ok(())
}

/// The lines of `task`'s block once `edits` are applied (`docs/format.md` §10).
fn promoted_block(doc: &Document, task: &Task, edits: &[LineEdit]) -> Result<Vec<String>> {
    let promote_failed = |detail: String| {
        TodoError::new(
            ErrorCode::WriteFailed,
            format!(
                "promote of {} failed: {detail}",
                doc.id_config.format_id(task.id)
            ),
        )
    };
    let applied = doc
        .line_doc
        .apply_edits(edits)
        .map_err(|e| promote_failed(e.to_string()))?;
    let edited = Document::parse_bytes_with_config(&applied.bytes, doc.id_config.clone())
        .map_err(|e| promote_failed(e.to_string()))?;
    let moved = edited
        .task_by_id(task.id)
        .ok_or_else(|| promote_failed("the task is missing after the edit".to_string()))?;
    let block = applied
        .doc
        .lines
        .get(moved.block_range.clone())
        .ok_or_else(|| promote_failed("the task block is out of range".to_string()))?;
    Ok(block.iter().map(|l| l.text.clone()).collect())
}
