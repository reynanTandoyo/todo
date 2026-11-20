//! `todo help` command: overview and per-command help with examples (T-005; R-OUT-8).

pub mod update;

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::cli::{COMMANDS, CommandKind, CommandMeta, Context};
use crate::error::{ErrorCode, Result, TodoError};
use crate::output::Output;

/// Metadata for the `help` command.
pub const META: CommandMeta = CommandMeta {
    name: "help",
    summary: "Show overview or detailed help for a command",
    kind: CommandKind::Meta,
    details: "Without a topic, lists all available commands. With a command name, shows its description and usage examples. Use `todo help agents` to print the agent rules.",
    examples: &["todo help", "todo help help", "todo help agents"],
};

/// Arguments for `todo help`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Command or topic name (e.g. `help`, `agents`).
    #[arg(value_name = "COMMAND")]
    pub topic: Option<String>,
}

#[derive(Serialize)]
struct CommandSummaryJson {
    name: &'static str,
    summary: &'static str,
    kind: &'static str,
}

#[derive(Serialize)]
struct HelpListJson {
    version: &'static str,
    commands: Vec<CommandSummaryJson>,
}

#[derive(Serialize)]
struct HelpCommandJson {
    name: &'static str,
    summary: &'static str,
    details: String,
    examples: &'static [&'static str],
}

/// What the levels of `difficulty` mean (R-LEVEL-1). `todo help add` and `todo help set` print it
/// after their own text. It names no agent or model.
pub const DIFFICULTY_LEVELS: &str = concat!(
    "Levels (the task's difficulty, 1 to 5): how much of the work is still open, what the agent has\n",
    "to find out, guess or decide. Not how urgent the task is (that is the priority).\n",
    "  1  Everything is written down: the files to change (files), the files to look at (read),\n",
    "     numbered steps (steps), the check (verify) and the result (accept). Nothing is left to\n",
    "     explore, guess or decide.\n",
    "  2  Small and clear, inside one part of the code, but the agent has to find the exact places\n",
    "     and work out the steps itself.\n",
    "  3  Needs an understanding of one part of the code and a few decisions that stay inside the task.\n",
    "  4  Spans several parts of the code, or needs design decisions that affect other tasks.\n",
    "  5  Open: needs research, a design, or decisions only a person can make. It is split before\n",
    "     anyone works on it.\n",
    "A task needs a level: add refuses one without it, and next hands out only tasks that have one.",
);

/// The details `todo help <command>` shows: the command's own text and, for the commands that set a
/// task's difficulty, the levels.
fn details_of(meta: &CommandMeta) -> String {
    if matches!(meta.name, "add" | "set") {
        format!("{}\n\n{DIFFICULTY_LEVELS}", meta.details)
    } else {
        meta.details.to_string()
    }
}

#[derive(Serialize)]
struct HelpAgentsJson {
    topic: &'static str,
    rules: &'static str,
}

const fn kind_label(kind: CommandKind) -> &'static str {
    match kind {
        CommandKind::Read => "read",
        CommandKind::Write => "write",
        CommandKind::Meta => "meta",
    }
}

/// Runs `todo help [topic]`.
pub fn run(args: Args, _ctx: &Context, out: &mut Output) -> Result<()> {
    match args.topic.as_deref() {
        None => print_overview(out, "help"),
        Some(topic) if topic.eq_ignore_ascii_case("agents") => {
            if out.is_json() {
                out.emit_json_ok(
                    "help",
                    &HelpAgentsJson {
                        topic: "agents",
                        rules: crate::agents::RULES_TEXT,
                    },
                );
            } else {
                out.write_str(crate::agents::RULES_TEXT);
            }
            Ok(())
        }
        Some(topic) if topic.eq_ignore_ascii_case("update") => {
            update::emit(out, std::env::current_exe().ok().as_deref());
            Ok(())
        }
        Some(topic) => {
            if let Some(meta) = COMMANDS.iter().find(|c| c.name.eq_ignore_ascii_case(topic)) {
                print_command_help(meta, out);
                Ok(())
            } else {
                Err(TodoError::new(
                    ErrorCode::Usage,
                    format!("unknown help topic: {topic}"),
                ))
            }
        }
    }
}

/// Prints the short overview when `todo` is invoked with no subcommand or `todo help` is run.
pub fn print_overview(out: &mut Output, schema_cmd: &str) -> Result<()> {
    if out.is_json() {
        let commands = COMMANDS
            .iter()
            .map(|c| CommandSummaryJson {
                name: c.name,
                summary: c.summary,
                kind: kind_label(c.kind),
            })
            .collect();
        out.emit_json_ok(
            schema_cmd,
            &HelpListJson {
                version: env!("CARGO_PKG_VERSION"),
                commands,
            },
        );
        return Ok(());
    }

    out.write_line(&format!(
        "todo {} — concurrency-safe editor for TODO.md task lists",
        env!("CARGO_PKG_VERSION")
    ));
    out.write_line("");
    out.write_line("Usage: todo [OPTIONS] <COMMAND>");
    out.write_line("");
    out.write_line("Commands:");
    for cmd in COMMANDS {
        out.write_line(&format!("  {:<12} {}", cmd.name, cmd.summary));
    }
    out.write_line("");
    out.write_line("Global options:");
    out.write_line("  --file <PATH>   Path to TODO.md (or set TODO_FILE)");
    out.write_line("  --agent <NAME>  Agent name for writes (or set TODO_AGENT)");
    out.write_line("  --json          Print a single JSON envelope on stdout");
    out.write_line("  --dry-run       Show what would change without writing");
    out.write_line("  --yes           Confirm prompts non-interactively");
    out.write_line("  --force         Override another agent's claim");
    out.write_line("  --version       Print version information");
    out.write_line("");
    out.write_line("Run `todo help <command>` for details and examples.");
    Ok(())
}

fn print_command_help(meta: &CommandMeta, out: &mut Output) {
    let details = details_of(meta);
    if out.is_json() {
        out.emit_json_ok(
            "help",
            &HelpCommandJson {
                name: meta.name,
                summary: meta.summary,
                details,
                examples: meta.examples,
            },
        );
        return;
    }

    out.write_line(&format!("{} — {}", meta.name, meta.summary));
    out.write_line("");
    out.write_line(&details);
    if !meta.examples.is_empty() {
        out.write_line("");
        out.write_line("Examples:");
        for ex in meta.examples {
            out.write_line(&format!("  {ex}"));
        }
    }
}
