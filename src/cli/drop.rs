//! `todo drop` command: mark a task dropped with a reason note (T-025; R-WRITE-6, R-WRITE-9, R-MILE-2).

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::cli::{
    CommandKind, CommandMeta, Context, edit_one_task, report_task_with_ready, resolve_text_arg,
};
use crate::doc::{Document, LineEdit, TaskChangeIntent, TaskId};
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::{
    DocWritePlan, OpEnv, OpsConfig, TaskJson, TaskOp, build_task_json, check_task_writable,
    edit_doc_with_found_and_state_and_write, resolve_task_in_doc,
};
use crate::output::Output;

/// Metadata for the `drop` command.
pub const META: CommandMeta = CommandMeta {
    name: "drop",
    summary: "Mark a task dropped with a reason",
    kind: CommandKind::Write,
    details: "Marks an `open`, `claimed`, or `blocked` task as `dropped`, removes any `claimed` timestamp, and appends a `dropped — <reason>` note. Requires `--note`. A milestone with unfinished entries below it is refused; with `--tree` the milestone and every entry below it that isn't done or dropped are dropped in one locked step, each with a `dropped with <milestone> — <reason>` note (another agent's claim needs `--force`).",
    examples: &[
        "todo drop T-012 --agent agent-1 --note \"superseded by T-020\"",
        "todo drop T-012 --agent coordinator --force --note \"out of scope\"",
        "todo drop T-010 --tree --agent coordinator --note \"scope cut\"",
    ],
};

/// Arguments for `todo drop`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Task ID (e.g. `T-012`, `t12`, or `12`).
    #[arg(value_name = "TASK")]
    pub task: String,

    /// Reason the task is dropped (required).
    #[arg(long, value_name = "TEXT")]
    pub note: String,

    /// Drop a milestone together with every entry below it that isn't done or dropped (R-MILE-2).
    #[arg(long)]
    pub tree: bool,
}

/// The JSON of `drop --tree`: the milestone as it is after the change, and the entries dropped below it.
#[derive(Serialize)]
struct DropTreeJson {
    #[serde(flatten)]
    task: TaskJson,
    dropped: Vec<String>,
}

/// Runs `todo drop <task> --note <reason>`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let resolved_note = resolve_text_arg(&args.note)?;
    let reason = resolved_note.trim();
    if reason.is_empty() {
        return Err(TodoError::new(
            ErrorCode::Usage,
            "--note reason cannot be empty for drop",
        ));
    }

    if args.tree {
        return run_tree(&args.task, reason, ctx, out);
    }

    let cfg = ctx.load_config()?;
    let op = TaskOp::Drop { reason };
    let res = edit_one_task(ctx, out, META.name, &args.task, &op, &cfg, None)?;
    report_task_with_ready(out, META.name, &res, false);
    Ok(())
}

/// `todo drop <milestone> --tree --note <reason>`: one locked write for the milestone and every
/// entry below it that isn't done or dropped (R-MILE-2).
fn run_tree(task_input: &str, reason: &str, ctx: &Context, out: &mut Output) -> Result<()> {
    let (found, cfg) = ctx.load_config_with_found()?;
    let res = edit_doc_with_found_and_state_and_write(
        ctx,
        found,
        &cfg,
        META.name,
        out,
        |doc, archive, _state, label, agent, now| {
            let env = OpEnv {
                cfg: &cfg,
                agent,
                force: ctx.force,
                now,
                session: ctx.session.as_deref(),
            };
            let (edits, touched, dropped) =
                plan_tree(doc, archive, label, &cfg, &env, task_input, reason)?;
            let milestone_id = touched[0];
            let arch_owned = archive.cloned();
            let cfg_owned = cfg.clone();
            Ok(DocWritePlan::Edits {
                edits,
                intent: TaskChangeIntent::modify_many(touched),
                build_value: Box::new(move |new_doc| {
                    let milestone = new_doc.task_by_id(milestone_id).ok_or_else(|| {
                        TodoError::new(
                            ErrorCode::WriteFailed,
                            "dropped milestone missing after verification",
                        )
                    })?;
                    let task = build_task_json(
                        new_doc,
                        milestone,
                        arch_owned.as_ref(),
                        false,
                        now,
                        &cfg_owned,
                    );
                    Ok((task, dropped))
                }),
            })
        },
    )?;

    let (task, dropped) = res.value;
    if out.is_json() {
        out.emit_json_ok(META.name, &DropTreeJson { task, dropped });
    } else {
        out.write_line(&format!("{}: drop ok", task.id));
        if !dropped.is_empty() {
            out.write_line(&format!("dropped below {}: {}", task.id, dropped.join(", ")));
        }
    }
    Ok(())
}

/// Plans the edits of `drop --tree`: the milestone first, then each unfinished entry below it in
/// file order. Returns the edits, the IDs they touch (milestone first) and the IDs dropped below.
fn plan_tree(
    doc: &Document,
    archive: Option<&Document>,
    label: &str,
    cfg: &OpsConfig,
    env: &OpEnv<'_>,
    task_input: &str,
    reason: &str,
) -> Result<(Vec<LineEdit>, Vec<TaskId>, Vec<String>)> {
    let milestone = resolve_task_in_doc(doc, task_input)?;
    let milestone_id = cfg.id_config.format_id(milestone.id);
    if !milestone.is_milestone() {
        return Err(TodoError::new(
            ErrorCode::BadValue,
            format!(
                "{milestone_id} is not a milestone; --tree drops a milestone and the entries below it"
            ),
        )
        .with_task(milestone_id));
    }
    check_task_writable(doc, milestone, label, archive)?;

    let mut edits = TaskOp::DropTree { reason }.build_edits(doc, milestone, env)?;
    let mut touched = vec![milestone.id];
    let mut dropped = Vec::new();
    for entry in doc.entries_below(milestone.id) {
        if entry.status.is_finished() {
            continue;
        }
        check_task_writable(doc, entry, label, archive)?;
        let within = TaskOp::DropWithin {
            milestone: &milestone_id,
            reason,
        };
        edits.extend(within.build_edits(doc, entry, env)?);
        touched.push(entry.id);
        dropped.push(cfg.id_config.format_id(entry.id));
    }
    edits.sort_by_key(|e| (e.range.start, e.range.end));
    Ok((edits, touched, dropped))
}
