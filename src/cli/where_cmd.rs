//! `todo where` command: display resolved file, repository root, state folder, lock/lease status, format version, and task counts (T-016; R-FIND-7).

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::cli::{CommandKind, CommandMeta, Context};
use crate::doc::Status;
use crate::error::Result;
use crate::lock::{LeaseStatus, query_status};
use crate::ops::load_for_read;
use crate::output::Output;
use crate::state::{LockHolder, StateDir};
use crate::version::RUNNING_VERSION;

/// Metadata for the `where` command.
pub const META: CommandMeta = CommandMeta {
    name: "where",
    summary: "Show resolved TODO.md, repo root, state folder, lock status, and task counts",
    kind: CommandKind::Read,
    details: "Prints the active TODO.md file and how it was found, the git repository root (if any), the state folder, the write lock and manual lease status, the format version, the tool stamp in the file and the running todo version, and the number of tasks per status.",
    examples: &["todo where", "todo where --json"],
};

/// Arguments for `todo where`.
#[derive(Debug, ClapArgs)]
pub struct Args {}

#[derive(Serialize)]
struct WriteLockJson {
    held: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    holder: Option<LockHolder>,
}

#[derive(Serialize)]
struct StatusCountsJson {
    open: usize,
    claimed: usize,
    blocked: usize,
    done: usize,
    dropped: usize,
    total: usize,
}

#[derive(Serialize)]
struct WhereJson {
    file: String,
    found_by: &'static str,
    repo_root: Option<String>,
    state_dir: String,
    write_lock: WriteLockJson,
    #[serde(skip_serializing_if = "Option::is_none")]
    lease: Option<LeaseStatus>,
    format_version: Option<u32>,
    /// The tool version stamp in the file (R-VER-1), as written; `None` when the file has none.
    tool_stamp: Option<String>,
    /// The version of the running todo.
    running_version: String,
    counts: StatusCountsJson,
}

/// Runs `todo where`.
pub fn run(_args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let cfg = ctx.load_config()?;
    let loaded = load_for_read(ctx, &cfg, false, false, out)?;
    if let Some(warning) = crate::find::synced_folder_warning(&loaded.found.path) {
        out.warn(warning);
    }
    let state = StateDir::from_found(&loaded.found)?;
    let status = query_status(&state)?;

    let mut counts = StatusCountsJson {
        open: 0,
        claimed: 0,
        blocked: 0,
        done: 0,
        dropped: 0,
        total: loaded.doc.tasks.len(),
    };
    for task in &loaded.doc.tasks {
        match task.status {
            Status::Open => counts.open += 1,
            Status::Claimed => counts.claimed += 1,
            Status::Blocked => counts.blocked += 1,
            Status::Done => counts.done += 1,
            Status::Dropped => counts.dropped += 1,
        }
    }

    let found_by = loaded.found.found_by.as_str();
    let repo_root = loaded
        .found
        .repo_root
        .as_ref()
        .map(|p| p.display().to_string());
    let format_version = loaded.doc.declared_format_version;
    let tool_stamp = loaded.doc.tool_stamp.as_ref().map(|s| s.value.clone());
    let running_version = RUNNING_VERSION.to_string();

    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &WhereJson {
                file: loaded.found.path.display().to_string(),
                found_by,
                repo_root,
                state_dir: state.path().display().to_string(),
                write_lock: WriteLockJson {
                    held: status.write_lock_held,
                    holder: status.holder,
                },
                lease: status.lease,
                format_version,
                tool_stamp,
                running_version,
                counts,
            },
        );
    } else {
        out.write_line(&format!(
            "File: {} (found by: {found_by})",
            loaded.found.path.display()
        ));
        out.write_line(&format!(
            "Repo root: {}",
            repo_root.as_deref().unwrap_or("none")
        ));
        out.write_line(&format!("State dir: {}", state.path().display()));
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
        match format_version {
            Some(v) => out.write_line(&format!("Format version: {v}")),
            None => out.write_line("Format version: omitted"),
        }
        match &tool_stamp {
            Some(v) => out.write_line(&format!("Tool stamp: {v}")),
            None => out.write_line("Tool stamp: omitted"),
        }
        out.write_line(&format!("Running todo version: {running_version}"));
        let summary = format!(
            "Tasks: {} total (open: {}, claimed: {}, blocked: {}, done: {}, dropped: {})",
            counts.total, counts.open, counts.claimed, counts.blocked, counts.done, counts.dropped
        );
        out.write_line(&summary);
    }

    Ok(())
}
