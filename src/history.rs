//! Numbered snapshots, history index, and outside-edit tracking (`R-HIST-2`, `R-HIST-3`).

use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::atomic::atomic_write_bytes;
use crate::error::{ErrorCode, Result, TodoError};
use crate::find::fs_path;
use crate::lock::current_time;
use crate::state::StateDir;

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
