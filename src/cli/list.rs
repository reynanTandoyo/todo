//! `todo list` command: list and filter tasks (T-020; R-READ-1, R-READ-4).

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::cli::{CommandKind, CommandMeta, Context};
use crate::doc::Task;
use crate::error::Result;
use crate::lock::current_time;
use crate::ops::{
    ListFilter, ListTaskJson, OpsConfig, StatusFilter, WhereClause, build_list_task_json,
    filter_tasks, load_for_read,
};
use crate::output::Output;

const DEFAULT_TERMINAL_WIDTH: usize = 100;
const MIN_TERMINAL_WIDTH: usize = 20;

/// Metadata for the `list` command.
pub const META: CommandMeta = CommandMeta {
    name: "list",
    summary: "List and filter tasks",
    kind: CommandKind::Read,
    details: "Prints one line per matching task (ID, priority, status, owner if any, and title) followed by a count. Done and dropped tasks are hidden unless `--status` or `--archive` is passed.",
    examples: &[
        "todo list",
        "todo list --status open,claimed --priority P0",
        "todo list --ready",
        "todo list --mine --agent alice",
        "todo list --where custom_tag=morning-shift",
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
    let cfg = OpsConfig::default();

    let status_filter = match args.status.as_deref() {
        Some(raw) => Some(StatusFilter::parse(raw)?),
        None => None,
    };

    let mut parsed_where = Vec::with_capacity(args.where_clauses.len());
    for raw in &args.where_clauses {
        parsed_where.push(WhereClause::parse(raw)?);
    }

    let mine_agent = if args.mine {
        Some(ctx.require_agent()?)
    } else {
        None
    };

    let loaded = load_for_read(ctx, &cfg, args.archive, !args.archive && args.ready, out)?;
    let now = current_time();

    let filter = ListFilter {
        status: status_filter,
        priority: args.priority.as_deref(),
        section: args.section.as_deref(),
        task_type: args.task_type.as_deref(),
        area: args.area.as_deref(),
        owner: args.owner.as_deref(),
        where_clauses: &parsed_where,
        ready_only: args.ready,
        mine_agent,
        stale_only: args.stale,
        archive_mode: args.archive,
    };

    let tasks = filter_tasks(&loaded.doc, loaded.archive.as_ref(), &filter, now, &cfg);

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

    let owner_width = tasks
        .iter()
        .filter_map(|t| t.owner.as_deref())
        .map(|o| o.chars().count())
        .max()
        .unwrap_or(0);
    let prio_width = tasks
        .iter()
        .filter_map(|t| t.priority.as_deref())
        .map(|p| p.chars().count())
        .max()
        .unwrap_or(1)
        .max(2);
    let max_width = if args.wide {
        None
    } else {
        Some(effective_terminal_width())
    };

    for task in &tasks {
        let line = format_task_line(task, &cfg, prio_width, owner_width, max_width);
        out.write_line(&line);
    }

    let count = tasks.len();
    let noun = if count == 1 { "task" } else { "tasks" };
    out.write_line(&format!("{count} {noun}"));
    Ok(())
}

fn format_task_line(
    task: &Task,
    cfg: &OpsConfig,
    prio_width: usize,
    owner_width: usize,
    max_width: Option<usize>,
) -> String {
    let id_str = cfg.id_config.format_id(task.id);
    let prio = task.priority.as_deref().unwrap_or("-");
    let status = task.status.as_str();

    let prefix = if owner_width > 0 {
        let owner = task.owner.as_deref().unwrap_or("-");
        format!("{id_str}  {prio:<prio_width$}  {status:<7}  {owner:<owner_width$}  ")
    } else {
        format!("{id_str}  {prio:<prio_width$}  {status:<7}  ")
    };

    match max_width {
        None => format!("{prefix}{}", task.title),
        Some(limit) => {
            let prefix_chars = prefix.chars().count();
            let avail = limit.saturating_sub(prefix_chars);
            let title = truncate_chars(&task.title, avail);
            format!("{prefix}{title}")
        }
    }
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }
    if max_chars == 0 {
        return String::new();
    }
    if max_chars == 1 {
        return "…".to_string();
    }
    let mut out: String = text.chars().take(max_chars - 1).collect();
    out.push('…');
    out
}

fn effective_terminal_width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|s| s.trim().parse::<usize>().ok())
        .filter(|&w| w > 0)
        .unwrap_or(DEFAULT_TERMINAL_WIDTH)
        .max(MIN_TERMINAL_WIDTH)
}
