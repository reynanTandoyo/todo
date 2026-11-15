//! `todo log` command: show journal entries for one task (`T-047`; R-HIST-4, R-READ-7).

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::cli::{CommandKind, CommandMeta, Context};
use crate::error::Result;
use crate::history::{JournalEntry, read_journal};
use crate::ops::{load_for_read, resolve_task_in_doc};
use crate::output::Output;
use crate::state::StateDir;

/// Metadata for the `log` command.
pub const META: CommandMeta = CommandMeta {
    name: "log",
    summary: "Show journal entries for a task",
    kind: CommandKind::Read,
    details: "Reads `journal.jsonl` from the state folder and prints every recorded write that touched the given task.",
    examples: &["todo log T-012", "todo log t12 --json"],
};

/// Arguments for `todo log`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Task ID (e.g. `T-012`, `t12`, or `12`) or unique title substring.
    #[arg(value_name = "TASK")]
    pub task: String,
}

#[derive(Serialize)]
struct LogJson {
    task: String,
    count: usize,
    entries: Vec<JournalEntry>,
}

/// Runs `todo log <task>`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let cfg = ctx.load_config()?;
    let loaded = load_for_read(ctx, &cfg, false, true, out)?;
    let state = StateDir::from_found(&loaded.found)?;

    let target_id = if let Some(parsed) = cfg.id_config.parse_user_id(&args.task) {
        cfg.id_config.format_id(parsed)
    } else if let Ok(t) = resolve_task_in_doc(&loaded.doc, &args.task) {
        cfg.id_config.format_id(t.id)
    } else if let Some(ref arch) = loaded.archive {
        let t = resolve_task_in_doc(arch, &args.task)?;
        cfg.id_config.format_id(t.id)
    } else {
        // Trigger the standard NOT_FOUND error.
        let t = resolve_task_in_doc(&loaded.doc, &args.task)?;
        cfg.id_config.format_id(t.id)
    };

    let all = read_journal(&state)?;
    let entries: Vec<JournalEntry> = all
        .into_iter()
        .filter(|e| e.tasks.iter().any(|id| id.eq_ignore_ascii_case(&target_id)))
        .collect();

    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &LogJson {
                task: target_id,
                count: entries.len(),
                entries,
            },
        );
        return Ok(());
    }

    if entries.is_empty() {
        out.write_line(&format!("no journal entries for {target_id}"));
        return Ok(());
    }

    for entry in &entries {
        let h_before: String = entry.hash_before.chars().take(8).collect();
        let h_after: String = entry.hash_after.chars().take(8).collect();
        out.write_line(&format!(
            "{}  {}  {}  [{}]  {} -> {}",
            entry.time,
            entry.agent,
            entry.command,
            entry.tasks.join(", "),
            h_before,
            h_after
        ));
    }
    Ok(())
}
