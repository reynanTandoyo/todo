//! The state folder: lock, lease, IDs and last-write records (T-012; R-HIST-1, R-LOCK-7).

use std::ffi::{OsStr, OsString};
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::atomic::{FileSnapshot, atomic_write_bytes};
use crate::error::{ErrorCode, Result, TodoError};
use crate::find::{FoundBy, FoundTodo, canonicalize_clean, fs_path, strip_verbatim_prefix};

/// Contents of `holder.json` written while holding the OS write lock (R-LOCK-3, §7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockHolder {
    /// Agent name holding the lock.
    pub agent: String,
    /// Subcommand holding the lock.
    pub command: String,
    /// Operating-system process ID of the holder.
    pub pid: u32,
    /// RFC 3339 timestamp when the lock was acquired.
    pub started_at: String,
}

/// Contents of `lease.json` for a manual editing lease (`todo lock`, R-LOCK-6, §7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lease {
    /// Agent name holding the lease.
    pub agent: String,
    /// RFC 3339 timestamp when the lease started.
    pub started_at: String,
    /// RFC 3339 timestamp when the lease expires.
    pub expires_at: String,
}

/// Contents of `ids.json` tracking the highest task ID number ever allocated (R-ID-2, §7).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdsState {
    /// Highest numeric task ID ever given out.
    pub highest_id: u64,
}

/// Contents of `last-write.json` recording the target file after the tool's last write (R-HIST-3, §7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastWrite {
    /// File size in bytes after the write.
    pub size: u64,
    /// Modification time seconds since Unix epoch (`0` if unavailable).
    pub mtime_secs: i64,
    /// Modification time sub-second nanoseconds.
    pub mtime_nanos: u32,
    /// Lowercase hex SHA-256 of the file bytes after the write.
    pub sha256: String,
}

impl LastWrite {
    /// Builds a [`LastWrite`] record from a [`FileSnapshot`].
    #[must_use]
    pub fn from_snapshot(snapshot: &FileSnapshot) -> Self {
        let (mtime_secs, mtime_nanos) = system_time_to_unix(snapshot.mtime);
        Self {
            size: snapshot.size,
            mtime_secs,
            mtime_nanos,
            sha256: snapshot.sha256_hex(),
        }
    }
}

fn system_time_to_unix(time: Option<SystemTime>) -> (i64, u32) {
    let Some(t) = time else {
        return (0, 0);
    };
    match t.duration_since(UNIX_EPOCH) {
        Ok(d) => (
            i64::try_from(d.as_secs()).unwrap_or(i64::MAX),
            d.subsec_nanos(),
        ),
        Err(e) => {
            let d = e.duration();
            let secs = i64::try_from(d.as_secs()).unwrap_or(i64::MAX);
            if d.subsec_nanos() == 0 {
                (-secs, 0)
            } else {
                (-secs - 1, 1_000_000_000 - d.subsec_nanos())
            }
        }
    }
}

/// Resolved path to the per-file state folder (R-HIST-1, R-LOCK-7).
///
/// The directory is not created until [`StateDir::ensure_created`] is called, so read-only
/// commands never leave state artifacts behind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateDir {
    path: PathBuf,
    in_git: bool,
}

impl StateDir {
    /// Resolves the state folder for a [`FoundTodo`] result (R-HIST-1, R-LOCK-7).
    ///
    /// Inside git, uses `<git dir>/todo/<repo-relative path of TODO.md>/` (using the git common
    /// folder for the main checkout / shared worktrees, and the worktree gitdir when a local
    /// worktree file is explicitly targeted). Outside git, uses `<dir of TODO.md>/.todo/`.
    pub fn from_found(found: &FoundTodo) -> Result<Self> {
        let canon_file = canonicalize_file_or_parent(&found.path)?;

        if let Some(ref repo_root) = found.repo_root {
            let base_git_dir = if found.found_by == FoundBy::WorktreeMain {
                found.git_common_dir.as_ref().or(found.git_dir.as_ref())
            } else {
                found.git_dir.as_ref().or(found.git_common_dir.as_ref())
            };

            if let Some(git_dir) = base_git_dir {
                let canon_root = canonicalize_clean(repo_root)
                    .unwrap_or_else(|_| strip_verbatim_prefix(repo_root));
                let canon_git =
                    canonicalize_clean(git_dir).unwrap_or_else(|_| strip_verbatim_prefix(git_dir));

                if let Some(rel) = normalized_rel_key(&canon_file, &canon_root) {
                    return Ok(Self {
                        path: canon_git.join("todo").join(rel),
                        in_git: true,
                    });
                }
            }
        }

        let parent = canon_file.parent().unwrap_or_else(|| Path::new("."));
        Ok(Self {
            path: parent.join(".todo"),
            in_git: false,
        })
    }

    /// Returns the path to the state directory.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns `true` if the state directory lives inside a `.git` folder.
    #[must_use]
    pub const fn is_in_git(&self) -> bool {
        self.in_git
    }

    /// Path to `write.lock` (§7).
    #[must_use]
    pub fn write_lock_path(&self) -> PathBuf {
        self.path.join("write.lock")
    }

    /// Path to `holder.json` (§7).
    #[must_use]
    pub fn holder_path(&self) -> PathBuf {
        self.path.join("holder.json")
    }

    /// Path to `lease.json` (§7).
    #[must_use]
    pub fn lease_path(&self) -> PathBuf {
        self.path.join("lease.json")
    }

    /// Path to `ids.json` (§7).
    #[must_use]
    pub fn ids_path(&self) -> PathBuf {
        self.path.join("ids.json")
    }

    /// Path to `last-write.json` (§7).
    #[must_use]
    pub fn last_write_path(&self) -> PathBuf {
        self.path.join("last-write.json")
    }

    /// Path to the `history/` directory (§7).
    #[must_use]
    pub fn history_dir(&self) -> PathBuf {
        self.path.join("history")
    }

    /// Path to `history/index.jsonl` (§7).
    #[must_use]
    pub fn history_index_path(&self) -> PathBuf {
        self.history_dir().join("index.jsonl")
    }

    /// Path to `journal.jsonl` (§7).
    #[must_use]
    pub fn journal_path(&self) -> PathBuf {
        self.path.join("journal.jsonl")
    }

    /// Creates the state directory on demand. Outside git, creates `.todo/.gitignore` containing `*` (R-HIST-1).
    pub fn ensure_created(&self) -> Result<()> {
        std::fs::create_dir_all(fs_path(&self.path).as_ref()).map_err(|e| {
            TodoError::new(
                ErrorCode::WriteFailed,
                format!(
                    "failed to create state directory {}: {e}",
                    self.path.display()
                ),
            )
        })?;

        if !self.in_git {
            let gitignore = self.path.join(".gitignore");
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(fs_path(&gitignore).as_ref())
            {
                Ok(mut f) => {
                    f.write_all(b"*\n").and_then(|()| f.flush()).map_err(|e| {
                        TodoError::new(
                            ErrorCode::WriteFailed,
                            format!("failed writing {}: {e}", gitignore.display()),
                        )
                    })?;
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
                Err(e) => {
                    return Err(TodoError::new(
                        ErrorCode::WriteFailed,
                        format!("failed creating {}: {e}", gitignore.display()),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Reads `holder.json`, returning `Ok(None)` if it does not exist.
    pub fn read_holder(&self) -> Result<Option<LockHolder>> {
        read_json_optional(&self.holder_path())
    }

    /// Atomically writes `holder.json`, creating the state folder if needed.
    pub fn write_holder(&self, holder: &LockHolder) -> Result<()> {
        self.ensure_created()?;
        write_json_atomic(&self.holder_path(), holder)
    }

    /// Removes `holder.json` if present.
    pub fn remove_holder(&self) -> Result<()> {
        remove_if_exists(&self.holder_path())
    }

    /// Reads `lease.json`, returning `Ok(None)` if it does not exist.
    pub fn read_lease(&self) -> Result<Option<Lease>> {
        read_json_optional(&self.lease_path())
    }

    /// Atomically writes `lease.json`, creating the state folder if needed.
    pub fn write_lease(&self, lease: &Lease) -> Result<()> {
        self.ensure_created()?;
        write_json_atomic(&self.lease_path(), lease)
    }

    /// Removes `lease.json` if present.
    pub fn remove_lease(&self) -> Result<()> {
        remove_if_exists(&self.lease_path())
    }

    /// Reads `ids.json`, returning `Ok(None)` if it does not exist.
    pub fn read_ids(&self) -> Result<Option<IdsState>> {
        read_json_optional(&self.ids_path())
    }

    /// Atomically writes `ids.json`, creating the state folder if needed.
    pub fn write_ids(&self, ids: &IdsState) -> Result<()> {
        self.ensure_created()?;
        write_json_atomic(&self.ids_path(), ids)
    }

    /// Reads `last-write.json`, returning `Ok(None)` if it does not exist.
    pub fn read_last_write(&self) -> Result<Option<LastWrite>> {
        read_json_optional(&self.last_write_path())
    }

    /// Atomically writes `last-write.json`, creating the state folder if needed.
    pub fn write_last_write(&self, last_write: &LastWrite) -> Result<()> {
        self.ensure_created()?;
        write_json_atomic(&self.last_write_path(), last_write)
    }
}

/// Resolves the task file's path by canonicalizing its folder, never the file itself.
///
/// While another process replaces `TODO.md` with a rename, opening the file to canonicalize
/// it can resolve to the replaced copy (`C:\$Extend\$Deleted\…` on Windows). That would give
/// this process a different state folder, and so a different lock (R-LOCK-7). The folder is
/// never replaced, so it resolves reliably.
fn canonicalize_file_or_parent(path: &Path) -> Result<PathBuf> {
    if let (Some(parent), Some(name)) = (path.parent(), path.file_name()) {
        let parent = if parent.as_os_str().is_empty() {
            Path::new(".")
        } else {
            parent
        };
        if let Ok(canon_parent) = canonicalize_clean(parent) {
            let name = name_as_stored(&canon_parent, name);
            return Ok(canon_parent.join(name));
        }
    }
    let abs = std::path::absolute(path).map_err(|e| {
        TodoError::new(
            ErrorCode::NoFile,
            format!("could not resolve path {}: {e}", path.display()),
        )
    })?;
    Ok(strip_verbatim_prefix(&abs))
}

/// The file name as stored in `dir`. Windows matches names ignoring case, so `todo.md` and
/// `TODO.md` must give the same state folder (R-LOCK-7); the listing gives the stored
/// spelling without opening the file. Falls back to `name` when nothing matches.
fn name_as_stored(dir: &Path, name: &OsStr) -> OsString {
    if cfg!(windows)
        && let Some(wanted) = name.to_str().map(str::to_lowercase)
        && let Ok(entries) = std::fs::read_dir(fs_path(dir).as_ref())
    {
        for entry in entries.flatten() {
            let stored = entry.file_name();
            if stored.to_str().is_some_and(|s| s.to_lowercase() == wanted) {
                return stored;
            }
        }
    }
    name.to_os_string()
}

fn normalized_rel_key(file: &Path, root: &Path) -> Option<PathBuf> {
    let rel = if let Ok(r) = file.strip_prefix(root) {
        r.to_path_buf()
    } else {
        #[cfg(windows)]
        {
            strip_prefix_case_insensitive(file, root)?
        }
        #[cfg(not(windows))]
        {
            return None;
        }
    };

    let mut out = PathBuf::new();
    for comp in rel.components() {
        if let Component::Normal(part) = comp {
            #[cfg(windows)]
            {
                out.push(part.to_string_lossy().to_lowercase());
            }
            #[cfg(not(windows))]
            {
                out.push(part);
            }
        }
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}

#[cfg(windows)]
fn strip_prefix_case_insensitive(path: &Path, prefix: &Path) -> Option<PathBuf> {
    let p_comps: Vec<Component<'_>> = path.components().collect();
    let pre_comps: Vec<Component<'_>> = prefix.components().collect();
    if p_comps.len() < pre_comps.len() {
        return None;
    }
    for (a, b) in p_comps.iter().zip(pre_comps.iter()) {
        if !a
            .as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(&b.as_os_str().to_string_lossy())
        {
            return None;
        }
    }
    let mut rest = PathBuf::new();
    for c in &p_comps[pre_comps.len()..] {
        rest.push(c.as_os_str());
    }
    Some(rest)
}

fn read_json_optional<T: DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    match std::fs::read(fs_path(path).as_ref()) {
        Ok(bytes) => {
            let value = serde_json::from_slice(&bytes).map_err(|e| {
                TodoError::new(
                    ErrorCode::WriteFailed,
                    format!("failed parsing {}: {e}", path.display()),
                )
            })?;
            Ok(Some(value))
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(TodoError::new(
            ErrorCode::WriteFailed,
            format!("failed reading {}: {e}", path.display()),
        )),
    }
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|e| {
        TodoError::new(
            ErrorCode::WriteFailed,
            format!("failed serializing {}: {e}", path.display()),
        )
    })?;
    bytes.push(b'\n');
    atomic_write_bytes(path, &bytes)
}

fn remove_if_exists(path: &Path) -> Result<()> {
    match std::fs::remove_file(fs_path(path).as_ref()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(TodoError::new(
            ErrorCode::WriteFailed,
            format!("failed removing {}: {e}", path.display()),
        )),
    }
}
