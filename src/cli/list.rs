//! `todo list` command: list and filter tasks (T-020; R-READ-1, R-READ-4).

use std::fmt::Write as _;

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::cli::{CommandKind, CommandMeta, Context};
use crate::doc::{EntryKind, Task};
use crate::error::Result;
use crate::lock::current_time;
use crate::ops::milestone::resolve_milestone;
use crate::ops::{
    ListFilter, ListTaskJson, OpsConfig, StatusFilter, WhereClause, build_list_task_json,
    filter_tasks, load_for_read_with_found,
};
use crate::output::Output;

const DEFAULT_TERMINAL_WIDTH: usize = 100;
const MIN_TERMINAL_WIDTH: usize = 20;

/// Metadata for the `list` command.
pub const META: CommandMeta = CommandMeta {
    name: "list",
    summary: "List and filter tasks",
    kind: CommandKind::Read,
    details: "Prints one line per matching task (ID, priority, status, owner if any, and title) followed by a count. Done and dropped tasks are hidden unless `--status`, `--by` or `--archive` is passed. `--by <name>` lists the tasks that person finished (the `by` field written by `done`, case-insensitive). `--kind <list>` keeps the given kinds (milestone, task, question, idea). `--milestone <task>` keeps the entries below that milestone, at any depth.",
    examples: &[
        "todo list",
        "todo list --status open,claimed --priority P0",
        "todo list --ready",
        "todo list --mine --agent alice",
        "todo list --where custom_tag=morning-shift",
        "todo list --by alice",
        "todo list --kind milestone",
        "todo list --milestone T-010",
        "todo list --archive",
    ],
};

/// Arguments for `todo list`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Filter by status (comma-separated list, or `all`).
    #[arg(long, value_name = "LIST")]
    pub status: Option<String>,

    /// Filter by priority (e.g. `P0`).
    #[arg(long, value_name = "PRIORITY")]
    pub priority: Option<String>,

    /// Filter by difficulty: a comma-separated list of configured difficulties (`R-WRITE-18`).
    #[arg(long, value_name = "LIST")]
    pub difficulty: Option<String>,

    /// Filter by section name (ignores leading numbers and case).
    #[arg(long, value_name = "SECTION")]
    pub section: Option<String>,

    /// Filter by task type.
    #[arg(long = "type", value_name = "TYPE")]
    pub task_type: Option<String>,

    /// Filter by task area.
    #[arg(long, value_name = "AREA")]
    pub area: Option<String>,

    /// Filter by owner agent name.
    #[arg(long, value_name = "AGENT")]
    pub owner: Option<String>,

    /// Filter by the person who finished the task (`by` field; case-insensitive; implies `--status all`).
    #[arg(long, value_name = "NAME")]
    pub by: Option<String>,

    /// Filter by any field (`key=value`, case-insensitive; repeatable).
    #[arg(long = "where", value_name = "KEY=VALUE")]
    pub where_clauses: Vec<String>,

    /// Show only tasks that `next` could hand out.
    #[arg(long)]
    pub ready: bool,

    /// Show only tasks claimed by `--agent` / `TODO_AGENT`.
    #[arg(long)]
    pub mine: bool,

    /// Show only tasks with stale claims.
    #[arg(long)]
    pub stale: bool,

    /// Filter by kind: a comma-separated list of `milestone`, `task`, `question` and `idea` (`R-KIND-3`).
    #[arg(long, value_name = "LIST")]
    pub kind: Option<String>,

    /// Show only the entries below this milestone, at any depth (an ID or a title; `R-MILE-3`).
    #[arg(long, value_name = "MILESTONE")]
    pub milestone: Option<String>,

    /// List tasks from `TODO-archive.md` instead of `TODO.md`.
    #[arg(long)]
    pub archive: bool,

    /// Do not shorten titles to the terminal width.
    #[arg(long)]
    pub wide: bool,
}

#[derive(Serialize)]
struct ListJson {
    archive: bool,
    count: usize,
    tasks: Vec<ListTaskJson>,
}

/// Runs `todo list`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let (found, cfg) = ctx.load_config_with_found()?;

    // `--by` looks at finished work, so it implies `--status all` unless a status is given (R-WRITE-16).
    let status_filter = match args.status.as_deref() {
        Some(raw) => Some(StatusFilter::parse(raw)?),
        None if args.by.is_some() => Some(StatusFilter::All),
        None => None,
    };

    let mut parsed_where = Vec::with_capacity(args.where_clauses.len());
    for raw in &args.where_clauses {
        parsed_where.push(WhereClause::parse(raw)?);
    }
    if let Some(name) = args.by.as_deref() {
        parsed_where.push(WhereClause {
            key: "by".to_string(),
            value: name.trim().to_string(),
        });
    }

    let mine_agent = if args.mine {
        Some(ctx.require_agent()?)
    } else {
        None
    };

    let kinds = match args.kind.as_deref() {
        Some(raw) => Some(EntryKind::parse_kind_list(raw)?),
        None => None,
    };

    let loaded =
        load_for_read_with_found(found, &cfg, args.archive, !args.archive && args.ready, out)?;
    let now = current_time();

    let milestone = match args.milestone.as_deref() {
        Some(raw) => Some(resolve_milestone(&loaded.doc, raw)?.id),
        None => None,
    };

    let difficulties = match args.difficulty.as_deref() {
        Some(raw) => Some(cfg.difficulty_list(raw)?),
        None => None,
    };
    let filter = ListFilter {
        status: status_filter,
        priority: args.priority.as_deref(),
        section: args.section.as_deref(),
        difficulties: difficulties.as_deref(),
        task_type: args.task_type.as_deref(),
        area: args.area.as_deref(),
        owner: args.owner.as_deref(),
        where_clauses: &parsed_where,
        ready_only: args.ready,
        mine_agent,
        stale_only: args.stale,
        archive_mode: args.archive,
        kinds: kinds.as_deref(),
        milestone,
    };

    let t_filter = crate::timing::PhaseTimer::start("list.filter");
    let tasks = filter_tasks(&loaded.doc, loaded.archive.as_ref(), &filter, now, &cfg);
    t_filter.finish();

    if out.is_json() {
        let task_jsons: Vec<ListTaskJson> = tasks
            .iter()
            .map(|t| build_list_task_json(&loaded.doc, t, loaded.archive.as_ref(), now, &cfg))
            .collect();
        out.emit_json_ok(
            META.name,
            &ListJson {
                archive: args.archive,
                count: task_jsons.len(),
                tasks: task_jsons,
            },
        );
        return Ok(());
    }

    let t_out = crate::timing::PhaseTimer::start("list.format_and_print");

    let owner_width = tasks
        .iter()
        .filter_map(|t| t.owner.as_deref())
        .map(char_len)
        .max()
        .unwrap_or(0);
    let prio_width = tasks
        .iter()
        .filter_map(|t| t.priority.as_deref())
        .map(char_len)
        .max()
        .unwrap_or(1)
        .max(2);
    // The difficulty column is shown only when a listed task has a difficulty (like `owner`).
    let difficulty_width = tasks
        .iter()
        .filter_map(|t| t.difficulty.as_deref())
        .map(char_len)
        .max()
        .unwrap_or(0);
    let max_width = if args.wide {
        None
    } else {
        Some(effective_terminal_width())
    };

    let use_color = out.use_color();
    let mut buf = String::with_capacity(16_500);
    let mut id_buf = String::with_capacity(16);

    for task in &tasks {
        write_task_line_into(
            &mut buf,
            task,
            &cfg,
            prio_width,
            difficulty_width,
            owner_width,
            max_width,
            use_color,
            &mut id_buf,
        );
        if buf.len() >= 16_384 {
            out.write_str(&buf);
            buf.clear();
            if out.broken_pipe() {
                break;
            }
        }
    }

    let count = tasks.len();
    let noun = if count == 1 { "task" } else { "tasks" };
    let _ = writeln!(buf, "{count} {noun}");
    out.write_str(&buf);
    t_out.finish();
    Ok(())
}

#[inline]
fn char_len(s: &str) -> usize {
    if s.is_ascii() {
        s.len()
    } else {
        s.chars().count()
    }
}

#[inline]
fn push_spaces(out: &mut String, count: usize) {
    const SPACES: &str = "                                                                ";
    let mut rem = count;
    while rem > 0 {
        let chunk = rem.min(SPACES.len());
        out.push_str(&SPACES[..chunk]);
        rem -= chunk;
    }
}

#[allow(clippy::too_many_arguments)]
fn write_task_line_into(
    out: &mut String,
    task: &Task,
    cfg: &OpsConfig,
    prio_width: usize,
    difficulty_width: usize,
    owner_width: usize,
    max_width: Option<usize>,
    use_color: bool,
    id_buf: &mut String,
) {
    id_buf.clear();
    let _ = write!(
        id_buf,
        "{}-{:0width$}",
        cfg.id_config.prefix,
        task.id.number(),
        width = cfg.id_config.width
    );
    let prio = task.priority.as_deref().unwrap_or("-");
    let status = task.status.as_str();
    let id_chars = char_len(id_buf);
    let prio_chars = char_len(prio);
    let prio_pad = prio_width.saturating_sub(prio_chars);

    if use_color {
        out.push_str("\x1b[36m");
        out.push_str(id_buf);
        out.push_str("\x1b[0m  ");
    } else {
        out.push_str(id_buf);
        out.push_str("  ");
    }

    out.push_str(prio);
    push_spaces(out, prio_pad + 2);

    if difficulty_width > 0 {
        let difficulty = task.difficulty.as_deref().unwrap_or("-");
        out.push_str(difficulty);
        push_spaces(out, difficulty_width.saturating_sub(char_len(difficulty)) + 2);
    }

    let status_code = match task.status {
        crate::doc::Status::Done => "32",
        crate::doc::Status::Claimed | crate::doc::Status::Blocked => "33",
        crate::doc::Status::Dropped => "31",
        crate::doc::Status::Open => "2",
    };
    let status_pad = 7usize.saturating_sub(status.len());
    if use_color {
        out.push_str("\x1b[");
        out.push_str(status_code);
        out.push('m');
        out.push_str(status);
        push_spaces(out, status_pad);
        out.push_str("\x1b[0m  ");
    } else {
        out.push_str(status);
        push_spaces(out, status_pad + 2);
    }

    let mut plain_prefix_chars = id_chars + 2 + prio_chars + prio_pad + 2 + 7 + 2;
    if difficulty_width > 0 {
        plain_prefix_chars += difficulty_width + 2;
    }
    if owner_width > 0 {
        let owner = task.owner.as_deref().unwrap_or("-");
        let owner_chars = char_len(owner);
        let owner_pad = owner_width.saturating_sub(owner_chars);
        out.push_str(owner);
        push_spaces(out, owner_pad + 2);
        plain_prefix_chars += owner_chars + owner_pad + 2;
    }

    match max_width {
        None => out.push_str(&task.title),
        Some(limit) => {
            let avail = limit.saturating_sub(plain_prefix_chars);
            push_truncated_chars(out, &task.title, avail);
        }
    }
    out.push('\n');
}

fn push_truncated_chars(out: &mut String, text: &str, max_chars: usize) {
    if text.len() <= max_chars {
        out.push_str(text);
        return;
    }
    if text.is_ascii() {
        if max_chars == 0 {
            return;
        }
        if max_chars == 1 {
            out.push('…');
            return;
        }
        out.push_str(&text[..max_chars - 1]);
        out.push('…');
        return;
    }
    if text.chars().count() <= max_chars {
        out.push_str(text);
        return;
    }
    if max_chars == 0 {
        return;
    }
    if max_chars == 1 {
        out.push('…');
        return;
    }
    for c in text.chars().take(max_chars - 1) {
        out.push(c);
    }
    out.push('…');
}

fn effective_terminal_width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|s| s.trim().parse::<usize>().ok())
        .filter(|&w| w > 0)
        .unwrap_or(DEFAULT_TERMINAL_WIDTH)
        .max(MIN_TERMINAL_WIDTH)
}
