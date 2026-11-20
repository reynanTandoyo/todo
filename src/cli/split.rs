//! `todo split` command: turn a task into a milestone with child tasks, in one locked write (T-156;
//! R-SPLIT-1, R-WRITE-13, R-MILE-1).

use std::collections::{BTreeSet, HashMap};

use clap::Args as ClapArgs;
use serde::Serialize;
use serde_json::Value;

use crate::cli::add::{
    ValidatedBatchEntry, at_batch_line, check_batch_titles, entry_lines, parse_batch_entries,
    read_batch_input, resolve_batch_depends,
};
use crate::cli::{CommandKind, CommandMeta, Context, resolve_opt_text_arg};
use crate::doc::{EntryKind, LineEdit, Status, Task, TaskChangeIntent, TaskId, sanitize_trimmed_value};
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::{
    DocWritePlan, OpsConfig, OwnershipDecision, TaskCommand, TaskEditBuilder, TaskJson,
    build_task_json, check_ownership, check_task_writable, edit_doc_with_found_and_state_and_write,
    forced_note_text, highest_task_number, resolve_task_in_doc,
};
use crate::output::Output;
use crate::state::IdsState;

/// Metadata for the `split` command.
pub const META: CommandMeta = CommandMeta {
    name: "split",
    summary: "Turn a task into a milestone with child tasks",
    kind: CommandKind::Write,
    details: "Replaces one task with several smaller ones in one locked write, all or nothing. The children are JSON lines in the shape of `add --from`, read from `--from <file|->`: each is added right after the task, with the task as its milestone, and may depend on another child with \"@ref\". The task becomes a milestone whose `goal` is `--goal`, or else its `accept`; without either the split is refused (MISSING_FIELD). It loses `difficulty`, `files`, `read`, `steps`, `verify`, `final`, `attempts`, `verified` and its claim, a blocked one becomes open, and a note names the children. Tasks that depended on it now wait for the whole milestone. Refuses a milestone, question, idea, done or dropped task and a task claimed by another agent without `--force`; a child that names a milestone or a section, or is a question or an idea, is refused. `--dry-run` prints the diff.",
    examples: &[
        "todo split T-020 --from split-T-020.jsonl --goal \"the renderer draws the scene\" --agent planner",
        "todo split T-020 --from - --dry-run --agent planner",
    ],
};

/// Fields a task loses when it becomes a milestone (R-SPLIT-1, `docs/format.md` §16): the ones that
/// belong to tasks. Its claim (`owner`, `claimed`, `session`) goes as well.
const FIELDS_OF_A_TASK: &[&str] = &[
    "difficulty",
    "files",
    "read",
    "steps",
    "verify",
    "final",
    "attempts",
    "verified",
];

/// Arguments for `todo split`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Task to split: its ID or title.
    #[arg(value_name = "TASK")]
    pub task: String,

    /// The child tasks as JSON lines, in a file or `-` (standard input); the shape of `add --from`.
    #[arg(long, value_name = "FILE|-")]
    pub from: String,

    /// Goal of the milestone the task becomes; without it the task's `accept` is the goal.
    #[arg(long, value_name = "TEXT")]
    pub goal: Option<String>,
}

/// The output of a split: the task as a milestone, and its children in the order given.
#[derive(Serialize)]
struct SplitJson {
    /// The task that became a milestone.
    task: TaskJson,
    /// IDs of the children, in order.
    ids: Vec<String>,
    /// The children, as `add --from` reports them.
    added: Vec<TaskJson>,
    /// Number of children.
    count: usize,
}

/// Runs `todo split <task> --from <file|->`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let (found, cfg) = ctx.load_config_with_found()?;
    let raw = read_batch_input(&args.from)?;
    let entries = parse_batch_entries(&raw, None, &cfg)?;
    check_children(&entries)?;
    let goal_flag = resolve_opt_text_arg(args.goal.as_deref())?
        .map(|g| sanitize_trimmed_value(&g))
        .filter(|g| !g.is_empty());

    let res = edit_doc_with_found_and_state_and_write(
        ctx,
        found,
        &cfg,
        META.name,
        out,
        |doc, archive, state, label, agent, now| {
            let task = resolve_task_in_doc(doc, &args.task)?;
            check_task_writable(doc, task, label, archive)?;
            let id_str = cfg.id_config.format_id(task.id);
            check_splittable(task, &id_str)?;
            let ownership = check_ownership(task, TaskCommand::Split, agent, ctx.force, &id_str)?;
            let goal = goal_flag
                .clone()
                .or_else(|| task.accept.clone())
                .ok_or_else(|| {
                    TodoError::new(
                        ErrorCode::MissingField,
                        format!(
                            "missing goal for the milestone {id_str}: pass --goal <TEXT>, or give {id_str} an accept line"
                        ),
                    )
                    .with_task(id_str.clone())
                    .with_detail("field", Value::String("goal".to_string()))
                })?;
            check_batch_titles(doc, &entries, &cfg)?;

            // The children take the next IDs after the highest one of TODO.md, the archive and
            // ids.json (R-ID-2), in the order given.
            let max_state = state.read_ids()?.map_or(0, |s| s.highest_id);
            let mut next_num = highest_task_number(doc, archive, max_state);
            let mut child_ids: Vec<(TaskId, String)> = Vec::with_capacity(entries.len());
            let mut ref_to_id: HashMap<String, String> = HashMap::new();
            for entry in &entries {
                next_num = next_num
                    .checked_add(1)
                    .ok_or_else(|| TodoError::new(ErrorCode::WriteFailed, "task ID overflow"))?;
                let child = TaskId(next_num);
                let child_text = cfg.id_config.format_id(child);
                if let Some(ref r) = entry.ref_name {
                    ref_to_id.insert(r.clone(), child_text.clone());
                }
                child_ids.push((child, child_text));
            }

            // Every check is done before ids.json is written: a refusal leaves no gap in the IDs.
            let date = OpsConfig::format_note_date(now);
            let mut child_lines = Vec::new();
            for (entry, (_, child_text)) in entries.iter().zip(child_ids.iter()) {
                let depends = resolve_batch_depends(&entry.depends_tokens, &ref_to_id)
                    .map_err(|e| at_batch_line(e, entry.line_no))?;
                child_lines.extend(entry_lines(
                    entry,
                    Some(&id_str),
                    depends.as_deref(),
                    child_text,
                    &date,
                    agent,
                ));
            }
            if !ctx.dry_run {
                state.write_ids(&IdsState {
                    highest_id: next_num,
                })?;
            }

            // The task itself: a milestone with its goal, without what only a task has and without
            // its claim. The note names the children; the children follow its block.
            let ids: Vec<String> = child_ids.iter().map(|(_, text)| text.clone()).collect();
            let mut builder = TaskEditBuilder::new(doc, task);
            if let OwnershipDecision::Forced { previous_owner } = &ownership {
                builder.add_note(
                    &date,
                    agent,
                    &forced_note_text(META.name, previous_owner),
                );
            }
            if task.status != Status::Open {
                builder.set_status(Status::Open);
            }
            builder.clear_owner_if_present();
            builder.remove_claimed();
            builder.set_field("type", "milestone");
            builder.set_field("goal", &goal);
            for key in FIELDS_OF_A_TASK {
                builder.unset_field(key);
            }
            builder.add_note(&date, agent, &format!("split into {}", ids.join(", ")));
            let (mut edits, _) = builder.build();
            // Appended after the note, which is also at the end of the block: applied in this order.
            edits.push(LineEdit::insert(task.block_range.end, child_lines));

            let intent = TaskChangeIntent {
                added_ids: child_ids.iter().map(|(id, _)| *id).collect::<BTreeSet<_>>(),
                ..TaskChangeIntent::modify_one(task.id)
            };
            let task_id = task.id;
            let id_for_err = id_str.clone();
            let arch_owned = archive.cloned();
            let cfg_owned = cfg.clone();
            Ok(DocWritePlan::Edits {
                edits,
                intent,
                build_value: Box::new(move |new_doc| {
                    let split_task = new_doc.task_by_id(task_id).ok_or_else(|| {
                        TodoError::new(
                            ErrorCode::WriteFailed,
                            format!("split task {id_for_err} missing after verification"),
                        )
                    })?;
                    let task_json = build_task_json(
                        new_doc,
                        split_task,
                        arch_owned.as_ref(),
                        false,
                        now,
                        &cfg_owned,
                    );
                    let mut added = Vec::with_capacity(child_ids.len());
                    for (id, text) in &child_ids {
                        let child = new_doc.task_by_id(*id).ok_or_else(|| {
                            TodoError::new(
                                ErrorCode::WriteFailed,
                                format!("added task {text} missing after verification"),
                            )
                        })?;
                        added.push(build_task_json(
                            new_doc,
                            child,
                            arch_owned.as_ref(),
                            false,
                            now,
                            &cfg_owned,
                        ));
                    }
                    Ok(SplitJson {
                        task: task_json,
                        count: added.len(),
                        ids,
                        added,
                    })
                }),
            })
        },
    )?;

    if out.is_json() {
        out.emit_json_ok(META.name, &res.value);
    } else {
        out.write_line(&format!("{}: split ok", res.value.task.id));
        out.write_line(&format!("split into {}", res.value.ids.join(", ")));
    }
    Ok(())
}

/// Refuses a task that cannot become a milestone (R-SPLIT-1): a milestone, question or idea is
/// `BAD_VALUE`, and a done or dropped task is `BAD_TRANSITION`.
fn check_splittable(task: &Task, id_str: &str) -> Result<()> {
    let kind = task.kind();
    if kind != EntryKind::Task {
        return Err(TodoError::new(
            ErrorCode::BadValue,
            format!("task {id_str} is a {}; only a task can be split", kind.as_str()),
        )
        .with_task(id_str));
    }
    if task.status.is_finished() {
        return Err(TodoError::new(
            ErrorCode::BadTransition,
            format!(
                "task {id_str} is {}; only an open or blocked task can be split",
                task.status.as_str()
            ),
        )
        .with_task(id_str));
    }
    Ok(())
}

/// Refuses a child the split cannot place (R-SPLIT-1, `BAD_VALUE`, before anything is locked): a
/// child takes the task as its milestone, so it names no milestone and no section, and only a task
/// or a milestone can be a child.
fn check_children(entries: &[ValidatedBatchEntry]) -> Result<()> {
    for entry in entries {
        if entry.milestone.is_some() {
            return Err(at_batch_line(
                TodoError::new(
                    ErrorCode::BadValue,
                    "a child names no milestone: the task being split is its milestone",
                ),
                entry.line_no,
            ));
        }
        if entry.section.is_some() {
            return Err(at_batch_line(
                TodoError::new(
                    ErrorCode::BadValue,
                    "a child takes the section of the task being split; remove \"section\"",
                ),
                entry.line_no,
            ));
        }
        let kind = EntryKind::from_type(entry.task_type.as_deref());
        if !matches!(kind, EntryKind::Task | EntryKind::Milestone) {
            return Err(at_batch_line(
                TodoError::new(
                    ErrorCode::BadValue,
                    format!(
                        "a {} is not part of the milestone tree; only tasks and milestones can be children",
                        kind.as_str()
                    ),
                ),
                entry.line_no,
            ));
        }
    }
    Ok(())
}
