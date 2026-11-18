//! `todo import` command: convert a Markdown checkbox list into format version 1 (`R-IMP-1`).

use std::path::{Path, PathBuf};

use clap::Args as ClapArgs;
use serde::Serialize;
use similar::TextDiff;

use crate::atomic::{atomic_replace, atomic_write_bytes, read_file_with_snapshot};
use crate::cli::{CommandKind, CommandMeta, Context};
use crate::doc::stamp_bytes;
use crate::error::{ErrorCode, Result, TodoError};
use crate::find::fs_path;
use crate::import::{ConvertedTask, ImportEnv, convert_checkbox};
use crate::lock::current_time;
use crate::ops::{OpsConfig, file_label, require_known_format_bytes};
use crate::output::Output;

/// Metadata for the `import` command.
pub const META: CommandMeta = CommandMeta {
    name: "import",
    summary: "Convert a Markdown checkbox list into a TODO.md",
    kind: CommandKind::Meta,
    details: "Converts the top-level `- [ ]`, `- [~]`, `- [!]` and `- [x]` lines under `##` sections of a Markdown checkbox list into tasks (open, claimed, blocked, done). The bold part of a line, or the text before ` — `, becomes the title and the rest a `description` field; trailing `(added: …)`, `(claimed: …)` and `(done: …)` become notes, owner and `commit`; `  - YYYY-MM-DD agent: text` sub-bullets become notes; tasks in sections named like \"decision\" or \"question\" get `type: question`. All other lines are kept byte for byte. Without `--yes` it prints the converted text (or a diff when the target exists) and writes nothing. `--out` defaults to the source file; an existing target is first saved as `<name>.pre-import`. The result is checked like `todo check`. Import does not take the write lock: use it on lists that agents are not using yet.",
    examples: &[
        "todo import --from checkbox notes.md",
        "todo import --from checkbox notes.md --out TODO.md --yes",
        "todo import --from checkbox \"C:\\My Notes\\plan.md\" --yes",
    ],
};

/// Arguments for `todo import`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Source format (only `checkbox` is supported).
    #[arg(long, value_name = "FORMAT", value_parser = ["checkbox"], required = true)]
    pub from: String,

    /// The Markdown checkbox list to convert.
    #[arg(value_name = "SOURCE")]
    pub source: PathBuf,

    /// Where to write the result (default: the source file itself).
    #[arg(long, value_name = "PATH")]
    pub out: Option<PathBuf>,
}

#[derive(Serialize)]
struct ImportJson {
    source: String,
    target: String,
    written: bool,
    backup: Option<String>,
    checkbox_lines: usize,
    tasks: usize,
    problems: usize,
    converted: Vec<ConvertedTask>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diff: Option<String>,
}

fn backup_path(target: &Path) -> PathBuf {
    let mut name = target.file_name().unwrap_or_default().to_os_string();
    name.push(".pre-import");
    target.with_file_name(name)
}

/// Runs `todo import`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let target = args.out.clone().unwrap_or_else(|| args.source.clone());
    let label = file_label(&target);
    let cfg = OpsConfig::load_next_to(&target)?;
    let source = read_file_with_snapshot(&args.source)?;
    let agent = ctx.agent.as_deref().unwrap_or("import");

    let env = ImportEnv {
        cfg: &cfg,
        agent,
        now: current_time(),
        label: &label,
    };
    let mut converted = convert_checkbox(&source.bytes, &env)?;
    // The imported TODO.md carries the tool stamp like any file this tool writes (R-VER-1).
    converted.bytes = stamp_bytes(&converted.bytes, cfg.id_config.clone()).map_err(|e| {
        TodoError::new(ErrorCode::WriteFailed, e.to_string()).with_file(label.clone())
    })?;

    let existing = match read_file_with_snapshot(&target) {
        Ok(read) => Some(read),
        Err(e) if e.code == ErrorCode::NoFile => None,
        Err(e) => return Err(e),
    };
    // An import would replace a file in a newer format; refuse it (R-VER-2).
    if let Some(old) = &existing {
        require_known_format_bytes(&old.bytes, &label, &cfg.id_config)?;
    }

    let write = ctx.yes && !ctx.dry_run;
    let backup = existing.as_ref().map(|_| backup_path(&target));
    if write
        && let Some(b) = &backup
        && fs_path(b).as_ref().exists()
    {
        return Err(TodoError::new(
            ErrorCode::UnsafeFile,
            format!(
                "{} already exists; move it away first so no older backup is lost",
                b.display()
            ),
        )
        .with_file(b.display().to_string()));
    }

    let new_text = String::from_utf8_lossy(&converted.bytes).into_owned();
    let diff = existing.as_ref().map(|old| {
        TextDiff::from_lines(String::from_utf8_lossy(&old.bytes).as_ref(), &new_text)
            .unified_diff()
            .header(&format!("a/{label}"), &format!("b/{label}"))
            .to_string()
    });

    if write {
        if let (Some(old), Some(b)) = (&existing, &backup) {
            std::fs::copy(fs_path(&target).as_ref(), fs_path(b).as_ref()).map_err(|e| {
                TodoError::new(
                    ErrorCode::WriteFailed,
                    format!("failed saving backup {}: {e}", b.display()),
                )
                .with_file(b.display().to_string())
            })?;
            atomic_replace(&target, &old.snapshot, &converted.bytes)?;
        } else {
            if let Some(parent) = target.parent()
                && !parent.as_os_str().is_empty()
            {
                std::fs::create_dir_all(fs_path(parent).as_ref()).map_err(|e| {
                    TodoError::new(
                        ErrorCode::WriteFailed,
                        format!("failed creating directory {}: {e}", parent.display()),
                    )
                })?;
            }
            atomic_write_bytes(&target, &converted.bytes)?;
        }
    }

    let summary = format!(
        "converted {} checkbox lines into {} tasks; todo check: ok (0 problems)",
        converted.checkbox_lines, converted.tasks
    );

    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &ImportJson {
                source: args.source.display().to_string(),
                target: target.display().to_string(),
                written: write,
                backup: backup
                    .filter(|_| write)
                    .map(|b| b.display().to_string()),
                checkbox_lines: converted.checkbox_lines,
                tasks: converted.tasks,
                problems: 0,
                converted: converted.converted,
                text: (!write && existing.is_none()).then(|| new_text.clone()),
                diff: if write { None } else { diff.clone() },
            },
        );
        return Ok(());
    }

    if write {
        if let Some(b) = &backup {
            out.write_line(&format!("saved the old {label} as {}", b.display()));
        }
        out.write_line(&format!("wrote {}", target.display()));
        out.write_line(&summary);
        return Ok(());
    }

    match &diff {
        Some(d) if d.is_empty() => out.write_line("no changes"),
        Some(d) => {
            out.write_str(d);
            if !d.ends_with('\n') {
                out.write_str("\n");
            }
        }
        None => out.write_str(&new_text),
    }
    out.info(&summary);
    out.info("nothing written; run again with --yes to write the result");
    Ok(())
}
