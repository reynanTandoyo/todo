//! `todo summary` command: session-start overview of at most 40 lines (`R-READ-5`).

use std::collections::BTreeMap;

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::cli::{CommandKind, CommandMeta, Context, load_read};
use crate::doc::{EntryKind, Progress, Status};
use crate::error::Result;
use crate::lock::current_time;
use crate::ops::{is_task_ready, ready_sort_key};
use crate::output::Output;

/// Metadata for the `summary` command.
pub const META: CommandMeta = CommandMeta {
    name: "summary",
    summary: "Show a compact session-start overview of TODO.md",
    kind: CommandKind::Read,
    details: "Prints at most 40 lines summarizing counts by status and open priority, the top-level milestones that are not done or dropped with their progress, active/stale claims, blocked tasks with their latest reason, the next 5 ready tasks (in the order `next` uses), milestones ready to close, open questions, and how many ideas are waiting (ideas are not counted as tasks).",
    examples: &["todo summary", "todo summary --json"],
};

/// Arguments for `todo summary`.
#[derive(Debug, ClapArgs)]
pub struct Args {}

#[derive(Serialize)]
struct StatusCounts {
    open: usize,
    claimed: usize,
    blocked: usize,
    done: usize,
    dropped: usize,
    /// Work entries only: `open + claimed + blocked + done + dropped`.
    total: usize,
    /// Ideas that are not done or dropped. Ideas are not work, so they are left out of the
    /// other counts (`R-KIND-1`).
    ideas: usize,
}

#[derive(Serialize)]
struct PriorityCount {
    priority: String,
    count: usize,
}

/// A top-level milestone that is not done or dropped, with its progress (`R-MILE-3`).
#[derive(Serialize)]
struct MilestoneSummary {
    id: String,
    title: String,
    priority: Option<String>,
    status: &'static str,
    progress: Progress,
}

#[derive(Serialize)]
struct ClaimedSummary {
    id: String,
    title: String,
    owner: Option<String>,
    claimed: Option<String>,
    age_hours: Option<i64>,
    stale: bool,
}

#[derive(Serialize)]
struct BlockedSummary {
    id: String,
    title: String,
    reason: Option<String>,
}

#[derive(Serialize)]
struct ReadySummary {
    id: String,
    title: String,
    priority: Option<String>,
    section: Option<String>,
}

#[derive(Serialize)]
struct QuestionSummary {
    id: String,
    title: String,
    status: &'static str,
}

#[derive(Serialize)]
struct MilestoneReadySummary {
    id: String,
    title: String,
}

#[derive(Serialize)]
struct SummaryJson {
    file: String,
    counts: StatusCounts,
    open_by_priority: Vec<PriorityCount>,
    milestones: Vec<MilestoneSummary>,
    claimed: Vec<ClaimedSummary>,
    blocked: Vec<BlockedSummary>,
    next_ready: Vec<ReadySummary>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    ready_to_close: Vec<MilestoneReadySummary>,
    open_questions: Vec<QuestionSummary>,
}

const MAX_TEXT_LINES: usize = 40;

/// Runs `todo summary`.
pub fn run(_args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let (cfg, loaded) = load_read(ctx, out, false, true)?;
    let doc = &loaded.doc;
    let archive = loaded.archive.as_ref();
    let now = current_time();

    let mut counts = StatusCounts {
        open: 0,
        claimed: 0,
        blocked: 0,
        done: 0,
        dropped: 0,
        total: 0,
        ideas: 0,
    };

    let mut prio_map: BTreeMap<String, usize> = BTreeMap::new();
    let mut claimed = Vec::new();
    let mut blocked = Vec::new();
    let mut ready_candidates = Vec::new();
    let mut open_questions = Vec::new();

    for task in &doc.tasks {
        if task.kind() == EntryKind::Idea {
            if !matches!(task.status, Status::Done | Status::Dropped) {
                counts.ideas += 1;
            }
            continue;
        }
        match task.status {
            Status::Open => {
                counts.open += 1;
                let key = task
                    .priority
                    .as_deref()
                    .map(|p| {
                        cfg.priorities
                            .iter()
                            .find(|c| c.eq_ignore_ascii_case(p))
                            .cloned()
                            .unwrap_or_else(|| p.to_string())
                    })
                    .unwrap_or_else(|| "none".to_string());
                *prio_map.entry(key).or_insert(0) += 1;
            }
            Status::Claimed => {
                counts.claimed += 1;
                let age_hours = cfg.claim_age(task, now).map(|d| d.num_hours());
                let stale = cfg.is_claim_stale(task, now);
                claimed.push(ClaimedSummary {
                    id: doc.id_config.format_id(task.id),
                    title: task.title.clone(),
                    owner: task.owner.clone(),
                    claimed: task.claimed.clone(),
                    age_hours,
                    stale,
                });
            }
            Status::Blocked => {
                counts.blocked += 1;
                let reason = task
                    .notes
                    .iter()
                    .rev()
                    .find(|n| !n.text.trim().is_empty())
                    .map(|n| n.text.trim().to_string());
                blocked.push(BlockedSummary {
                    id: doc.id_config.format_id(task.id),
                    title: task.title.clone(),
                    reason,
                });
            }
            Status::Done => counts.done += 1,
            Status::Dropped => counts.dropped += 1,
        }

        let is_question = task
            .task_type
            .as_deref()
            .is_some_and(|ty| ty.eq_ignore_ascii_case("question"));
        if is_question && !matches!(task.status, Status::Done | Status::Dropped) {
            open_questions.push(QuestionSummary {
                id: doc.id_config.format_id(task.id),
                title: task.title.clone(),
                status: task.status.as_str(),
            });
        } else if !is_question && is_task_ready(doc, task, archive, now, &cfg) {
            ready_candidates.push(task);
        }
    }

    counts.total = counts.open + counts.claimed + counts.blocked + counts.done + counts.dropped;
    let started = doc.started_milestones();
    ready_candidates.sort_by_key(|t| ready_sort_key(doc, t, &started, &cfg));
    let next_ready: Vec<ReadySummary> = ready_candidates
        .into_iter()
        .take(5)
        .map(|t| ReadySummary {
            id: doc.id_config.format_id(t.id),
            title: t.title.clone(),
            priority: t.priority.clone(),
            section: t
                .section
                .and_then(|idx| doc.sections.get(idx).map(|s| s.name.clone())),
        })
        .collect();

    let mut open_by_priority = Vec::new();
    for p in &cfg.priorities {
        if let Some(c) = prio_map.remove(p) {
            open_by_priority.push(PriorityCount {
                priority: p.clone(),
                count: c,
            });
        }
    }
    for (p, c) in prio_map {
        open_by_priority.push(PriorityCount {
            priority: p,
            count: c,
        });
    }

    // Top-level milestones that are open or blocked; a milestone is never claimed (R-MILE-2).
    let milestones: Vec<MilestoneSummary> = doc
        .tasks
        .iter()
        .filter(|t| {
            t.is_milestone()
                && matches!(t.status, Status::Open | Status::Blocked)
                && doc.milestone_ancestors(t).is_empty()
        })
        .map(|t| MilestoneSummary {
            id: doc.id_config.format_id(t.id),
            title: t.title.clone(),
            priority: t.priority.clone(),
            status: t.status.as_str(),
            progress: doc.progress(t.id),
        })
        .collect();

    let ready_to_close = doc
        .milestones_ready_to_close()
        .into_iter()
        .map(|m| MilestoneReadySummary {
            id: doc.id_config.format_id(m.id),
            title: m.title.clone(),
        })
        .collect();

    let payload = SummaryJson {
        file: loaded.file_label.clone(),
        counts,
        open_by_priority,
        milestones,
        claimed,
        blocked,
        next_ready,
        ready_to_close,
        open_questions,
    };

    if out.is_json() {
        out.emit_json_ok(META.name, &payload);
        return Ok(());
    }

    let lines = format_summary_lines(&payload);
    for line in lines {
        out.write_line(&line);
    }
    Ok(())
}

fn format_summary_lines(s: &SummaryJson) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!(
        "{}: {} total (open {}, claimed {}, blocked {}, done {}, dropped {})",
        s.file,
        s.counts.total,
        s.counts.open,
        s.counts.claimed,
        s.counts.blocked,
        s.counts.done,
        s.counts.dropped
    ));

    if s.open_by_priority.is_empty() {
        lines.push("open by priority: none".to_string());
    } else {
        let parts: Vec<String> = s
            .open_by_priority
            .iter()
            .map(|p| format!("{}={}", p.priority, p.count))
            .collect();
        lines.push(format!("open by priority: {}", parts.join(" ")));
    }

    if !s.milestones.is_empty() {
        lines.push(format!("milestones ({}):", s.milestones.len()));
        for m in s.milestones.iter().take(5) {
            let prio = m.priority.as_deref().unwrap_or("—");
            lines.push(format!(
                "  {} [{}] · {} · {}/{} done, {}",
                m.id, prio, m.title, m.progress.done, m.progress.total, m.status
            ));
        }
        if s.milestones.len() > 5 {
            lines.push(format!(
                "  ... and {} more milestones",
                s.milestones.len() - 5
            ));
        }
    }

    if s.counts.ideas > 0 {
        lines.push(format!(
            "ideas: {} (not work; `todo promote <ID> --priority P` turns one into a task)",
            s.counts.ideas
        ));
    }

    if !s.claimed.is_empty() {
        lines.push(format!("claimed ({}):", s.claimed.len()));
        for c in s.claimed.iter().take(6) {
            let owner = c.owner.as_deref().unwrap_or("—");
            let age = c
                .age_hours
                .map_or_else(|| "?".to_string(), |h| format!("{h}h"));
            let stale_tag = if c.stale { " [STALE]" } else { "" };
            lines.push(format!(
                "  {} · {} ({}, {}){}",
                c.id, c.title, owner, age, stale_tag
            ));
        }
        if s.claimed.len() > 6 {
            lines.push(format!("  ... and {} more claimed", s.claimed.len() - 6));
        }
    }

    if !s.blocked.is_empty() {
        lines.push(format!("blocked ({}):", s.blocked.len()));
        for b in s.blocked.iter().take(5) {
            let reason = b.reason.as_deref().unwrap_or("no reason recorded");
            lines.push(format!("  {} · {} — {}", b.id, b.title, reason));
        }
        if s.blocked.len() > 5 {
            lines.push(format!("  ... and {} more blocked", s.blocked.len() - 5));
        }
    }

    if s.next_ready.is_empty() {
        lines.push("next ready: none".to_string());
    } else {
        lines.push("next ready:".to_string());
        for r in &s.next_ready {
            let prio = r.priority.as_deref().unwrap_or("—");
            lines.push(format!("  {} [{}] · {}", r.id, prio, r.title));
        }
    }

    if !s.ready_to_close.is_empty() {
        lines.push(format!("ready to close ({}):", s.ready_to_close.len()));
        for m in s.ready_to_close.iter().take(5) {
            lines.push(format!("  {} · {} — todo done {}", m.id, m.title, m.id));
        }
        if s.ready_to_close.len() > 5 {
            lines.push(format!(
                "  ... and {} more ready to close",
                s.ready_to_close.len() - 5
            ));
        }
    }

    if !s.open_questions.is_empty() {
        lines.push(format!("open questions ({}):", s.open_questions.len()));
        for q in s.open_questions.iter().take(5) {
            lines.push(format!("  {} [{}] · {}", q.id, q.status, q.title));
        }
        if s.open_questions.len() > 5 {
            lines.push(format!(
                "  ... and {} more questions",
                s.open_questions.len() - 5
            ));
        }
    }

    if lines.len() > MAX_TEXT_LINES {
        lines.truncate(MAX_TEXT_LINES - 1);
        lines.push("... (truncated to 40 lines)".to_string());
    }
    lines
}
