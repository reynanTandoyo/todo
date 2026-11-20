//! `todo plan` command: what the planner still has to give back, split, detail, level or check
//! before every task can go to a worker (T-149; R-LEVEL-2, R-READ-4).

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::cli::{CommandKind, CommandMeta, Context, load_read};
use crate::doc::{NO_REASON, Task};
use crate::error::{ExitCode, Result};
use crate::ops::milestone::resolve_milestone;
use crate::ops::plan::{Group, build_plan};
use crate::ops::read::level_problem;
use crate::ops::waves::{OpenQuestionTargets, detail_gaps_with};
use crate::output::Output;

/// Metadata for the `plan` command.
pub const META: CommandMeta = CommandMeta {
    name: "plan",
    summary: "List what the planner still has to give back, split, detail, level or check",
    kind: CommandKind::Read,
    details: "Lists, in this order, what the planner still has to do before every task can go to a worker (R-LEVEL-2): tasks a worker gave back (`todo stuck`), with the problem and the attempts; tasks above the target level (`target_difficulty`, by default the lowest level), hardest first; tasks at or below the target level that are not ready for lack of detail, with what is missing (R-DETAIL-2); tasks without a difficulty. Two groups are counted apart and do not change the exit code: tasks kept above the target level on purpose with `final` (R-LEVEL-3), and done tasks with a `verify` command and no `verified` line (R-VERIFY-2; not listed while `verify = false`). Each task is listed in the first group it fits. Done, dropped and claimed entries, milestones, ideas and questions are left out. `<milestone>` shows only the entries below it. Exit 0 when the first four groups are empty, 1 otherwise. Reads only; takes no lock.",
    examples: &["todo plan", "todo plan T-010", "todo plan --json"],
};

/// Arguments for `todo plan`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Show only the entries below this milestone (an ID or a title).
    pub milestone: Option<String>,
}

/// A task that a worker gave back, with its problem and its attempts (`R-STUCK-1`, group 1).
#[derive(Debug, Serialize)]
struct GivenBackEntry {
    id: String,
    title: String,
    level: Option<String>,
    problem: Option<String>,
    attempts: Option<String>,
}

/// A task above the target level (group 2).
#[derive(Debug, Serialize)]
struct SplitEntry {
    id: String,
    title: String,
    level: Option<String>,
    priority: Option<String>,
}

/// A task at or below the target level whose detail is incomplete, with what is missing (group 3).
#[derive(Debug, Serialize)]
struct DetailEntry {
    id: String,
    title: String,
    level: Option<String>,
    missing: Vec<String>,
}

/// A task without a usable difficulty, with the reason (group 4).
#[derive(Debug, Serialize)]
struct LevelessEntry {
    id: String,
    title: String,
    priority: Option<String>,
    status: &'static str,
    /// Why the level is not usable: no difficulty, or one that is not configured (`R-LEVEL-1`).
    reason: Option<String>,
}

/// A task kept above the target level on purpose, with its reason (group 5).
#[derive(Debug, Serialize)]
struct KeptEntry {
    id: String,
    title: String,
    level: Option<String>,
    reason: Option<String>,
}

/// A done task whose `verify` command has not passed, with the command (group 6).
#[derive(Debug, Serialize)]
struct UnverifiedEntry {
    id: String,
    title: String,
    verify: Option<String>,
}

/// The planner's list as `plan --json` returns it (`todo.plan.v1`).
#[derive(Debug, Serialize)]
struct PlanJson {
    file: String,
    /// The milestone the list is limited to, or `null` for the whole file.
    milestone: Option<String>,
    /// Whether groups 1 to 4 are empty, which is exit 0.
    clean: bool,
    given_back: Vec<GivenBackEntry>,
    to_split: Vec<SplitEntry>,
    to_detail: Vec<DetailEntry>,
    without_level: Vec<LevelessEntry>,
    kept_on_purpose: Vec<KeptEntry>,
    unverified: Vec<UnverifiedEntry>,
}

/// Runs `todo plan`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let (cfg, loaded) = load_read(ctx, out, false, true)?;
    let doc = &loaded.doc;
    let archive = loaded.archive.as_ref();
    let milestone = match args.milestone.as_deref() {
        Some(raw) => Some(resolve_milestone(doc, raw)?.id),
        None => None,
    };
    let plan = build_plan(doc, archive, &cfg, milestone);
    let questions = OpenQuestionTargets::new(doc, archive, &cfg);
    let id = |t: &Task| doc.id_config.format_id(t.id);

    let payload = PlanJson {
        file: loaded.file_label.clone(),
        milestone: milestone.map(|m| doc.id_config.format_id(m)),
        clean: plan.is_clean(),
        given_back: plan
            .given_back
            .iter()
            .map(|t| GivenBackEntry {
                id: id(t),
                title: t.title.clone(),
                level: t.difficulty.clone(),
                problem: t.blocked_reason().map(str::to_string),
                attempts: t.attempts.clone(),
            })
            .collect(),
        to_split: plan
            .to_split
            .iter()
            .map(|t| SplitEntry {
                id: id(t),
                title: t.title.clone(),
                level: t.difficulty.clone(),
                priority: t.priority.clone(),
            })
            .collect(),
        to_detail: plan
            .to_detail
            .iter()
            .map(|t| DetailEntry {
                id: id(t),
                title: t.title.clone(),
                level: t.difficulty.clone(),
                missing: detail_gaps_with(t, &cfg, &questions),
            })
            .collect(),
        without_level: plan
            .without_level
            .iter()
            .map(|t| LevelessEntry {
                reason: level_problem(t, &cfg),
                id: id(t),
                title: t.title.clone(),
                priority: t.priority.clone(),
                status: t.status.as_str(),
            })
            .collect(),
        kept_on_purpose: plan
            .kept_on_purpose
            .iter()
            .map(|t| KeptEntry {
                id: id(t),
                title: t.title.clone(),
                level: t.difficulty.clone(),
                reason: t.final_reason.clone(),
            })
            .collect(),
        unverified: plan
            .unverified
            .iter()
            .map(|t| UnverifiedEntry {
                id: id(t),
                title: t.title.clone(),
                verify: t.verify.clone(),
            })
            .collect(),
    };
    // Read-only: the output needs nothing from the documents any more (T-186).
    loaded.leak_for_exit();

    if out.is_json() {
        out.emit_json_ok(META.name, &payload);
    } else {
        for line in text_lines(&payload) {
            out.write_line(&line);
        }
    }
    if !payload.clean {
        out.set_exit(ExitCode::Refused);
    }
    Ok(())
}

/// One section of the text output: its heading with the count, then one row per task. An empty
/// section is left out. The counted-apart groups say so in the heading (R-LEVEL-2).
fn push_section(lines: &mut Vec<String>, group: Group, heading: &str, rows: Vec<String>) {
    if rows.is_empty() {
        return;
    }
    let apart = if group.is_counted_apart() {
        ", counted apart"
    } else {
        ""
    };
    lines.push(format!("{heading} ({}){apart}:", rows.len()));
    lines.extend(rows.into_iter().map(|row| format!("  {row}")));
}

/// The text output: the groups in the order of the list, the empty ones left out.
fn text_lines(list: &PlanJson) -> Vec<String> {
    let mut lines = Vec::new();
    let level = |value: &Option<String>| value.as_deref().unwrap_or("—").to_string();

    let rows = list
        .given_back
        .iter()
        .map(|g| {
            let attempts = g
                .attempts
                .as_deref()
                .map_or(String::new(), |a| format!(" (attempts {a})"));
            let problem = g.problem.as_deref().unwrap_or(NO_REASON);
            format!("{} · {} — {problem}{attempts}", g.id, g.title)
        })
        .collect();
    push_section(
        &mut lines,
        Group::GivenBack,
        "given back to the planner",
        rows,
    );

    let rows = list
        .to_split
        .iter()
        .map(|s| format!("{} [level {}] · {}", s.id, level(&s.level), s.title))
        .collect();
    push_section(&mut lines, Group::ToSplit, "to split, hardest first", rows);

    let rows = list
        .to_detail
        .iter()
        .map(|d| {
            format!(
                "{} [level {}] · {} — missing: {}",
                d.id,
                level(&d.level),
                d.title,
                d.missing.join(", ")
            )
        })
        .collect();
    push_section(&mut lines, Group::ToDetail, "to detail", rows);

    let rows = list
        .without_level
        .iter()
        .map(|t| format!("{} · {} — {}", t.id, t.title, t.reason.as_deref().unwrap_or(NO_REASON)))
        .collect();
    push_section(&mut lines, Group::WithoutLevel, "without a level", rows);

    let rows = list
        .kept_on_purpose
        .iter()
        .map(|k| {
            format!(
                "{} [level {}] · {} — {}",
                k.id,
                level(&k.level),
                k.title,
                k.reason.as_deref().unwrap_or(NO_REASON)
            )
        })
        .collect();
    push_section(
        &mut lines,
        Group::KeptOnPurpose,
        "kept above the target level on purpose",
        rows,
    );

    let rows = list
        .unverified
        .iter()
        .map(|u| {
            format!(
                "{} · {} — verify: {}",
                u.id,
                u.title,
                u.verify.as_deref().unwrap_or(NO_REASON)
            )
        })
        .collect();
    push_section(
        &mut lines,
        Group::Unverified,
        "done, not verified",
        rows,
    );

    if lines.is_empty() {
        lines.push("nothing for the planner: every task can go to a worker".to_string());
    }
    lines
}
