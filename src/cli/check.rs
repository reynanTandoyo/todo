//! `todo check` command: validate `TODO.md`, dependencies, milestones, config values, temp files, and rules (`R-CHECK-1`, `R-DEP-3`, `R-MILE-5`).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use clap::Args as ClapArgs;
use serde::Serialize;
use serde_json::json;

use crate::agents::{
    ManagedBlockStatus, RULES_TEXT, check_managed_block_in_text, upsert_managed_block,
};
use crate::atomic::{atomic_write_bytes, read_file_with_snapshot, validate_utf8};
use crate::cli::{CommandKind, CommandMeta, Context};
use crate::doc::{Document, FieldLocation, LineEdit, Status, Task, TaskChangeIntent, TaskId};
use crate::error::{ErrorCode, ExitCode, Result, TodoError};
use crate::find::fs_path;
use crate::history::{OutsideEditOutcome, detect_outside_edit};
use crate::lock::current_time;
use crate::ops::identity::{mailmap_proper_names, repo_root};
use crate::ops::scope::{outside_folder_reason, raw_entries};
use crate::ops::waves::detail_gaps;
use crate::ops::{
    DocWritePlan, OpsConfig, edit_doc_and_write, file_label, read_optional_archive,
    require_known_format,
};
use crate::output::Output;
use crate::state::StateDir;

/// Metadata for the `check` command.
pub const META: CommandMeta = CommandMeta {
    name: "check",
    summary: "Check TODO.md for format, dependency, claim, and state problems",
    kind: CommandKind::Read,
    details: "Reports problems as `file:line: message` and exits 1 if any are found, or 0 when clean. Pass `--fix` (with `--agent`) to make mechanical repairs only: canonical IDs and separators, missing `status: open`, leftover temporary files, and outdated rules blocks. A warning that does not change the exit code: each unfinished task at or below the target level whose detail is incomplete, with what it lacks (R-DETAIL-2).",
    examples: &[
        "todo check",
        "todo check --fix --agent agent-1",
        "todo check --json",
    ],
};

/// Arguments for `todo check`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Make mechanical repairs only (canonical IDs/separators, missing `status: open`, leftover `.tmp` files, outdated rules blocks).
    #[arg(long)]
    pub fix: bool,
}

/// One problem reported by `todo check` (`R-CHECK-1`, `R-OUT-4`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckProblem {
    /// File label or path where the problem was found.
    pub file: String,
    /// 1-based line number (`1` for file-level/temp-file problems).
    pub line: usize,
    /// Plain-English description of the problem.
    pub message: String,
}

impl CheckProblem {
    fn format(&self) -> String {
        format!("{}:{}: {}", self.file, self.line, self.message)
    }
}

#[derive(Serialize)]
struct CheckOkJson {
    clean: bool,
    count: usize,
    problems: Vec<CheckProblem>,
}

/// Runs `todo check`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    if args.fix {
        apply_mechanical_fixes(ctx, out)?;
    }

    let (label, problems) = collect_problems(ctx, out)?;

    if problems.is_empty() {
        if out.is_json() {
            out.emit_json_ok(
                META.name,
                &CheckOkJson {
                    clean: true,
                    count: 0,
                    problems: Vec::new(),
                },
            );
        } else {
            out.write_line(&format!("{label}: ok (0 problems)"));
        }
        return Ok(());
    }

    if !out.is_json() {
        for p in &problems {
            out.write_line(&p.format());
        }
    }

    let count = problems.len();
    Err(TodoError::new(
        ErrorCode::BadValue,
        format!("{count} problem(s) found"),
    )
    .with_exit_code(ExitCode::Refused)
    .with_detail("count", json!(count))
    .with_detail("problems", json!(problems)))
}

/// Runs all `check` validations on the active `TODO.md` and returns `(file_label, problems)`.
pub(crate) fn collect_problems(
    ctx: &Context,
    out: &mut Output,
) -> Result<(String, Vec<CheckProblem>)> {
    let found = ctx.find_todo(out)?;
    let cfg = OpsConfig::load_next_to(&found.path)?;
    let state = StateDir::from_found(&found)?;
    let label = file_label(&found.path);

    let read = read_file_with_snapshot(&found.path)?;
    if let OutsideEditOutcome::Recorded {
        warning: Some(warn),
    } = detect_outside_edit(&found.path, &label, &state, &read.snapshot, &cfg.lock)?
    {
        out.warn(warn);
    }

    let mut problems: Vec<CheckProblem> = Vec::new();

    match validate_utf8(&read.bytes, &label) {
        Err(err) => {
            problems.push(CheckProblem {
                file: label.clone(),
                line: err.details.line.unwrap_or(1),
                message: err.message,
            });
        }
        Ok(text) => {
            let doc = Document::parse_bytes_with_config(&read.bytes, cfg.id_config.clone())
                .map_err(|e| {
                    TodoError::new(ErrorCode::UnsafeFile, e.to_string())
                        .with_file(label.clone())
                        .with_line(e.line)
                })?;
            require_known_format(&doc, &label)?;

            // 1. Parser problems (conflict markers, unclosed fence/comment, unknown format version,
            //    duplicate IDs, repeated fields, unknown statuses, probable mistake headings).
            for p in &doc.problems {
                problems.push(CheckProblem {
                    file: label.clone(),
                    line: p.line,
                    message: p.message.clone(),
                });
            }

            // 2. Archive duplicate IDs and archive loading for dependency checks.
            let archive_path = cfg.archive_path(&found.path);
            let archive = read_optional_archive(&archive_path, cfg.id_config.clone())?;
            if let Some(ref arch) = archive {
                for dup in doc.check_archive_duplicates(arch) {
                    problems.push(CheckProblem {
                        file: label.clone(),
                        line: dup.line,
                        message: dup.message,
                    });
                }
            }

            // 3. Per-task semantic checks.
            let now = current_time();
            check_tasks(&doc, archive.as_ref(), &cfg, &label, now, &mut problems);

            // 3b. `by` names against `.mailmap` (warnings only; R-WRITE-16).
            warn_unmapped_by(&doc, &found.path, &label, out);

            // 3c. `files` entries that leave the folder of TODO.md (warnings only; R-SCOPE-1).
            warn_files_entries(&doc, &label, out);

            // 3d. Done tasks whose `verify` command has not passed yet (warnings only; R-VERIFY-2).
            warn_unverified_done(&doc, &cfg, &label, out);

            // 3e. Unfinished tasks at or below the target level whose detail is incomplete (warnings
            //     only; R-DETAIL-2).
            warn_detail_gaps(&doc, archive.as_ref(), &cfg, &label, out);

            // 4. Dependency checks (missing IDs, dropped dependencies, cycles; R-DEP-3).
            check_dependencies(&doc, archive.as_ref(), &label, &mut problems);

            // 4b. Milestone tree and reference checks (R-MILE-5, R-KIND-2).
            check_milestones(&doc, archive.as_ref(), &label, &mut problems);

            // 5. Outdated managed rules block in TODO.md (R-AGENT-3).
            if let ManagedBlockStatus::Outdated { begin_line, .. } =
                check_managed_block_in_text(text)
            {
                problems.push(CheckProblem {
                    file: label.clone(),
                    line: begin_line,
                    message:
                        "outdated agent rules block (run `todo agents install --yes` or update)"
                            .to_string(),
                });
            }
        }
    }

    // 6. Outdated managed rules block in sibling AGENTS.md / CLAUDE.md if present.
    if let Some(parent) = found.path.parent() {
        for filename in ["AGENTS.md", "CLAUDE.md"] {
            let candidate = parent.join(filename);
            if let Ok(content) = fs::read_to_string(fs_path(&candidate).as_ref())
                && let ManagedBlockStatus::Outdated { begin_line, .. } =
                    check_managed_block_in_text(&content)
            {
                problems.push(CheckProblem {
                    file: filename.to_string(),
                    line: begin_line,
                    message: "outdated agent rules block (run `todo agents install --yes`)"
                        .to_string(),
                });
            }
        }

        // 7. Leftover temporary files next to TODO.md or in the state folder.
        check_leftover_temps(parent, &label, &cfg.archive_file, &mut problems);
    }
    if fs_path(state.path()).exists() {
        check_state_temps(state.path(), &mut problems);
    }

    Ok((label, problems))
}

/// Per-task semantic checks (claims, owners, blocked reasons, configured values).
pub(crate) fn check_tasks(
    doc: &Document,
    _archive: Option<&Document>,
    cfg: &OpsConfig,
    label: &str,
    now: chrono::DateTime<chrono::FixedOffset>,
    problems: &mut Vec<CheckProblem>,
) {
    for task in &doc.tasks {
        let id_str = doc.id_config.format_id(task.id);
        let line = task.heading_line + 1;

        if task.status == Status::Claimed {
            if task.owner.is_none() {
                problems.push(CheckProblem {
                    file: label.to_string(),
                    line,
                    message: format!("{id_str}: claimed task has no owner"),
                });
            }
            if task.claimed.is_none() {
                problems.push(CheckProblem {
                    file: label.to_string(),
                    line,
                    message: format!("{id_str}: claimed task has no claimed timestamp"),
                });
            }
            if cfg.is_claim_stale(task, now) {
                let owner = task.owner.as_deref().unwrap_or("unknown");
                problems.push(CheckProblem {
                    file: label.to_string(),
                    line,
                    message: format!("{id_str}: stale claim by {owner}"),
                });
            }
        }

        if task.status == Status::Open && task.owner.is_some() {
            problems.push(CheckProblem {
                file: label.to_string(),
                line,
                message: format!("{id_str}: open task still has an owner"),
            });
        }

        // A problem like the other task state problems above: the reason is the one summary and
        // the HTML report show (T-142; `Task::blocked_reason`).
        if task.status == Status::Blocked && task.blocked_reason().is_none() {
            problems.push(CheckProblem {
                file: label.to_string(),
                line,
                message: format!("{id_str}: blocked task has no reason note"),
            });
        }

        if let Some(ref prio) = task.priority
            && !cfg.priorities.iter().any(|p| p.eq_ignore_ascii_case(prio))
        {
            problems.push(CheckProblem {
                file: label.to_string(),
                line,
                message: format!("{id_str}: priority {prio:?} is not in configured priorities"),
            });
        }

        if !cfg.types.is_empty()
            && let Some(ref ty) = task.task_type
            && !cfg.types.iter().any(|t| t.eq_ignore_ascii_case(ty))
        {
            problems.push(CheckProblem {
                file: label.to_string(),
                line,
                message: format!("{id_str}: type {ty:?} is not in configured types"),
            });
        }

        if !cfg.areas.is_empty()
            && let Some(ref area) = task.area
            && !cfg.areas.iter().any(|a| a.eq_ignore_ascii_case(area))
        {
            problems.push(CheckProblem {
                file: label.to_string(),
                line,
                message: format!("{id_str}: area {area:?} is not in configured areas"),
            });
        }

        // `difficulties` always has values (default 1 to 5), so there is no empty-list case. A done or
        // dropped entry keeps the difficulty it was finished with, as a record (format.md §18).
        if let Some(ref difficulty) = task.difficulty
            && !matches!(task.status, Status::Done | Status::Dropped)
            && !cfg
                .difficulties
                .iter()
                .any(|d| d.eq_ignore_ascii_case(difficulty))
        {
            problems.push(CheckProblem {
                file: label.to_string(),
                line,
                message: format!(
                    "{id_str}: difficulty {difficulty:?} is not in configured difficulties"
                ),
            });
        }
    }
}

/// Warns about each `files` entry that is absolute or leaves the folder of `TODO.md` (`R-SCOPE-1`,
/// `docs/format.md` §19). A warning only: it never changes the exit code (`R-CHECK-1`).
fn warn_files_entries(doc: &Document, label: &str, out: &mut Output) {
    for task in &doc.tasks {
        let Some(files) = task.files.as_deref() else {
            continue;
        };
        let id_str = doc.id_config.format_id(task.id);
        for entry in raw_entries(files) {
            if let Some(reason) = outside_folder_reason(entry) {
                out.warn(format!(
                    "{label}:{}: {id_str}: files entry {entry:?} {reason}",
                    task.heading_line + 1
                ));
            }
        }
    }
}

/// Warns about every `by` value that is not a proper name in the repository's `.mailmap`
/// (`R-WRITE-16`). Does nothing without a `.mailmap`; never changes the exit code.
fn warn_unmapped_by(doc: &Document, todo_path: &Path, label: &str, out: &mut Output) {
    let Some(root) = todo_path.parent().and_then(repo_root) else {
        return;
    };
    let Ok(bytes) = fs::read(fs_path(&root.join(".mailmap"))) else {
        return;
    };
    let names = mailmap_proper_names(&String::from_utf8_lossy(&bytes));
    for task in &doc.tasks {
        let Some(by) = task.field_value("by") else {
            continue;
        };
        if by.is_empty() || names.iter().any(|n| n.eq_ignore_ascii_case(by)) {
            continue;
        }
        out.warn(format!(
            "{label}:{}: {}: by {by:?} is not a name in .mailmap",
            task.heading_line + 1,
            doc.id_config.format_id(task.id)
        ));
    }
}

/// Warns about each done task that has a `verify` command and no `verified` line (`R-VERIFY-2`).
/// Nothing is warned about while `verify = false`, since no check can pass then. Never changes the
/// exit code.
fn warn_unverified_done(doc: &Document, cfg: &OpsConfig, label: &str, out: &mut Output) {
    if !cfg.verify_enabled {
        return;
    }
    for task in &doc.tasks {
        if task.status != Status::Done
            || task.verify.is_none()
            || task.field_value("verified").is_some()
        {
            continue;
        }
        out.warn(format!(
            "{label}:{}: {}: done, but its verify command has not passed (todo verify --wave)",
            task.heading_line + 1,
            doc.id_config.format_id(task.id)
        ));
    }
}

/// Warns about each unfinished task at or below the target level whose detail is incomplete, with
/// what it lacks (`R-DETAIL-2`): such a task is not handed out until the detail is there. Finished
/// entries are not checked, since no worker is handed them. Never changes the exit code.
fn warn_detail_gaps(
    doc: &Document,
    archive: Option<&Document>,
    cfg: &OpsConfig,
    label: &str,
    out: &mut Output,
) {
    for task in &doc.tasks {
        if task.status.is_finished() {
            continue;
        }
        let missing = detail_gaps(task, doc, archive, cfg);
        if missing.is_empty() {
            continue;
        }
        out.warn(format!(
            "{label}:{}: {}: not ready, its detail is incomplete: {}",
            task.heading_line + 1,
            doc.id_config.format_id(task.id),
            missing.join(", ")
        ));
    }
}

/// Dependency checks: missing IDs, dropped dependencies, cycles (`R-DEP-3`).
pub(crate) fn check_dependencies(
    doc: &Document,
    archive: Option<&Document>,
    label: &str,
    problems: &mut Vec<CheckProblem>,
) {
    let mut by_id: BTreeMap<TaskId, (Status, usize)> = BTreeMap::new();
    for t in &doc.tasks {
        by_id.entry(t.id).or_insert((t.status, t.heading_line + 1));
    }
    if let Some(arch) = archive {
        for t in &arch.tasks {
            by_id.entry(t.id).or_insert((t.status, t.heading_line + 1));
        }
    }

    for task in &doc.tasks {
        let id_str = doc.id_config.format_id(task.id);
        let task_line = task.heading_line + 1;
        for dep in &task.depends {
            let Some(dep_id) = dep.id else {
                problems.push(CheckProblem {
                    file: label.to_string(),
                    line: task_line,
                    message: format!("{id_str}: dependency {:?} does not exist", dep.raw),
                });
                continue;
            };
            let dep_str = doc.id_config.format_id(dep_id);
            match by_id.get(&dep_id) {
                None => {
                    problems.push(CheckProblem {
                        file: label.to_string(),
                        line: task_line,
                        message: format!("{id_str}: dependency {dep_str} does not exist"),
                    });
                }
                Some((Status::Dropped, _)) => {
                    problems.push(CheckProblem {
                        file: label.to_string(),
                        line: task_line,
                        message: format!(
                            "{id_str}: depends on dropped task {dep_str} (may need rethinking)"
                        ),
                    });
                }
                Some(_) => {}
            }
        }
    }

    // Cycle detection across active tasks in `doc` (R-DEP-3, R-MILE-6). A task waits for its own
    // `depends` and for those of every milestone above it. A milestone is finished only when the
    // entries below it are, so it waits for its direct children. A cycle is reported only when it
    // holds a dependency edge: a loop made of milestone parents alone is a milestone problem.
    let mut adj: BTreeMap<TaskId, (usize, Vec<TaskId>)> = BTreeMap::new();
    let mut dep_edges: BTreeSet<(TaskId, TaskId)> = BTreeSet::new();
    let index = doc.milestone_index();
    for t in &doc.tasks {
        if adj.contains_key(&t.id) {
            continue;
        }
        let mut edges: Vec<TaskId> = t.depends.iter().filter_map(|d| d.id).collect();
        // Only tasks are handed out, so only a task inherits the depends of the milestones above it.
        if !t.is_milestone() {
            for milestone in doc.milestone_ancestors(t) {
                edges.extend(milestone.depends.iter().filter_map(|d| d.id));
            }
        }
        dep_edges.extend(edges.iter().map(|&dep| (t.id, dep)));
        if t.is_milestone() {
            edges.extend(index.children_of(t.id).iter().map(|c| c.id));
        }
        adj.insert(t.id, (t.heading_line + 1, edges));
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Color {
        White,
        Gray,
        Black,
    }

    struct CycleCtx<'a> {
        adj: &'a BTreeMap<TaskId, (usize, Vec<TaskId>)>,
        dep_edges: &'a BTreeSet<(TaskId, TaskId)>,
        doc: &'a Document,
        label: &'a str,
        color: BTreeMap<TaskId, Color>,
        stack: Vec<TaskId>,
        reported: BTreeSet<Vec<TaskId>>,
        problems: &'a mut Vec<CheckProblem>,
    }

    fn dfs(u: TaskId, cx: &mut CycleCtx<'_>) {
        cx.color.insert(u, Color::Gray);
        cx.stack.push(u);

        if let Some((line, deps)) = cx.adj.get(&u) {
            for &v in deps {
                match cx.color.get(&v).copied() {
                    Some(Color::Gray) => {
                        if let Some(pos) = cx.stack.iter().position(|&x| x == v) {
                            let cycle = &cx.stack[pos..];
                            // `u` is on top of the stack, so the closing edge is `u -> v`.
                            let has_dependency = cx.dep_edges.contains(&(u, v))
                                || cycle
                                    .windows(2)
                                    .any(|w| cx.dep_edges.contains(&(w[0], w[1])));
                            let mut key = cycle.to_vec();
                            key.sort_unstable();
                            if has_dependency
                                && !is_parent_loop(cx.doc, cycle)
                                && cx.reported.insert(key)
                            {
                                let mut chain: Vec<String> = cycle
                                    .iter()
                                    .map(|&id| cx.doc.id_config.format_id(id))
                                    .collect();
                                chain.push(cx.doc.id_config.format_id(v));
                                cx.problems.push(CheckProblem {
                                    file: cx.label.to_string(),
                                    line: *line,
                                    message: format!("dependency cycle: {}", chain.join(" -> ")),
                                });
                            }
                        }
                    }
                    Some(Color::White) => dfs(v, cx),
                    _ => {}
                }
            }
        }

        cx.stack.pop();
        cx.color.insert(u, Color::Black);
    }

    let ids: Vec<TaskId> = adj.keys().copied().collect();
    let mut cx = CycleCtx {
        adj: &adj,
        dep_edges: &dep_edges,
        doc,
        label,
        color: adj.keys().map(|&id| (id, Color::White)).collect(),
        stack: Vec::new(),
        reported: BTreeSet::new(),
        problems,
    };
    for id in ids {
        if cx.color.get(&id).copied() == Some(Color::White) {
            dfs(id, &mut cx);
        }
    }
}

/// Whether `cycle` is exactly one loop of milestone parents: following `milestone` from any member
/// visits every member once and comes back to the start. `check_milestones` reports such loops
/// (`R-MILE-5`), so the dependency check leaves them out to report each cycle once.
fn is_parent_loop(doc: &Document, cycle: &[TaskId]) -> bool {
    let Some(&start) = cycle.first() else {
        return false;
    };
    let mut seen: BTreeSet<TaskId> = BTreeSet::new();
    let mut current = start;
    for _ in 0..cycle.len() {
        if !cycle.contains(&current) || !seen.insert(current) {
            return false;
        }
        let Some(parent) = doc
            .task_by_id(current)
            .and_then(|t| doc.milestone_parent(t))
            .filter(|p| p.is_milestone())
        else {
            return false;
        };
        current = parent.id;
    }
    current == start && seen.len() == cycle.len()
}

/// Milestone and reference checks (`R-MILE-5`, `R-KIND-2`): a `milestone` field naming a missing
/// ID or an entry that is not a milestone, a cycle of milestones, a milestone without a goal, a
/// done or dropped milestone with unfinished entries below it, an entry below an archived milestone
/// that is done or dropped, and an `about` field naming a missing ID.
///
/// An ID is missing only when neither `TODO.md` nor `TODO-archive.md` holds it (`R-DEP-3`). Each
/// problem is reported at the line that holds it: the `milestone` or `about` line for a bad
/// reference, the `milestone` line of the entry that closes a cycle, and the heading otherwise.
pub(crate) fn check_milestones(
    doc: &Document,
    archive: Option<&Document>,
    label: &str,
    problems: &mut Vec<CheckProblem>,
) {
    for task in &doc.tasks {
        let id_str = doc.id_config.format_id(task.id);
        let heading = task.heading_line + 1;

        if task.is_milestone() && task.goal.is_none() {
            problems.push(milestone_problem(
                label,
                heading,
                format!("{id_str}: milestone has no goal"),
            ));
        }

        if let Some(raw) = task.milestone.as_deref() {
            let line = task.milestone_loc.map_or(heading, |loc| loc.line() + 1);
            match lookup_entry(doc, archive, raw) {
                None => problems.push(milestone_problem(
                    label,
                    line,
                    format!("{id_str}: milestone {} does not exist", shown_ref(doc, raw)),
                )),
                Some(parent) if !parent.is_milestone() => problems.push(milestone_problem(
                    label,
                    line,
                    format!("{id_str}: {} is not a milestone", shown_ref(doc, raw)),
                )),
                Some(_) => {}
            }
        }

        if let Some(field) = task.unknown_fields.iter().find(|f| f.key == "about")
            && lookup_entry(doc, archive, &field.value).is_none()
        {
            problems.push(milestone_problem(
                label,
                field.line + 1,
                format!(
                    "{id_str}: about {} does not exist",
                    shown_ref(doc, &field.value)
                ),
            ));
        }
    }

    // Each entry names one parent, so a walk up the tree either ends or returns to its own path.
    // Only milestones are parents, so every cycle is made of milestones; each cycle is reported
    // once, at the `milestone` line of the entry that closes it.
    let mut settled: BTreeSet<TaskId> = BTreeSet::new();
    for start in &doc.tasks {
        let mut path: Vec<&Task> = Vec::new();
        let mut current = Some(start);
        while let Some(task) = current {
            if settled.contains(&task.id) {
                break;
            }
            if let Some(pos) = path.iter().position(|t| t.id == task.id) {
                let mut chain: Vec<String> = path[pos..]
                    .iter()
                    .map(|t| doc.id_config.format_id(t.id))
                    .collect();
                chain.push(doc.id_config.format_id(task.id));
                if let Some(closing) = path.last() {
                    let line = closing
                        .milestone_loc
                        .map_or(closing.heading_line, FieldLocation::line)
                        + 1;
                    problems.push(milestone_problem(
                        label,
                        line,
                        format!("milestone cycle: {}", chain.join(" -> ")),
                    ));
                }
                break;
            }
            path.push(task);
            current = doc.milestone_parent(task).filter(|p| p.is_milestone());
        }
        settled.extend(path.iter().map(|t| t.id));
    }

    let index = doc.milestone_index();
    for milestone in doc
        .tasks
        .iter()
        .filter(|t| t.is_milestone() && matches!(t.status, Status::Done | Status::Dropped))
    {
        let unfinished = index.unfinished_below(milestone.id);
        if unfinished.is_empty() {
            continue;
        }
        let listed: Vec<String> = unfinished
            .iter()
            .map(|t| format!("{} ({})", doc.id_config.format_id(t.id), t.status.as_str()))
            .collect();
        problems.push(milestone_problem(
            label,
            milestone.heading_line + 1,
            format!(
                "{}: {} milestone has unfinished entries below it: {}",
                doc.id_config.format_id(milestone.id),
                milestone.status.as_str(),
                listed.join(", ")
            ),
        ));
    }

    if let Some(arch) = archive {
        for task in doc
            .tasks
            .iter()
            .filter(|t| matches!(t.status, Status::Open | Status::Claimed | Status::Blocked))
        {
            if let Some(closed) = archived_closed_ancestor(doc, arch, task) {
                problems.push(milestone_problem(
                    label,
                    task.heading_line + 1,
                    format!(
                        "{}: milestone {} is archived as {}, but this entry is {}",
                        doc.id_config.format_id(task.id),
                        arch.id_config.format_id(closed.id),
                        closed.status.as_str(),
                        task.status.as_str()
                    ),
                ));
            }
        }
    }
}

fn milestone_problem(label: &str, line: usize, message: String) -> CheckProblem {
    CheckProblem {
        file: label.to_string(),
        line,
        message,
    }
}

/// The entry `raw` names as an ID, looking in `TODO.md` first and then in the archive (`R-DEP-3`).
fn lookup_entry<'a>(
    doc: &'a Document,
    archive: Option<&'a Document>,
    raw: &str,
) -> Option<&'a Task> {
    let id = doc.id_config.parse_user_id(raw)?;
    doc.task_by_id(id)
        .or_else(|| archive.and_then(|arch| arch.task_by_id(id)))
}

/// `raw` as a canonical ID when it parses, otherwise as the quoted text that was written.
fn shown_ref(doc: &Document, raw: &str) -> String {
    doc.id_config.parse_user_id(raw).map_or_else(
        || format!("{:?}", raw.trim()),
        |id| doc.id_config.format_id(id),
    )
}

/// The done or dropped milestone above `task` that lives in the archive, if any. The walk goes up
/// through `TODO.md` and then through the archive, and stops at a missing entry, a non-milestone
/// or a cycle.
fn archived_closed_ancestor<'a>(
    doc: &'a Document,
    archive: &'a Document,
    task: &Task,
) -> Option<&'a Task> {
    let mut seen: BTreeSet<TaskId> = BTreeSet::from([task.id]);
    let mut parent_id = doc.milestone_parent_id(task);
    while let Some(id) = parent_id {
        if !seen.insert(id) {
            return None;
        }
        if let Some(parent) = doc.task_by_id(id) {
            if !parent.is_milestone() {
                return None;
            }
            parent_id = doc.milestone_parent_id(parent);
        } else {
            let parent = archive.task_by_id(id)?;
            if !parent.is_milestone() {
                return None;
            }
            if matches!(parent.status, Status::Done | Status::Dropped) {
                return Some(parent);
            }
            parent_id = archive.milestone_parent_id(parent);
        }
    }
    None
}

fn check_leftover_temps(
    dir: &Path,
    todo_name: &str,
    archive_name: &str,
    problems: &mut Vec<CheckProblem>,
) {
    let Ok(entries) = fs::read_dir(fs_path(dir).as_ref()) else {
        return;
    };
    let todo_prefix = format!("{todo_name}.");
    let arch_prefix = format!("{archive_name}.");
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".tmp")
            && (name.starts_with(&todo_prefix) || name.starts_with(&arch_prefix))
        {
            problems.push(CheckProblem {
                file: name,
                line: 1,
                message: "leftover temporary file".to_string(),
            });
        }
    }
}

fn check_state_temps(state_dir: &Path, problems: &mut Vec<CheckProblem>) {
    let Ok(entries) = fs::read_dir(fs_path(state_dir).as_ref()) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".tmp") {
            problems.push(CheckProblem {
                file: entry.path().display().to_string(),
                line: 1,
                message: "leftover temporary file".to_string(),
            });
        }
    }
}

fn apply_mechanical_fixes(ctx: &Context, out: &mut Output) -> Result<()> {
    let _ = ctx.require_agent()?;
    let cfg = ctx.load_config()?;

    let write_res = edit_doc_and_write(ctx, &cfg, "check", out, |doc, _archive, _label, _agent, _now| {
        let mut edits: Vec<LineEdit> = Vec::new();
        let mut intent = TaskChangeIntent::default();

        for task in &doc.tasks {
            if !doc.blocking_problems_for_task(task.id).is_empty() {
                continue;
            }
            let canonical_id = cfg.id_config.format_id(task.id);
            let needs_heading = task.raw_id != canonical_id || task.separator != " \u{00b7} ";
            let needs_status = task.status_line.is_none();

            if needs_heading && needs_status {
                edits.push(LineEdit::replace_range(
                    task.heading_line..task.heading_line + 1,
                    vec![
                        format!("### {canonical_id} \u{00b7} {}", task.title),
                        "- status: open".to_string(),
                    ],
                ));
                intent.modified_ids.insert(task.id);
            } else if needs_heading {
                edits.push(LineEdit::replace_line(
                    task.heading_line,
                    format!("### {canonical_id} \u{00b7} {}", task.title),
                ));
                intent.modified_ids.insert(task.id);
            } else if needs_status {
                edits.push(LineEdit::insert(
                    task.heading_line + 1,
                    vec!["- status: open".to_string()],
                ));
                intent.modified_ids.insert(task.id);
            }
        }

        let doc_text = doc
            .line_doc
            .lines
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        if let ManagedBlockStatus::Outdated {
            begin_line,
            end_line,
        } = check_managed_block_in_text(&doc_text)
        {
            let canonical_lines: Vec<String> =
                RULES_TEXT.trim().lines().map(str::to_string).collect();
            edits.push(LineEdit::replace_range(
                begin_line..(end_line - 1),
                canonical_lines,
            ));
        }

        if edits.is_empty() {
            Ok(DocWritePlan::Unchanged(()))
        } else {
            Ok(DocWritePlan::Edits {
                edits,
                intent,
                build_value: Box::new(|_| Ok(())),
            })
        }
    })?;

    if !ctx.dry_run {
        let state = StateDir::from_found(&write_res.found)?;
        if let Some(parent) = write_res.found.path.parent() {
            let todo_name = file_label(&write_res.found.path);
            let todo_prefix = format!("{todo_name}.");
            let arch_prefix = format!("{}.", cfg.archive_file);
            if let Ok(entries) = fs::read_dir(fs_path(parent).as_ref()) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if name.ends_with(".tmp")
                        && (name.starts_with(&todo_prefix) || name.starts_with(&arch_prefix))
                    {
                        let _ = fs::remove_file(fs_path(&entry.path()).as_ref());
                    }
                }
            }

            for filename in ["AGENTS.md", "CLAUDE.md"] {
                let candidate = parent.join(filename);
                if let Ok(bytes) = fs::read(fs_path(&candidate).as_ref())
                    && let Ok(text) = std::str::from_utf8(&bytes)
                    && matches!(
                        check_managed_block_in_text(text),
                        ManagedBlockStatus::Outdated { .. }
                    )
                {
                    let updated = upsert_managed_block(Some(&bytes), filename)?;
                    atomic_write_bytes(&candidate, &updated)?;
                }
            }
        }

        if fs_path(state.path()).exists()
            && let Ok(entries) = fs::read_dir(fs_path(state.path()).as_ref())
        {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.ends_with(".tmp") {
                    let _ = fs::remove_file(fs_path(&entry.path()).as_ref());
                }
            }
        }
    }

    Ok(())
}
