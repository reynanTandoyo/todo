//! File scopes (`R-SCOPE-1`, `docs/format.md` §19): the files a task may change, whether two tasks
//! overlap, and the claims of other agents that hold files a task needs.
//!
//! A task *holds* its files while it is claimed, or blocked with an owner, and its claim is fresh
//! (`R-CLAIM-4`). `next`, `next --claim` and `claim` keep a task off the files another agent holds;
//! `show --json` and `list` report them as `files_busy`.

use std::collections::HashMap;

use chrono::{DateTime, Duration, FixedOffset};
use serde_json::json;

use crate::doc::{Document, Status, Task, agent_names_equal};
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::config::OpsConfig;

/// How an entry of `files` names what it covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// One file, such as `src/shader.c`.
    Path,
    /// A folder, ending in `/`, and everything below it.
    Folder,
    /// A pattern with `*`; a `**` part matches any number of folders.
    Pattern,
}

/// One entry of a `files` value, normalised for comparison (`docs/format.md` §19).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeEntry {
    /// The entry with `/` separators, no `.` parts and no `:<line>` suffix; lower case on Windows.
    text: String,
    kind: Kind,
}

/// The entries of a `files` value as written: one per comma-separated item, trimmed, without the
/// empty items.
pub fn raw_entries(value: &str) -> impl Iterator<Item = &str> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
}

/// The entries of a `files` value, ready for comparison.
#[must_use]
pub fn parse_files(value: &str) -> Vec<ScopeEntry> {
    raw_entries(value).filter_map(ScopeEntry::parse).collect()
}

impl ScopeEntry {
    /// Parses one entry as written. `None` when nothing is left once `.` parts are removed.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        let slashed = strip_line_suffix(raw.trim()).replace('\\', "/");
        let folder = slashed.ends_with('/');
        let parts: Vec<&str> = slashed
            .split('/')
            .filter(|part| !part.is_empty() && *part != ".")
            .collect();
        if parts.is_empty() {
            return None;
        }
        let mut text = parts.join("/");
        if folder {
            text.push('/');
        }
        if cfg!(windows) {
            text = text.to_lowercase();
        }
        let kind = if text.contains('*') {
            Kind::Pattern
        } else if folder {
            Kind::Folder
        } else {
            Kind::Path
        };
        // A pattern that names folders covers everything below them: `src/*/` is `src/*/**`.
        if kind == Kind::Pattern && text.ends_with('/') {
            text.push_str("**");
        }
        Some(Self { text, kind })
    }

    /// The text before the first `*`, which is the part of a pattern that is fixed.
    fn fixed(&self) -> &str {
        self.text.split('*').next().unwrap_or_default()
    }

    /// Whether the entry names one file, not a folder or a pattern (`R-DETAIL-2`).
    #[must_use]
    pub fn is_single_file(&self) -> bool {
        self.kind == Kind::Path
    }

    /// The first part of the entry, unless it holds a `*`. Two entries whose first parts differ
    /// never overlap, so [`overlap_lanes`] compares only entries that share one.
    fn first_part(&self) -> Option<&str> {
        let first = self.text.split('/').next().unwrap_or_default();
        (!first.contains('*')).then_some(first)
    }
}

/// Removes a trailing `:<digits>` line number, as `todo scan` writes it (`src/main.rs:42`).
fn strip_line_suffix(text: &str) -> &str {
    match text.rsplit_once(':') {
        Some((head, digits))
            if !head.is_empty()
                && !digits.is_empty()
                && digits.bytes().all(|b| b.is_ascii_digit()) =>
        {
            head
        }
        _ => text,
    }
}

/// Whether two entries overlap (`R-SCOPE-1`): they are equal, one lies under a folder named by the
/// other, or one can match the other. Two patterns overlap unless their fixed parts differ, so an
/// unclear case counts as an overlap.
#[must_use]
pub fn entries_overlap(a: &ScopeEntry, b: &ScopeEntry) -> bool {
    match (a.kind, b.kind) {
        (Kind::Path, Kind::Path) => a.text == b.text,
        (Kind::Path, Kind::Folder) => a.text.starts_with(&b.text),
        (Kind::Folder, Kind::Path) => b.text.starts_with(&a.text),
        (Kind::Folder, Kind::Folder) => related(&a.text, &b.text),
        (Kind::Path, Kind::Pattern) => glob_matches(&b.text, &a.text),
        (Kind::Pattern, Kind::Path) => glob_matches(&a.text, &b.text),
        (Kind::Folder, Kind::Pattern) => related(&a.text, b.fixed()),
        (Kind::Pattern, Kind::Folder) => related(a.fixed(), &b.text),
        (Kind::Pattern, Kind::Pattern) => related(a.fixed(), b.fixed()),
    }
}

/// Whether two lists of entries overlap: some entry of one overlaps some entry of the other.
#[must_use]
pub fn scopes_overlap(a: &[ScopeEntry], b: &[ScopeEntry]) -> bool {
    a.iter().any(|x| b.iter().any(|y| entries_overlap(x, y)))
}

/// Groups items into lanes (`R-WAVE-1`): items whose file scopes overlap, directly or through other
/// items, share a lane, because they must run one after another. `scopes` holds the parsed `files`
/// of each item, in order. The result gives the lane of each item, numbered from 0 in order of first
/// appearance; an item without files is a lane of its own.
///
/// The result is the one a comparison of every pair of items gives (see [`scopes_overlap`]). Entries
/// are compared only with entries that can overlap them: those with the same first part, and those
/// whose first part holds a `*`.
#[must_use]
pub fn overlap_lanes(scopes: &[Vec<ScopeEntry>]) -> Vec<usize> {
    let entries: Vec<(usize, &ScopeEntry)> = scopes
        .iter()
        .enumerate()
        .flat_map(|(item, list)| list.iter().map(move |entry| (item, entry)))
        .collect();
    let mut parent: Vec<usize> = (0..scopes.len()).collect();
    let mut by_part: HashMap<&str, Vec<usize>> = HashMap::new();
    let mut wildcard: Vec<usize> = Vec::new();
    for (g, &(item, entry)) in entries.iter().enumerate() {
        let earlier: Vec<usize> = match entry.first_part() {
            Some(part) => by_part
                .get(part)
                .into_iter()
                .flatten()
                .chain(wildcard.iter())
                .copied()
                .collect(),
            None => (0..g).collect(),
        };
        for h in earlier {
            let (other, other_entry) = entries[h];
            let root_a = find_root(&mut parent, item);
            let root_b = find_root(&mut parent, other);
            if root_a != root_b && entries_overlap(entry, other_entry) {
                parent[root_a] = root_b;
            }
        }
        match entry.first_part() {
            Some(part) => by_part.entry(part).or_default().push(g),
            None => wildcard.push(g),
        }
    }
    let mut lane_of_root: HashMap<usize, usize> = HashMap::new();
    (0..scopes.len())
        .map(|item| {
            let root = find_root(&mut parent, item);
            let next = lane_of_root.len();
            *lane_of_root.entry(root).or_insert(next)
        })
        .collect()
}

/// The root of `x` in a forest of items, with path halving.
fn find_root(parent: &mut [usize], mut x: usize) -> usize {
    while parent[x] != x {
        parent[x] = parent[parent[x]];
        x = parent[x];
    }
    x
}

/// Whether one of two texts starts with the other.
fn related(x: &str, y: &str) -> bool {
    x.starts_with(y) || y.starts_with(x)
}

/// Whether `pattern` matches the whole of `path`: `*` matches any characters except `/`, and a
/// `**` part matches any number of folders (`docs/format.md` §19).
fn glob_matches(pattern: &str, path: &str) -> bool {
    let parts: Vec<&str> = pattern.split('/').collect();
    let names: Vec<&str> = path.split('/').collect();
    segments_match(&parts, &names)
}

fn segments_match(pattern: &[&str], names: &[&str]) -> bool {
    match pattern.split_first() {
        None => names.is_empty(),
        Some((&"**", rest)) => (0..=names.len()).any(|skip| {
            names
                .get(skip..)
                .is_some_and(|tail| segments_match(rest, tail))
        }),
        Some((part, rest)) => match names.split_first() {
            Some((name, names_rest)) => {
                wildcard_matches(part, name) && segments_match(rest, names_rest)
            }
            None => false,
        },
    }
}

/// Whether `text` matches `pattern`, where `*` matches any run of characters.
fn wildcard_matches(pattern: &str, text: &str) -> bool {
    let pat: Vec<char> = pattern.chars().collect();
    let txt: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0usize, 0usize);
    // The last `*` seen, and the text position it has consumed up to.
    let mut star: Option<(usize, usize)> = None;
    while ti < txt.len() {
        match pat.get(pi) {
            Some(&'*') => {
                star = Some((pi, ti));
                pi += 1;
            }
            Some(&c) if txt.get(ti) == Some(&c) => {
                pi += 1;
                ti += 1;
            }
            _ => match star {
                Some((star_pi, star_ti)) => {
                    pi = star_pi + 1;
                    star = Some((star_pi, star_ti + 1));
                    ti = star_ti + 1;
                }
                None => return false,
            },
        }
    }
    pat.iter().skip(pi).all(|&c| c == '*')
}

/// Why an entry of `files` is not a path inside the folder of `TODO.md`, or `None` when it is.
#[must_use]
pub fn outside_folder_reason(raw: &str) -> Option<&'static str> {
    let stripped = strip_line_suffix(raw.trim());
    if stripped.starts_with(['/', '\\'])
        || matches!(stripped.as_bytes(), [d, b':', ..] if d.is_ascii_alphabetic())
    {
        return Some("is absolute");
    }
    if stripped.split(['/', '\\']).any(|part| part == "..") {
        return Some("leaves the folder of TODO.md");
    }
    None
}

/// Whether `task` holds its files (`R-SCOPE-1`): it is claimed with a fresh claim, or it is blocked
/// with an owner and a fresh claim. A task that is open, done or dropped holds nothing.
#[must_use]
pub fn holds_files(task: &Task, now: DateTime<FixedOffset>, cfg: &OpsConfig) -> bool {
    match task.status {
        Status::Claimed => !cfg.is_claim_stale(task, now),
        Status::Blocked => task.owner.is_some() && claim_within_ttl(task, now, cfg),
        Status::Open | Status::Done | Status::Dropped => false,
    }
}

/// Whether the claim of `task` is no older than `claim_ttl_hours` (`R-CLAIM-4`). A claim without a
/// valid timestamp is not fresh.
fn claim_within_ttl(task: &Task, now: DateTime<FixedOffset>, cfg: &OpsConfig) -> bool {
    let ttl_secs = i64::try_from(cfg.claim_ttl_hours)
        .unwrap_or(i64::MAX / 3600)
        .saturating_mul(3600);
    cfg.claim_age(task, now)
        .is_some_and(|age| age <= Duration::seconds(ttl_secs))
}

/// The tasks that hold files `task` names (`R-SCOPE-1`), in file order. `task` itself is not one of
/// them, and neither are the tasks that `agent` holds: `agent` is the one asking (`next`, `claim`),
/// or the owner of `task` for `show` and `list`. `None` counts every holder as another agent.
#[must_use]
pub fn files_busy<'a>(
    doc: &'a Document,
    task: &Task,
    agent: Option<&str>,
    now: DateTime<FixedOffset>,
    cfg: &OpsConfig,
) -> Vec<&'a Task> {
    let Some(files) = task.files.as_deref() else {
        return Vec::new();
    };
    let mine = parse_files(files);
    if mine.is_empty() {
        return Vec::new();
    }
    doc.tasks
        .iter()
        .filter(|other| {
            other.id != task.id
                && holds_files(other, now, cfg)
                && !agent.is_some_and(|a| {
                    other
                        .owner
                        .as_deref()
                        .is_some_and(|owner| agent_names_equal(owner, a))
                })
                && other
                    .files
                    .as_deref()
                    .is_some_and(|theirs| scopes_overlap(&mine, &parse_files(theirs)))
        })
        .collect()
}

/// The `FILES_BUSY` refusal (`R-CLAIM-2`) for the claim of `task_id`, naming the tasks in the way.
#[must_use]
pub fn files_busy_error(task_id: &str, busy: &[&Task], cfg: &OpsConfig) -> TodoError {
    let ids: Vec<String> = busy.iter().map(|t| cfg.id_config.format_id(t.id)).collect();
    let in_the_way: Vec<String> = busy
        .iter()
        .zip(&ids)
        .map(|(t, id)| format!("{id} ({})", holder_text(t)))
        .collect();
    TodoError::new(
        ErrorCode::FilesBusy,
        format!(
            "task {task_id} shares files with {}; pass --force to claim it anyway",
            in_the_way.join(", ")
        ),
    )
    .with_task(task_id)
    .with_detail("files_busy", json!(ids))
}

/// How a holder is described in a refusal: its status and owner.
fn holder_text(task: &Task) -> String {
    let owner = task.owner.as_deref().unwrap_or("nobody");
    match task.status {
        Status::Blocked => format!("blocked, owned by {owner}"),
        _ => format!("claimed by {owner}"),
    }
}

/// Checks the file scope of a claim of `task` by `agent` (`R-CLAIM-2`, `R-SCOPE-1`).
///
/// The claim is refused with `FILES_BUSY` when another agent holds a file the task names, unless
/// `force` is given. With `force` the result is the note that records the override
/// (`docs/format.md` §9). Refreshing one's own live claim takes no new files, so it is not checked.
pub fn check_claim_files(
    doc: &Document,
    task: &Task,
    agent: &str,
    force: bool,
    now: DateTime<FixedOffset>,
    cfg: &OpsConfig,
) -> Result<Option<String>> {
    let own_live_claim = task.status == Status::Claimed
        && task
            .owner
            .as_deref()
            .is_some_and(|owner| agent_names_equal(owner, agent))
        && !cfg.is_claim_stale(task, now);
    if own_live_claim {
        return Ok(None);
    }
    let busy = files_busy(doc, task, Some(agent), now, cfg);
    if busy.is_empty() {
        return Ok(None);
    }
    if force {
        let ids: Vec<String> = busy.iter().map(|t| cfg.id_config.format_id(t.id)).collect();
        return Ok(Some(format!(
            "forced claim (files held by {})",
            ids.join(", ")
        )));
    }
    Err(files_busy_error(
        &cfg.id_config.format_id(task.id),
        &busy,
        cfg,
    ))
}
