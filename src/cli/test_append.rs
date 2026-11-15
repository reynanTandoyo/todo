//! Hidden test-only write command for exercising write locks in separate processes (T-013; TEST-5).

use std::path::PathBuf;
use std::time::Duration;

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::cli::{CommandKind, CommandMeta, Context};
use crate::error::Result;
use crate::lock::{LockConfig, WriteOutcome, locked_read_modify_write};
use crate::output::Output;
use crate::state::StateDir;

/// Metadata for `__test-append` (hidden from normal CLI help).
pub const META: CommandMeta = CommandMeta {
    name: "__test-append",
    summary: "Test-only write command for locking tests",
    kind: CommandKind::Write,
    details: "Internal test hook compiled only under the `test-hooks` feature.",
    examples: &[],
};

/// Arguments for `todo __test-append`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Text line to append to `TODO.md`.
    #[arg(long, value_name = "TEXT")]
    pub text: String,

    /// Optional duration in milliseconds to sleep while holding the write lock.
    #[arg(long, default_value_t = 0, value_name = "MS")]
    pub hold_ms: u64,

    /// Optional marker file created as soon as the write lock is held.
    #[arg(long, value_name = "PATH")]
    pub ready_file: Option<PathBuf>,
}

#[derive(Serialize)]
struct TestAppendJson<'a> {
    appended: &'a str,
}

/// Runs `todo __test-append`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let agent = ctx.require_agent()?;
    let found = ctx.find_todo(out)?;
    let state = StateDir::from_found(&found)?;
    let cfg = LockConfig::default();

    locked_read_modify_write(
        &found.path,
        &state,
        agent,
        META.name,
        &cfg,
        |bytes, _snap| {
            if let Some(ref ready) = args.ready_file {
                let _ = std::fs::write(ready, b"ready\n");
            }
            if args.hold_ms > 0 {
                std::thread::sleep(Duration::from_millis(args.hold_ms));
            }
            let mut new_bytes = bytes.to_vec();
            if !new_bytes.is_empty() && !new_bytes.ends_with(b"\n") {
                new_bytes.push(b'\n');
            }
            new_bytes.extend_from_slice(args.text.as_bytes());
            new_bytes.push(b'\n');
            Ok(WriteOutcome::Changed {
                new_bytes,
                value: (),
            })
        },
    )?;

    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &TestAppendJson {
                appended: &args.text,
            },
        );
    } else {
        out.write_line(&format!("appended: {}", args.text));
    }
    Ok(())
}
