//! `todo next` command: pick the highest-priority ready task, optionally claiming it atomically (T-023; R-READ-3, R-DEP-1).

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::cli::{CommandKind, CommandMeta, Context, write_milestone_chain};
use crate::doc::TaskChangeIntent;
use crate::error::{ErrorCode, Result, TodoError};
use crate::lock::current_time;
use crate::ops::{
    DocWritePlan, NextFilter, TaskJson, WhereClause, build_claim_edits, build_task_json,
    check_task_writable, edit_doc_with_found_and_state_and_write, load_for_read_with_found,
    plan_claim, select_next_task,
};
use crate::output::Output;

/// Metadata for the `next` command.
pub const META: CommandMeta = CommandMeta {
    name: "next",
    summary: "Pick the next ready task (optionally claiming it atomically)",
    kind: CommandKind::Read,
    details: "Selects the highest-priority ready task (open or stale claimed, with all dependencies done or dropped, excluding questions, ideas and milestones), breaking ties by file order. When `--agent` or `TODO_AGENT` is set, returns the agent's own existing claim first unless `--new` is passed. With `--claim`, picks and claims atomically inside a single write lock.",
    examples: &[
        "todo next",
        "todo next --agent agent-1",
        "todo next --claim --agent agent-1",
        "todo next --claim --new --section Backend --agent agent-1",
        "todo next --type bug --where area=cli",
        "todo next --claim --max-difficulty medium --agent agent-1",
    ],
};

/// Arguments for `todo next`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Atomically claim the selected task (requires `--agent` / `TODO_AGENT`).
    #[arg(long)]
    pub claim: bool,

    /// Skip tasks already claimed by you and pick the next ready task instead.
    #[arg(long)]
    pub new: bool,

    /// Restrict candidates to a section (ignores leading numbers and case).
    #[arg(long, value_name = "SECTION")]
    pub section: Option<String>,

    /// Restrict candidates to a task type.
    #[arg(long = "type", value_name = "TYPE")]
    pub task_type: Option<String>,

    /// Restrict candidates to a task area.
    #[arg(long, value_name = "AREA")]
    pub area: Option<String>,

    /// Restrict candidates to tasks with one of these difficulties (comma-separated; `R-WRITE-18`).
    #[arg(long, value_name = "LIST")]
    pub difficulty: Option<String>,

    /// Restrict candidates to tasks of this difficulty or easier (`R-WRITE-18`).
    #[arg(long = "max-difficulty", value_name = "VALUE")]
    pub max_difficulty: Option<String>,

    /// Filter by `key=value` predicate (repeatable, ANDed).
    #[arg(long = "where", value_name = "KEY=VALUE")]
    pub where_clauses: Vec<String>,
}

/// JSON payload emitted by `todo next`.
#[derive(Debug, Clone, Serialize)]
struct NextJson {
    task: Option<TaskJson>,
    claimed: bool,
    already_claimed: bool,
}

/// Runs `todo next`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let (found, cfg) = ctx.load_config_with_found()?;
    let where_clauses = args
        .where_clauses
        .iter()
        .map(|raw| WhereClause::parse(raw))
        .collect::<Result<Vec<_>>>()?;
    let difficulties = match args.difficulty.as_deref() {
        Some(raw) => Some(cfg.difficulty_list(raw)?),
        None => None,
    };
    let max_difficulty = match args.max_difficulty.as_deref() {
        Some(raw) => Some(cfg.canonical_difficulty(raw)?),
        None => None,
    };

    let payload = if args.claim {
        let _ = ctx.require_agent()?;
        let write_res = edit_doc_with_found_and_state_and_write(
            ctx,
            found,
            &cfg,
            META.name,
            out,
            |doc, archive, _state, label, agent, now| {
                let filter = NextFilter {
                    section: args.section.as_deref(),
                    task_type: args.task_type.as_deref(),
                    area: args.area.as_deref(),
                    difficulties: difficulties.as_deref(),
                    max_difficulty: max_difficulty.as_deref(),
                    where_clauses: &where_clauses,
                    agent: Some(agent),
                    ignore_own_claim: args.new,
                };
                let Some(sel) = select_next_task(doc, archive, &filter, now, &cfg) else {
                    return Ok(DocWritePlan::Unchanged(NextJson {
                        task: None,
                        claimed: false,
                        already_claimed: false,
                    }));
                };

                let task = sel.task;
                let task_id = task.id;
                let already_claimed = sel.already_theirs;
                check_task_writable(doc, task, label, archive)?;

                let plan = plan_claim(task, agent, ctx.force, now, &cfg)?;
                let (edits, _) = build_claim_edits(doc, task, &plan, ctx.session.as_deref(), now);
                if edits.is_empty() {
                    let task_json = build_task_json(doc, task, archive, false, now, &cfg);
                    return Ok(DocWritePlan::Unchanged(NextJson {
                        task: Some(task_json),
                        claimed: true,
                        already_claimed,
                    }));
                }

                let arch_owned = archive.cloned();
                let cfg_owned = cfg.clone();
                let id_str = cfg.id_config.format_id(task_id);
                let intent = TaskChangeIntent::modify_one(task_id);
                Ok(DocWritePlan::Edits {
                    edits,
                    intent,
                    build_value: Box::new(move |new_doc| {
                        let updated_task = new_doc.task_by_id(task_id).ok_or_else(|| {
                            TodoError::new(
                                ErrorCode::WriteFailed,
                                format!("claimed task {id_str} missing after verification"),
                            )
                        })?;
                        let task_json = build_task_json(
                            new_doc,
                            updated_task,
                            arch_owned.as_ref(),
                            false,
                            now,
                            &cfg_owned,
                        );
                        Ok(NextJson {
                            task: Some(task_json),
                            claimed: true,
                            already_claimed,
                        })
                    }),
                })
            },
        )?;
        write_res.value
    } else {
        let loaded = load_for_read_with_found(found, &cfg, false, true, out)?;
        let now = current_time();
        let filter = NextFilter {
            section: args.section.as_deref(),
            task_type: args.task_type.as_deref(),
            area: args.area.as_deref(),
            difficulties: difficulties.as_deref(),
            max_difficulty: max_difficulty.as_deref(),
            where_clauses: &where_clauses,
            agent: ctx.agent.as_deref(),
            ignore_own_claim: args.new,
        };
        match select_next_task(&loaded.doc, loaded.archive.as_ref(), &filter, now, &cfg) {
            Some(sel) => NextJson {
                task: Some(build_task_json(
                    &loaded.doc,
                    sel.task,
                    loaded.archive.as_ref(),
                    false,
                    now,
                    &cfg,
                )),
                claimed: false,
                already_claimed: sel.already_theirs,
            },
            None => NextJson {
                task: None,
                claimed: false,
                already_claimed: false,
            },
        }
    };

    if out.is_json() {
        out.emit_json_ok(META.name, &payload);
    } else if let Some(ref task) = payload.task {
        let prio = task.priority.as_deref().unwrap_or("-");
        let suffix = match (payload.already_claimed, payload.claimed) {
            (true, true) => " [already yours; claim refreshed]",
            (true, false) => " [already claimed by you]",
            (false, true) => " [claimed]",
            (false, false) => "",
        };
        let line = match task.owner.as_deref() {
            Some(owner) => format!(
                "{} \u{00B7} {} \u{00B7} {} \u{00B7} {} \u{00B7} {}{}",
                task.id, prio, task.status, owner, task.title, suffix
            ),
            None => format!(
                "{} \u{00B7} {} \u{00B7} {} \u{00B7} {}{}",
                task.id, prio, task.status, task.title, suffix
            ),
        };
        out.write_line(&line);
        write_milestone_chain(out, &task.id, &task.milestones);
    } else {
        out.write_line("no ready tasks");
    }

    Ok(())
}
