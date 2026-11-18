//! `todo skill` command: install the Claude Code skill from the built-in rules text (`T-054`; R-AGENT-5).

use std::fs;

use clap::{Args as ClapArgs, Subcommand};
use serde::Serialize;
use similar::TextDiff;

use crate::agents::project::SKILL_REL_PATH;
use crate::agents::render_claude_skill;
use crate::atomic::atomic_write_bytes;
use crate::cli::{CommandKind, CommandMeta, Context};
use crate::error::{ErrorCode, Result, TodoError};
use crate::find::fs_path;
use crate::ops::require_yes_or_preview;
use crate::output::Output;

/// Metadata for the `skill` command.
pub const META: CommandMeta = CommandMeta {
    name: "skill",
    summary: "Install the Claude Code skill for working with TODO.md",
    kind: CommandKind::Meta,
    details: "`skill install --yes` writes `.claude/skills/todo/SKILL.md` generated from the same built-in rules text as `todo help agents`. The file goes in the folder that holds `TODO.md`, whatever the current folder.",
    examples: &["todo skill install", "todo skill install --yes"],
};

/// Arguments for `todo skill`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Subcommand (`install`).
    #[command(subcommand)]
    pub action: SkillAction,
}

/// Subcommands under `todo skill`.
#[derive(Debug, Subcommand)]
pub enum SkillAction {
    /// Write `.claude/skills/todo/SKILL.md`.
    Install,
}

#[derive(Serialize)]
struct SkillInstallJson {
    file: String,
    applied: bool,
    changed: bool,
    created: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    diff: Option<String>,
}

/// Runs `todo skill`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    match args.action {
        SkillAction::Install => run_install(ctx, out),
    }
}

fn run_install(ctx: &Context, out: &mut Output) -> Result<()> {
    // The skill is in the folder of TODO.md, not the current folder (T-127).
    let target = ctx.project_dir(out)?.join(SKILL_REL_PATH);
    let label = SKILL_REL_PATH.to_string();

    let old_bytes = match fs::read(fs_path(&target).as_ref()) {
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
    let created = old_bytes.is_none();
    let new_text = render_claude_skill();
    let new_bytes = new_text.as_bytes();
    let changed = old_bytes.as_deref() != Some(new_bytes);

    if !ctx.yes || ctx.dry_run {
        let old_text = old_bytes
            .as_deref()
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default();
        let diff = TextDiff::from_lines(&old_text, &new_text)
            .unified_diff()
            .header(&format!("a/{label}"), &format!("b/{label}"))
            .to_string();

        if out.is_json() {
            out.set_dry_run_diff(diff.clone());
            out.emit_json_ok(
                META.name,
                &SkillInstallJson {
                    file: label,
                    applied: false,
                    changed,
                    created,
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
            format!("would write {label}")
        } else {
            diff
        };
        let _ = require_yes_or_preview(ctx, &preview, out);
        return Ok(());
    }

    if changed {
        if let Some(parent) = target.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(fs_path(parent).as_ref()).map_err(|e| {
                TodoError::new(
                    ErrorCode::WriteFailed,
                    format!("could not create {}: {e}", parent.display()),
                )
            })?;
        }
        atomic_write_bytes(&target, new_bytes)?;
    }

    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &SkillInstallJson {
                file: label,
                applied: true,
                changed,
                created,
                diff: None,
            },
        );
    } else if !changed {
        out.write_line(&format!("{label}: already up to date"));
    } else if created {
        out.write_line(&format!("{label}: created"));
    } else {
        out.write_line(&format!("{label}: updated"));
    }

    Ok(())
}
