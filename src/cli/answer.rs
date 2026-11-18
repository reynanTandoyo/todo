//! `todo answer` command: record an answer note and mark a question done (T-031; R-WRITE-11).
//! When the question names an entry in its `about` field, that entry is printed after the result
//! (R-KIND-2).

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::cli::{CommandKind, CommandMeta, Context, edit_one_task, load_read, resolve_text_arg};
use crate::doc::{Document, Task, TaskId, sanitize_trimmed_value};
use crate::error::{ErrorCode, Result, TodoError};
use crate::lock::current_time;
use crate::ops::{LoadedRead, OpsConfig, TaskJson, TaskOp, build_task_json};
use crate::output::Output;

/// Metadata for the `answer` command.
pub const META: CommandMeta = CommandMeta {
    name: "answer",
    summary: "Answer a question and mark it done",
    kind: CommandKind::Write,
    details: "Adds the answer as a note on the question task and marks it `done`. If the question names an entry in its `about` field, that entry is printed after the result, as `show` prints it; with `--json` it is returned under `about`.",
    examples: &[
        "todo answer T-012 \"Yes, use exponential backoff capped at 30s\" --agent human",
    ],
};

/// Arguments for `todo answer`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Question task ID or unique title substring.
    #[arg(value_name = "TASK")]
    pub task: String,

    /// Answer text (recorded as a note).
    #[arg(value_name = "TEXT")]
    pub text: String,
}

/// `answer --json`: the question as `TaskJson`, plus the entry it is about when there is one.
#[derive(Serialize)]
struct AnswerJson<'a> {
    #[serde(flatten)]
    task: &'a TaskJson,
    #[serde(skip_serializing_if = "Option::is_none")]
    about: Option<&'a TaskJson>,
}

/// The entry a question names in its `about` field, read back after the answer was written.
struct AboutEntry {
    /// Canonical ID of the entry.
    id: String,
    /// The entry as `show --json` would return it.
    json: TaskJson,
    /// The entry's block lines, as `show` prints them.
    block: Vec<String>,
}

/// Runs `todo answer <task> <text>`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let raw_text = resolve_text_arg(&args.text)?;
    let answer_text = sanitize_trimmed_value(&raw_text);
    if answer_text.is_empty() {
        return Err(TodoError::new(
            ErrorCode::BadValue,
            "answer text cannot be empty",
        ));
    }
    let op = TaskOp::Answer {
        text: &answer_text,
    };
    let cfg = ctx.load_config()?;
    let res = edit_one_task(ctx, out, META.name, &args.task, &op, &cfg, None)?;
    let about = about_entry(ctx, out, &res.task, &cfg)?;

    if out.is_json() {
        let json = AnswerJson {
            task: &res.task,
            about: about.as_ref().map(|entry| &entry.json),
        };
        out.emit_json_ok(META.name, &json);
    } else {
        out.write_line(&format!("{}: {} ok", res.task_id, META.name));
        if let Some(entry) = &about {
            out.write_line("");
            out.write_line(&format!("About {}:", entry.id));
            for line in &entry.block {
                out.write_line(line);
            }
        }
    }
    Ok(())
}

/// The entry `question` names in its `about` field, if the field holds an ID that exists.
fn about_entry(
    ctx: &Context,
    out: &mut Output,
    question: &TaskJson,
    cfg: &OpsConfig,
) -> Result<Option<AboutEntry>> {
    let Some(target) = question
        .fields
        .get("about")
        .and_then(|raw| cfg.id_config.parse_user_id(raw))
    else {
        return Ok(None);
    };
    let (_, loaded) = load_read(ctx, out, false, true)?;
    let Some((primary, secondary, entry, archived)) = locate(&loaded, target) else {
        return Ok(None);
    };
    Ok(Some(AboutEntry {
        id: cfg.id_config.format_id(target),
        json: build_task_json(primary, entry, secondary, archived, current_time(), cfg),
        block: primary.line_doc.lines[entry.block_range.clone()]
            .iter()
            .map(|line| line.text.clone())
            .collect(),
    }))
}

/// Finds the entry `id` in `TODO.md`, or else in the archive, as `show` does. Returns the primary
/// document, the secondary one, the entry and whether the entry is archived.
fn locate(loaded: &LoadedRead, id: TaskId) -> Option<(&Document, Option<&Document>, &Task, bool)> {
    if let Some(task) = loaded.doc.task_by_id(id) {
        return Some((&loaded.doc, loaded.archive.as_ref(), task, false));
    }
    let archive = loaded.archive.as_ref()?;
    let task = archive.task_by_id(id)?;
    Some((archive, Some(&loaded.doc), task, true))
}
