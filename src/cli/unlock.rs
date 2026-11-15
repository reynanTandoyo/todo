//! `todo unlock` command: release a manual editing lease (T-015; R-LOCK-6).

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::cli::{CommandKind, CommandMeta, Context};
use crate::error::Result;
use crate::lock::{LockConfig, UnlockOutcome, release_lease};
use crate::output::Output;
use crate::state::StateDir;

/// Metadata for the `unlock` command.
pub const META: CommandMeta = CommandMeta {
    name: "unlock",
    summary: "End a manual editing lease on TODO.md",
    kind: CommandKind::Write,
    details: "Ends the active manual editing lease held by the named agent. An expired lease counts as ended. Pass `--force` to end another agent's live lease.",
    examples: &["todo unlock --agent alice", "todo unlock --agent bob --force"],
};

/// Arguments for `todo unlock`.
#[derive(Debug, ClapArgs)]
pub struct Args {}

#[derive(Serialize)]
struct UnlockJson {
    unlocked: bool,
    forced: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    previous_holder: Option<String>,
}

/// Runs `todo unlock`.
pub fn run(_args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let agent = ctx.require_agent()?;
    let found = ctx.find_todo(out)?;
    let state = StateDir::from_found(&found)?;
    let cfg = LockConfig::default();

    match release_lease(&state, agent, ctx.force, &cfg)? {
        UnlockOutcome::Unlocked { agent: holder } => {
            if out.is_json() {
                out.emit_json_ok(
                    META.name,
                    &UnlockJson {
                        unlocked: true,
                        forced: false,
                        previous_holder: Some(holder),
                    },
                );
            } else {
                out.write_line(&format!("unlocked (agent: {holder})"));
            }
        }
        UnlockOutcome::Forced { previous_holder } => {
            if out.is_json() {
                out.emit_json_ok(
                    META.name,
                    &UnlockJson {
                        unlocked: true,
                        forced: true,
                        previous_holder: Some(previous_holder),
                    },
                );
            } else {
                out.write_line(&format!(
                    "unlocked (forced; ended lease held by {previous_holder})"
                ));
            }
        }
        UnlockOutcome::NotActive => {
            if out.is_json() {
                out.emit_json_ok(
                    META.name,
                    &UnlockJson {
                        unlocked: false,
                        forced: false,
                        previous_holder: None,
                    },
                );
            } else {
                out.write_line("no active lease");
            }
        }
    }
    Ok(())
}
