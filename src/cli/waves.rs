//! `todo waves` command: how much unfinished work can run in parallel, and the longest chain (T-163;
//! R-WAVE-1, R-READ-4).

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::cli::{CommandKind, CommandMeta, Context, load_read};
use crate::doc::{Document, Task};
use crate::error::Result;
use crate::ops::OpsConfig;
use crate::ops::milestone::resolve_milestone;
use crate::ops::waves::{self, detail_gaps};
use crate::output::Output;

/// Metadata for the `waves` command.
pub const META: CommandMeta = CommandMeta {
    name: "waves",
    summary: "Group the unfinished tasks into waves: rounds, workers at once, longest chain",
    kind: CommandKind::Read,
    details: "Groups the unfinished tasks into waves (R-WAVE-1). Wave 1 holds every task that is ready now, claimed ones included; wave n holds the tasks that become ready when the waves before it are done. Readiness uses each task's dependencies and those of the milestones above it; a dependency on a milestone waits for the entries directly below it. Tasks in one wave whose `files` overlap form one lane, because they must run one after another. Prints the number of waves, the size of each (tasks and lanes), the most workers useful at once (the most lanes in any wave) and the longest dependency chain, task by task. A task without a level is held back with its reason, as `next` holds it back (R-LEVEL-1); a task at or below the target level (`target_difficulty`, by default the lowest level) without its detail (R-DETAIL-2) is placed by its dependencies and marked `not ready`, with what it lacks. Blocked tasks (given back or not), tasks below a milestone that is blocked, done or dropped (the rule `next` uses, R-MILE-6), and tasks that wait on a missing ID or on a cycle are listed apart, each with its reason. `<milestone>` shows only the tasks below it, with the wave numbers of the whole file. Reads only; takes no lock.",
    examples: &["todo waves", "todo waves T-010", "todo waves --json"],
};

/// Arguments for `todo waves`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Show only the tasks below this milestone (an ID or a title).
    pub milestone: Option<String>,
}

/// A task in a wave or in the longest chain, as `waves --json` returns it.
#[derive(Debug, Clone, Serialize)]
struct TaskEntry {
    id: String,
    title: String,
    status: &'static str,
    owner: Option<String>,
    /// The difficulty as written, which is the task's level; `null` when it has none.
    level: Option<String>,
    /// What the task lacks at or below the target level (`R-DETAIL-2`); `[]` when it has all of it,
    /// or is above the target level.
    missing: Vec<String>,
}

/// A task that is not placed in a wave, with the reason.
#[derive(Debug, Serialize)]
struct ApartEntry {
    id: String,
    title: String,
    status: &'static str,
    reason: String,
}

/// The size of a wave: its tasks and its lanes.
#[derive(Debug, Serialize)]
struct WaveSize {
    tasks: usize,
    lanes: usize,
}

/// One wave: its size, and its lanes, each listing its tasks in the order they run.
#[derive(Debug, Serialize)]
struct WaveEntry {
    wave: usize,
    size: WaveSize,
    lanes: Vec<Vec<TaskEntry>>,
}

/// The payload of `todo waves --json` (`todo.waves.v1`).
#[derive(Debug, Serialize)]
struct WavesJson {
    file: String,
    /// The milestone the view is limited to, by canonical ID; `null` for the whole file.
    milestone: Option<String>,
    /// Unfinished tasks in the view, placed or apart.
    tasks: usize,
    /// The number of waves, which is the length of the longest chain.
    wave_count: usize,
    /// The most workers useful at once: the most lanes in any wave.
    max_workers: usize,
    /// The longest chain of placed tasks, the first one first.
    longest_chain: Vec<TaskEntry>,
    waves: Vec<WaveEntry>,
    /// Tasks not placed in a wave, in file order.
    apart: Vec<ApartEntry>,
}

/// Runs `todo waves [<milestone>]`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let (cfg, loaded) = load_read(ctx, out, false, true)?;
    let doc = &loaded.doc;
    let milestone = match args.milestone.as_deref() {
        Some(raw) => Some(resolve_milestone(doc, raw)?.id),
        None => None,
    };
    let archive = loaded.archive.as_ref();
    let plan = waves::plan_waves(doc, archive, milestone);

    let payload = WavesJson {
        file: loaded.file_label.clone(),
        milestone: milestone.map(|id| doc.id_config.format_id(id)),
        tasks: plan.task_count(),
        wave_count: plan.waves.len(),
        max_workers: plan.max_workers(),
        longest_chain: plan
            .longest_chain
            .iter()
            .map(|task| task_entry(doc, archive, &cfg, task))
            .collect(),
        waves: plan
            .waves
            .iter()
            .map(|wave| WaveEntry {
                wave: wave.number,
                size: WaveSize {
                    tasks: wave.lanes.iter().map(Vec::len).sum(),
                    lanes: wave.lanes.len(),
                },
                lanes: wave
                    .lanes
                    .iter()
                    .map(|lane| {
                        lane.iter()
                            .map(|task| task_entry(doc, archive, &cfg, task))
                            .collect()
                    })
                    .collect(),
            })
            .collect(),
        apart: plan
            .apart
            .iter()
            .map(|apart| ApartEntry {
                id: doc.id_config.format_id(apart.task.id),
                title: apart.task.title.clone(),
                status: apart.task.status.as_str(),
                reason: apart.reason.clone(),
            })
            .collect(),
    };

    if out.is_json() {
        out.emit_json_ok(META.name, &payload);
        return Ok(());
    }
    out.write_str(&render_text(&payload));
    Ok(())
}

fn task_entry(doc: &Document, archive: Option<&Document>, cfg: &OpsConfig, task: &Task) -> TaskEntry {
    TaskEntry {
        id: doc.id_config.format_id(task.id),
        title: task.title.clone(),
        status: task.status.as_str(),
        owner: task.owner.clone(),
        level: task.difficulty.clone(),
        missing: detail_gaps(task, doc, archive, cfg),
    }
}

/// One task as the text output shows it: its ID and title, then who holds it and what it lacks.
fn task_text(entry: &TaskEntry) -> String {
    let mut text = format!("{} {}", entry.id, entry.title);
    if entry.status == "claimed" {
        match entry.owner.as_deref() {
            Some(owner) => text.push_str(&format!(" [claimed by {owner}]")),
            None => text.push_str(" [claimed]"),
        }
    }
    if !entry.missing.is_empty() {
        text.push_str(&format!(" [not ready: {}]", entry.missing.join(", ")));
    }
    text
}

/// `count` with the singular or the plural word, such as `1 lane` or `2 lanes`.
fn counted(count: usize, singular: &str, plural: &str) -> String {
    let word = if count == 1 { singular } else { plural };
    format!("{count} {word}")
}

fn render_text(payload: &WavesJson) -> String {
    let scope = match &payload.milestone {
        Some(id) => format!("{}, below {id}", payload.file),
        None => payload.file.clone(),
    };
    let mut lines = Vec::new();
    if payload.tasks == 0 {
        lines.push(format!("{scope}: no unfinished task"));
    } else {
        lines.push(format!(
            "{scope}: {} in {}; up to {} at once",
            counted(payload.tasks, "unfinished task", "unfinished tasks"),
            counted(payload.wave_count, "wave", "waves"),
            counted(payload.max_workers, "worker", "workers"),
        ));
    }
    for wave in &payload.waves {
        if wave.size.tasks == 0 {
            lines.push(format!("wave {}: no task", wave.wave));
            continue;
        }
        lines.push(format!(
            "wave {}: {} in {}",
            wave.wave,
            counted(wave.size.tasks, "task", "tasks"),
            counted(wave.size.lanes, "lane", "lanes"),
        ));
        for (number, lane) in wave.lanes.iter().enumerate() {
            let tasks: Vec<String> = lane.iter().map(task_text).collect();
            lines.push(format!("  lane {}: {}", number + 1, tasks.join(" then ")));
        }
    }
    if !payload.longest_chain.is_empty() {
        let ids: Vec<&str> = payload.longest_chain.iter().map(|t| t.id.as_str()).collect();
        lines.push(format!(
            "longest chain, {}: {}",
            counted(payload.longest_chain.len(), "task", "tasks"),
            ids.join(" -> ")
        ));
    }
    if !payload.apart.is_empty() {
        lines.push(format!("not in a wave ({}):", payload.apart.len()));
        for apart in &payload.apart {
            lines.push(format!("  {} {}: {}", apart.id, apart.title, apart.reason));
        }
    }
    let mut text = lines.join("\n");
    text.push('\n');
    text
}
