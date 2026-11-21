//! `todo next` command: pick the highest-priority ready task, optionally claiming it atomically (T-023; R-READ-3, R-DEP-1).

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::cli::brief::write_brief;
use crate::cli::{CommandKind, CommandMeta, Context, write_milestone_chain};
use crate::doc::TaskChangeIntent;
use crate::error::{ErrorCode, Result, TodoError};
use crate::lock::current_time;
use crate::ops::milestone::resolve_milestone;
use crate::ops::{
    BriefJson, DocWritePlan, NextFilter, TaskJson, WhereClause, build_brief, build_claim_edits,
    build_task_json, check_task_writable, edit_doc_keeping_documents, load_for_read_with_found,
    plan_claim, select_next_task,
};
use crate::output::Output;

/// Metadata for the `next` command.
pub const META: CommandMeta = CommandMeta {
    name: "next",
    summary: "Pick the next ready task (optionally claiming it atomically)",
    kind: CommandKind::Read,
    details: "Selects the highest-priority ready task (open or stale claimed, with all dependencies done or dropped, the dependencies of every milestone above it too, and no milestone above it blocked, done or dropped; a task at or below the target level (`target_difficulty`, by default the lowest level) only when its detail is complete (R-DETAIL-2), and only when no open question is about it; excluding questions, ideas and milestones, tasks without a usable level (no difficulty, or one that is not configured; R-LEVEL-1), and skipping a task whose `files` overlap a task another agent holds). Ties go to tasks in a milestone where work has started, then to file order. `--milestone <task>` keeps only the tasks below that milestone, at any depth. When `--agent` or `TODO_AGENT` is set, returns the agent's own existing claim first unless `--new` is passed. With `--claim`, picks and claims atomically inside a single write lock. With `--brief`, prints the work order of the picked task after the claim, as `todo brief` does (R-DETAIL-3).",
    examples: &[
        "todo next",
        "todo next --agent agent-1",
        "todo next --claim --agent agent-1",
        "todo next --claim --new --section Backend --agent agent-1",
        "todo next --type bug --where area=cli",
        "todo next --claim --max-difficulty medium --agent agent-1",
        "todo next --milestone T-010 --claim --agent agent-1",
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

    /// Only tasks below this milestone, at any depth (an ID or a title; `R-MILE-6`).
    #[arg(long, value_name = "TASK")]
    pub milestone: Option<String>,

    /// Print the work order of the picked task after the claim, as `todo brief` does (R-DETAIL-3).
    #[arg(long)]
    pub brief: bool,
}

/// JSON payload emitted by `todo next`.
#[derive(Debug, Clone, Serialize)]
struct NextJson {
    task: Option<TaskJson>,
    claimed: bool,
    already_claimed: bool,
    /// The work order of the task, with `--brief` (R-DETAIL-3); absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    brief: Option<BriefJson>,
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
        let write_res = edit_doc_keeping_documents(
            ctx,
            found,
            &cfg,
            META.name,
            out,
            |doc, archive, _state, label, agent, now| {
                let milestone = match args.milestone.as_deref() {
                    Some(raw) => Some(resolve_milestone(doc, raw)?.id),
                    None => None,
                };
                let filter = NextFilter {
                    section: args.section.as_deref(),
                    task_type: args.task_type.as_deref(),
                    area: args.area.as_deref(),
                    difficulties: difficulties.as_deref(),
                    max_difficulty: max_difficulty.as_deref(),
                    where_clauses: &where_clauses,
                    agent: Some(agent),
                    ignore_own_claim: args.new,
                    milestone,
                };
                let Some(sel) = select_next_task(doc, archive, &filter, now, &cfg) else {
                    return Ok(DocWritePlan::Unchanged(NextJson {
                        task: None,
                        claimed: false,
                        already_claimed: false,
                        brief: None,
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
                    let brief = args
                        .brief
                        .then(|| build_brief(doc, archive, task, &cfg, Some(agent)));
                    return Ok(DocWritePlan::Unchanged(NextJson {
                        task: Some(task_json),
                        claimed: true,
                        already_claimed,
                        brief,
                    }));
                }

                let arch_owned = archive.cloned();
                let cfg_owned = cfg.clone();
                let agent_owned = agent.to_string();
                let want_brief = args.brief;
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
                        // The work order is built from the file as the claim left it.
                        let brief = want_brief.then(|| {
                            build_brief(
                                new_doc,
                                arch_owned.as_ref(),
                                updated_task,
                                &cfg_owned,
                                Some(&agent_owned),
                            )
                        });
                        Ok(NextJson {
                            task: Some(task_json),
                            claimed: true,
                            already_claimed,
                            brief,
                        })
                    }),
                })
            },
        )?;
        write_res.value
    } else {
        let loaded = load_for_read_with_found(found, &cfg, false, true, out)?;
        let now = current_time();
        let milestone = match args.milestone.as_deref() {
            Some(raw) => Some(resolve_milestone(&loaded.doc, raw)?.id),
            None => None,
        };
        let filter = NextFilter {
            section: args.section.as_deref(),
            task_type: args.task_type.as_deref(),
            area: args.area.as_deref(),
            difficulties: difficulties.as_deref(),
            max_difficulty: max_difficulty.as_deref(),
            where_clauses: &where_clauses,
            agent: ctx.agent.as_deref(),
            ignore_own_claim: args.new,
            milestone,
        };
        let payload = match select_next_task(&loaded.doc, loaded.archive.as_ref(), &filter, now, &cfg)
        {
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
                brief: args.brief.then(|| {
                    build_brief(
                        &loaded.doc,
                        loaded.archive.as_ref(),
                        sel.task,
                        &cfg,
                        ctx.agent.as_deref(),
                    )
                }),
            },
            None => NextJson {
                task: None,
                claimed: false,
                already_claimed: false,
                brief: None,
            },
        };
        // Read-only: the work order is built, so the documents are not needed any more (T-186).
        loaded.leak_for_exit();
        payload
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
        match &payload.brief {
            Some(brief) => write_brief(out, brief),
            None => write_milestone_chain(out, &task.id, &task.milestones),
        }
    } else {
        out.write_line("no ready tasks");
    }

    Ok(())
}
