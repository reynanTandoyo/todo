//! `todo lock` command: acquire a manual editing lease (T-015; R-LOCK-6).

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::cli::{CommandKind, CommandMeta, Context};
use crate::error::Result;
use crate::lock::{LockConfig, acquire_lease};
use crate::output::Output;
use crate::state::StateDir;

/// Metadata for the `lock` command.
pub const META: CommandMeta = CommandMeta {
    name: "lock",
    summary: "Take a manual editing lease on TODO.md",
    kind: CommandKind::Write,
    details: "Grants the named agent a temporary lease (default 10 minutes, max 60) for editing TODO.md by hand. While active, other agents' writes wait and fail with exit 3 (`LEASE_HELD`), while the holder's own commands continue normally.",
    examples: &[
        "todo lock --agent alice",
        "todo lock --agent alice --minutes 20",
    ],
};

/// Arguments for `todo lock`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Lease length in minutes (default 10, maximum 60).
    #[arg(long, value_name = "N")]
    pub minutes: Option<u64>,
}

#[derive(Serialize)]
struct LockJson {
    agent: String,
    started_at: String,
    expires_at: String,
    minutes: u64,
}

/// Runs `todo lock`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let agent = ctx.require_agent()?;
    let found = ctx.find_todo(out)?;
    let state = StateDir::from_found(&found)?;
    let cfg = LockConfig::default();
    let minutes = args.minutes.unwrap_or(cfg.lease_minutes);
    let lease = acquire_lease(&state, agent, args.minutes, &cfg)?;

    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &LockJson {
                agent: lease.agent,
                started_at: lease.started_at,
                expires_at: lease.expires_at,
                minutes,
            },
        );
    } else {
        out.write_line(&format!(
            "locked by {} until {} ({minutes} min)",
            lease.agent, lease.expires_at
        ));
    }
    Ok(())
}
