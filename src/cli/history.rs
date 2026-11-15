//! `todo history` command: list saved snapshots (`R-HIST-2`).

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::cli::{CommandKind, CommandMeta, Context};
use crate::error::Result;
use crate::history::{SnapshotEntry, read_history};
use crate::ops::OpsConfig;
use crate::output::Output;
use crate::state::StateDir;

/// Metadata for the `history` command.
pub const META: CommandMeta = CommandMeta {
    name: "history",
    summary: "List saved snapshots of TODO.md",
    kind: CommandKind::Read,
    details: "Lists numbered snapshots recorded before writes (and any detected outside edits). Does not create state files when none exist.",
    examples: &["todo history", "todo history --limit 10", "todo history --json"],
};

/// Arguments for `todo history`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Show only the newest N snapshots.
    #[arg(long, value_name = "N")]
    pub limit: Option<usize>,
}

#[derive(Serialize)]
struct HistoryJson {
    snapshots: Vec<SnapshotEntry>,
    count: usize,
}

/// Runs `todo history`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let found = ctx.find_todo(out)?;
    let _cfg = OpsConfig::load_next_to(&found.path)?;
    let state = StateDir::from_found(&found)?;
    let mut entries = read_history(&state)?;
    if let Some(limit) = args.limit
        && entries.len() > limit
    {
        let start = entries.len() - limit;
        entries = entries.split_off(start);
    }

    if out.is_json() {
        let count = entries.len();
        out.emit_json_ok(
            "todo.history.v1",
            &HistoryJson {
                snapshots: entries,
                count,
            },
        );
        return Ok(());
    }

    if entries.is_empty() {
        out.write_line("no snapshots recorded yet");
        return Ok(());
    }

    for entry in &entries {
        let short_hash = entry.sha256.get(..12).unwrap_or(&entry.sha256);
        let tag = if entry.outside_edit {
            " [outside edit]"
        } else {
            ""
        };
        out.write_line(&format!(
            "#{:06}  {}  {}  {}  {}{}",
            entry.number, entry.time, entry.agent, entry.command, short_hash, tag
        ));
    }
    Ok(())
}
