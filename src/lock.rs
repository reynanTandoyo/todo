//! The write lock and the manual lease (T-013, T-015; R-LOCK-1..8).

use std::fs::{File, OpenOptions, TryLockError};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use chrono::{DateTime, FixedOffset, Local, Utc};
use serde::Serialize;
use serde_json::Value;

use crate::atomic::{self, FileSnapshot, random_u64};
use crate::error::{ErrorCode, Result, TodoError};
use crate::find::fs_path;
use crate::state::{LastWrite, Lease, LockHolder, StateDir};

const RETRY_MIN_MS: u64 = 40;
const RETRY_MAX_MS: u64 = 160;

/// Timing configuration for locks and leases (from `todo.toml` defaults in §5.14).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LockConfig {
    /// How long a write waits for the write lock or a live lease (`lock_wait_seconds`, default 30).
    pub lock_wait_seconds: u64,
    /// Default manual lease duration in minutes (`lease_minutes`, default 10).
    pub lease_minutes: u64,
    /// Maximum allowed manual lease duration in minutes (`lease_max_minutes`, default 60).
    pub lease_max_minutes: u64,
    /// Number of snapshots to keep in `history/` (`snapshots_keep`, default 200).
    pub snapshots_keep: u64,
}

impl Default for LockConfig {
    fn default() -> Self {
        Self {
            lock_wait_seconds: 30,
            lease_minutes: 10,
            lease_max_minutes: 60,
            snapshots_keep: 200,
        }
    }
}

impl LockConfig {
    /// Returns the effective wait timeout for acquiring the write lock or waiting on a lease.
    #[must_use]
    pub fn effective_wait(&self) -> Duration {
        #[cfg(feature = "test-hooks")]
        {
            if let Ok(ms_str) = std::env::var("TODO_TEST_LOCK_WAIT_MS")
                && let Ok(ms) = ms_str.parse::<u64>()
            {
                return Duration::from_millis(ms);
            }
        }
        Duration::from_secs(self.lock_wait_seconds)
    }
}

/// Returns the current local time with its UTC offset (overridable in `test-hooks` builds).
#[must_use]
pub fn current_time() -> DateTime<FixedOffset> {
    #[cfg(feature = "test-hooks")]
    {
        if let Ok(raw) = std::env::var("TODO_TEST_NOW")
            && let Ok(dt) = DateTime::parse_from_rfc3339(raw.trim())
        {
            return dt;
        }
    }
    Local::now().fixed_offset()
}

/// Returns `true` if `lease` has not yet expired relative to `now` (compared in UTC).
#[must_use]
pub fn is_lease_live(lease: &Lease, now: DateTime<FixedOffset>) -> bool {
    let Ok(expires) = DateTime::parse_from_rfc3339(&lease.expires_at) else {
        return false;
    };
    now.with_timezone(&Utc) < expires.with_timezone(&Utc)
}

/// Outcome returned by a `locked_read_modify_write` transformation closure.
#[derive(Debug, Clone)]
pub enum WriteOutcome<T> {
    /// No lines changed; `TODO.md` is left untouched on disk (R-EDIT-6).
    Unchanged(T),
    /// New file bytes to atomically write to `TODO.md`.
    Changed {
        /// Complete replacement bytes for `TODO.md`.
        new_bytes: Vec<u8>,
        /// Caller-defined value returned after the write succeeds.
        value: T,
    },
}

/// Internal outcome allowing callers that already parsed both before and after documents
/// to supply precomputed `touched_tasks` for the write journal (avoiding double re-parse).
pub(crate) enum WriteOutcomeFull<T> {
    Unchanged(T),
    Changed {
        new_bytes: Vec<u8>,
        touched_tasks: Option<Vec<String>>,
        value: T,
        /// The migration that this write applies to the file or its archive (R-MIG-1), if any.
        migration: Option<MigrationWrite>,
    },
}

/// A migration that a write applies in the same step as its own change (R-MIG-1): what the
/// journal entry and the standard error line say, and the archive file when it is migrated too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MigrationWrite {
    /// What was migrated, such as `TODO.md: format 1 to 2`.
    pub summary: String,
    /// The archive file, written after the snapshot of `TODO.md` and before `TODO.md` is replaced.
    pub archive: Option<ArchiveWrite>,
}

/// A whole-file replacement of the archive file, guarded by the snapshot it was read with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArchiveWrite {
    /// Path of `TODO-archive.md`.
    pub path: PathBuf,
    /// Snapshot of the archive file when it was read; the write fails if it changed since.
    pub expected: FileSnapshot,
    /// The migrated archive text.
    pub new_bytes: Vec<u8>,
}

/// RAII guard holding the operating-system exclusive lock on `write.lock`.
///
/// Removes `holder.json` before releasing `write.lock` on drop.
pub struct WriteLockGuard {
    file: File,
    holder_path: PathBuf,
}

impl WriteLockGuard {
    /// Acquires the OS write lock on `state.write_lock_path()`, retrying every 40–160 ms up to `wait_timeout`.
    pub fn acquire(
        state: &StateDir,
        agent: &str,
        command: &str,
        wait_timeout: Duration,
    ) -> Result<Self> {
        state.ensure_created()?;
        let lock_path = state.write_lock_path();
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(fs_path(&lock_path).as_ref())
            .map_err(|e| {
                TodoError::new(
                    ErrorCode::WriteFailed,
                    format!("failed opening lock file {}: {e}", lock_path.display()),
                )
            })?;

        let start = Instant::now();
        loop {
            match file.try_lock() {
                Ok(()) => {
                    let holder = LockHolder {
                        agent: agent.to_owned(),
                        command: command.to_owned(),
                        pid: std::process::id(),
                        started_at: current_time().to_rfc3339(),
                    };
                    if let Err(e) = state.write_holder(&holder) {
                        let _ = file.unlock();
                        return Err(e);
                    }
                    return Ok(Self {
                        file,
                        holder_path: state.holder_path(),
                    });
                }
                Err(TryLockError::WouldBlock) => {
                    let elapsed = start.elapsed();
                    if elapsed >= wait_timeout {
                        return Err(busy_error(state));
                    }
                    let remaining = wait_timeout.saturating_sub(elapsed);
                    let jitter = random_retry_delay().min(remaining);
                    if !jitter.is_zero() {
                        std::thread::sleep(jitter);
                    }
                }
                Err(TryLockError::Error(e)) => {
                    return Err(TodoError::new(
                        ErrorCode::WriteFailed,
                        format!("failed locking {}: {e}", lock_path.display()),
                    ));
                }
            }
        }
    }
}

impl Drop for WriteLockGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(fs_path(&self.holder_path).as_ref());
        let _ = self.file.unlock();
    }
}

/// Executes a write operation under the exclusive write lock (R-LOCK-1..4, R-LOCK-6).
///
/// 1. Acquires `write.lock` (retrying every 40–160 ms up to `cfg.effective_wait()`).
/// 2. Checks `lease.json`; if another agent holds a live lease, releases `write.lock`, waits, and
///    retries up to the same timeout before returning `LEASE_HELD` (exit 3).
/// 3. Reads `todo_path` and its metadata/SHA-256 snapshot under the lock (never from a pre-lock copy).
/// 4. Calls `modify(&bytes, &snapshot)`.
/// 5. If `WriteOutcome::Changed`, atomically replaces `todo_path` and updates `last-write.json`.
/// 6. Releases the write lock when returning.
pub fn locked_read_modify_write<T, F>(
    todo_path: &Path,
    state: &StateDir,
    agent: &str,
    command: &str,
    cfg: &LockConfig,
    modify: F,
) -> Result<T>
where
    F: FnMut(&[u8], &FileSnapshot) -> Result<WriteOutcome<T>>,
{
    let mut warnings = Vec::new();
    locked_read_modify_write_with_warnings(
        todo_path,
        state,
        agent,
        command,
        cfg,
        &mut warnings,
        modify,
    )
}

/// Like [`locked_read_modify_write`], but also collects any outside-edit warnings (`R-HIST-3`)
/// into `warnings`.
pub fn locked_read_modify_write_with_warnings<T, F>(
    todo_path: &Path,
    state: &StateDir,
    agent: &str,
    command: &str,
    cfg: &LockConfig,
    warnings: &mut Vec<String>,
    mut modify: F,
) -> Result<T>
where
    F: FnMut(&[u8], &FileSnapshot) -> Result<WriteOutcome<T>>,
{
    locked_read_modify_write_full(
        todo_path,
        state,
        agent,
        command,
        cfg,
        None,
        warnings,
        |bytes, snap| {
            Ok(match modify(bytes, snap)? {
                WriteOutcome::Unchanged(val) => WriteOutcomeFull::Unchanged(val),
                WriteOutcome::Changed { new_bytes, value } => WriteOutcomeFull::Changed {
                    new_bytes,
                    touched_tasks: None,
                    value,
                    migration: None,
                },
            })
        },
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn locked_read_modify_write_full<T, F>(
    todo_path: &Path,
    state: &StateDir,
    agent: &str,
    command: &str,
    cfg: &LockConfig,
    id_config: Option<&crate::doc::IdConfig>,
    warnings: &mut Vec<String>,
    mut modify: F,
) -> Result<T>
where
    F: FnMut(&[u8], &FileSnapshot) -> Result<WriteOutcomeFull<T>>,
{
    let wait_timeout = cfg.effective_wait();
    let deadline = Instant::now() + wait_timeout;

    let lock_timer = crate::timing::PhaseTimer::start("lock.acquire");
    let guard = loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let guard = WriteLockGuard::acquire(state, agent, command, remaining)?;

        if let Some(lease) = state.read_lease()?
            && is_lease_live(&lease, current_time())
            && !lease.agent.eq_ignore_ascii_case(agent)
        {
            drop(guard);
            let now_inst = Instant::now();
            if now_inst >= deadline {
                return Err(lease_held_error(&lease));
            }
            let remaining = deadline.saturating_duration_since(now_inst);
            let jitter = random_retry_delay().min(remaining);
            if !jitter.is_zero() {
                std::thread::sleep(jitter);
            }
            continue;
        }

        break guard;
    };
    lock_timer.finish();

    let read_timer = crate::timing::PhaseTimer::start("lock.read_file");
    let read = atomic::read_file_with_snapshot(todo_path)?;
    read_timer.finish();
    let label = todo_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "TODO.md".to_string());
    let outside_timer = crate::timing::PhaseTimer::start("lock.outside_edit");
    if let crate::history::OutsideEditOutcome::Recorded {
        warning: Some(warn),
    } = crate::history::detect_outside_edit_locked(
        &label,
        state,
        &read.bytes,
        &read.snapshot,
        cfg.snapshots_keep,
    )? {
        warnings.push(warn);
    }
    outside_timer.finish();
    match modify(&read.bytes, &read.snapshot)? {
        WriteOutcomeFull::Unchanged(val) => {
            drop(guard);
            Ok(val)
        }
        WriteOutcomeFull::Changed {
            new_bytes,
            touched_tasks,
            value,
            migration,
        } => {
            let before_sha_hex = read.snapshot.sha256_hex();
            let snap_timer = crate::timing::PhaseTimer::start("lock.record_snapshot");
            crate::history::record_snapshot(
                state,
                &read.bytes,
                &before_sha_hex,
                agent,
                command,
                false,
                cfg.snapshots_keep,
            )?;
            snap_timer.finish();
            // A migrated archive is written after the snapshot and before TODO.md is replaced, so
            // a failed archive write leaves TODO.md as it was (R-MIG-1).
            if let Some(archive) = migration.as_ref().and_then(|m| m.archive.as_ref()) {
                atomic::atomic_replace(&archive.path, &archive.expected, &archive.new_bytes)?;
            }
            let replace_timer = crate::timing::PhaseTimer::start("lock.atomic_replace");
            atomic::atomic_replace(todo_path, &read.snapshot, &new_bytes)?;
            replace_timer.finish();
            let post_timer = crate::timing::PhaseTimer::start("lock.post_hash_and_last_write");
            let post_mtime = std::fs::metadata(fs_path(todo_path).as_ref())
                .ok()
                .and_then(|m| m.modified().ok());
            let post_snap = FileSnapshot {
                size: new_bytes.len() as u64,
                mtime: post_mtime,
                sha256: atomic::sha256_bytes(&new_bytes),
            };
            state.write_last_write(&LastWrite::from_snapshot(&post_snap))?;
            post_timer.finish();
            let journal_timer = crate::timing::PhaseTimer::start("lock.journal");
            let after_sha_hex = post_snap.sha256_hex();
            let tasks = match touched_tasks {
                Some(t) => t,
                None => {
                    let resolved_id_config = id_config.cloned().unwrap_or_else(|| {
                        crate::ops::OpsConfig::load_next_to(todo_path)
                            .map(|c| c.id_config)
                            .unwrap_or_default()
                    });
                    crate::history::diff_task_ids(&read.bytes, &new_bytes, &resolved_id_config)
                }
            };
            crate::history::record_journal_with_tasks(
                state,
                agent,
                command,
                tasks,
                &before_sha_hex,
                &after_sha_hex,
                migration.as_ref().map(|m| m.summary.as_str()),
            )?;
            journal_timer.finish();
            drop(guard);
            Ok(value)
        }
    }
}

pub(crate) fn random_retry_delay() -> Duration {
    let span = RETRY_MAX_MS - RETRY_MIN_MS + 1;
    let ms = RETRY_MIN_MS + (random_u64() % span);
    Duration::from_millis(ms)
}

fn busy_error(state: &StateDir) -> TodoError {
    match state.read_holder().ok().flatten() {
        Some(h) => TodoError::new(
            ErrorCode::Busy,
            format!(
                "write lock busy: held by agent {} (command `{}`, pid {}, since {})",
                h.agent, h.command, h.pid, h.started_at
            ),
        )
        .with_owner(h.agent)
        .with_detail("command", Value::String(h.command))
        .with_detail("pid", Value::Number(h.pid.into()))
        .with_detail("started_at", Value::String(h.started_at)),
        None => TodoError::new(ErrorCode::Busy, "write lock busy: held by another process"),
    }
}

pub(crate) fn lease_held_error(lease: &Lease) -> TodoError {
    TodoError::new(
        ErrorCode::LeaseHeld,
        format!(
            "manual lease held by {} until {}",
            lease.agent, lease.expires_at
        ),
    )
    .with_owner(lease.agent.clone())
    .with_detail("started_at", Value::String(lease.started_at.clone()))
    .with_detail("expires_at", Value::String(lease.expires_at.clone()))
}

/// Active manual lease information returned by [`query_status`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LeaseStatus {
    /// Agent holding the lease.
    pub agent: String,
    /// RFC 3339 timestamp when the lease started.
    pub started_at: String,
    /// RFC 3339 timestamp when the lease expires.
    pub expires_at: String,
    /// Whole seconds remaining until expiry (at least `0`).
    pub remaining_seconds: i64,
}

/// Snapshot of the write lock and manual lease returned by [`query_status`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LockStatus {
    /// Whether `write.lock` is currently locked by an OS process.
    pub write_lock_held: bool,
    /// Holder metadata from `holder.json` if `write_lock_held` is `true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holder: Option<LockHolder>,
    /// Active (non-expired) manual lease, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lease: Option<LeaseStatus>,
}

/// Outcome of [`release_lease`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnlockOutcome {
    /// Ended the caller's own active lease.
    Unlocked { agent: String },
    /// Ended another agent's active lease via `--force`.
    Forced { previous_holder: String },
    /// No active lease existed (or an expired lease was cleared).
    NotActive,
}

/// Acquires or refreshes a manual editing lease for `agent` (`todo lock`, R-LOCK-6).
///
/// Waits up to `cfg.effective_wait()` if another agent holds the write lock or a live lease.
pub fn acquire_lease(
    state: &StateDir,
    agent: &str,
    minutes: Option<u64>,
    cfg: &LockConfig,
) -> Result<Lease> {
    let mins = minutes.unwrap_or(cfg.lease_minutes);
    if mins == 0 || mins > cfg.lease_max_minutes {
        return Err(TodoError::new(
            ErrorCode::BadValue,
            format!(
                "lease duration must be between 1 and {} minutes (got {mins})",
                cfg.lease_max_minutes
            ),
        ));
    }

    let wait_timeout = cfg.effective_wait();
    let deadline = Instant::now() + wait_timeout;

    let guard = loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let guard = WriteLockGuard::acquire(state, agent, "lock", remaining)?;

        if let Some(existing) = state.read_lease()?
            && is_lease_live(&existing, current_time())
            && !existing.agent.eq_ignore_ascii_case(agent)
        {
            drop(guard);
            let now_inst = Instant::now();
            if now_inst >= deadline {
                return Err(lease_held_error(&existing));
            }
            let remaining = deadline.saturating_duration_since(now_inst);
            let jitter = random_retry_delay().min(remaining);
            if !jitter.is_zero() {
                std::thread::sleep(jitter);
            }
            continue;
        }
        break guard;
    };

    let now = current_time();
    let duration = lease_duration(mins);
    let expires = now + duration;
    let lease = Lease {
        agent: agent.to_owned(),
        started_at: now.to_rfc3339(),
        expires_at: expires.to_rfc3339(),
    };
    state.write_lease(&lease)?;
    drop(guard);
    Ok(lease)
}

fn lease_duration(minutes: u64) -> chrono::Duration {
    #[cfg(feature = "test-hooks")]
    {
        if let Ok(secs_str) = std::env::var("TODO_TEST_LEASE_SECONDS")
            && let Ok(secs) = secs_str.parse::<i64>()
            && secs > 0
        {
            return chrono::Duration::seconds(secs);
        }
    }
    let mins_i64 = i64::try_from(minutes).unwrap_or(60);
    chrono::Duration::minutes(mins_i64)
}

/// Ends an active manual editing lease (`todo unlock`, R-LOCK-6).
///
/// An expired lease counts as already ended. Ending another agent's live lease requires
/// `force == true`; otherwise returns `NOT_OWNER` (exit 1).
pub fn release_lease(
    state: &StateDir,
    agent: &str,
    force: bool,
    cfg: &LockConfig,
) -> Result<UnlockOutcome> {
    release_lease_inner(None, state, agent, force, cfg)
}

/// Ends an active manual editing lease and records any edits made to `todo_path` during the
/// lease as a snapshot without emitting an outside-edit warning (`R-LOCK-6`, `R-HIST-3`).
pub fn release_lease_with_file(
    todo_path: &Path,
    state: &StateDir,
    agent: &str,
    force: bool,
    cfg: &LockConfig,
) -> Result<UnlockOutcome> {
    release_lease_inner(Some(todo_path), state, agent, force, cfg)
}

fn release_lease_inner(
    todo_path: Option<&Path>,
    state: &StateDir,
    agent: &str,
    force: bool,
    cfg: &LockConfig,
) -> Result<UnlockOutcome> {
    let guard = WriteLockGuard::acquire(state, agent, "unlock", cfg.effective_wait())?;

    let outcome = match state.read_lease()? {
        None => UnlockOutcome::NotActive,
        Some(lease) if !is_lease_live(&lease, current_time()) => {
            if let Some(path) = todo_path {
                crate::history::record_lease_edit_if_changed(
                    path,
                    state,
                    &lease.agent,
                    cfg.snapshots_keep,
                )?;
            }
            state.remove_lease()?;
            UnlockOutcome::NotActive
        }
        Some(lease) if lease.agent.eq_ignore_ascii_case(agent) => {
            if let Some(path) = todo_path {
                crate::history::record_lease_edit_if_changed(
                    path,
                    state,
                    &lease.agent,
                    cfg.snapshots_keep,
                )?;
            }
            state.remove_lease()?;
            UnlockOutcome::Unlocked { agent: lease.agent }
        }
        Some(lease) => {
            if force {
                if let Some(path) = todo_path {
                    crate::history::record_lease_edit_if_changed(
                        path,
                        state,
                        &lease.agent,
                        cfg.snapshots_keep,
                    )?;
                }
                state.remove_lease()?;
                UnlockOutcome::Forced {
                    previous_holder: lease.agent,
                }
            } else {
                drop(guard);
                return Err(TodoError::new(
                    ErrorCode::NotOwner,
                    format!(
                        "lease is held by {} until {}; pass --force to end it",
                        lease.agent, lease.expires_at
                    ),
                )
                .with_owner(lease.agent)
                .with_detail("expires_at", Value::String(lease.expires_at)));
            }
        }
    };

    drop(guard);
    Ok(outcome)
}

/// Reads the current write-lock and lease status without blocking or creating state files (R-LOCK-5, R-LOCK-6).
pub fn query_status(state: &StateDir) -> Result<LockStatus> {
    if !fs_path(state.path()).exists() {
        return Ok(LockStatus {
            write_lock_held: false,
            holder: None,
            lease: None,
        });
    }

    let write_lock_held = is_write_lock_held(state).unwrap_or(false);
    let holder = if write_lock_held {
        state.read_holder().ok().flatten()
    } else {
        None
    };

    let now = current_time();
    let lease = match state.read_lease()? {
        Some(l) if is_lease_live(&l, now) => {
            let remaining_seconds = DateTime::parse_from_rfc3339(&l.expires_at)
                .map(|exp| {
                    exp.with_timezone(&Utc)
                        .signed_duration_since(now.with_timezone(&Utc))
                        .num_seconds()
                        .max(0)
                })
                .unwrap_or(0);
            Some(LeaseStatus {
                agent: l.agent,
                started_at: l.started_at,
                expires_at: l.expires_at,
                remaining_seconds,
            })
        }
        _ => None,
    };

    Ok(LockStatus {
        write_lock_held,
        holder,
        lease,
    })
}

fn is_write_lock_held(state: &StateDir) -> io::Result<bool> {
    let lock_path = state.write_lock_path();
    let file = match OpenOptions::new()
        .read(true)
        .write(true)
        .open(fs_path(&lock_path).as_ref())
    {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e),
    };
    match file.try_lock() {
        Ok(()) => {
            let _ = file.unlock();
            Ok(false)
        }
        Err(TryLockError::WouldBlock) => Ok(true),
        Err(TryLockError::Error(e)) => Err(e),
    }
}
