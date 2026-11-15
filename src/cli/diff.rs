//! `todo diff` command: compare two snapshots or a snapshot against the current file (`R-HIST-2`).

use clap::Args as ClapArgs;
use serde::Serialize;
use similar::TextDiff;

use crate::atomic::read_file_with_snapshot;
use crate::cli::{CommandKind, CommandMeta, Context};
use crate::error::Result;
use crate::history::{parse_snapshot_number, read_snapshot_bytes};
use crate::ops::file_label;
use crate::output::Output;
use crate::state::StateDir;

/// Metadata for the `diff` command.
pub const META: CommandMeta = CommandMeta {
    name: "diff",
    summary: "Show changes between snapshots or against the current TODO.md",
    kind: CommandKind::Read,
    details: "With one snapshot number `n`, compares snapshot `n` against the current `TODO.md`. With two numbers `n` and `m`, compares snapshot `n` against snapshot `m`.",
    examples: &["todo diff 1", "todo diff 1 3", "todo diff 1 --json"],
};

/// Arguments for `todo diff`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// First snapshot number (e.g. `1` or `000001`).
    #[arg(value_name = "N")]
    pub from: String,

    /// Optional second snapshot number (defaults to current `TODO.md`).
    #[arg(value_name = "M")]
    pub to: Option<String>,
}

#[derive(Serialize)]
struct DiffJson {
    from: u64,
    to: Option<u64>,
    changed: bool,
    diff: String,
}

/// Runs `todo diff`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let from_num = parse_snapshot_number(&args.from)?;
    let to_num = args
        .to
        .as_deref()
        .map(parse_snapshot_number)
        .transpose()?;

    let found = ctx.find_todo(out)?;
    let state = StateDir::from_found(&found)?;
    let label = file_label(&found.path);

    let (from_entry, from_bytes) = read_snapshot_bytes(&state, from_num)?;
    let (to_label, to_bytes) = match to_num {
        Some(m) => {
            let (to_entry, bytes) = read_snapshot_bytes(&state, m)?;
            (format!("{label}#{}", to_entry.number), bytes)
        }
        None => {
            let snap = read_file_with_snapshot(&found.path)?;
            (label, snap.bytes)
        }
    };

    let old_text = String::from_utf8_lossy(&from_bytes);
    let new_text = String::from_utf8_lossy(&to_bytes);
    let diff = TextDiff::from_lines(old_text.as_ref(), new_text.as_ref())
        .unified_diff()
        .header(
            &format!("a/{}#{}", file_label(&found.path), from_entry.number),
            &format!("b/{to_label}"),
        )
        .to_string();

    let changed = from_bytes != to_bytes;

    if out.is_json() {
        out.emit_json_ok(
            "todo.diff.v1",
            &DiffJson {
                from: from_num,
                to: to_num,
                changed,
                diff,
            },
        );
        return Ok(());
    }

    if !changed || diff.is_empty() {
        out.write_line("no changes");
    } else {
        out.write_str(&diff);
        if !diff.ends_with('\n') {
            out.write_str("\n");
        }
    }
    Ok(())
}
