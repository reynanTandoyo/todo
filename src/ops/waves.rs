//! Waves (`R-WAVE-1`): how much unfinished work can run in parallel, and the longest chain of tasks.
//!
//! Wave 1 holds the unfinished tasks that nothing unfinished waits for; wave `n` holds the tasks
//! that wait only for tasks of the waves before it. The dependencies are the ones `next` uses: a
//! task's own `depends` and those of the milestones above it (`R-DEP-1`, `R-MILE-6`). A dependency
//! on a milestone waits for the entries directly below it. Tasks in one wave whose `files` overlap
//! (`R-SCOPE-1`) form one lane, because they must run one after another.
//!
//! The planner works in one pass over a topological order (Kahn's algorithm), so a long chain of
//! tasks never recurses deeply. Whatever cannot be ordered, because it waits on a blocked entry, a
//! missing ID or a cycle, is listed apart with the reason.

use std::collections::{HashMap, VecDeque};

use crate::doc::{DependencyRef, Document, EntryKind, Status, Task, TaskId};
use crate::ops::config::OpsConfig;
use crate::ops::scope::{ScopeEntry, overlap_lanes, parse_files, raw_entries};

/// The most `files` entries a task at the lowest level may have (`R-DETAIL-2`, the default of
/// `level1_max_files`).
pub const LEVEL1_MAX_FILES: usize = 3;

/// One wave: the tasks that become ready in round `number`, grouped into lanes. Each lane lists its
/// tasks in file order; the tasks of one lane must run one after another.
pub struct Wave<'a> {
    /// The round, counted from 1.
    pub number: usize,
    /// The lanes, in order of their first task in file order.
    pub lanes: Vec<Vec<&'a Task>>,
}

/// An unfinished task that is not placed in a wave, with the reason it cannot be (`R-WAVE-1`).
pub struct Apart<'a> {
    /// The task.
    pub task: &'a Task,
    /// The reason, such as `blocked: waiting on a key` or `depends on T-999, which does not exist`.
    pub reason: String,
}

/// The result of [`plan_waves`].
pub struct WavePlan<'a> {
    /// Waves `1..=n`, where `n` is the length of the longest chain. A wave with no task is kept, so
    /// that the numbers stay in step with the chain.
    pub waves: Vec<Wave<'a>>,
    /// The tasks that are not placed, in file order.
    pub apart: Vec<Apart<'a>>,
    /// The longest chain of placed tasks, the first task first. Its length is the number of waves.
    pub longest_chain: Vec<&'a Task>,
}

impl WavePlan<'_> {
    /// The most workers that can work at once: the most lanes in any wave (`R-WAVE-1`).
    #[must_use]
    pub fn max_workers(&self) -> usize {
        self.waves
            .iter()
            .map(|wave| wave.lanes.len())
            .max()
            .unwrap_or(0)
    }

    /// The number of unfinished tasks in the plan, placed or apart.
    #[must_use]
    pub fn task_count(&self) -> usize {
        let placed: usize = self
            .waves
            .iter()
            .map(|wave| wave.lanes.iter().map(Vec::len).sum::<usize>())
            .sum();
        placed + self.apart.len()
    }
}

/// Plans the waves of the unfinished tasks of `doc` (`R-WAVE-1`). `archive` answers for dependencies
/// on finished entries that were archived.
///
/// With `milestone`, only the tasks below it are shown. The whole file is still scheduled, so the
/// wave numbers are those of the whole file, and the longest chain is the one that ends at the
/// milestone's last task; it may run through tasks outside the milestone.
#[must_use]
pub fn plan_waves<'a>(
    doc: &'a Document,
    archive: Option<&Document>,
    milestone: Option<TaskId>,
) -> WavePlan<'a> {
    let label = |id: TaskId| doc.id_config.format_id(id);
    // Every unfinished task and milestone is a node, in file order.
    let nodes: Vec<&'a Task> = doc
        .tasks
        .iter()
        .filter(|t| {
            matches!(t.kind(), EntryKind::Task | EntryKind::Milestone) && is_unfinished(t.status)
        })
        .collect();
    let node_of: HashMap<TaskId, usize> =
        nodes.iter().enumerate().map(|(i, t)| (t.id, i)).collect();
    let index = doc.milestone_index();

    // `preds[i]` are the nodes that node `i` waits for; `reason[i]` says why node `i` is not placed.
    let mut preds: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    let mut reason: Vec<Option<String>> = vec![None; nodes.len()];
    for (i, node) in nodes.iter().enumerate() {
        let mut deps: Vec<&DependencyRef> = node.depends.iter().collect();
        let mut own: Option<String> = None;
        if node.is_milestone() {
            if node.status == Status::Blocked {
                own = Some("blocked milestone".to_string());
            }
            // A milestone is finished when the entries directly below it are.
            preds[i].extend(
                index
                    .children_of(node.id)
                    .iter()
                    .filter_map(|child| node_of.get(&child.id).copied()),
            );
        } else {
            if node.status == Status::Blocked {
                own = Some(blocked_reason(node));
            }
            for above in doc.milestone_ancestors(node) {
                if own.is_none() && above.status == Status::Blocked {
                    own = Some(format!("below blocked milestone {}", label(above.id)));
                }
                deps.extend(above.depends.iter());
            }
        }
        reason[i] = own;
        for dep in deps {
            let problem = match resolve(doc, archive, &node_of, dep) {
                Resolved::Finished => continue,
                Resolved::Node(j) => {
                    preds[i].push(j);
                    continue;
                }
                Resolved::Unknown => format!("depends on {}, which does not exist", dep.raw),
                Resolved::NotWork(id) => format!(
                    "depends on {}, which is neither a task nor a milestone and is not finished",
                    label(id)
                ),
            };
            if reason[i].is_none() {
                reason[i] = Some(problem);
            }
        }
    }

    // Kahn's algorithm: a node is processed once everything it waits for is. Nodes left over wait
    // on a cycle, directly or through other nodes.
    let mut indegree: Vec<usize> = preds.iter().map(Vec::len).collect();
    let mut waited_for_by: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    for (i, list) in preds.iter().enumerate() {
        for &p in list {
            waited_for_by[p].push(i);
        }
    }
    let mut queue: VecDeque<usize> = (0..nodes.len()).filter(|&i| indegree[i] == 0).collect();
    let mut order: Vec<usize> = Vec::with_capacity(nodes.len());
    while let Some(i) = queue.pop_front() {
        order.push(i);
        for &j in &waited_for_by[i] {
            indegree[j] -= 1;
            if indegree[j] == 0 {
                queue.push_back(j);
            }
        }
    }

    // A task's level is one more than the highest level it waits for; a milestone takes the highest
    // level of what is below it. A node that waits on an unplaced one is not placed either.
    let mut level: Vec<usize> = vec![0; nodes.len()];
    for &i in &order {
        if reason[i].is_none() {
            let blocker = preds[i].iter().find(|&&p| reason[p].is_some()).copied();
            if let Some(p) = blocker {
                reason[i] = Some(format!("waits on {}", label(nodes[p].id)));
            }
        }
        if reason[i].is_none() {
            let before = preds[i].iter().map(|&p| level[p]).max().unwrap_or(0);
            level[i] = before + usize::from(nodes[i].kind() == EntryKind::Task);
        }
    }
    explain_cycles(&nodes, &preds, &indegree, &label, &mut reason);

    let shown = |task: &Task| {
        milestone.is_none_or(|m| doc.milestone_ancestors(task).iter().any(|a| a.id == m))
    };
    let mut placed: Vec<usize> = Vec::new();
    let mut apart: Vec<Apart<'a>> = Vec::new();
    for (i, node) in nodes.iter().enumerate() {
        if node.kind() != EntryKind::Task || !shown(node) {
            continue;
        }
        match &reason[i] {
            Some(text) => apart.push(Apart {
                task: node,
                reason: text.clone(),
            }),
            None => placed.push(i),
        }
    }

    let longest = placed.iter().map(|&i| level[i]).max().unwrap_or(0);
    let mut waves: Vec<Wave<'a>> = Vec::with_capacity(longest);
    for number in 1..=longest {
        let members: Vec<usize> = placed
            .iter()
            .copied()
            .filter(|&i| level[i] == number)
            .collect();
        let scopes: Vec<Vec<ScopeEntry>> = members
            .iter()
            .map(|&i| {
                nodes[i]
                    .files
                    .as_deref()
                    .map(parse_files)
                    .unwrap_or_default()
            })
            .collect();
        let lane_of = overlap_lanes(&scopes);
        let lane_count = lane_of.iter().max().map_or(0, |&m| m + 1);
        let mut lanes: Vec<Vec<&'a Task>> = vec![Vec::new(); lane_count];
        for (&i, &lane) in members.iter().zip(&lane_of) {
            lanes[lane].push(nodes[i]);
        }
        waves.push(Wave { number, lanes });
    }

    let mut longest_chain: Vec<&'a Task> = Vec::new();
    if let Some(&last) = placed.iter().find(|&&i| level[i] == longest) {
        let mut current = last;
        let mut chain: Vec<&'a Task> = vec![nodes[current]];
        while level[current] > 1 {
            match task_below(&preds, &level, &nodes, current, level[current] - 1) {
                Some(next) => {
                    chain.push(nodes[next]);
                    current = next;
                }
                None => break,
            }
        }
        chain.reverse();
        longest_chain = chain;
    }

    WavePlan {
        waves,
        apart,
        longest_chain,
    }
}

/// Gives every node left unordered a reason: a cycle it is part of, or the node it waits for.
///
/// A node left unordered has an unordered node among the nodes it waits for, so following those
/// links from any of them ends in a cycle. Each node gets one reason, and every walk stops at nodes
/// that already have one.
fn explain_cycles(
    nodes: &[&Task],
    preds: &[Vec<usize>],
    indegree: &[usize],
    label: &dyn Fn(TaskId) -> String,
    reason: &mut [Option<String>],
) {
    let mut position: Vec<Option<usize>> = vec![None; nodes.len()];
    for start in 0..nodes.len() {
        if indegree[start] == 0 || reason[start].is_some() {
            continue;
        }
        let mut path: Vec<usize> = Vec::new();
        let mut cycle_at: Option<usize> = None;
        let mut current = start;
        loop {
            if let Some(k) = position[current] {
                cycle_at = Some(k);
                break;
            }
            if reason[current].is_some() {
                break;
            }
            position[current] = Some(path.len());
            path.push(current);
            match preds[current].iter().copied().find(|&p| indegree[p] > 0) {
                Some(next) => current = next,
                None => break,
            }
        }
        if let Some(k) = cycle_at {
            let members = &path[k..];
            let text: Vec<String> = members
                .iter()
                .chain(members.first())
                .map(|&v| label(nodes[v].id))
                .collect();
            let text = text.join(" -> ");
            for &v in members {
                if reason[v].is_none() {
                    reason[v] = Some(format!("in a dependency cycle: {text}"));
                }
            }
        }
        for j in 0..path.len() {
            let v = path[j];
            if reason[v].is_none() {
                let next = path.get(j + 1).copied().unwrap_or(current);
                reason[v] = Some(format!("waits on {}", label(nodes[next].id)));
            }
        }
        for &v in &path {
            position[v] = None;
        }
    }
}

/// The task that node `start` waits for at level `wanted`, looking through milestones: a milestone
/// has the level of the entry below it that is highest.
fn task_below(
    preds: &[Vec<usize>],
    level: &[usize],
    nodes: &[&Task],
    start: usize,
    wanted: usize,
) -> Option<usize> {
    let mut frontier: Vec<usize> = preds[start].clone();
    loop {
        let next = *frontier.iter().find(|&&p| level[p] == wanted)?;
        if nodes[next].kind() == EntryKind::Task {
            return Some(next);
        }
        frontier = preds[next].clone();
    }
}

/// What a dependency token names, as far as waves are concerned.
enum Resolved {
    /// A finished task, which imposes nothing.
    Finished,
    /// An unfinished task or milestone, by its index among the nodes.
    Node(usize),
    /// An ID that names no entry, or a token that is not an ID.
    Unknown,
    /// An unfinished entry that is neither a task nor a milestone, such as a question.
    NotWork(TaskId),
}

fn resolve(
    doc: &Document,
    archive: Option<&Document>,
    node_of: &HashMap<TaskId, usize>,
    dep: &DependencyRef,
) -> Resolved {
    let Some(id) = dep.id else {
        return Resolved::Unknown;
    };
    if let Some(task) = doc.task_by_id(id) {
        if task.status.is_finished() {
            return Resolved::Finished;
        }
        return node_of
            .get(&id)
            .map_or(Resolved::NotWork(id), |&j| Resolved::Node(j));
    }
    match archive.and_then(|a| a.task_by_id(id)) {
        Some(task) if task.status.is_finished() => Resolved::Finished,
        Some(_) => Resolved::NotWork(id),
        None => Resolved::Unknown,
    }
}

/// The reason a blocked task is apart: given back to the planner (with its problem), or blocked.
fn blocked_reason(task: &Task) -> String {
    match (task.is_given_back(), task.blocked_reason()) {
        (true, Some(problem)) => format!("given back: {problem}"),
        (true, None) => "given back".to_string(),
        (false, Some(problem)) => format!("blocked: {problem}"),
        (false, None) => "blocked".to_string(),
    }
}

fn is_unfinished(status: Status) -> bool {
    matches!(status, Status::Open | Status::Claimed | Status::Blocked)
}

/// What a task lacks for its level (`R-LEVEL-1`, `R-DETAIL-2`), in a fixed order; empty when it is
/// complete.
///
/// A task without a configured difficulty lacks `level`. A task at the lowest level (the first of
/// `difficulties`) needs `why`, `accept`, `steps`, `verify` and `files`, with at most
/// [`LEVEL1_MAX_FILES`] entries, none of them a folder or a pattern. Tasks above that level are not
/// checked. The rule's other two checks, the paths of `read` and the open questions about the task,
/// are not part of this list yet.
#[must_use]
pub fn detail_gaps(task: &Task, cfg: &OpsConfig) -> Vec<String> {
    let Some(rank) = cfg.difficulty_rank(task.difficulty.as_deref()) else {
        return vec!["level".to_string()];
    };
    if rank != 0 {
        return Vec::new();
    }
    let blank = |value: Option<&str>| value.is_none_or(|v| v.trim().is_empty());
    let mut missing = Vec::new();
    for (name, value) in [
        ("why", task.why.as_deref()),
        ("accept", task.accept.as_deref()),
    ] {
        if blank(value) {
            missing.push(name.to_string());
        }
    }
    if task.steps.is_empty() {
        missing.push("steps".to_string());
    }
    if blank(task.verify.as_deref()) {
        missing.push("verify".to_string());
    }
    let files = task.files.as_deref().unwrap_or_default();
    let entries = parse_files(files);
    if entries.is_empty() {
        missing.push("files".to_string());
    } else {
        if raw_entries(files).count() > LEVEL1_MAX_FILES {
            missing.push(format!("files: more than {LEVEL1_MAX_FILES}"));
        }
        if entries.iter().any(|entry| !entry.is_single_file()) {
            missing.push("files: folder or pattern".to_string());
        }
    }
    missing
}
