//! `todo completions <shell>`: print a tab-completion script (`R-OUT-10`).

use clap::{Args as ClapArgs, Command, CommandFactory};
use clap_complete::Shell;
use serde::Serialize;

use crate::cli::{Cli, CommandKind, CommandMeta, Context};
use crate::error::{ErrorCode, Result, TodoError};
use crate::output::Output;

/// Metadata for the `completions` command.
pub const META: CommandMeta = CommandMeta {
    name: "completions",
    summary: "Print a tab-completion script for powershell, bash, zsh, fish or elvish",
    kind: CommandKind::Meta,
    details: "Prints a completion script generated from the command definitions, so commands, options and their values complete with Tab. In PowerShell, add it to your profile (`notepad $PROFILE`): `todo completions powershell | Out-String | Invoke-Expression`. For bash use `source <(todo completions bash)`; zsh, fish and elvish scripts go into their usual completion folders.",
    examples: &[
        "todo completions powershell | Out-String | Invoke-Expression",
        "todo completions bash",
        "todo completions zsh",
    ],
};

/// Arguments for `todo completions`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// The shell to generate the script for.
    #[arg(value_enum, value_name = "SHELL")]
    pub shell: Shell,
}

#[derive(Serialize)]
struct CompletionsJson {
    shell: String,
    script: String,
}

/// The `todo` command without its hidden subcommands (`clap_complete` would list them).
fn visible_command() -> Command {
    let full = Cli::command();
    let mut cmd = Command::new("todo")
        .version(crate::cli::VERSION_STRING)
        .disable_help_subcommand(true)
        .args(full.get_arguments().cloned())
        .subcommands(full.get_subcommands().filter(|s| !s.is_hide_set()).cloned());
    if let Some(about) = full.get_about() {
        cmd = cmd.about(about.clone());
    }
    cmd
}

/// Builds the completion script of `shell` for the `todo` command line.
pub fn completion_script(shell: Shell) -> Result<String> {
    let mut cmd = visible_command();
    let mut buf: Vec<u8> = Vec::new();
    clap_complete::generate(shell, &mut cmd, "todo", &mut buf);
    String::from_utf8(buf).map_err(|e| {
        TodoError::new(
            ErrorCode::BadValue,
            format!("completion script is not valid UTF-8: {e}"),
        )
    })
}

/// Runs `todo completions`.
pub fn run(args: Args, _ctx: &Context, out: &mut Output) -> Result<()> {
    let script = completion_script(args.shell)?;
    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &CompletionsJson {
                shell: args.shell.to_string(),
                script,
            },
        );
    } else {
        out.write_str(&script);
        if !script.ends_with('\n') {
            out.write_str("\n");
        }
    }
    Ok(())
}
