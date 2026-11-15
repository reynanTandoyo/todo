//! `todo status` command: display write lock and manual lease status (T-015; R-LOCK-6).

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::cli::{CommandKind, CommandMeta, Context};
use crate::error::Result;
use crate::lock::{LeaseStatus, query_status};
use crate::output::Output;
use crate::state::{LockHolder, StateDir};

/// Metadata for the `status` command.
pub const META: CommandMeta = CommandMeta {
    name: "status",
    summary: "Show write lock and manual lease status",
    kind: CommandKind::Read,
    details: "Shows which TODO.md file is resolved, whether the OS write lock is currently held, and whether a manual editing lease is active. Never waits on or acquires the write lock.",
    examples: &["todo status", "todo status --json"],
};

/// Arguments for `todo status`.
#[derive(Debug, ClapArgs)]
pub struct Args {}

#[derive(Serialize)]
struct WriteLockJson {
    held: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    holder: Option<LockHolder>,
}

#[derive(Serialize)]
struct StatusJson {
    file: String,
    state_dir: String,
    write_lock: WriteLockJson,
    #[serde(skip_serializing_if = "Option::is_none")]
    lease: Option<LeaseStatus>,
}

/// Runs `todo status`.
pub fn run(_args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let found = ctx.find_todo(out)?;
    let state = StateDir::from_found(&found)?;
    let status = query_status(&state)?;

    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &StatusJson {
                file: found.path.display().to_string(),
                state_dir: state.path().display().to_string(),
                write_lock: WriteLockJson {
                    held: status.write_lock_held,
                    holder: status.holder,
                },
                lease: status.lease,
            },
        );
    } else {
        out.write_line(&format!("File: {}", found.path.display()));
        match status.holder {
            Some(h) if status.write_lock_held => {
                out.write_line(&format!(
                    "Write lock: held by {} (command: {}, pid: {}, since: {})",
                    h.agent, h.command, h.pid, h.started_at
                ));
            }
            _ if status.write_lock_held => {
                out.write_line("Write lock: held (holder unknown)");
            }
            _ => {
                out.write_line("Write lock: free");
            }
        }
        match status.lease {
            Some(l) => {
                out.write_line(&format!(
                    "Lease: held by {} until {} ({}s remaining)",
                    l.agent, l.expires_at, l.remaining_seconds
                ));
            }
            None => {
                out.write_line("Lease: none");
            }
        }
    }
    Ok(())
}
