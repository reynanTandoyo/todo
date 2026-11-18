//! `todo init` command: initialize a new `TODO.md` with format line, managed rules block, and `## Tasks` (`R-AGENT-2`).

use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::PathBuf;

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::agents::render_managed_block;
use crate::cli::{CommandKind, CommandMeta, Context};
use crate::doc::tool_line_text;
use crate::error::{ErrorCode, Result, TodoError};
use crate::find::fs_path;
use crate::output::Output;
use crate::version::RUNNING_VERSION;

/// Metadata for the `init` command.
pub const META: CommandMeta = CommandMeta {
    name: "init",
    summary: "Create a new TODO.md with format line, agent rules, and ## Tasks",
    kind: CommandKind::Meta,
    details: "Creates a new TODO.md (or `--file <PATH>`) containing the format line, title, managed agent-rules block, and `## Tasks` section. Refuses if the file already exists. Prints the snippet to add to AGENTS.md.",
    examples: &[
        "todo init",
        "todo init --title \"My Project\"",
        "todo init --file docs/TODO.md",
    ],
};

/// Arguments for `todo init`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Document heading title (defaults to "Tasks").
    #[arg(long, default_value = "Tasks")]
    pub title: String,
}

#[derive(Serialize)]
struct InitJson {
    file: String,
    created: bool,
    agents_snippet: String,
}

/// Runs `todo init`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let target = ctx
        .file
        .clone()
        .unwrap_or_else(|| PathBuf::from("TODO.md"));
    let label = target.display().to_string();

    let title = args.title.trim();
    if title.is_empty() {
        return Err(TodoError::new(
            ErrorCode::BadValue,
            "--title must not be empty",
        ));
    }

    let managed = render_managed_block("\n");
    let stamp = tool_line_text(RUNNING_VERSION);
    let content = format!(
        "<!-- todo-format: 1 -->\n{stamp}\n# {title}\n\n{managed}\n\n## Tasks\n"
    );

    if let Some(parent) = target.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(fs_path(parent).as_ref()).map_err(|e| {
            TodoError::new(
                ErrorCode::WriteFailed,
                format!("failed creating directory {}: {e}", parent.display()),
            )
            .with_file(label.clone())
        })?;
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(fs_path(&target).as_ref())
        .map_err(|e| {
            if e.kind() == io::ErrorKind::AlreadyExists {
                TodoError::new(
                    ErrorCode::UnsafeFile,
                    format!("{label} already exists; refusing to overwrite"),
                )
                .with_file(label.clone())
            } else {
                TodoError::new(
                    ErrorCode::WriteFailed,
                    format!("failed creating {label}: {e}"),
                )
                .with_file(label.clone())
            }
        })?;

    file.write_all(content.as_bytes())
        .and_then(|()| file.flush())
        .map_err(|e| {
            TodoError::new(
                ErrorCode::WriteFailed,
                format!("failed writing {label}: {e}"),
            )
            .with_file(label.clone())
        })?;

    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &InitJson {
                file: label,
                created: true,
                agents_snippet: managed,
            },
        );
        return Ok(());
    }

    out.write_line(&format!("created {label}"));
    out.write_line("");
    out.write_line("Add these lines to AGENTS.md (or run `todo agents install --yes`):");
    out.write_line("");
    out.write_line(&managed);
    Ok(())
}
