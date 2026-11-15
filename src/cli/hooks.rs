//! `todo hooks` command: install or uninstall Claude Code hooks in `.claude/settings.json` (`T-053`; R-AGENT-4).

use std::fs;
use std::path::{Path, PathBuf};

use clap::{Args as ClapArgs, Subcommand};
use serde::Serialize;
use serde_json::{Map, Value, json};
use similar::TextDiff;

use crate::atomic::atomic_write_bytes;
use crate::cli::{CommandKind, CommandMeta, Context};
use crate::error::{ErrorCode, Result, TodoError};
use crate::find::fs_path;
use crate::ops::require_yes_or_preview;
use crate::output::Output;

const SETTINGS_REL_PATH: &str = ".claude/settings.json";
const BACKUP_REL_PATH: &str = ".claude/settings.json.bak";
const MANAGED_KEY: &str = "_managed_by";
const MANAGED_VAL: &str = "todo";

/// Metadata for the `hooks` command.
pub const META: CommandMeta = CommandMeta {
    name: "hooks",
    summary: "Install or uninstall Claude Code hooks in .claude/settings.json",
    kind: CommandKind::Meta,
    details: "`hooks install --yes` adds a `SessionStart` hook (`todo summary`), a `PreToolUse` hook (`todo guard` for `Edit|Write|MultiEdit`) and a `SessionEnd` hook (`todo session-end`, which releases the ending session's claims) to `.claude/settings.json`, keeping a backup and preserving formatting. `hooks uninstall --yes` removes only the entries added by `todo`.",
    examples: &[
        "todo hooks install",
        "todo hooks install --yes",
        "todo hooks uninstall --yes",
    ],
};

/// Arguments for `todo hooks`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Subcommand (`install` or `uninstall`).
    #[command(subcommand)]
    pub action: HooksAction,
}

/// Subcommands under `todo hooks`.
#[derive(Debug, Subcommand)]
pub enum HooksAction {
    /// Add `todo` SessionStart, PreToolUse and SessionEnd hooks to `.claude/settings.json`.
    Install,
    /// Remove only the `todo`-managed hooks from `.claude/settings.json`.
    Uninstall,
}

#[derive(Serialize)]
struct HooksJson {
    file: String,
    action: &'static str,
    applied: bool,
    changed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    backup: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diff: Option<String>,
}

/// Runs `todo hooks`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let settings_path = PathBuf::from(SETTINGS_REL_PATH);
    let backup_path = PathBuf::from(BACKUP_REL_PATH);
    let label = SETTINGS_REL_PATH.to_string();

    let old_bytes = match fs::read(fs_path(&settings_path).as_ref()) {
        Ok(b) => Some(b),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            return Err(TodoError::new(
                ErrorCode::WriteFailed,
                format!("could not read {label}: {e}"),
            )
            .with_file(label));
        }
    };

    let old_text = match &old_bytes {
        Some(b) => std::str::from_utf8(b).map_err(|e| {
            TodoError::new(
                ErrorCode::BadValue,
                format!("{label} is not valid UTF-8: {e}"),
            )
            .with_file(label.clone())
        })?,
        None => "",
    };

    let mut root: Value = if old_text.trim().is_empty() {
        Value::Object(Map::new())
    } else {
        serde_json::from_str(old_text).map_err(|e| {
            TodoError::new(
                ErrorCode::BadValue,
                format!("{label} is not valid JSON: {e}"),
            )
            .with_file(label.clone())
        })?
    };

    let action_name = match args.action {
        HooksAction::Install => {
            install_hooks(&mut root)?;
            "install"
        }
        HooksAction::Uninstall => {
            uninstall_hooks(&mut root)?;
            "uninstall"
        }
    };

    let new_text = format_preserving_style(old_text, &root)?;
    let new_bytes = new_text.as_bytes();
    let changed = old_bytes.as_deref() != Some(new_bytes);

    if !ctx.yes || ctx.dry_run {
        let diff = TextDiff::from_lines(old_text, &new_text)
            .unified_diff()
            .header(&format!("a/{label}"), &format!("b/{label}"))
            .to_string();

        if out.is_json() {
            out.set_dry_run_diff(diff.clone());
            out.emit_json_ok(
                META.name,
                &HooksJson {
                    file: label,
                    action: action_name,
                    applied: false,
                    changed,
                    backup: None,
                    diff: Some(diff),
                },
            );
            return Ok(());
        }

        if !changed {
            out.write_line(&format!("{label}: already up to date"));
            return Ok(());
        }

        let preview = if diff.is_empty() {
            format!("would update {label}")
        } else {
            diff
        };
        let _ = require_yes_or_preview(ctx, &preview, out);
        return Ok(());
    }

    let mut wrote_backup = None;
    if changed {
        if let Some(ref orig) = old_bytes {
            atomic_write_bytes(&backup_path, orig)?;
            wrote_backup = Some(BACKUP_REL_PATH.to_string());
        }
        if let Some(parent) = settings_path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(fs_path(parent).as_ref()).map_err(|e| {
                TodoError::new(
                    ErrorCode::WriteFailed,
                    format!("could not create {}: {e}", parent.display()),
                )
            })?;
        }
        atomic_write_bytes(&settings_path, new_bytes)?;
    }

    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &HooksJson {
                file: label,
                action: action_name,
                applied: true,
                changed,
                backup: wrote_backup,
                diff: None,
            },
        );
    } else if changed {
        out.write_line(&format!("{label}: {action_name} ok"));
    } else {
        out.write_line(&format!("{label}: already up to date"));
    }

    Ok(())
}

fn install_hooks(root: &mut Value) -> Result<()> {
    let root_obj = root.as_object_mut().ok_or_else(|| {
        TodoError::new(
            ErrorCode::BadValue,
            format!("{SETTINGS_REL_PATH} root must be a JSON object"),
        )
    })?;
    let hooks_val = root_obj
        .entry("hooks")
        .or_insert_with(|| Value::Object(Map::new()));
    let hooks_obj = hooks_val.as_object_mut().ok_or_else(|| {
        TodoError::new(
            ErrorCode::BadValue,
            "`hooks` in settings.json must be a JSON object",
        )
    })?;

    // 1. SessionStart -> todo summary
    let session_entry = json!({
        MANAGED_KEY: MANAGED_VAL,
        "hooks": [
            {
                "type": "command",
                "command": "todo summary"
            }
        ]
    });
    upsert_hook_entry(hooks_obj, "SessionStart", session_entry)?;

    // 2. PreToolUse (Edit|Write|MultiEdit) -> todo guard
    let guard_entry = json!({
        MANAGED_KEY: MANAGED_VAL,
        "matcher": "Edit|Write|MultiEdit",
        "hooks": [
            {
                "type": "command",
                "command": "todo guard"
            }
        ]
    });
    upsert_hook_entry(hooks_obj, "PreToolUse", guard_entry)?;

    // 3. SessionEnd -> todo session-end (releases the claims of the ending session; R-CLAIM-7)
    let end_entry = json!({
        MANAGED_KEY: MANAGED_VAL,
        "hooks": [
            {
                "type": "command",
                "command": "todo session-end"
            }
        ]
    });
    upsert_hook_entry(hooks_obj, "SessionEnd", end_entry)?;

    Ok(())
}

fn upsert_hook_entry(
    hooks_obj: &mut Map<String, Value>,
    event: &str,
    entry: Value,
) -> Result<()> {
    let arr_val = hooks_obj
        .entry(event)
        .or_insert_with(|| Value::Array(Vec::new()));
    let arr = arr_val.as_array_mut().ok_or_else(|| {
        TodoError::new(
            ErrorCode::BadValue,
            format!("`hooks.{event}` in settings.json must be a JSON array"),
        )
    })?;
    if let Some(existing) = arr.iter_mut().find(|v| is_managed_entry(v)) {
        *existing = entry;
    } else {
        arr.push(entry);
    }
    Ok(())
}

fn uninstall_hooks(root: &mut Value) -> Result<()> {
    let Some(root_obj) = root.as_object_mut() else {
        return Ok(());
    };
    let Some(hooks_val) = root_obj.get_mut("hooks") else {
        return Ok(());
    };
    let Some(hooks_obj) = hooks_val.as_object_mut() else {
        return Ok(());
    };

    for event in ["SessionStart", "PreToolUse", "SessionEnd"] {
        if let Some(Value::Array(arr)) = hooks_obj.get_mut(event) {
            arr.retain(|v| !is_managed_entry(v));
        }
        if hooks_obj
            .get(event)
            .and_then(|v| v.as_array())
            .is_some_and(Vec::is_empty)
        {
            hooks_obj.remove(event);
        }
    }
    if hooks_obj.is_empty() {
        root_obj.remove("hooks");
    }
    Ok(())
}

fn is_managed_entry(val: &Value) -> bool {
    val.as_object()
        .and_then(|m| m.get(MANAGED_KEY))
        .and_then(|v| v.as_str())
        == Some(MANAGED_VAL)
}

fn format_preserving_style(old_text: &str, value: &Value) -> Result<String> {
    let indent = detect_indent(old_text);
    let eol = if old_text.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let trailing_newline = if old_text.is_empty() {
        true
    } else {
        old_text.ends_with('\n')
    };

    let mut buf = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(indent.as_bytes());
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
    value.serialize(&mut ser).map_err(|e| {
        TodoError::new(
            ErrorCode::WriteFailed,
            format!("failed serializing {SETTINGS_REL_PATH}: {e}"),
        )
    })?;
    let mut rendered = String::from_utf8(buf).map_err(|e| {
        TodoError::new(
            ErrorCode::WriteFailed,
            format!("serialized JSON is not UTF-8: {e}"),
        )
    })?;
    if eol == "\r\n" {
        rendered = rendered.replace('\n', "\r\n");
    }
    if trailing_newline {
        rendered.push_str(eol);
    }
    Ok(rendered)
}

fn detect_indent(text: &str) -> String {
    for line in text.lines() {
        let ws: String = line
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();
        if !ws.is_empty() {
            return ws;
        }
    }
    "  ".to_string()
}

/// Ensures unused Path import warning doesn't trigger if refactored.
#[allow(dead_code)]
fn _settings_exists(dir: &Path) -> bool {
    dir.join(SETTINGS_REL_PATH).exists()
}
