//! `todo session-end`: the hidden command behind the Claude Code `SessionEnd` hook (`R-CLAIM-7`).
//!
//! Claude Code runs the hook with a JSON object on standard input that carries `session_id`. The
//! command releases every claim tagged with that session (the same step as
//! `todo release --session <id>`) and stays quiet when there is nothing to do, so ending a session
//! never fails because of a missing `TODO.md`.

use std::io::{IsTerminal, Read as _};

use clap::Args as ClapArgs;
use serde::Serialize;
use serde_json::Value;

use crate::cli::release::release_session_claims;
use crate::cli::{CommandKind, CommandMeta, Context, validate_session_id};
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::TaskJson;
use crate::output::Output;

/// Agent name written on the release notes when no `--agent` or `TODO_AGENT` is set.
const HOOK_AGENT: &str = "session-end";

/// Metadata for the hidden `session-end` command.
pub const META: CommandMeta = CommandMeta {
    name: "session-end",
    summary: "Claude Code SessionEnd hook: release the claims of the ending session",
    kind: CommandKind::Meta,
    details: "Reads the hook's JSON from standard input and releases every claim tagged with its `session_id`. Installed by `todo hooks install`.",
    examples: &["echo '{\"session_id\":\"abc\"}' | todo session-end"],
};

/// Arguments for `todo session-end`.
#[derive(Debug, ClapArgs)]
pub struct Args {}

#[derive(Serialize)]
struct SessionEndJson {
    session: Option<String>,
    released: Vec<TaskJson>,
    count: usize,
}

/// Runs `todo session-end`.
pub fn run(_args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let from_input = read_session_id()?;
    let session = from_input.or_else(|| ctx.session.clone());

    let released = match &session {
        None => Vec::new(),
        Some(id) => {
            let mut hook_ctx = ctx.clone();
            if hook_ctx.agent.is_none() {
                hook_ctx.agent = Some(HOOK_AGENT.to_string());
            }
            match release_session_claims(&hook_ctx, out, id) {
                Ok(r) => r,
                // No TODO.md here: nothing to release.
                Err(e) if e.code == ErrorCode::NoFile => Vec::new(),
                Err(e) => return Err(e),
            }
        }
    };

    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &SessionEndJson {
                session,
                count: released.len(),
                released,
            },
        );
    } else {
        for t in &released {
            out.write_line(&format!("{}: released", t.id));
        }
    }
    Ok(())
}

/// Reads the hook JSON from standard input and returns its valid `session_id`, if any.
fn read_session_id() -> Result<Option<String>> {
    if std::io::stdin().is_terminal() {
        return Ok(None);
    }
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf).map_err(|e| {
        TodoError::new(
            ErrorCode::BadValue,
            format!("failed reading hook input from stdin: {e}"),
        )
    })?;
    Ok(session_id_from_json(&buf))
}

/// Extracts a valid `session_id` from the hook's JSON text (a leading BOM is ignored).
pub(crate) fn session_id_from_json(text: &str) -> Option<String> {
    let text = text.trim().trim_start_matches('\u{feff}');
    let value: Value = serde_json::from_str(text).ok()?;
    let id = value.get("session_id")?.as_str()?;
    validate_session_id(id).ok().map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::session_id_from_json;

    #[test]
    fn session_id_is_read_from_hook_json() {
        let json = r#"{"session_id":"abc-123","hook_event_name":"SessionEnd","reason":"clear"}"#;
        assert_eq!(session_id_from_json(json).as_deref(), Some("abc-123"));
        assert_eq!(
            session_id_from_json("\u{feff}  {\"session_id\":\"x\"}\n").as_deref(),
            Some("x")
        );
        assert_eq!(session_id_from_json("{}"), None);
        assert_eq!(session_id_from_json("not json"), None);
        assert_eq!(session_id_from_json(r#"{"session_id":"bad id"}"#), None);
        assert_eq!(session_id_from_json(r#"{"session_id":7}"#), None);
    }
}
