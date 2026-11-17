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

use crate::doc::{Document, Task};
use crate::error::{ErrorCode, ExitCode, Result, TodoError};
use crate::find::{
    ExplicitSource, FindOptions, FoundBy, FoundTodo, WorktreeMode, find_local_worktree_toml,
    find_todo,
};
use crate::ops::edit_task_with_found_and_write;
use crate::ops::{
    NewlyReadyTask, OpEnv, OpsConfig, TaskJson, TaskOp, WriteTaskResult, edit_task_and_write,
};
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

    /// Agent session: tags new claims (overrides `TODO_SESSION` and `CLAUDE_CODE_SESSION_ID`);
    /// with `release`, releases every claim of that session.
    #[arg(long, global = true, value_name = "ID")]
    pub session: Option<String>,
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
    /// Session written on new claims: `--session`, else `TODO_SESSION`, else
    /// `CLAUDE_CODE_SESSION_ID` (`R-CLAIM-7`). Invalid values from the environment are ignored.
    pub session: Option<String>,
    /// The session given with `--session` only (what `release --session` acts on).
    pub session_arg: Option<String>,
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

        let session_arg = match &opts.session {
            Some(raw) => Some(validate_session_id(raw)?.to_owned()),
            None => None,
        };
        let session = session_arg.clone().or_else(|| {
            ["TODO_SESSION", "CLAUDE_CODE_SESSION_ID"]
                .into_iter()
                .filter_map(non_empty_env)
                .find(|raw| validate_session_id(raw).is_ok())
        });

        Ok(Self {
            file,
            file_source,
            json: opts.json,
            agent,
            dry_run: opts.dry_run,
            yes: opts.yes,
            force: opts.force,
            no_color,
            session,
            session_arg,
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
        let _t = crate::timing::PhaseTimer::start("ctx_load_config");
        let (_, cfg) = self.resolve_todo_and_config_quiet()?;
        Ok(cfg)
    }

    /// Locates `TODO.md` and loads `todo.toml` next to it without emitting discovery notices yet.
    pub(crate) fn load_config_with_found(&self) -> Result<(FoundTodo, OpsConfig)> {
        let _t = crate::timing::PhaseTimer::start("ctx_load_config");
        self.resolve_todo_and_config_quiet()
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

pub(crate) fn emit_find_notices(found: &FoundTodo, out: &mut Output) {
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

/// Validates an agent session ID (`R-CLAIM-7`): 1 to 128 characters from `A-Z`, `a-z`, `0-9`,
/// `.`, `_`, `-`, `:`. Claude Code's `session_id` (a UUID) fits.
pub fn validate_session_id(id: &str) -> Result<&str> {
    let ok = !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b':'));
    if ok {
        Ok(id)
    } else {
        Err(TodoError::new(
            ErrorCode::Usage,
            format!(
                "invalid session ID {id:?}: must be 1 to 128 characters from A-Z, a-z, 0-9, ., _, -, :"
            ),
        ))
    }
}

/// Loads the settings and the parsed `TODO.md` (or its archive) for a read command.
///
/// One discovery pass: `todo.toml` and `TODO.md` are located together, and the discovery notices
/// are printed once. Commands that must parse their own arguments before loading keep the two
/// steps apart; see `docs/refactoring.md`, chunk 3.
pub(crate) fn load_read(
    ctx: &Context,
    out: &mut Output,
    from_archive: bool,
    load_archive_for_deps: bool,
) -> Result<(OpsConfig, crate::ops::LoadedRead)> {
    let (found, cfg) = ctx.load_config_with_found()?;
    let loaded = crate::ops::load_for_read_with_found(
        found,
        &cfg,
        from_archive,
        load_archive_for_deps,
        out,
    )?;
    Ok((cfg, loaded))
}

/// Edits one task with a shared [`TaskOp`] and runs the write pipeline.
///
/// `found` is a [`FoundTodo`] already located together with `cfg` (see
/// [`Context::load_config_with_found`]); with `None` the pipeline locates the file itself.
pub(crate) fn edit_one_task(
    ctx: &Context,
    out: &mut Output,
    name: &str,
    task: &str,
    op: &TaskOp<'_>,
    cfg: &OpsConfig,
    found: Option<FoundTodo>,
) -> Result<WriteTaskResult<()>> {
    let edit = |doc: &Document, t: &Task, _archive: Option<&Document>, agent: &str, now| {
        let env = OpEnv {
            cfg,
            agent,
            force: ctx.force,
            now,
            session: ctx.session.as_deref(),
        };
        Ok((op.build_edits(doc, t, &env)?, ()))
    };
    match found {
        Some(found) => edit_task_with_found_and_write(ctx, found, cfg, name, task, out, edit),
        None => edit_task_and_write(ctx, cfg, name, task, out, edit),
    }
}

/// The whole body of a simple task command: load settings, run `op` on `task`, print the result.
pub(crate) fn run_task_op(
    ctx: &Context,
    out: &mut Output,
    name: &str,
    task: &str,
    op: &TaskOp<'_>,
) -> Result<()> {
    let cfg = ctx.load_config()?;
    let res = edit_one_task(ctx, out, name, task, op, &cfg, None)?;
    report_task_ok(out, name, &res);
    Ok(())
}

/// Prints the result of a task edit: the task as JSON, or `<id>: <name> ok`.
pub(crate) fn report_task_ok(out: &mut Output, name: &str, res: &WriteTaskResult<()>) {
    if out.is_json() {
        out.emit_json_ok(name, &res.task);
    } else {
        out.write_line(&format!("{}: {name} ok", res.task_id));
    }
}

#[derive(Serialize)]
struct TaskReadyJson<'a> {
    #[serde(flatten)]
    task: &'a TaskJson,
    #[serde(skip_serializing_if = "<[NewlyReadyTask]>::is_empty")]
    newly_ready: &'a [NewlyReadyTask],
}

/// Like [`report_task_ok`], also listing tasks that became ready (`done`, `drop`).
/// With `show_accept` the text output includes the task's `accept` line.
pub(crate) fn report_task_with_ready(
    out: &mut Output,
    name: &str,
    res: &WriteTaskResult<()>,
    show_accept: bool,
) {
    if out.is_json() {
        let json = TaskReadyJson {
            task: &res.task,
            newly_ready: &res.newly_ready,
        };
        out.emit_json_ok(name, &json);
        return;
    }
    out.write_line(&format!("{}: {name} ok", res.task_id));
    if show_accept && let Some(ref accept) = res.task.accept {
        out.write_line(&format!("accept: {accept}"));
    }
    for ready in &res.newly_ready {
        out.write_line(&format!("now ready: {} {}", ready.id, ready.title));
    }
}

#[cfg(feature = "test-hooks")]
pub mod test_append;

pub mod session_end;

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
            /// Hidden: the Claude Code `SessionEnd` hook (`R-CLAIM-7`).
            #[command(name = "session-end", hide = true)]
            SessionEnd(session_end::Args),
        }

        impl Command {
            /// Returns the static metadata for this subcommand.
            #[must_use]
            pub const fn meta(&self) -> &'static CommandMeta {
                match self {
                    $(Self::$variant(_) => &$module::META,)*
                    #[cfg(feature = "test-hooks")]
                    Self::TestAppend(_) => &test_append::META,
                    Self::SessionEnd(_) => &session_end::META,
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
                    Self::SessionEnd(args) => session_end::run(args, ctx, out),
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
    Completions => completions,
    Claim => claim,
    Diff => diff,
    Done => done,
    Drop => drop,
    Edit => edit,
    Guard => guard,
    Help => help,
    History => history,
    Hooks => hooks,
    Import => import,
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
    Schema => schema,
    Search => search,
    Set => set,
    Show => show,
    Skill => skill,
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
    let _total_timer = crate::timing::PhaseTimer::start("total");
    let parse_timer = crate::timing::PhaseTimer::start("cli_parse");
    let raw_args: Vec<OsString> = args.into_iter().map(Into::into).collect();
    let json_hint = raw_args.iter().any(|a| a == "--json");

    match Cli::try_parse_from(&raw_args) {
        Ok(cli) => {
            parse_timer.finish();
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
                    // clap's message starts with its own "error: "; the output layer adds
                    // one too, so strip clap's to avoid "error: error: ...".
                    let text = err.to_string();
                    let text = text.trim();
                    let message = text.strip_prefix("error: ").unwrap_or(text);
                    let todo_err = TodoError::new(ErrorCode::Usage, message);
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
