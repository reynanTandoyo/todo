//! The write lock and the manual lease (T-013, T-015; R-LOCK-1..8).

use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use chrono::{DateTime, FixedOffset, Local, Utc};
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
}

impl Default for LockConfig {
    fn default() -> Self {
        Self {
            lock_wait_seconds: 30,
            lease_minutes: 10,
            lease_max_minutes: 60,
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
    mut modify: F,
) -> Result<T>
where
    F: FnMut(&[u8], &FileSnapshot) -> Result<WriteOutcome<T>>,
{
    let wait_timeout = cfg.effective_wait();
    let deadline = Instant::now() + wait_timeout;

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

    let read = atomic::read_file_with_snapshot(todo_path)?;
    match modify(&read.bytes, &read.snapshot)? {
        WriteOutcome::Unchanged(val) => {
            drop(guard);
            Ok(val)
        }
        WriteOutcome::Changed { new_bytes, value } => {
            atomic::atomic_replace(todo_path, &read.snapshot, &new_bytes)?;
            let post_mtime = std::fs::metadata(fs_path(todo_path).as_ref())
                .ok()
                .and_then(|m| m.modified().ok());
            let post_snap = FileSnapshot {
                size: new_bytes.len() as u64,
                mtime: post_mtime,
                sha256: atomic::sha256_bytes(&new_bytes),
            };
            state.write_last_write(&LastWrite::from_snapshot(&post_snap))?;
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
