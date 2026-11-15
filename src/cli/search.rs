//! `todo search` command: search titles, fields, and notes (`T-046`; R-READ-6).

use std::collections::BTreeSet;

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::cli::{CommandKind, CommandMeta, Context};
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::load_for_read;
use crate::output::Output;

/// Metadata for the `search` command.
pub const META: CommandMeta = CommandMeta {
    name: "search",
    summary: "Search titles, fields, and notes for matching text",
    kind: CommandKind::Read,
    details: "Finds case-insensitive substring matches across task titles, fields, and notes, printing each match with its task ID. Pass `--archive` to search `TODO-archive.md`.",
    examples: &[
        "todo search parser",
        "todo search \"outside edit\" --archive",
        "todo search lock --json",
    ],
};

/// Arguments for `todo search`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Text to search for (case-insensitive substring).
    #[arg(value_name = "TEXT")]
    pub text: String,

    /// Search `TODO-archive.md` instead of `TODO.md`.
    #[arg(long)]
    pub archive: bool,
}

/// One matching line within a task (`R-READ-6`).
#[derive(Debug, Clone, Serialize)]
pub struct SearchMatchJson {
    /// Canonical task ID (e.g. `"T-012"`).
    pub id: String,
    /// Task status (`open`, `claimed`, `blocked`, `done`, `dropped`).
    pub status: &'static str,
    /// Task title.
    pub title: String,
    /// Where the match occurred (`"title"`, `"note"`, or the field key).
    pub field: String,
    /// 1-based line number in the file.
    pub line: usize,
    /// Matching text.
    pub text: String,
}

#[derive(Serialize)]
struct SearchJson {
    archive: bool,
    query: String,
    count: usize,
    matches: Vec<SearchMatchJson>,
}

/// Runs `todo search <text>`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let query = args.text.trim();
    if query.is_empty() {
        return Err(TodoError::new(
            ErrorCode::Usage,
            "search text cannot be empty",
        ));
    }
    let query_lower = query.to_lowercase();

    let cfg = ctx.load_config()?;
    let loaded = load_for_read(ctx, &cfg, args.archive, false, out)?;

    let mut matches = Vec::new();
    for task in &loaded.doc.tasks {
        let id = cfg.id_config.format_id(task.id);
        let status = task.status.as_str();

        if task.title.to_lowercase().contains(&query_lower) {
            matches.push(SearchMatchJson {
                id: id.clone(),
                status,
                title: task.title.clone(),
                field: "title".to_string(),
                line: task.heading_line + 1,
                text: task.title.clone(),
            });
        }

        for entry in &task.field_lines {
            let combined = format!("{}: {}", entry.key, entry.value);
            if combined.to_lowercase().contains(&query_lower) {
                matches.push(SearchMatchJson {
                    id: id.clone(),
                    status,
                    title: task.title.clone(),
                    field: entry.key.clone(),
                    line: entry.line + 1,
                    text: combined,
                });
            }
        }

        for note in &task.notes {
            let formatted = format!("{} ({}): {}", note.date_raw, note.agent, note.text);
            if formatted.to_lowercase().contains(&query_lower) {
                matches.push(SearchMatchJson {
                    id: id.clone(),
                    status,
                    title: task.title.clone(),
                    field: "note".to_string(),
                    line: note.line + 1,
                    text: formatted,
                });
            }
        }
    }

    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &SearchJson {
                archive: args.archive,
                query: query.to_string(),
                count: matches.len(),
                matches,
            },
        );
        return Ok(());
    }

    let mut seen_tasks = BTreeSet::new();
    for m in &matches {
        seen_tasks.insert(m.id.as_str());
        let id_col = out.cyan(&m.id);
        let field_col = out.dim(&format!("[{}]", m.field));
        out.write_line(&format!("{id_col}  {field_col}  {}", m.text));
    }
    let m_noun = if matches.len() == 1 {
        "match"
    } else {
        "matches"
    };
    let t_noun = if seen_tasks.len() == 1 {
        "task"
    } else {
        "tasks"
    };
    out.write_line(&format!(
        "{} {m_noun} in {} {t_noun}",
        matches.len(),
        seen_tasks.len()
    ));
    Ok(())
}
