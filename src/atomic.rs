//! Atomic replacement of TODO.md (T-014; R-IO-1..5).

use std::ffi::OsStr;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime};

use sha2::{Digest, Sha256};

use crate::error::{ErrorCode, Result, TodoError};
use crate::find::fs_path;

const RENAME_RETRY_BUDGET: Duration = Duration::from_secs(2);
const RENAME_RETRY_STEP: Duration = Duration::from_millis(25);

/// File size, modification time, and SHA-256 digest captured when reading a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSnapshot {
    /// File length in bytes.
    pub size: u64,
    /// Last modification timestamp from filesystem metadata, if available.
    pub mtime: Option<SystemTime>,
    /// Raw SHA-256 digest of the file contents.
    pub sha256: [u8; 32],
}

impl FileSnapshot {
    /// Returns the SHA-256 digest formatted as lowercase hexadecimal.
    #[must_use]
    pub fn sha256_hex(&self) -> String {
        encode_hex(&self.sha256)
    }
}

/// Contents and metadata snapshot obtained when reading a file under the write lock.
#[derive(Debug, Clone)]
pub struct ReadFile {
    /// Exact bytes read from disk.
    pub bytes: Vec<u8>,
    /// Metadata and hash snapshot for R-IO-2 verification.
    pub snapshot: FileSnapshot,
}

/// Computes lowercase hex SHA-256 of arbitrary bytes.
#[must_use]
pub fn sha256_bytes(bytes: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(bytes);
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

/// Formats a 32-byte digest as lowercase hex.
#[must_use]
pub fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

/// Validates UTF-8 bytes and reports the exact byte position and 1-based line on error (R-IO-4, format.md §13).
pub fn validate_utf8<'a>(bytes: &'a [u8], file_label: &str) -> Result<&'a str> {
    std::str::from_utf8(bytes).map_err(|err| {
        let byte_pos = err.valid_up_to();
        let line = bytes[..byte_pos].iter().filter(|&&b| b == b'\n').count() + 1;
        TodoError::new(
            ErrorCode::UnsafeFile,
            format!("invalid UTF-8 at byte {byte_pos}"),
        )
        .with_file(file_label)
        .with_line(line)
    })
}

/// Reads a file into memory and records its size, modification time, and SHA-256 hash.
pub fn read_file_with_snapshot(path: &Path) -> Result<ReadFile> {
    let fp = fs_path(path);
    let mut file = File::open(fp.as_ref()).map_err(|e| map_read_error(path, &e))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|e| map_read_error(path, &e))?;
    let meta = file.metadata().map_err(|e| map_read_error(path, &e))?;
    let snapshot = FileSnapshot {
        size: bytes.len() as u64,
        mtime: meta.modified().ok(),
        sha256: sha256_bytes(&bytes),
    };
    Ok(ReadFile { bytes, snapshot })
}

/// Refuses a read-only target file with `READ_ONLY` (exit 4) per R-IO-5.
pub fn check_writable(path: &Path) -> Result<()> {
    let fp = fs_path(path);
    let meta = std::fs::metadata(fp.as_ref()).map_err(|e| map_read_error(path, &e))?;
    if meta.permissions().readonly() {
        return Err(TodoError::new(
            ErrorCode::ReadOnly,
            format!("{} is read-only and cannot be modified", path.display()),
        )
        .with_file(path.display().to_string()));
    }
    Ok(())
}

/// Atomically replaces `target` with `new_bytes` (R-IO-1, R-IO-2, R-IO-3, R-IO-5).
///
/// Writes to `<target>.<pid>.<random>.tmp` in the same folder, flushes to disk, verifies that
/// `target` still matches `expected` (size, modification time, and SHA-256), and renames over
/// `target` (retrying up to 2 seconds on Windows sharing violations). Never leaves a temporary
/// file behind.
pub fn atomic_replace(target: &Path, expected: &FileSnapshot, new_bytes: &[u8]) -> Result<()> {
    check_writable(target)?;

    let (tmp_path, mut tmp_file, mut guard) = create_temp_file_for(target)?;
    tmp_file
        .write_all(new_bytes)
        .and_then(|()| tmp_file.flush())
        .and_then(|()| tmp_file.sync_data())
        .map_err(|e| {
            TodoError::new(
                ErrorCode::WriteFailed,
                format!("failed writing temporary file {}: {e}", tmp_path.display()),
            )
            .with_file(target.display().to_string())
        })?;
    drop(tmp_file);

    #[cfg(feature = "test-hooks")]
    maybe_run_before_rename_hook();

    verify_unchanged(target, expected)?;
    rename_with_retry(&tmp_path, target, true)?;
    guard.disarm();
    Ok(())
}

/// Atomically writes or replaces an auxiliary file (such as state JSON files) without pre-read verification.
pub(crate) fn atomic_write_bytes(target: &Path, bytes: &[u8]) -> Result<()> {
    let (tmp_path, mut tmp_file, mut guard) = create_temp_file_for(target)?;
    tmp_file
        .write_all(bytes)
        .and_then(|()| tmp_file.flush())
        .map_err(|e| {
            TodoError::new(
                ErrorCode::WriteFailed,
                format!("failed writing temporary file {}: {e}", tmp_path.display()),
            )
            .with_file(target.display().to_string())
        })?;
    drop(tmp_file);

    rename_with_retry(&tmp_path, target, false)?;
    guard.disarm();
    Ok(())
}

fn verify_unchanged(target: &Path, expected: &FileSnapshot) -> Result<()> {
    let deadline = Instant::now() + RENAME_RETRY_BUDGET;
    let current = loop {
        match read_snapshot_for_verify(target) {
            Ok(snap) => break snap,
            Err(VerifyReadError::NotFound) => {
                return Err(TodoError::new(
                    ErrorCode::ChangedDuringWrite,
                    format!(
                        "{} disappeared while preparing write; nothing was written",
                        target.display()
                    ),
                )
                .with_file(target.display().to_string()));
            }
            Err(VerifyReadError::Io(e)) => {
                if is_sharing_violation(&e) && Instant::now() < deadline {
                    std::thread::sleep(RENAME_RETRY_STEP);
                    continue;
                }
                return Err(TodoError::new(
                    ErrorCode::WriteFailed,
                    format!("could not verify {} before rename: {e}", target.display()),
                )
                .with_file(target.display().to_string()));
            }
        }
    };

    if &current != expected {
        return Err(TodoError::new(
            ErrorCode::ChangedDuringWrite,
            format!(
                "{} was modified by another program during the write; nothing was written",
                target.display()
            ),
        )
        .with_file(target.display().to_string()));
    }
    Ok(())
}

enum VerifyReadError {
    NotFound,
    Io(io::Error),
}

fn read_snapshot_for_verify(target: &Path) -> std::result::Result<FileSnapshot, VerifyReadError> {
    let fp = fs_path(target);
    let mut file = File::open(fp.as_ref()).map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            VerifyReadError::NotFound
        } else {
            VerifyReadError::Io(e)
        }
    })?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(VerifyReadError::Io)?;
    let meta = file.metadata().map_err(VerifyReadError::Io)?;
    Ok(FileSnapshot {
        size: bytes.len() as u64,
        mtime: meta.modified().ok(),
        sha256: sha256_bytes(&bytes),
    })
}

fn rename_with_retry(tmp_path: &Path, target: &Path, check_readonly: bool) -> Result<()> {
    let deadline = Instant::now() + RENAME_RETRY_BUDGET;
    loop {
        match std::fs::rename(fs_path(tmp_path).as_ref(), fs_path(target).as_ref()) {
            Ok(()) => return Ok(()),
            Err(e) => {
                if check_readonly
                    && std::fs::metadata(fs_path(target).as_ref())
                        .is_ok_and(|m| m.permissions().readonly())
                {
                    return Err(TodoError::new(
                        ErrorCode::ReadOnly,
                        format!("{} is read-only and cannot be modified", target.display()),
                    )
                    .with_file(target.display().to_string()));
                }
                if is_retryable_rename_error(&e) && Instant::now() < deadline {
                    std::thread::sleep(RENAME_RETRY_STEP);
                    continue;
                }
                return Err(TodoError::new(
                    ErrorCode::WriteFailed,
                    format!("failed to replace {}: {e}", target.display()),
                )
                .with_file(target.display().to_string()));
            }
        }
    }
}

fn is_sharing_violation(err: &io::Error) -> bool {
    #[cfg(windows)]
    {
        // ERROR_SHARING_VIOLATION (32) or ERROR_LOCK_VIOLATION (33)
        matches!(err.raw_os_error(), Some(32 | 33))
    }
    #[cfg(not(windows))]
    {
        let _ = err;
        false
    }
}

fn is_retryable_rename_error(err: &io::Error) -> bool {
    #[cfg(windows)]
    {
        // ERROR_SHARING_VIOLATION (32), ERROR_LOCK_VIOLATION (33), ERROR_ACCESS_DENIED (5, e.g. MoveFileExW while open without FILE_SHARE_DELETE)
        matches!(err.raw_os_error(), Some(32 | 33 | 5))
    }
    #[cfg(not(windows))]
    {
        err.kind() == io::ErrorKind::WouldBlock
    }
}

fn create_temp_file_for(target: &Path) -> Result<(PathBuf, File, TempFileGuard)> {
    let parent = match target.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    let base_name = target
        .file_name()
        .unwrap_or_else(|| OsStr::new("TODO.md"))
        .to_string_lossy();
    let pid = std::process::id();

    for attempt in 0..16 {
        let rand = random_u64();
        let tmp_name = format!("{base_name}.{pid}.{rand:016x}.tmp");
        let tmp_path = parent.join(tmp_name);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(fs_path(&tmp_path).as_ref())
        {
            Ok(file) => {
                let guard = TempFileGuard {
                    path: tmp_path.clone(),
                    armed: true,
                };
                return Ok((tmp_path, file, guard));
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists && attempt < 15 => continue,
            Err(e) => {
                return Err(TodoError::new(
                    ErrorCode::WriteFailed,
                    format!(
                        "failed to create temporary file in {}: {e}",
                        parent.display()
                    ),
                )
                .with_file(target.display().to_string()));
            }
        }
    }

    Err(TodoError::new(
        ErrorCode::WriteFailed,
        format!(
            "failed to allocate unique temporary file in {}",
            parent.display()
        ),
    )
    .with_file(target.display().to_string()))
}

struct TempFileGuard {
    path: PathBuf,
    armed: bool,
}

impl TempFileGuard {
    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = std::fs::remove_file(fs_path(&self.path).as_ref());
        }
    }
}

fn map_read_error(path: &Path, err: &io::Error) -> TodoError {
    if err.kind() == io::ErrorKind::NotFound {
        TodoError::new(
            ErrorCode::NoFile,
            format!(
                "TODO file not found: {} — run `todo init` to create one",
                path.display()
            ),
        )
        .with_file(path.display().to_string())
    } else {
        TodoError::new(
            ErrorCode::WriteFailed,
            format!("failed reading {}: {err}", path.display()),
        )
        .with_file(path.display().to_string())
    }
}

pub(crate) fn random_u64() -> u64 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(COUNTER.fetch_add(1, Ordering::Relaxed));
    hasher.write_u32(std::process::id());
    hasher.finish()
}

#[cfg(feature = "test-hooks")]
fn maybe_run_before_rename_hook() {
    if let Ok(ready_path) = std::env::var("TODO_TEST_BEFORE_RENAME_READY_FILE")
        && !ready_path.is_empty()
    {
        let _ = std::fs::write(&ready_path, b"ready\n");
    }
    if let Ok(ms_str) = std::env::var("TODO_TEST_BEFORE_RENAME_HOLD_MS")
        && let Ok(ms) = ms_str.parse::<u64>()
        && ms > 0
    {
        std::thread::sleep(Duration::from_millis(ms));
    }
}
