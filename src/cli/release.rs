//! `todo release` command: release one task, all stale claims, or all claims of an agent (T-022, T-033; R-WRITE-3, R-CLAIM-3, R-CLAIM-6).

use std::collections::BTreeSet;

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::cli::{CommandKind, CommandMeta, Context, run_task_op, validate_agent_name};
use crate::doc::{Status, TaskChangeIntent, agent_names_equal};
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::{
    DocWritePlan, OpsConfig, TaskEditBuilder, TaskJson, TaskOp, build_task_json,
    check_task_writable, edit_doc_and_write, forced_note_text,
};
use crate::output::Output;

/// Metadata for the `release` command.
pub const META: CommandMeta = CommandMeta {
    name: "release",
    summary: "Release a claimed task, all stale claims, all claims of an agent or of a session",
    kind: CommandKind::Write,
    details: "Gives a task back: a `claimed` task becomes `open`, while a `blocked` task stays `blocked`. Removes the owner and claim time. With `--stale`, releases every stale claim in one locked step. With `--all`, releases all claims of the calling agent (or of `--owner <NAME>` with `--force`). With `--session <ID>`, releases every claim tagged with that agent session and adds the note `released — session <ID> ended`.",
    examples: &[
        "todo release T-012 --agent agent-1",
        "todo release T-012 --agent coordinator --force",
        "todo release --stale --agent coordinator",
        "todo release --all --agent agent-1",
        "todo release --all --owner agent-1 --agent coordinator --force",
        "todo release --session 6f0c1c2e-1111-4222-8333-944455556666 --agent coordinator",
    ],
};

/// Arguments for `todo release`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Task ID or unique title substring (omit when using `--stale` or `--all`).
    #[arg(value_name = "TASK")]
    pub task: Option<String>,

    /// Release every stale claim in one locked step.
    #[arg(long)]
    pub stale: bool,

    /// Release all claims held by an agent in one locked step.
    #[arg(long)]
    pub all: bool,

    /// Target agent whose claims to release with `--all` (requires `--force` if different from `--agent`).
    #[arg(long, value_name = "NAME")]
    pub owner: Option<String>,
}

#[derive(Serialize)]
struct BulkReleaseJson {
    released: Vec<TaskJson>,
    count: usize,
}

/// Runs `todo release`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    if let Some(session) = ctx.session_arg.as_deref() {
        if args.stale || args.all || args.task.is_some() || args.owner.is_some() {
            return Err(TodoError::new(
                ErrorCode::Usage,
                "--session releases every claim of that session and cannot be combined with a task, --stale, --all or --owner",
            ));
        }
        let released = release_session_claims(ctx, out, session)?;
        emit_bulk_result(&released, out);
        return Ok(());
    }

    if args.stale && args.all {
        return Err(TodoError::new(
            ErrorCode::Usage,
            "cannot combine --stale and --all",
        ));
    }

    if args.stale {
        if args.task.is_some() || args.owner.is_some() {
            return Err(TodoError::new(
                ErrorCode::Usage,
                "--stale cannot be combined with a task argument or --owner",
            ));
        }
        return run_bulk_stale(ctx, out);
    }

    if args.all {
        let target_owner = match args.owner.as_deref().or(args.task.as_deref()) {
            Some(raw) => Some(validate_agent_name(raw)?.to_owned()),
            None => None,
        };
        return run_bulk_all(target_owner, ctx, out);
    }

    let Some(task_arg) = args.task else {
        return Err(TodoError::new(
            ErrorCode::Usage,
            "missing task: pass <TASK>, --stale, or --all",
        ));
    };

    run_task_op(ctx, out, META.name, &task_arg, &TaskOp::Release)
}

fn run_bulk_stale(ctx: &Context, out: &mut Output) -> Result<()> {
    let cfg = ctx.load_config()?;
    let cfg_clone = cfg.clone();

    let res = edit_doc_and_write(
        ctx,
        &cfg,
        META.name,
        out,
        |doc, archive, label, agent, now| {
            let stale_tasks: Vec<_> = doc
                .tasks
                .iter()
                .filter(|t| cfg.is_claim_stale(t, now))
                .collect();

            if stale_tasks.is_empty() {
                return Ok(DocWritePlan::Unchanged(Vec::new()));
            }

            let mut all_edits = Vec::new();
            let mut modified_ids = BTreeSet::new();
            let date = OpsConfig::format_note_date(now);

            for task in &stale_tasks {
                check_task_writable(doc, task, label, archive)?;
                modified_ids.insert(task.id);

                let mut builder = TaskEditBuilder::new(doc, task);
                builder.set_status(Status::Open);
                builder.set_owner(None);
                builder.remove_claimed();
                let prev = task.owner.as_deref().unwrap_or("unknown");
                builder.add_note(
                    &date,
                    agent,
                    &format!("released stale claim (was held by {prev})"),
                );
                let (edits, _) = builder.build();
                all_edits.extend(edits);
            }

            all_edits.sort_by_key(|e| e.range.start);
            let intent = TaskChangeIntent {
                modified_ids: modified_ids.clone(),
                ..TaskChangeIntent::default()
            };
            let arch_owned = archive.cloned();
            let cfg_owned = cfg_clone.clone();

            Ok(DocWritePlan::Edits {
                edits: all_edits,
                intent,
                build_value: Box::new(move |new_doc| {
                    let mut released = Vec::with_capacity(modified_ids.len());
                    for id in modified_ids {
                        if let Some(t) = new_doc.task_by_id(id) {
                            released.push(build_task_json(
                                new_doc,
                                t,
                                arch_owned.as_ref(),
                                false,
                                now,
                                &cfg_owned,
                            ));
                        }
                    }
                    Ok(released)
                }),
            })
        },
    )?;

    emit_bulk_result(&res.value, out);
    Ok(())
}

fn run_bulk_all(target_owner: Option<String>, ctx: &Context, out: &mut Output) -> Result<()> {
    let caller = ctx.require_agent()?.to_owned();
    let target = target_owner.unwrap_or_else(|| caller.clone());
    let is_other = !agent_names_equal(&target, &caller);
    if is_other && !ctx.force {
        return Err(TodoError::new(
            ErrorCode::NotOwner,
            format!("cannot release claims of '{target}' without --force"),
        )
        .with_owner(target));
    }

    let cfg = ctx.load_config()?;
    let cfg_clone = cfg.clone();

    let res = edit_doc_and_write(
        ctx,
        &cfg,
        META.name,
        out,
        |doc, archive, label, agent, now| {
            let matching: Vec<_> = doc
                .tasks
                .iter()
                .filter(|t| {
                    matches!(t.status, Status::Claimed | Status::Blocked)
                        && t.owner
                            .as_deref()
                            .is_some_and(|o| agent_names_equal(o, &target))
                })
                .collect();

            if matching.is_empty() {
                return Ok(DocWritePlan::Unchanged(Vec::new()));
            }

            let mut all_edits = Vec::new();
            let mut modified_ids = BTreeSet::new();
            let date = OpsConfig::format_note_date(now);

            for task in &matching {
                check_task_writable(doc, task, label, archive)?;
                modified_ids.insert(task.id);

                let mut builder = TaskEditBuilder::new(doc, task);
                if task.status == Status::Claimed {
                    builder.set_status(Status::Open);
                }
                builder.set_owner(None);
                builder.remove_claimed();
                if is_other {
                    builder.add_note(&date, agent, &forced_note_text("release", &target));
                }
                let (edits, _) = builder.build();
                all_edits.extend(edits);
            }

            all_edits.sort_by_key(|e| e.range.start);
            let intent = TaskChangeIntent {
                modified_ids: modified_ids.clone(),
                ..TaskChangeIntent::default()
            };
            let arch_owned = archive.cloned();
            let cfg_owned = cfg_clone.clone();

            Ok(DocWritePlan::Edits {
                edits: all_edits,
                intent,
                build_value: Box::new(move |new_doc| {
                    let mut released = Vec::with_capacity(modified_ids.len());
                    for id in modified_ids {
                        if let Some(t) = new_doc.task_by_id(id) {
                            released.push(build_task_json(
                                new_doc,
                                t,
                                arch_owned.as_ref(),
                                false,
                                now,
                                &cfg_owned,
                            ));
                        }
                    }
                    Ok(released)
                }),
            })
        },
    )?;

    emit_bulk_result(&res.value, out);
    Ok(())
}

/// Releases every claimed or blocked task whose `session` field is `session`, in one locked step
/// (`R-CLAIM-7`): the owner, claim time and session are removed (a claimed task becomes `open`) and
/// the note `released — session <id> ended` is added. Needs `ctx.agent` for the notes.
///
/// Shared by `todo release --session` and the `SessionEnd` hook command.
pub(crate) fn release_session_claims(
    ctx: &Context,
    out: &mut Output,
    session: &str,
) -> Result<Vec<TaskJson>> {
    let cfg = ctx.load_config()?;
    let cfg_clone = cfg.clone();

    let res = edit_doc_and_write(
        ctx,
        &cfg,
        META.name,
        out,
        |doc, archive, label, agent, now| {
            let matching: Vec<_> = doc
                .tasks
                .iter()
                .filter(|t| {
                    matches!(t.status, Status::Claimed | Status::Blocked)
                        && t.field_value("session") == Some(session)
                })
                .collect();

            if matching.is_empty() {
                return Ok(DocWritePlan::Unchanged(Vec::new()));
            }

            let mut all_edits = Vec::new();
            let mut modified_ids = BTreeSet::new();
            let date = OpsConfig::format_note_date(now);

            for task in &matching {
                check_task_writable(doc, task, label, archive)?;
                modified_ids.insert(task.id);

                let mut builder = TaskEditBuilder::new(doc, task);
                if task.status == Status::Claimed {
                    builder.set_status(Status::Open);
                }
                builder.clear_owner_if_present();
                builder.remove_claimed();
                builder.add_note(
                    &date,
                    agent,
                    &format!("released — session {session} ended"),
                );
                let (edits, _) = builder.build();
                all_edits.extend(edits);
            }

            all_edits.sort_by_key(|e| e.range.start);
            let intent = TaskChangeIntent {
                modified_ids: modified_ids.clone(),
                ..TaskChangeIntent::default()
            };
            let arch_owned = archive.cloned();
            let cfg_owned = cfg_clone.clone();

            Ok(DocWritePlan::Edits {
                edits: all_edits,
                intent,
                build_value: Box::new(move |new_doc| {
                    let mut released = Vec::with_capacity(modified_ids.len());
                    for id in modified_ids {
                        if let Some(t) = new_doc.task_by_id(id) {
                            released.push(build_task_json(
                                new_doc,
                                t,
                                arch_owned.as_ref(),
                                false,
                                now,
                                &cfg_owned,
                            ));
                        }
                    }
                    Ok(released)
                }),
            })
        },
    )?;

    Ok(res.value)
}

fn emit_bulk_result(released: &[TaskJson], out: &mut Output) {
    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &BulkReleaseJson {
                count: released.len(),
                released: released.to_vec(),
            },
        );
    } else if released.is_empty() {
        out.write_line("no matching claims to release");
    } else {
        for t in released {
            out.write_line(&format!("{}: released", t.id));
        }
    }
}
