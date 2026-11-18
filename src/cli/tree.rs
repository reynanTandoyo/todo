//! `todo tree` command: milestones and their tasks, with progress (T-088; R-MILE-3, R-READ-4, R-KIND-3).

use std::collections::BTreeSet;
use std::fmt::Write as _;

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::cli::{CommandKind, CommandMeta, Context, load_read};
use crate::doc::{Document, EntryKind, MilestoneIndex, Progress, Status, Task, TaskId};
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::OpsConfig;
use crate::ops::milestone::resolve_milestone;
use crate::ops::read::about_id;
use crate::output::Output;

/// Spaces per level of nesting in the text tree.
const INDENT: usize = 2;

/// Metadata for the `tree` command.
pub const META: CommandMeta = CommandMeta {
    name: "tree",
    summary: "Show milestones and their tasks as a tree, with progress",
    kind: CommandKind::Read,
    details: "Prints the milestone tree: each milestone with its goal and its progress (done/total tasks counted over every task below it, at any depth), its tasks indented below it, and top-level tasks last. Done and dropped entries are hidden unless `--all`; `--all` also shows the entries archived in `TODO-archive.md`, marked (archived). `<milestone>` shows that milestone and what is below it. `--area <list>` shows only the tasks in those areas, with the milestones above them. Without a milestone or `--area`, open questions and ideas are listed after the tree. `--json` returns the tree nested.",
    examples: &[
        "todo tree",
        "todo tree T-010",
        "todo tree --area cli,docs",
        "todo tree --all",
        "todo tree --json",
    ],
};

/// Arguments for `todo tree`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Show only this milestone (an ID or a title) and what is below it.
    pub milestone: Option<String>,

    /// Show only tasks in these areas (comma-separated), with the milestones above them.
    #[arg(long, value_name = "LIST")]
    pub area: Option<String>,

    /// Also show done and dropped entries, including those archived in `TODO-archive.md`.
    #[arg(long)]
    pub all: bool,
}

/// A milestone or task in the tree, as `tree --json` returns it (`R-MILE-3`).
#[derive(Debug, Serialize)]
struct TreeNode {
    id: String,
    title: String,
    /// `milestone` or `task`.
    kind: &'static str,
    status: &'static str,
    priority: Option<String>,
    difficulty: Option<String>,
    area: Option<String>,
    /// The goal of a milestone; `null` for a task.
    goal: Option<String>,
    /// Progress of a milestone; `null` for a task.
    progress: Option<Progress>,
    /// Whether the entry is in `TODO-archive.md` (shown with `--all`).
    archived: bool,
    /// Entries below this milestone, in file order; always empty for a task.
    children: Vec<TreeNode>,
}

/// A question or idea listed apart from the tree (`R-KIND-3`).
#[derive(Debug, Serialize)]
struct ApartEntry {
    id: String,
    title: String,
    status: &'static str,
    /// The entry the question or idea is about (`about`), if it names one.
    about: Option<String>,
    /// Whether the entry is in `TODO-archive.md` (shown with `--all`).
    archived: bool,
}

#[derive(Serialize)]
struct TreeJson {
    file: String,
    all: bool,
    /// The milestone the tree was asked for, by canonical ID.
    milestone: Option<String>,
    /// The areas the tree was limited to.
    area: Option<Vec<String>>,
    /// Milestones in `nodes`, at any depth.
    milestones: usize,
    /// Tasks in `nodes`, at any depth.
    tasks: usize,
    nodes: Vec<TreeNode>,
    /// Open questions; listed only for the whole tree (no milestone and no `--area`).
    questions: Vec<ApartEntry>,
    /// Open ideas; listed only for the whole tree (no milestone and no `--area`).
    ideas: Vec<ApartEntry>,
}

/// Which entries the tree shows (`R-MILE-3`).
#[derive(Clone, Copy)]
struct Selection<'a> {
    /// Done and dropped entries, and archived ones, are shown.
    all: bool,
    /// Only tasks in these areas (ignoring case); `None` for every task.
    areas: Option<&'a [String]>,
}

/// A milestone the user asked for: the document it is in, the milestone, and whether that document
/// is `TODO-archive.md`.
type Requested<'a> = (&'a Document, &'a Task, bool);

/// Runs `todo tree`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let areas = match args.area.as_deref() {
        Some(raw) => Some(parse_areas(raw)?),
        None => None,
    };

    // `--all` also shows archived entries, so it reads `TODO-archive.md` when the file exists.
    let (cfg, loaded) = load_read(ctx, out, false, args.all)?;
    let doc = &loaded.doc;
    let archive = if args.all {
        loaded.archive.as_ref()
    } else {
        None
    };
    let requested = match args.milestone.as_deref() {
        Some(raw) => Some(resolve_requested(doc, archive, raw)?),
        None => None,
    };
    let selection = Selection {
        all: args.all,
        areas: areas.as_deref(),
    };

    let mut nodes = Vec::new();
    match requested {
        Some((milestone_doc, root, archived)) => {
            nodes.extend(build_tree(milestone_doc, Some(root), selection, archived));
        }
        None => {
            nodes.extend(build_tree(doc, None, selection, false));
            if let Some(archived_doc) = archive {
                nodes.extend(build_tree(archived_doc, None, selection, true));
            }
        }
    }
    let (milestones, tasks) = count(&nodes);

    let mut questions = Vec::new();
    let mut ideas = Vec::new();
    if requested.is_none() && areas.is_none() {
        let sources = std::iter::once((doc, false)).chain(archive.map(|a| (a, true)));
        for (source, archived) in sources {
            questions.extend(apart_entries(
                source,
                args.all,
                EntryKind::Question,
                &cfg,
                archived,
            ));
            ideas.extend(apart_entries(
                source,
                args.all,
                EntryKind::Idea,
                &cfg,
                archived,
            ));
        }
    }

    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &TreeJson {
                file: loaded.file_label.clone(),
                all: args.all,
                milestone: requested.map(|(milestone_doc, root, _)| {
                    milestone_doc.id_config.format_id(root.id)
                }),
                area: areas,
                milestones,
                tasks,
                nodes,
                questions,
                ideas,
            },
        );
        return Ok(());
    }

    let text = render_text(&nodes, milestones, tasks, &questions, &ideas);
    out.write_str(&text);
    Ok(())
}

/// Splits a `--area` list such as `cli,docs`. An empty list is `BAD_VALUE`.
fn parse_areas(raw: &str) -> Result<Vec<String>> {
    let areas: Vec<String> = raw
        .split(',')
        .map(str::trim)
        .filter(|a| !a.is_empty())
        .map(str::to_string)
        .collect();
    if areas.is_empty() {
        return Err(TodoError::new(
            ErrorCode::BadValue,
            "--area needs one or more areas, separated by commas",
        ));
    }
    Ok(areas)
}

/// The milestone `raw` names: in `doc`, or else in the archive when there is one (`--all`).
fn resolve_requested<'a>(
    doc: &'a Document,
    archive: Option<&'a Document>,
    raw: &str,
) -> Result<Requested<'a>> {
    match resolve_milestone(doc, raw) {
        Err(err) if matches!(err.code, ErrorCode::NotFound) => match archive {
            Some(archived) => Ok((archived, resolve_milestone(archived, raw)?, true)),
            None => Err(err),
        },
        found => Ok((doc, found?, false)),
    }
}

/// Whether an entry with `status` is shown: done and dropped entries only with `--all`.
fn shows_status(status: Status, all: bool) -> bool {
    all || matches!(status, Status::Open | Status::Claimed | Status::Blocked)
}

/// Whether `task` is in one of `areas` (ignoring case); with no area list, every task is.
fn area_matches(task: &Task, areas: Option<&[String]>) -> bool {
    areas.is_none_or(|list| {
        task.area
            .as_deref()
            .is_some_and(|area| list.iter().any(|a| a.eq_ignore_ascii_case(area.trim())))
    })
}

/// Builds the tree for `doc`. With `requested`, only that milestone and what is below it; without,
/// the top-level milestones and then the top-level tasks, each in file order (`R-MILE-3`).
/// `archived` marks every node as coming from `TODO-archive.md`.
fn build_tree(
    doc: &Document,
    requested: Option<&Task>,
    selection: Selection<'_>,
    archived: bool,
) -> Vec<TreeNode> {
    let mut builder = TreeBuilder {
        doc,
        index: doc.milestone_index(),
        selection,
        archived,
        visited: BTreeSet::new(),
    };
    let mut nodes = Vec::new();
    if let Some(milestone) = requested {
        nodes.extend(builder.node(milestone, true));
        return nodes;
    }
    for task in doc
        .tasks
        .iter()
        .filter(|t| t.is_milestone() && doc.milestone_ancestors(t).is_empty())
    {
        nodes.extend(builder.node(task, false));
    }
    for task in doc
        .tasks
        .iter()
        .filter(|t| t.kind() == EntryKind::Task && doc.milestone_ancestors(t).is_empty())
    {
        nodes.extend(builder.node(task, false));
    }
    // Entries in a milestone cycle always have an ancestor, so no walk from the top reaches them.
    // Show them as roots rather than hide them; entries already shown are skipped.
    for task in doc
        .tasks
        .iter()
        .filter(|t| matches!(t.kind(), EntryKind::Task | EntryKind::Milestone))
    {
        nodes.extend(builder.node(task, false));
    }
    nodes
}

/// Builds nodes for the tree, visiting each entry once so a cycle of milestones cannot loop.
struct TreeBuilder<'a> {
    doc: &'a Document,
    /// The milestone tree, indexed once for the whole build (T-125).
    index: MilestoneIndex<'a>,
    selection: Selection<'a>,
    archived: bool,
    visited: BTreeSet<TaskId>,
}

impl<'a> TreeBuilder<'a> {
    /// Builds `task` and the entries below it. Returns `None` when nothing of it is shown. `root`
    /// marks the milestone the user asked for, which is always shown.
    fn node(&mut self, task: &'a Task, root: bool) -> Option<TreeNode> {
        if !self.visited.insert(task.id) {
            return None;
        }
        let all = self.selection.all;
        if task.is_milestone() {
            let entries: Vec<&'a Task> = self.index.children_of(task.id).to_vec();
            let children: Vec<TreeNode> = entries
                .into_iter()
                .filter(|child| matches!(child.kind(), EntryKind::Task | EntryKind::Milestone))
                .filter_map(|child| self.node(child, false))
                .collect();
            // Without an area list, a milestone is shown with its status; with one, only when a
            // task under it matches (the milestones above the tasks shown).
            let own = root || (self.selection.areas.is_none() && shows_status(task.status, all));
            if !own && children.is_empty() {
                return None;
            }
            return Some(self.node_for(task, children));
        }
        if task.kind() != EntryKind::Task
            || !shows_status(task.status, all)
            || !area_matches(task, self.selection.areas)
        {
            return None;
        }
        Some(self.node_for(task, Vec::new()))
    }

    fn node_for(&self, task: &Task, children: Vec<TreeNode>) -> TreeNode {
        let milestone = task.is_milestone();
        TreeNode {
            id: self.doc.id_config.format_id(task.id),
            title: task.title.clone(),
            kind: task.kind().as_str(),
            status: task.status.as_str(),
            priority: task.priority.clone(),
            difficulty: task.difficulty.clone(),
            area: task.area.clone(),
            goal: if milestone { task.goal.clone() } else { None },
            progress: milestone.then(|| self.index.progress(task.id)),
            archived: self.archived,
            children,
        }
    }
}

/// Counts the milestones and tasks in `nodes`, at any depth.
fn count(nodes: &[TreeNode]) -> (usize, usize) {
    nodes.iter().fold((0, 0), |(milestones, tasks), node| {
        let (below_milestones, below_tasks) = count(&node.children);
        if node.kind == EntryKind::Milestone.as_str() {
            (milestones + 1 + below_milestones, tasks + below_tasks)
        } else {
            (milestones + below_milestones, tasks + 1 + below_tasks)
        }
    })
}

/// The open questions or ideas of `kind`, in file order, listed apart from the tree (`R-KIND-3`).
fn apart_entries(
    doc: &Document,
    all: bool,
    kind: EntryKind,
    cfg: &OpsConfig,
    archived: bool,
) -> Vec<ApartEntry> {
    doc.tasks
        .iter()
        .filter(|t| t.kind() == kind && shows_status(t.status, all))
        .map(|t| ApartEntry {
            id: doc.id_config.format_id(t.id),
            title: t.title.clone(),
            status: t.status.as_str(),
            about: about_id(t, cfg).map(|id| doc.id_config.format_id(id)),
            archived,
        })
        .collect()
}

fn render_text(
    nodes: &[TreeNode],
    milestones: usize,
    tasks: usize,
    questions: &[ApartEntry],
    ideas: &[ApartEntry],
) -> String {
    let mut text = render_tree(nodes);
    let _ = writeln!(
        text,
        "{}, {}",
        plural(milestones, "milestone", "milestones"),
        plural(tasks, "task", "tasks")
    );
    if !questions.is_empty() || !ideas.is_empty() {
        text.push('\n');
    }
    render_apart(&mut text, "questions", questions);
    render_apart(&mut text, "ideas", ideas);
    text
}

/// One line per milestone or task, indented by depth, with the goal of each milestone under it.
/// Priority, difficulty and status line up in columns; the area and progress follow the title.
fn render_tree(nodes: &[TreeNode]) -> String {
    let mut rows: Vec<(usize, &TreeNode)> = Vec::new();
    flatten(nodes, 0, &mut rows);

    let first_width = rows
        .iter()
        .map(|(depth, node)| depth * INDENT + char_len(&node.id))
        .max()
        .unwrap_or(0);
    let priority_width = rows
        .iter()
        .map(|(_, node)| char_len(priority_text(node)))
        .max()
        .unwrap_or(1);
    let difficulty_width = rows
        .iter()
        .filter_map(|(_, node)| node.difficulty.as_deref())
        .map(char_len)
        .max()
        .unwrap_or(0);
    let status_width = rows
        .iter()
        .map(|(_, node)| char_len(node.status))
        .max()
        .unwrap_or(0);

    let mut text = String::new();
    for (depth, node) in rows {
        let indent = " ".repeat(depth * INDENT);
        push_padded(&mut text, &format!("{indent}{}", node.id), first_width);
        text.push_str("  ");
        push_padded(&mut text, priority_text(node), priority_width);
        text.push_str("  ");
        if difficulty_width > 0 {
            push_padded(
                &mut text,
                node.difficulty.as_deref().unwrap_or("-"),
                difficulty_width,
            );
            text.push_str("  ");
        }
        push_padded(&mut text, node.status, status_width);
        text.push_str("  ");
        text.push_str(&node.title);
        if let Some(area) = node.area.as_deref() {
            let _ = write!(text, "  [{area}]");
        }
        if let Some(progress) = node.progress {
            let _ = write!(text, "  {}/{} done", progress.done, progress.total);
        }
        if node.archived {
            text.push_str("  (archived)");
        }
        text.push('\n');
        if let Some(goal) = node.goal.as_deref() {
            let _ = writeln!(text, "{indent}  goal: {goal}");
        }
    }
    text
}

fn flatten<'n>(nodes: &'n [TreeNode], depth: usize, rows: &mut Vec<(usize, &'n TreeNode)>) {
    for node in nodes {
        rows.push((depth, node));
        flatten(&node.children, depth + 1, rows);
    }
}

fn render_apart(text: &mut String, label: &str, entries: &[ApartEntry]) {
    if entries.is_empty() {
        return;
    }
    let _ = writeln!(text, "{label} ({}):", entries.len());
    for entry in entries {
        let about = entry
            .about
            .as_deref()
            .map(|id| format!(" (about {id})"))
            .unwrap_or_default();
        let archived = if entry.archived { "  (archived)" } else { "" };
        let _ = writeln!(
            text,
            "  {} [{}] · {}{about}{archived}",
            entry.id, entry.status, entry.title
        );
    }
}

fn priority_text(node: &TreeNode) -> &str {
    node.priority.as_deref().unwrap_or("-")
}

fn plural(count: usize, one: &str, many: &str) -> String {
    let noun = if count == 1 { one } else { many };
    format!("{count} {noun}")
}

fn char_len(text: &str) -> usize {
    text.chars().count()
}

fn push_padded(buf: &mut String, text: &str, width: usize) {
    buf.push_str(text);
    for _ in char_len(text)..width {
        buf.push(' ');
    }
}
