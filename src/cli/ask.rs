//! `todo ask` command: add a question task to `question_section` (T-031; R-WRITE-11).

use clap::Args as ClapArgs;

use crate::cli::{CommandKind, CommandMeta, Context};
use crate::doc::{
    CANONICAL_HEADING_SEPARATOR, CANONICAL_NOBODY_OWNER, LineEdit, Status, TaskChangeIntent,
    TaskId, sanitize_trimmed_value,
};
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::{
    DocWritePlan, OpsConfig, build_task_json, edit_doc_with_state_and_write,
    matching_section_indices,
};
use crate::output::Output;
use crate::state::IdsState;

/// Metadata for the `ask` command.
pub const META: CommandMeta = CommandMeta {
    name: "ask",
    summary: "Ask a question for a person to answer",
    kind: CommandKind::Write,
    details: "Adds a task of `type: question` to the section named by `question_section` (default `Questions`, created at the end of `TODO.md` if missing). `next` never hands out questions.",
    examples: &[
        "todo ask \"Should retries use exponential backoff?\" --agent agent-1",
    ],
};

/// Arguments for `todo ask`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Question text (becomes the task title).
    #[arg(value_name = "QUESTION")]
    pub question: String,

    /// Optional priority (e.g. `P1`).
    #[arg(long, value_name = "VALUE")]
    pub priority: Option<String>,

    /// Optional area (e.g. `cli`).
    #[arg(long, value_name = "VALUE")]
    pub area: Option<String>,
}

/// Runs `todo ask <question>`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let clean_question = sanitize_trimmed_value(&args.question);
    if clean_question.is_empty() {
        return Err(TodoError::new(
            ErrorCode::BadValue,
            "question cannot be empty",
        ));
    }

    let cfg = ctx.load_config()?;
    let priority_val = match args.priority.as_deref() {
        Some(raw) => {
            let clean = sanitize_trimmed_value(raw);
            let Some(canonical) = cfg
                .priorities
                .iter()
                .find(|p| p.eq_ignore_ascii_case(&clean))
            else {
                return Err(TodoError::new(
                    ErrorCode::BadValue,
                    format!(
                        "invalid priority '{clean}'; expected one of: {}",
                        cfg.priorities.join(", ")
                    ),
                ));
            };
            Some(canonical.clone())
        }
        None => None,
    };
    let area_val = match args.area.as_deref() {
        Some(raw) => {
            let clean = sanitize_trimmed_value(raw);
            if clean.is_empty() {
                None
            } else if !cfg.areas.is_empty() {
                let Some(canonical) = cfg.areas.iter().find(|a| a.eq_ignore_ascii_case(&clean))
                else {
                    return Err(TodoError::new(
                        ErrorCode::BadValue,
                        format!(
                            "invalid area '{clean}'; expected one of: {}",
                            cfg.areas.join(", ")
                        ),
                    ));
                };
                Some(canonical.clone())
            } else {
                Some(clean)
            }
        }
        None => None,
    };

    let cfg_clone = cfg.clone();
    let res = edit_doc_with_state_and_write(
        ctx,
        &cfg,
        META.name,
        out,
        |doc, archive, state, _label, agent, now| {
            if let Some(existing) = doc.tasks.iter().find(|t| {
                !matches!(t.status, Status::Done | Status::Dropped)
                    && t.title.trim().eq_ignore_ascii_case(&clean_question)
            }) {
                let existing_id = cfg.id_config.format_id(existing.id);
                return Err(TodoError::new(
                    ErrorCode::DuplicateTitle,
                    format!("task {existing_id} already has title {clean_question:?}"),
                )
                .with_task(existing_id));
            }

            let max_doc = doc.tasks.iter().map(|t| t.id.number()).max().unwrap_or(0);
            let max_arch = archive
                .iter()
                .flat_map(|a| a.tasks.iter())
                .map(|t| t.id.number())
                .max()
                .unwrap_or(0);
            let max_state = state.read_ids()?.map_or(0, |s| s.highest_id);
            let next_num = max_doc
                .max(max_arch)
                .max(max_state)
                .checked_add(1)
                .ok_or_else(|| TodoError::new(ErrorCode::WriteFailed, "task ID overflow"))?;
            let new_id = TaskId(next_num);
            let id_str = cfg.id_config.format_id(new_id);

            state.write_ids(&IdsState {
                highest_id: next_num,
            })?;

            let matches = matching_section_indices(&doc.sections, &cfg.question_section);
            let (insert_at, need_section_heading) = match matches.as_slice() {
                [] => (doc.line_doc.lines.len(), true),
                [only] => (doc.sections[*only].content_end, false),
                many => {
                    let matched: Vec<&str> =
                        many.iter().map(|&i| doc.sections[i].name.as_str()).collect();
                    return Err(TodoError::new(
                        ErrorCode::AmbiguousSection,
                        format!(
                            "question_section '{}' is ambiguous (matches: {})",
                            cfg.question_section,
                            matched.join(", ")
                        ),
                    ));
                }
            };

            let mut lines = Vec::new();
            if insert_at > 0
                && !doc.line_doc.lines[insert_at - 1]
                    .text
                    .trim()
                    .is_empty()
            {
                lines.push(String::new());
            }
            if need_section_heading {
                lines.push(format!("## {}", cfg.question_section));
                lines.push(String::new());
            }
            lines.push(format!(
                "### {id_str}{CANONICAL_HEADING_SEPARATOR}{clean_question}"
            ));
            lines.push("- status: open".to_string());
            lines.push(format!("- owner: {CANONICAL_NOBODY_OWNER}"));

            let mut combined_parts = Vec::new();
            if let Some(ref p) = priority_val {
                combined_parts.push(format!("priority: {p}"));
            }
            combined_parts.push("type: question".to_string());
            if let Some(ref a) = area_val {
                combined_parts.push(format!("area: {a}"));
            }
            lines.push(format!("- {}", combined_parts.join(" \u{00b7} ")));

            let date = OpsConfig::format_note_date(now);
            lines.push(format!("- note ({date}, {agent}): asked"));

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
                            format!("added question {err_id} missing after verification"),
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
        out.write_line(&format!("{}: ask ok", res.value.0));
    }
    Ok(())
}
