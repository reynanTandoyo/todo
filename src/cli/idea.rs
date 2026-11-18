//! `todo idea` command: record an idea in `idea_section`, where it is not work (R-KIND-1).

use clap::Args as ClapArgs;

use crate::cli::{CommandKind, CommandMeta, Context, resolve_text_arg};
use crate::doc::{Document, LineEdit, Status, TaskChangeIntent, TaskId, sanitize_trimmed_value};
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::{
    DocWritePlan, NewTask, OpsConfig, build_task_json, edit_doc_with_state_and_write,
    highest_task_number, matching_section_indices,
};
use crate::output::Output;
use crate::state::IdsState;

/// Metadata for the `idea` command.
pub const META: CommandMeta = CommandMeta {
    name: "idea",
    summary: "Record an idea, which is not work until promoted",
    kind: CommandKind::Write,
    details: "Adds a task of `type: idea` to the section named by `idea_section` (default `Ideas`, created at the end of `TODO.md` if missing). Ideas need no priority, and `next` never hands them out. `--about` names the task or milestone the idea is about; it must exist. `promote` turns an idea into a task.",
    examples: &[
        "todo idea \"Hot-reload shaders while the scene runs\" --agent agent-1",
        "todo idea \"Cache parsed files\" --about T-012 --agent agent-1",
    ],
};

/// Arguments for `todo idea`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Idea text (becomes the task title).
    #[arg(value_name = "IDEA")]
    pub idea: String,

    /// Task or milestone the idea is about (task ID).
    #[arg(long, value_name = "TASK")]
    pub about: Option<String>,
}

/// Runs `todo idea <text> [--about <task>]`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let raw_idea = resolve_text_arg(&args.idea)?;
    let clean_idea = sanitize_trimmed_value(&raw_idea);
    if clean_idea.is_empty() {
        return Err(TodoError::new(ErrorCode::BadValue, "idea cannot be empty"));
    }
    let cfg = ctx.load_config()?;
    let about_target = match args.about.as_deref() {
        Some(raw) => Some(cfg.id_config.parse_user_id(raw).ok_or_else(|| {
            TodoError::new(
                ErrorCode::BadValue,
                format!("--about expects a task ID such as T-002, not {raw:?}"),
            )
        })?),
        None => None,
    };
    let cfg_clone = cfg.clone();
    let res = edit_doc_with_state_and_write(
        ctx,
        &cfg,
        META.name,
        out,
        |doc, archive, state, _label, agent, now| {
            // Duplicate titles among active tasks are refused, as for `add` and `ask` (R-WRITE-1).
            if let Some(existing) = doc.tasks.iter().find(|t| {
                !matches!(t.status, Status::Done | Status::Dropped)
                    && t.title.trim().eq_ignore_ascii_case(&clean_idea)
            }) {
                let existing_id = cfg.id_config.format_id(existing.id);
                return Err(TodoError::new(
                    ErrorCode::DuplicateTitle,
                    format!("task {existing_id} already has title {clean_idea:?}"),
                )
                .with_task(existing_id));
            }

            let about_id = match about_target {
                Some(target) => Some(resolve_about(doc, archive, &cfg, target)?),
                None => None,
            };

            let max_state = state.read_ids()?.map_or(0, |s| s.highest_id);
            let next_num = highest_task_number(doc, archive, max_state)
                .checked_add(1)
                .ok_or_else(|| TodoError::new(ErrorCode::WriteFailed, "task ID overflow"))?;
            let new_id = TaskId(next_num);
            let id_str = cfg.id_config.format_id(new_id);

            if !ctx.dry_run {
                state.write_ids(&IdsState {
                    highest_id: next_num,
                })?;
            }

            let (insert_at, need_heading) = idea_insertion_point(doc, &cfg.idea_section)?;

            // The block is the canonical task block (format.md §10): `type: idea`, no priority.
            let date = OpsConfig::format_note_date(now);
            let about_fields: Vec<(String, String)> = about_id
                .iter()
                .map(|id| ("about".to_string(), id.clone()))
                .collect();
            let block = NewTask {
                title: &clean_idea,
                task_type: Some("idea"),
                extra: &about_fields,
                ..NewTask::default()
            }
            .lines(&id_str, &date, agent);

            let mut lines = Vec::with_capacity(block.len() + 2);
            if need_heading {
                if insert_at > 0
                    && doc
                        .line_doc
                        .lines
                        .get(insert_at - 1)
                        .is_some_and(|l| !l.text.trim().is_empty())
                {
                    lines.push(String::new());
                }
                lines.push(format!("## {}", cfg.idea_section));
            }
            lines.extend(block);

            let edit = LineEdit::insert(insert_at, lines);
            let intent = TaskChangeIntent::add_one(new_id);
            let arch_owned = archive.cloned();
            let cfg_owned = cfg_clone.clone();
            let err_id = id_str.clone();

            Ok(DocWritePlan::Edits {
                edits: vec![edit],
                intent,
                build_value: Box::new(move |new_doc| {
                    let added = new_doc.task_by_id(new_id).ok_or_else(|| {
                        TodoError::new(
                            ErrorCode::WriteFailed,
                            format!("added idea {err_id} missing after verification"),
                        )
                    })?;
                    let task_json = build_task_json(
                        new_doc,
                        added,
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
        out.write_line(&format!("{}: idea ok", res.value.0));
    }
    Ok(())
}

/// The canonical ID of the task or milestone `target` names, which must exist in `TODO.md` or the
/// archive (`R-KIND-1`, as `ask --about` does, `R-KIND-2`).
fn resolve_about(
    doc: &Document,
    archive: Option<&Document>,
    cfg: &OpsConfig,
    target: TaskId,
) -> Result<String> {
    let exists = doc.task_by_id(target).is_some()
        || archive.is_some_and(|a| a.task_by_id(target).is_some());
    if !exists {
        let id_str = cfg.id_config.format_id(target);
        return Err(
            TodoError::new(ErrorCode::NotFound, format!("task {id_str} not found")).with_task(id_str),
        );
    }
    Ok(cfg.id_config.format_id(target))
}

/// Where a new idea goes: the end of `section` (`insert_at`, `false`), or the end of the file with
/// a new heading for `section` (`true`) when no section matches. An ambiguous name is refused.
fn idea_insertion_point(doc: &Document, section: &str) -> Result<(usize, bool)> {
    let matches = matching_section_indices(&doc.sections, section);
    match matches.as_slice() {
        [] => Ok((doc.line_doc.lines.len(), true)),
        [only] => Ok((doc.sections[*only].content_end, false)),
        many => {
            let matched: Vec<&str> = many.iter().map(|&i| doc.sections[i].name.as_str()).collect();
            Err(TodoError::new(
                ErrorCode::AmbiguousSection,
                format!(
                    "idea_section '{section}' is ambiguous (matches: {})",
                    matched.join(", ")
                ),
            ))
        }
    }
}
