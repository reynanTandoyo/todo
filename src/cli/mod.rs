//! The command line: global options, help and the command registry (T-005).
//!
//! # Adding a command
//!
//! Each subcommand lives in its own module under `src/cli/<name>.rs` and exposes:
//! - `pub const META: CommandMeta`
//! - `pub struct Args` (deriving `clap::Args`)
//! - `pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()>`
//!
//! To register it, add one line `Variant => module,` inside [`register_commands!`] below.
//! Write commands (`CommandKind::Write`) automatically enforce `--agent` / `TODO_AGENT`
//! (`R-CLAIM-5`) before `run` is called.

use std::ffi::OsString;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};

use clap::{Args as ClapArgs, Parser, Subcommand, error::ErrorKind};
use serde::Serialize;

use crate::error::{ErrorCode, ExitCode, Result, TodoError};
use crate::find::{
    ExplicitSource, FindOptions, FoundBy, FoundTodo, WorktreeMode, find_local_worktree_toml,
    find_todo,
};
use crate::ops::OpsConfig;
use crate::output::Output;

/// Whether a command reads `TODO.md`, writes `TODO.md`, or is meta/help.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandKind {
    /// Reads `TODO.md` without modifying it.
    Read,
    /// Modifies `TODO.md` (requires an agent name per R-CLAIM-5).
    Write,
    /// Help, overview, or other non-file metadata command.
    Meta,
}

/// Static metadata for a registered subcommand.
#[derive(Debug, Clone, Copy)]
pub struct CommandMeta {
    /// Subcommand name as typed on the command line.
    pub name: &'static str,
    /// Short one-line description shown in `todo` and `todo help`.
    pub summary: &'static str,
    /// Command category (`Read`, `Write`, or `Meta`).
    pub kind: CommandKind,
    /// Detailed description shown by `todo help <command>`.
    pub details: &'static str,
    /// Usage examples shown by `todo help <command>`.
    pub examples: &'static [&'static str],
}

/// Global options accepted before or after the subcommand.
#[derive(Debug, Clone, ClapArgs)]
pub struct GlobalOpts {
    /// Path to `TODO.md` (overrides `TODO_FILE`).
    #[arg(long, global = true, value_name = "PATH")]
    pub file: Option<PathBuf>,

    /// Print a single JSON envelope on standard output.
    #[arg(long, global = true)]
    pub json: bool,

    /// Agent name for write commands (overrides `TODO_AGENT`).
    #[arg(long, global = true, value_name = "NAME")]
    pub agent: Option<String>,

    /// Preview changes without writing to disk.
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Confirm prompts non-interactively.
    #[arg(long, global = true)]
    pub yes: bool,

    /// Override another agent's claim.
    #[arg(long, global = true)]
    pub force: bool,
}

/// Resolved runtime context built from CLI flags, environment variables, and defaults (R-CFG-3).
#[derive(Debug, Clone)]
pub struct Context {
    /// Explicit path to `TODO.md` from `--file` or `TODO_FILE`, if any.
    pub file: Option<PathBuf>,
    /// Which source provided `file` (`--file` vs `TODO_FILE`), if any.
    pub file_source: Option<ExplicitSource>,
    /// Whether `--json` output mode is active.
    pub json: bool,
    /// Validated agent name from `--agent` or `TODO_AGENT`, if any.
    pub agent: Option<String>,
    /// Whether `--dry-run` was passed.
    pub dry_run: bool,
    /// Whether `--yes` was passed.
    pub yes: bool,
    /// Whether `--force` was passed.
    pub force: bool,
    /// Whether `NO_COLOR` is set and non-empty.
    pub no_color: bool,
}

impl Context {
    /// Resolves [`Context`] using R-CFG-3 precedence: CLI flags, then environment variables, then defaults.
    pub fn from_global(opts: &GlobalOpts) -> Result<Self> {
        let (file, file_source) = match &opts.file {
            Some(p) => (Some(p.clone()), Some(ExplicitSource::Flag)),
            None => match non_empty_env("TODO_FILE") {
                Some(raw) => (Some(PathBuf::from(raw)), Some(ExplicitSource::Env)),
                None => (None, None),
            },
        };

        let agent = match &opts.agent {
            Some(raw) => Some(validate_agent_name(raw)?.to_owned()),
            None => match non_empty_env("TODO_AGENT") {
                Some(raw) => Some(validate_agent_name(&raw)?.to_owned()),
                None => None,
            },
        };

        let no_color = non_empty_env("NO_COLOR").is_some();

        Ok(Self {
            file,
            file_source,
            json: opts.json,
            agent,
            dry_run: opts.dry_run,
            yes: opts.yes,
            force: opts.force,
            no_color,
        })
    }

    /// Returns the validated agent name, or `NO_AGENT` (exit 2) per R-CLAIM-5.
    pub fn require_agent(&self) -> Result<&str> {
        self.agent.as_deref().ok_or_else(|| {
            TodoError::new(
                ErrorCode::NoAgent,
                "missing agent name: pass --agent <name> or set TODO_AGENT",
            )
        })
    }

    /// Locates `TODO.md` using the resolved context and emits any discovery warnings/notices.
    ///
    /// Also checks `todo.toml` for `worktree = "local"` (`R-CFG-2`, `R-FIND-5`).
    pub fn find_todo(&self, out: &mut Output) -> Result<FoundTodo> {
        let (found, _) = self.find_todo_and_config(out)?;
        Ok(found)
    }

    /// Locates `TODO.md` and loads `todo.toml` next to it, respecting `worktree = "local"`.
    pub fn find_todo_and_config(&self, out: &mut Output) -> Result<(FoundTodo, OpsConfig)> {
        let (found, cfg) = self.resolve_todo_and_config_quiet()?;
        emit_find_notices(&found, out);
        Ok((found, cfg))
    }

    /// Loads `OpsConfig` from `todo.toml` next to `TODO.md` without emitting discovery notices.
    pub fn load_config(&self) -> Result<OpsConfig> {
        let (_, cfg) = self.resolve_todo_and_config_quiet()?;
        Ok(cfg)
    }

    fn resolve_todo_and_config_quiet(&self) -> Result<(FoundTodo, OpsConfig)> {
        if self.file.is_none()
            && let Some(local_toml) = find_local_worktree_toml(None)
        {
            let local_cfg = OpsConfig::load_from_file(&local_toml)?;
            if local_cfg.worktree == WorktreeMode::Local {
                let found = find_todo(&FindOptions {
                    explicit_file: None,
                    explicit_source: None,
                    start_dir: None,
                    worktree: WorktreeMode::Local,
                })?;
                let cfg = OpsConfig::load_next_to(&found.path)?;
                return Ok((found, cfg));
            }
        }

        let found = find_todo(&FindOptions {
            explicit_file: self.file.as_deref(),
            explicit_source: self.file_source,
            start_dir: None,
            worktree: WorktreeMode::Shared,
        })?;
        let cfg = OpsConfig::load_next_to(&found.path)?;
        if self.file.is_none()
            && cfg.worktree == WorktreeMode::Local
            && found.found_by == FoundBy::WorktreeMain
        {
            let local_found = find_todo(&FindOptions {
                explicit_file: None,
                explicit_source: None,
                start_dir: None,
                worktree: WorktreeMode::Local,
            })?;
            let local_cfg = OpsConfig::load_next_to(&local_found.path)?;
            return Ok((local_found, local_cfg));
        }
        Ok((found, cfg))
    }

    /// Locates `TODO.md` with an explicit [`WorktreeMode`] and emits any discovery warnings/notices.
    pub fn find_todo_with_mode(
        &self,
        worktree: WorktreeMode,
        out: &mut Output,
    ) -> Result<FoundTodo> {
        let found = find_todo(&FindOptions {
            explicit_file: self.file.as_deref(),
            explicit_source: self.file_source,
            start_dir: None,
            worktree,
        })?;
        emit_find_notices(&found, out);
        Ok(found)
    }
}

fn emit_find_notices(found: &FoundTodo, out: &mut Output) {
    for w in &found.warnings {
        out.warn(w);
    }
    if !out.is_json()
        && let Some(ref notice) = found.notice
    {
        let _ = writeln!(std::io::stderr(), "{notice}");
    }
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

/// Resolves a text option value: `-` reads standard input, `@path` reads a UTF-8 file,
/// and any other value is returned unchanged (`R-WRITE-12`).
pub fn resolve_text_arg(raw: &str) -> Result<String> {
    if raw == "-" {
        let mut buf = Vec::new();
        std::io::stdin().read_to_end(&mut buf).map_err(|e| {
            TodoError::new(
                ErrorCode::BadValue,
                format!("failed to read standard input: {e}"),
            )
        })?;
        let text = String::from_utf8(buf).map_err(|e| {
            TodoError::new(
                ErrorCode::BadValue,
                format!("standard input is not valid UTF-8: {e}"),
            )
        })?;
        return Ok(text.strip_prefix('\u{feff}').unwrap_or(&text).to_string());
    }
    if let Some(path_str) = raw.strip_prefix('@') {
        if path_str.is_empty() {
            return Err(TodoError::new(
                ErrorCode::Usage,
                "empty file path after '@' in text option",
            ));
        }
        let path = Path::new(path_str);
        let bytes = std::fs::read(crate::find::fs_path(path).as_ref()).map_err(|e| {
            TodoError::new(
                ErrorCode::NoFile,
                format!("could not read {}: {e}", path.display()),
            )
            .with_file(path.display().to_string())
        })?;
        let text = String::from_utf8(bytes).map_err(|e| {
            TodoError::new(
                ErrorCode::BadValue,
                format!("file {} is not valid UTF-8: {e}", path.display()),
            )
            .with_file(path.display().to_string())
        })?;
        return Ok(text.strip_prefix('\u{feff}').unwrap_or(&text).to_string());
    }
    Ok(raw.to_string())
}

/// Convenience wrapper around [`resolve_text_arg`] for optional text flags (`R-WRITE-12`).
pub fn resolve_opt_text_arg(raw: Option<&str>) -> Result<Option<String>> {
    match raw {
        Some(s) => Ok(Some(resolve_text_arg(s)?)),
        None => Ok(None),
    }
}

/// Validates an agent name per `docs/format.md` §11:
/// 1 to 64 characters from `A-Z`, `a-z`, `0-9`, `.`, `_`, `-`.
pub fn validate_agent_name(name: &str) -> Result<&str> {
    if name.is_empty() || name.len() > 64 {
        return Err(TodoError::new(
            ErrorCode::Usage,
            format!(
                "invalid agent name {name:?}: must be 1 to 64 characters from A-Z, a-z, 0-9, ., _, -"
            ),
        ));
    }
    if !name
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        return Err(TodoError::new(
            ErrorCode::Usage,
            format!("invalid agent name {name:?}: must contain only A-Z, a-z, 0-9, ., _, -"),
        ));
    }
    Ok(name)
}

#[cfg(feature = "test-hooks")]
pub mod test_append;

macro_rules! register_commands {
    ($($variant:ident => $module:ident),* $(,)?) => {
        $(pub mod $module;)*

        /// All registered subcommands.
        pub const COMMANDS: &[CommandMeta] = &[$($module::META),*];

        /// Parsed subcommand.
        #[derive(Debug, Subcommand)]
        pub enum Command {
            $(
                #[command(about = $module::META.summary)]
                $variant($module::Args),
            )*
            #[cfg(feature = "test-hooks")]
            #[command(name = "__test-append", hide = true)]
            TestAppend(test_append::Args),
        }

        impl Command {
            /// Returns the static metadata for this subcommand.
            #[must_use]
            pub const fn meta(&self) -> &'static CommandMeta {
                match self {
                    $(Self::$variant(_) => &$module::META,)*
                    #[cfg(feature = "test-hooks")]
                    Self::TestAppend(_) => &test_append::META,
                }
            }

            /// Dispatches the subcommand, enforcing `NO_AGENT` for write commands.
            pub fn run(self, ctx: &Context, out: &mut Output) -> Result<()> {
                if self.meta().kind == CommandKind::Write {
                    ctx.require_agent()?;
                }
                match self {
                    $(Self::$variant(args) => $module::run(args, ctx, out),)*
                    #[cfg(feature = "test-hooks")]
                    Self::TestAppend(args) => test_append::run(args, ctx, out),
                }
            }
        }
    };
}

// One-line command registration: add `Variant => module,` here.
register_commands! {
    Add => add,
    Agents => agents,
    Answer => answer,
    Apply => apply,
    Archive => archive,
    Ask => ask,
    Block => block,
    Check => check,
    Claim => claim,
    Diff => diff,
    Done => done,
    Drop => drop,
    Help => help,
    History => history,
    Init => init,
    List => list,
    Lock => lock,
    Log => log,
    Move => move_cmd,
    Next => next,
    Note => note,
    Release => release,
    Reopen => reopen,
    Restore => restore,
    Search => search,
    Set => set,
    Show => show,
    Status => status,
    Summary => summary,
    Unblock => unblock,
    Unlock => unlock,
    Where => where_cmd,
}

/// Version string including the build commit (`REL-7`).
pub const VERSION_STRING: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("TODO_GIT_COMMIT"),
    ")"
);

/// Top-level `todo` CLI parser.
#[derive(Debug, Parser)]
#[command(
    name = "todo",
    version = VERSION_STRING,
    about = "Concurrency-safe editor for TODO.md task lists",
    disable_help_subcommand = true
)]
pub struct Cli {
    /// Global options.
    #[command(flatten)]
    pub global: GlobalOpts,

    /// Subcommand to run; omit for a short overview.
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Serialize)]
struct VersionJson {
    version: &'static str,
    commit: &'static str,
}

/// Runs the CLI with process arguments and returns the [`ExitCode`].
#[must_use]
pub fn run_cli() -> ExitCode {
    run_cli_from(std::env::args_os())
}

/// Runs the CLI from an explicit argument iterator and returns the [`ExitCode`].
pub fn run_cli_from<I, T>(args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let raw_args: Vec<OsString> = args.into_iter().map(Into::into).collect();
    let json_hint = raw_args.iter().any(|a| a == "--json");

    match Cli::try_parse_from(&raw_args) {
        Ok(cli) => {
            let mut out = Output::stdio(cli.global.json);
            let cmd_name = cli.command.as_ref().map_or("overview", |c| c.meta().name);

            let exit = match Context::from_global(&cli.global) {
                Ok(ctx) => {
                    let result = match cli.command {
                        Some(cmd) => cmd.run(&ctx, &mut out),
                        None => help::print_overview(&mut out, "overview"),
                    };
                    match result {
                        Ok(()) => ExitCode::Ok,
                        Err(err) => {
                            out.emit_error(cmd_name, &err);
                            err.exit_code
                        }
                    }
                }
                Err(err) => {
                    out.emit_error(cmd_name, &err);
                    err.exit_code
                }
            };

            out.flush();
            if out.broken_pipe() {
                ExitCode::Ok
            } else {
                exit
            }
        }
        Err(err) => {
            let mut out = Output::stdio(json_hint);
            let exit = match err.kind() {
                ErrorKind::DisplayHelp => {
                    out.write_str(&err.render().to_string());
                    ExitCode::Ok
                }
                ErrorKind::DisplayVersion => {
                    if json_hint {
                        out.emit_json_ok(
                            "version",
                            &VersionJson {
                                version: env!("CARGO_PKG_VERSION"),
                                commit: env!("TODO_GIT_COMMIT"),
                            },
                        );
                    } else {
                        out.write_str(&err.render().to_string());
                    }
                    ExitCode::Ok
                }
                _ => {
                    let todo_err = TodoError::new(ErrorCode::Usage, err.to_string().trim());
                    out.emit_error("cli", &todo_err);
                    ExitCode::Usage
                }
            };
            out.flush();
            if out.broken_pipe() {
                ExitCode::Ok
            } else {
                exit
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_agent_names_accepted() {
        for name in ["alice", "agent-1", "Bot_2.0", "A", &"x".repeat(64)] {
            assert!(validate_agent_name(name).is_ok(), "expected valid: {name}");
        }
    }

    #[test]
    fn invalid_agent_names_rejected() {
        for name in ["", "has space", "bad/slash", "a@b", &"x".repeat(65)] {
            let err = validate_agent_name(name).unwrap_err();
            assert_eq!(err.code, ErrorCode::Usage);
            assert_eq!(err.exit_code, ExitCode::Usage);
        }
    }
}
