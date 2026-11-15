//! Numbered snapshots, history index, outside-edit tracking, and write journal (`R-HIST-2`, `R-HIST-3`, `R-HIST-4`).

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::atomic::{FileSnapshot, atomic_write_bytes, read_file_with_snapshot};
use crate::doc::id::FastU64Map;
use crate::doc::{Document, IdConfig, Line, TaskId};
use crate::error::{ErrorCode, Result, TodoError};
use crate::find::fs_path;
use crate::lock::{LockConfig, WriteLockGuard, current_time, is_lease_live};
use crate::state::{LastWrite, StateDir};

const fn is_false(b: &bool) -> bool {
    !*b
}

/// One entry in `history/index.jsonl` (`R-HIST-2`, `R-HIST-3`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotEntry {
    /// 1-based monotonic snapshot number.
    pub number: u64,
    /// RFC 3339 timestamp when the snapshot was recorded.
    pub time: String,
    /// Agent that triggered the write (or `"outside"` for outside edits).
    pub agent: String,
    /// Command that triggered the write (or `"outside edit"`).
    pub command: String,
    /// Lowercase hex SHA-256 of the snapshot file bytes.
    pub sha256: String,
    /// File name inside `history/` (e.g. `"000001.md"`).
    pub file: String,
    /// `true` when this snapshot captured an edit made outside the `todo` tool (`R-HIST-3`).
    #[serde(default, skip_serializing_if = "is_false")]
    pub outside_edit: bool,
}

/// Formats the snapshot file name for `number` (e.g. `1` -> `"000001.md"`).
#[must_use]
pub fn snapshot_filename(number: u64) -> String {
    format!("{number:06}.md")
}

/// Parses a snapshot number from CLI input (`1` or `000001`).
pub fn parse_snapshot_number(raw: &str) -> Result<u64> {
    let trimmed = raw.trim();
    let num: u64 = trimmed.parse().map_err(|_| {
        TodoError::new(
            ErrorCode::BadValue,
            format!("invalid snapshot number {raw:?}: expected a positive integer"),
        )
    })?;
    if num == 0 {
        return Err(TodoError::new(
            ErrorCode::BadValue,
            "invalid snapshot number 0: snapshot numbers start at 1",
        ));
    }
    Ok(num)
}

/// Reads all entries from `history/index.jsonl`, returning an empty list if none exist yet.
pub fn read_history(state: &StateDir) -> Result<Vec<SnapshotEntry>> {
    let path = state.history_index_path();
    let content = match fs::read_to_string(fs_path(&path).as_ref()) {
        Ok(s) => s,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(TodoError::new(
                ErrorCode::WriteFailed,
                format!("failed reading {}: {e}", path.display()),
            ));
        }
    };

    let mut entries = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let entry: SnapshotEntry = serde_json::from_str(trimmed).map_err(|e| {
            TodoError::new(
                ErrorCode::WriteFailed,
                format!(
                    "failed parsing {}:{}: {e}",
                    path.display(),
                    idx.saturating_add(1)
                ),
            )
        })?;
        entries.push(entry);
    }
    Ok(entries)
}

/// Reads the [`SnapshotEntry`] and raw bytes for snapshot `number`.
pub fn read_snapshot_bytes(state: &StateDir, number: u64) -> Result<(SnapshotEntry, Vec<u8>)> {
    let entries = read_history(state)?;
    let entry = entries
        .into_iter()
        .find(|e| e.number == number)
        .ok_or_else(|| {
            TodoError::new(
                ErrorCode::NotFound,
                format!("snapshot {number} not found in history"),
            )
        })?;

    let snap_path = state.history_dir().join(&entry.file);
    let bytes = fs::read(fs_path(&snap_path).as_ref()).map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            TodoError::new(
                ErrorCode::NotFound,
                format!("snapshot {number} file {} is missing", snap_path.display()),
            )
        } else {
            TodoError::new(
                ErrorCode::WriteFailed,
                format!("failed reading snapshot {}: {e}", snap_path.display()),
            )
        }
    })?;
    Ok((entry, bytes))
}

/// Saves `bytes` as a new numbered snapshot in `history/` and prunes to `snapshots_keep` (`R-HIST-2`).
///
/// Must be called while holding the state write lock (or when creating a fresh state).
pub fn record_snapshot(
    state: &StateDir,
    bytes: &[u8],
    sha256: &str,
    agent: &str,
    command: &str,
    outside_edit: bool,
    snapshots_keep: u64,
) -> Result<Option<SnapshotEntry>> {
    let mut entries = read_history(state)?;

    if snapshots_keep == 0 {
        if !entries.is_empty() {
            for old in entries.drain(..) {
                remove_snapshot_file(&state.history_dir().join(&old.file));
            }
            write_history_index(state, &entries)?;
        }
        return Ok(None);
    }

    state.ensure_created()?;
    let hist_dir = state.history_dir();
    fs::create_dir_all(fs_path(&hist_dir).as_ref()).map_err(|e| {
        TodoError::new(
            ErrorCode::WriteFailed,
            format!("failed creating history dir {}: {e}", hist_dir.display()),
        )
    })?;

    let number = entries.iter().map(|e| e.number).max().unwrap_or(0) + 1;
    let file = snapshot_filename(number);
    let snap_path = hist_dir.join(&file);
    atomic_write_bytes(&snap_path, bytes)?;

    let entry = SnapshotEntry {
        number,
        time: current_time().to_rfc3339(),
        agent: agent.to_owned(),
        command: command.to_owned(),
        sha256: sha256.to_owned(),
        file,
        outside_edit,
    };
    entries.push(entry.clone());

    let keep = usize::try_from(snapshots_keep).unwrap_or(usize::MAX);
    if entries.len() > keep {
        let excess = entries.len() - keep;
        for old in entries.drain(..excess) {
            remove_snapshot_file(&hist_dir.join(&old.file));
        }
    }

    write_history_index(state, &entries)?;
    Ok(Some(entry))
}

fn remove_snapshot_file(path: &Path) {
    let _ = fs::remove_file(fs_path(path).as_ref());
}

fn write_history_index(state: &StateDir, entries: &[SnapshotEntry]) -> Result<()> {
    let mut out = String::new();
    for entry in entries {
        let line = serde_json::to_string(entry).map_err(|e| {
            TodoError::new(
                ErrorCode::WriteFailed,
                format!("failed serializing history index entry: {e}"),
            )
        })?;
        out.push_str(&line);
        out.push('\n');
    }
    atomic_write_bytes(&state.history_index_path(), out.as_bytes())
}

/// Result of checking `last-write.json` against the current `TODO.md` bytes (`R-HIST-3`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutsideEditOutcome {
    /// No outside edit was detected (or no `last-write.json` exists yet, or a live lease is active).
    None,
    /// An outside edit (or expired-lease edit) was recorded as a snapshot.
    Recorded {
        /// Warning message to display once, or `None` for edits made under a lease.
        warning: Option<String>,
    },
}

/// Checks for an outside edit from a read-only command, acquiring the write lock only if a hash
/// mismatch is observed (`R-HIST-3`).
pub fn detect_outside_edit(
    todo_path: &Path,
    label: &str,
    state: &StateDir,
    current_snap: &FileSnapshot,
    lock_cfg: &LockConfig,
) -> Result<OutsideEditOutcome> {
    if !fs_path(state.path()).exists() {
        return Ok(OutsideEditOutcome::None);
    }
    let Some(last) = state.read_last_write()? else {
        return Ok(OutsideEditOutcome::None);
    };
    if last.sha256 == current_snap.sha256_hex() {
        return Ok(OutsideEditOutcome::None);
    }
    if let Some(lease) = state.read_lease()?
        && is_lease_live(&lease, current_time())
    {
        return Ok(OutsideEditOutcome::None);
    }

    let _guard = WriteLockGuard::acquire(state, "todo", "outside-edit", lock_cfg.effective_wait())?;
    let fresh = read_file_with_snapshot(todo_path)?;
    detect_outside_edit_locked(
        label,
        state,
        &fresh.bytes,
        &fresh.snapshot,
        lock_cfg.snapshots_keep,
    )
}

/// Checks for an outside edit while the caller already holds the state write lock (`R-HIST-3`).
pub fn detect_outside_edit_locked(
    label: &str,
    state: &StateDir,
    current_bytes: &[u8],
    current_snap: &FileSnapshot,
    snapshots_keep: u64,
) -> Result<OutsideEditOutcome> {
    let Some(last) = state.read_last_write()? else {
        return Ok(OutsideEditOutcome::None);
    };
    let current_hash = current_snap.sha256_hex();
    if last.sha256 == current_hash {
        return Ok(OutsideEditOutcome::None);
    }

    if let Some(lease) = state.read_lease()? {
        if is_lease_live(&lease, current_time()) {
            return Ok(OutsideEditOutcome::None);
        }
        // Lease expired before unlock: record the hand edit without a warning and clear the expired lease.
        let _ = record_snapshot(
            state,
            current_bytes,
            &current_hash,
            &lease.agent,
            "lease edit",
            false,
            snapshots_keep,
        )?;
        state.remove_lease()?;
        state.write_last_write(&LastWrite::from_snapshot(current_snap))?;
        return Ok(OutsideEditOutcome::Recorded { warning: None });
    }

    let entry = record_snapshot(
        state,
        current_bytes,
        &current_hash,
        "outside",
        "outside edit",
        true,
        snapshots_keep,
    )?;
    state.write_last_write(&LastWrite::from_snapshot(current_snap))?;
    let warning = entry.map(|e| {
        format!(
            "{label}: edited outside the tool (saved snapshot #{:06})",
            e.number
        )
    });
    Ok(OutsideEditOutcome::Recorded { warning })
}

/// Records any hand edits made during a manual lease when `todo unlock` runs, without a warning (`R-HIST-3`, `R-LOCK-6`).
pub fn record_lease_edit_if_changed(
    todo_path: &Path,
    state: &StateDir,
    lease_agent: &str,
    snapshots_keep: u64,
) -> Result<()> {
    let Some(last) = state.read_last_write()? else {
        return Ok(());
    };
    let Ok(fresh) = read_file_with_snapshot(todo_path) else {
        return Ok(());
    };
    let current_hash = fresh.snapshot.sha256_hex();
    if last.sha256 == current_hash {
        return Ok(());
    }
    let _ = record_snapshot(
        state,
        &fresh.bytes,
        &current_hash,
        lease_agent,
        "lease edit",
        false,
        snapshots_keep,
    )?;
    state.write_last_write(&LastWrite::from_snapshot(&fresh.snapshot))?;
    Ok(())
}

/// One entry in `journal.jsonl` recording a completed write (`R-HIST-4`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalEntry {
    /// RFC 3339 timestamp when the write completed.
    pub time: String,
    /// Agent that performed the write.
    pub agent: String,
    /// Subcommand name (e.g. `"claim"`, `"done"`, `"apply"`).
    pub command: String,
    /// Command-line arguments passed to `todo`.
    pub arguments: Vec<String>,
    /// Canonical task IDs touched by this write (added, modified, or removed).
    pub tasks: Vec<String>,
    /// Lowercase hex SHA-256 of `TODO.md` before the write.
    pub hash_before: String,
    /// Lowercase hex SHA-256 of `TODO.md` after the write.
    pub hash_after: String,
}

/// Appends one [`JournalEntry`] line to `journal.jsonl` (`R-HIST-4`).
pub fn append_journal(state: &StateDir, entry: &JournalEntry) -> Result<()> {
    state.ensure_created()?;
    let path = state.journal_path();
    let line = serde_json::to_string(entry).map_err(|e| {
        TodoError::new(
            ErrorCode::WriteFailed,
            format!("failed serializing journal entry: {e}"),
        )
    })?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(fs_path(&path).as_ref())
        .map_err(|e| {
            TodoError::new(
                ErrorCode::WriteFailed,
                format!("failed opening {}: {e}", path.display()),
            )
        })?;
    writeln!(file, "{line}").map_err(|e| {
        TodoError::new(
            ErrorCode::WriteFailed,
            format!("failed writing {}: {e}", path.display()),
        )
    })?;
    Ok(())
}

/// Reads all entries from `journal.jsonl`, returning an empty list if none exist yet (`R-HIST-4`, `R-READ-7`).
pub fn read_journal(state: &StateDir) -> Result<Vec<JournalEntry>> {
    let path = state.journal_path();
    let content = match fs::read_to_string(fs_path(&path).as_ref()) {
        Ok(s) => s,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(TodoError::new(
                ErrorCode::WriteFailed,
                format!("failed reading {}: {e}", path.display()),
            ));
        }
    };

    let mut entries = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let entry: JournalEntry = serde_json::from_str(trimmed).map_err(|e| {
            TodoError::new(
                ErrorCode::WriteFailed,
                format!(
                    "failed parsing {}:{}: {e}",
                    path.display(),
                    idx.saturating_add(1)
                ),
            )
        })?;
        entries.push(entry);
    }
    Ok(entries)
}

/// Computes the canonical task IDs added, modified, or removed between two parsed documents.
#[must_use]
pub fn diff_doc_task_ids(before: &Document, after: &Document) -> Vec<String> {
    let mut before_map: FastU64Map<u64, &[Line]> =
        FastU64Map::with_capacity_and_hasher(before.tasks.len(), Default::default());
    for t in &before.tasks {
        before_map.insert(t.id.number(), &before.line_doc.lines[t.block_range.clone()]);
    }
    let mut after_map: FastU64Map<u64, &[Line]> =
        FastU64Map::with_capacity_and_hasher(after.tasks.len(), Default::default());
    for t in &after.tasks {
        after_map.insert(t.id.number(), &after.line_doc.lines[t.block_range.clone()]);
    }

    let mut touched_after = Vec::new();
    for (&num, &after_lines) in &after_map {
        match before_map.get(&num) {
            Some(&before_lines) if before_lines == after_lines => {}
            _ => touched_after.push(num),
        }
    }
    touched_after.sort_unstable();

    let mut removed_before = Vec::new();
    for &num in before_map.keys() {
        if !after_map.contains_key(&num) {
            removed_before.push(num);
        }
    }
    removed_before.sort_unstable();

    let mut touched = Vec::with_capacity(touched_after.len() + removed_before.len());
    for num in touched_after {
        touched.push(after.id_config.format_id(TaskId(num)));
    }
    for num in removed_before {
        touched.push(before.id_config.format_id(TaskId(num)));
    }
    touched
}

/// Computes the canonical task IDs added, modified, or removed between `before` and `after`.
#[must_use]
pub fn diff_task_ids(before: &[u8], after: &[u8], id_config: &IdConfig) -> Vec<String> {
    let Ok(doc_before) = Document::parse_bytes_with_config(before, id_config.clone()) else {
        return Vec::new();
    };
    let Ok(doc_after) = Document::parse_bytes_with_config(after, id_config.clone()) else {
        return Vec::new();
    };
    diff_doc_task_ids(&doc_before, &doc_after)
}

/// Records one journal entry for a completed write (`R-HIST-4`), using precomputed `tasks`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn record_journal_with_tasks(
    state: &StateDir,
    agent: &str,
    command: &str,
    tasks: Vec<String>,
    hash_before: &str,
    hash_after: &str,
) -> Result<()> {
    let arguments: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|s| s.to_string_lossy().into_owned())
        .collect();
    let entry = JournalEntry {
        time: current_time().to_rfc3339(),
        agent: agent.to_string(),
        command: command.to_string(),
        arguments,
        tasks,
        hash_before: hash_before.to_string(),
        hash_after: hash_after.to_string(),
    };
    append_journal(state, &entry)
}

/// Records one journal entry for a completed write (`R-HIST-4`).
#[allow(clippy::too_many_arguments)]
pub fn record_journal_for_write(
    state: &StateDir,
    id_config: &IdConfig,
    agent: &str,
    command: &str,
    before_bytes: &[u8],
    after_bytes: &[u8],
    hash_before: &str,
    hash_after: &str,
) -> Result<()> {
    let tasks = diff_task_ids(before_bytes, after_bytes, id_config);
    record_journal_with_tasks(state, agent, command, tasks, hash_before, hash_after)
}
