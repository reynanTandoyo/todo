//! `todo check` command: validate `TODO.md`, dependencies, config values, temp files, and rules (`R-CHECK-1`, `R-DEP-3`).

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
use crate::doc::{Document, LineEdit, Status, TaskChangeIntent, TaskId};
use crate::error::{ErrorCode, ExitCode, Result, TodoError};
use crate::find::fs_path;
use crate::history::{OutsideEditOutcome, detect_outside_edit};
use crate::lock::current_time;
use crate::ops::identity::{mailmap_proper_names, repo_root};
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
    details: "Reports problems as `file:line: message` and exits 1 if any are found, or 0 when clean. Pass `--fix` (with `--agent`) to make mechanical repairs only: canonical IDs and separators, missing `status: open`, leftover temporary files, and outdated rules blocks.",
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

            // 4. Dependency checks (missing IDs, dropped dependencies, cycles; R-DEP-3).
            check_dependencies(&doc, archive.as_ref(), &label, &mut problems);

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

        if task.status == Status::Blocked
            && !task.notes.iter().any(|n| !n.text.trim().is_empty())
        {
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

    // Cycle detection across active tasks in `doc`.
    let mut adj: BTreeMap<TaskId, (usize, Vec<TaskId>)> = BTreeMap::new();
    for t in &doc.tasks {
        adj.entry(t.id).or_insert_with(|| {
            (
                t.heading_line + 1,
                t.depends.iter().filter_map(|d| d.id).collect(),
            )
        });
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Color {
        White,
        Gray,
        Black,
    }

    struct CycleCtx<'a> {
        adj: &'a BTreeMap<TaskId, (usize, Vec<TaskId>)>,
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
                            let mut key = cycle.to_vec();
                            key.sort_unstable();
                            if cx.reported.insert(key) {
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
