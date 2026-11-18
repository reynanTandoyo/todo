//! `todo move` command: move a task to another section without altering its lines (T-030; R-WRITE-10).

use clap::Args as ClapArgs;

use crate::cli::add::find_section;
use crate::cli::{CommandKind, CommandMeta, Context};
use crate::doc::{Document, LineEdit, Task, TaskChangeIntent};
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::{
    DocWritePlan, OpsConfig, OwnershipDecision, TaskCommand, build_task_json, check_ownership,
    check_task_writable, edit_doc_and_write, forced_note_text, resolve_task_in_doc,
};
use crate::output::Output;

/// Metadata for the `move` command.
pub const META: CommandMeta = CommandMeta {
    name: "move",
    summary: "Move a task to the end of another section",
    kind: CommandKind::Write,
    details: "Moves a task with all its lines verbatim to the end of another `## ` section, leaving the rest of the file unchanged.",
    examples: &["todo move T-012 --section Done --agent agent-1"],
};

/// Arguments for `todo move`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Task ID or unique title substring.
    #[arg(value_name = "TASK")]
    pub task: String,

    /// Target section name (exact or unique substring match).
    #[arg(long, value_name = "NAME")]
    pub section: String,
}

/// Whole-line edits that move `task` to the end of section `target_idx`, replacing its block with
/// `task_lines` (`R-WRITE-10`). `todo move` passes the block as it is; `todo promote --section`
/// passes the block after its own field edits, so both keep the same layout.
pub(crate) fn section_move_edits(
    doc: &Document,
    task: &Task,
    target_idx: usize,
    task_lines: Vec<String>,
) -> Vec<LineEdit> {
    let insert_at = doc.sections[target_idx].content_end;
    let lines = &doc.line_doc.lines;

    let mut inserted = Vec::with_capacity(task_lines.len() + 1);
    if insert_at > 0 && !lines[insert_at - 1].text.trim().is_empty() {
        inserted.push(String::new());
    }
    inserted.extend(task_lines);

    // Delete the task block plus one adjacent blank line when safe (without overlapping `insert_at`).
    let mut del_start = task.block_range.start;
    let mut del_end = task.block_range.end;
    if del_end < lines.len() && lines[del_end].text.trim().is_empty() && del_end + 1 != insert_at {
        del_end += 1;
    } else if del_start > 0
        && lines[del_start - 1].text.trim().is_empty()
        && del_start - 1 != insert_at
    {
        del_start -= 1;
    }

    let mut edits = vec![
        LineEdit::remove(del_start..del_end),
        LineEdit::insert(insert_at, inserted),
    ];
    edits.sort_by_key(|e| e.range.start);
    edits
}

/// Runs `todo move <task> --section <name>`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let cfg = ctx.load_config()?;
    let cfg_clone = cfg.clone();

    let res = edit_doc_and_write(
        ctx,
        &cfg,
        META.name,
        out,
        |doc, archive, label, agent, now| {
            let task = resolve_task_in_doc(doc, &args.task)?;
            check_task_writable(doc, task, label, archive)?;

            let id_str = cfg.id_config.format_id(task.id);
            let ownership = check_ownership(task, TaskCommand::Set, agent, ctx.force, &id_str)?;

            if doc.sections.is_empty() {
                return Err(TodoError::new(
                    ErrorCode::UnknownSection,
                    "TODO.md has no `## ` sections",
                ));
            }
            let target_idx = find_section(doc, &args.section)?;

            if task.section == Some(target_idx) {
                let task_json = build_task_json(doc, task, archive, false, now, &cfg_clone);
                return Ok(DocWritePlan::Unchanged((id_str, task_json)));
            }

            let mut task_lines: Vec<String> = doc.line_doc.lines[task.block_range.clone()]
                .iter()
                .map(|l| l.text.clone())
                .collect();

            if let OwnershipDecision::Forced { previous_owner } = ownership {
                let date = OpsConfig::format_note_date(now);
                task_lines.push(format!(
                    "- note ({date}, {agent}): {}",
                    forced_note_text("move", &previous_owner)
                ));
            }

            let edits = section_move_edits(doc, task, target_idx, task_lines);
            let task_id = task.id;
            let arch_owned = archive.cloned();
            let cfg_owned = cfg_clone.clone();
            let err_id = id_str.clone();
            let intent = TaskChangeIntent::modify_one(task_id);

            Ok(DocWritePlan::Edits {
                edits,
                intent,
                build_value: Box::new(move |new_doc| {
                    let moved = new_doc.task_by_id(task_id).ok_or_else(|| {
                        TodoError::new(
                            ErrorCode::WriteFailed,
                            format!("moved task {err_id} missing after verification"),
                        )
                    })?;
                    let task_json = build_task_json(
                        new_doc,
                        moved,
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
        out.write_line(&format!("{}: move ok", res.value.0));
    }
    Ok(())
}
