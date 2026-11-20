//! Runtime configuration, `todo.toml` loading (T-007; R-CFG-1, R-CFG-2), and time/archive helpers.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, FixedOffset, Utc};

use crate::doc::{IdConfig, Task};
use crate::error::{ErrorCode, Result, TodoError};
use crate::find::{WorktreeMode, fs_path};
use crate::lock::LockConfig;

/// File name of the optional project configuration file next to `TODO.md` (`R-CFG-2`).
pub const CONFIG_FILENAME: &str = "todo.toml";

/// Default `claim_ttl_hours` from requirements §5.14 (`R-CLAIM-4`).
pub const DEFAULT_CLAIM_TTL_HOURS: u64 = 24;

/// Default `archive_file` name from requirements §5.14 (`format.md` §14).
pub const DEFAULT_ARCHIVE_FILE: &str = "TODO-archive.md";

/// Default `question_section` name from requirements §5.14 (`R-WRITE-11`).
pub const DEFAULT_QUESTION_SECTION: &str = "Questions";

/// Default `idea_section` name from requirements §5.14 (`R-KIND-1`).
pub const DEFAULT_IDEA_SECTION: &str = "Ideas";

/// Default `snapshots_keep` count from requirements §5.14 (`R-HIST-2`).
pub const DEFAULT_SNAPSHOTS_KEEP: u64 = 200;

/// Effective configuration for task operations (defaults from requirements §5.14).
///
/// Built with defaults when no `todo.toml` is present (`R-CFG-1`), or loaded from `todo.toml`
/// next to `TODO.md` (`R-CFG-2`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpsConfig {
    /// Task ID prefix and zero-padding width (`R-ID-1`).
    pub id_config: IdConfig,
    /// Ordered priority names from most urgent to least urgent (`R-CFG-2`).
    pub priorities: Vec<String>,
    /// Fields required of a task by `todo add` (`R-WRITE-1`, default `["priority", "difficulty"]`;
    /// milestones, questions and ideas need none of them, `R-LEVEL-1`).
    pub required_fields: Vec<String>,
    /// Allowed `type` values; empty allows any (`R-CFG-2`).
    pub types: Vec<String>,
    /// Allowed `area` values; empty allows any (`R-CFG-2`).
    pub areas: Vec<String>,
    /// Allowed `difficulty` values, easiest first; never empty (`R-CFG-2`, `R-WRITE-18`, default
    /// the levels `1` to `5`).
    pub difficulties: Vec<String>,
    /// `true` when `todo.toml` sets `difficulties` itself. The format 2 to 3 migration then keeps
    /// the difficulty values as they are (`R-MIG-1`).
    pub difficulties_set: bool,
    /// Default section name for `todo add` when `--section` is omitted (`R-WRITE-1`).
    pub default_section: Option<String>,
    /// Section name for `todo ask` (`R-WRITE-11`, default `"Questions"`).
    pub question_section: String,
    /// Section name for `todo idea` (`R-KIND-1`, default `"Ideas"`).
    pub idea_section: String,
    /// File name of the archive file next to `TODO.md` (`format.md` §14).
    pub archive_file: String,
    /// Number of hours before a claim becomes stale (`R-CLAIM-4`).
    pub claim_ttl_hours: u64,
    /// Write lock and manual lease timing settings (`R-LOCK-3`, `R-LOCK-6`).
    pub lock: LockConfig,
    /// Number of snapshots to keep in the state folder (`R-CFG-2`, default `200`).
    pub snapshots_keep: u64,
    /// How linked git worktrees resolve `TODO.md` (`R-FIND-5`, `R-CFG-2`).
    pub worktree: WorktreeMode,
}

impl Default for OpsConfig {
    fn default() -> Self {
        Self {
            id_config: IdConfig::default(),
            priorities: vec![
                "P0".to_string(),
                "P1".to_string(),
                "P2".to_string(),
                "P3".to_string(),
            ],
            required_fields: vec!["priority".to_string(), "difficulty".to_string()],
            types: Vec::new(),
            areas: Vec::new(),
            difficulties: ["1", "2", "3", "4", "5"].map(str::to_string).to_vec(),
            difficulties_set: false,
            default_section: None,
            question_section: DEFAULT_QUESTION_SECTION.to_string(),
            idea_section: DEFAULT_IDEA_SECTION.to_string(),
            archive_file: DEFAULT_ARCHIVE_FILE.to_string(),
            claim_ttl_hours: DEFAULT_CLAIM_TTL_HOURS,
            lock: LockConfig::default(),
            snapshots_keep: DEFAULT_SNAPSHOTS_KEEP,
            worktree: WorktreeMode::Shared,
        }
    }
}

impl OpsConfig {
    /// Returns the path to the archive file sitting next to `todo_path` (`format.md` §14).
    #[must_use]
    pub fn archive_path(&self, todo_path: &Path) -> PathBuf {
        todo_path.with_file_name(&self.archive_file)
    }

    /// Returns a sort rank for `priority` (lower = more urgent).
    ///
    /// Unrecognized or missing priorities sort after all configured priorities (`format.md` §8).
    #[must_use]
    pub fn priority_rank(&self, priority: Option<&str>) -> usize {
        let Some(p) = priority else {
            return self.priorities.len();
        };
        self.priorities
            .iter()
            .position(|cfg_p| cfg_p.eq_ignore_ascii_case(p))
            .unwrap_or(self.priorities.len())
    }

    /// Returns the position of `difficulty` in [`Self::difficulties`] (0 = easiest), ignoring
    /// case; `None` when there is no difficulty or it is not a configured one.
    #[must_use]
    pub fn difficulty_rank(&self, difficulty: Option<&str>) -> Option<usize> {
        let d = difficulty?;
        self.difficulties
            .iter()
            .position(|cfg_d| cfg_d.eq_ignore_ascii_case(d.trim()))
    }

    /// Returns the configured spelling of one `difficulty` value (`R-WRITE-18`).
    ///
    /// An unknown value is refused with `BAD_VALUE` and the list of allowed values.
    pub fn canonical_difficulty(&self, raw: &str) -> Result<String> {
        let value = raw.trim();
        match self
            .difficulties
            .iter()
            .find(|d| d.eq_ignore_ascii_case(value))
        {
            Some(canonical) => Ok(canonical.clone()),
            None => Err(TodoError::new(
                ErrorCode::BadValue,
                format!(
                    "invalid difficulty '{value}'; expected one of: {}",
                    self.difficulties.join(", ")
                ),
            )),
        }
    }

    /// Parses a comma-separated list of difficulties, such as `--difficulty easy,medium`, into
    /// configured spellings. An empty list or an unknown value is refused with `BAD_VALUE`.
    pub fn difficulty_list(&self, raw: &str) -> Result<Vec<String>> {
        let mut out = Vec::new();
        for part in raw.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            out.push(self.canonical_difficulty(part)?);
        }
        if out.is_empty() {
            return Err(TodoError::new(
                ErrorCode::BadValue,
                format!(
                    "no difficulty given; expected one or more of: {}",
                    self.difficulties.join(", ")
                ),
            ));
        }
        Ok(out)
    }

    /// Computes the age of `task`'s claim relative to `now` in UTC (`R-IO-6`).
    ///
    /// Returns `None` if `task` has no valid `claimed` timestamp.
    #[must_use]
    pub fn claim_age(&self, task: &Task, now: DateTime<FixedOffset>) -> Option<Duration> {
        let claimed_at = task.parse_claimed_at()?;
        Some(
            now.with_timezone(&Utc)
                .signed_duration_since(claimed_at.with_timezone(&Utc)),
        )
    }

    /// Returns `true` if `task` is `claimed` and its claim is stale relative to `now` (`R-CLAIM-4`).
    ///
    /// A `claimed` task without a valid `claimed` timestamp counts as stale (`format.md` §8).
    #[must_use]
    pub fn is_claim_stale(&self, task: &Task, now: DateTime<FixedOffset>) -> bool {
        if task.status != crate::doc::Status::Claimed {
            return false;
        }
        let Some(age) = self.claim_age(task, now) else {
            return true;
        };
        let ttl_secs = i64::try_from(self.claim_ttl_hours)
            .unwrap_or(i64::MAX / 3600)
            .saturating_mul(3600);
        age > Duration::seconds(ttl_secs)
    }

    /// Formats `now` as canonical `- claimed:` local timestamp with UTC offset (`R-IO-6`, `format.md` §8).
    #[must_use]
    pub fn format_claimed_timestamp(now: DateTime<FixedOffset>) -> String {
        now.format("%Y-%m-%dT%H:%M%:z").to_string()
    }

    /// Formats `now` as canonical local date for `- note (YYYY-MM-DD, agent): ...` (`R-IO-6`, `format.md` §9).
    #[must_use]
    pub fn format_note_date(now: DateTime<FixedOffset>) -> String {
        now.format("%Y-%m-%d").to_string()
    }

    /// Loads `todo.toml` sitting next to `todo_path`, or returns [`OpsConfig::default`] if absent (`R-CFG-1`, `R-CFG-2`).
    pub fn load_next_to(todo_path: &Path) -> Result<Self> {
        let toml_path = todo_path.with_file_name(CONFIG_FILENAME);
        Self::load_from_file(&toml_path)
    }

    /// Loads `todo.toml` from `toml_path`, or returns [`OpsConfig::default`] if the file does not exist.
    pub fn load_from_file(toml_path: &Path) -> Result<Self> {
        match std::fs::read_to_string(fs_path(toml_path).as_ref()) {
            Ok(content) => Self::from_toml_str(&content),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(TodoError::new(
                ErrorCode::BadValue,
                format!("could not read {}: {e}", toml_path.display()),
            )
            .with_file(CONFIG_FILENAME)),
        }
    }

    /// Parses and validates a `todo.toml` string, returning `todo.toml:<line>: message` on error (`R-CFG-2`).
    pub fn from_toml_str(content: &str) -> Result<Self> {
        let table: BTreeMap<String, toml::Spanned<toml::Value>> =
            toml::from_str(content).map_err(|err| {
                let line = err.span().map_or(1, |s| line_at_offset(content, s.start));
                TodoError::new(
                    ErrorCode::BadValue,
                    format!("invalid TOML: {}", err.message()),
                )
                .with_file(CONFIG_FILENAME)
                .with_line(line)
            })?;

        let mut cfg = Self::default();
        let mut lease_minutes_line: Option<usize> = None;
        let mut lease_max_line: Option<usize> = None;

        for (key, spanned_val) in table {
            let line = line_at_offset(content, spanned_val.span().start);
            let val = spanned_val.into_inner();
            match key.as_str() {
                "id_prefix" => {
                    let s = expect_string(&key, val, line)?;
                    let trimmed = s.trim();
                    if trimmed.is_empty()
                        || trimmed.contains('-')
                        || trimmed.chars().any(char::is_whitespace)
                    {
                        return Err(toml_err(
                            line,
                            "id_prefix must be a non-empty string without '-' or whitespace",
                        ));
                    }
                    cfg.id_config.prefix = trimmed.to_string();
                }
                "id_width" => {
                    let n = expect_u64(&key, val, line, 1)?;
                    let width = usize::try_from(n)
                        .ok()
                        .filter(|&w| w <= 16)
                        .ok_or_else(|| toml_err(line, "id_width must be between 1 and 16"))?;
                    cfg.id_config.width = width;
                }
                "priorities" => {
                    let items = expect_string_list(&key, val, line, true)?;
                    if items.is_empty() {
                        return Err(toml_err(line, "priorities must not be empty"));
                    }
                    cfg.priorities = items;
                }
                "required_fields" => {
                    cfg.required_fields = expect_string_list(&key, val, line, true)?;
                }
                "types" => {
                    cfg.types = expect_string_list(&key, val, line, true)?;
                }
                "areas" => {
                    cfg.areas = expect_string_list(&key, val, line, true)?;
                }
                "difficulties" => {
                    let items = expect_string_list(&key, val, line, true)?;
                    if items.is_empty() {
                        return Err(toml_err(line, "difficulties must not be empty"));
                    }
                    cfg.difficulties = items;
                    cfg.difficulties_set = true;
                }
                "default_section" => {
                    let s = expect_string(&key, val, line)?;
                    let trimmed = s.trim();
                    cfg.default_section = if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    };
                }
                "question_section" => {
                    let s = expect_string(&key, val, line)?;
                    let trimmed = s.trim();
                    if trimmed.is_empty() {
                        return Err(toml_err(line, "question_section must not be empty"));
                    }
                    cfg.question_section = trimmed.to_string();
                }
                "idea_section" => {
                    let s = expect_string(&key, val, line)?;
                    let trimmed = s.trim();
                    if trimmed.is_empty() {
                        return Err(toml_err(line, "idea_section must not be empty"));
                    }
                    cfg.idea_section = trimmed.to_string();
                }
                "archive_file" => {
                    let s = expect_string(&key, val, line)?;
                    let trimmed = s.trim();
                    if trimmed.is_empty()
                        || trimmed == "."
                        || trimmed == ".."
                        || trimmed.contains('/')
                        || trimmed.contains('\\')
                    {
                        return Err(toml_err(
                            line,
                            "archive_file must be a non-empty file name without path separators",
                        ));
                    }
                    cfg.archive_file = trimmed.to_string();
                }
                "claim_ttl_hours" => {
                    cfg.claim_ttl_hours = expect_u64(&key, val, line, 1)?;
                }
                "lock_wait_seconds" => {
                    cfg.lock.lock_wait_seconds = expect_u64(&key, val, line, 0)?;
                }
                "lease_minutes" => {
                    cfg.lock.lease_minutes = expect_u64(&key, val, line, 1)?;
                    lease_minutes_line = Some(line);
                }
                "lease_max_minutes" => {
                    cfg.lock.lease_max_minutes = expect_u64(&key, val, line, 1)?;
                    lease_max_line = Some(line);
                }
                "snapshots_keep" => {
                    let keep = expect_u64(&key, val, line, 0)?;
                    cfg.snapshots_keep = keep;
                    cfg.lock.snapshots_keep = keep;
                }
                "worktree" => {
                    let s = expect_string(&key, val, line)?;
                    cfg.worktree = match s.as_str() {
                        "shared" => WorktreeMode::Shared,
                        "local" => WorktreeMode::Local,
                        _ => {
                            return Err(toml_err(
                                line,
                                format!(
                                    "invalid worktree mode {s:?}; expected \"shared\" or \"local\""
                                ),
                            ));
                        }
                    };
                }
                _ => {
                    return Err(toml_err(line, format!("unknown setting `{key}`")));
                }
            }
        }

        if cfg.lock.lease_minutes > cfg.lock.lease_max_minutes {
            let line = lease_minutes_line.or(lease_max_line).unwrap_or(1);
            return Err(toml_err(
                line,
                format!(
                    "lease_minutes ({}) cannot exceed lease_max_minutes ({})",
                    cfg.lock.lease_minutes, cfg.lock.lease_max_minutes
                ),
            ));
        }

        Ok(cfg)
    }
}

fn line_at_offset(content: &str, offset: usize) -> usize {
    let clamped = offset.min(content.len());
    content.as_bytes()[..clamped]
        .iter()
        .filter(|&&b| b == b'\n')
        .count()
        + 1
}

fn toml_err(line: usize, message: impl Into<String>) -> TodoError {
    TodoError::new(ErrorCode::BadValue, message)
        .with_file(CONFIG_FILENAME)
        .with_line(line)
}

fn expect_string(key: &str, val: toml::Value, line: usize) -> Result<String> {
    match val {
        toml::Value::String(s) => Ok(s),
        _ => Err(toml_err(line, format!("{key} must be a string"))),
    }
}

fn expect_u64(key: &str, val: toml::Value, line: usize, min: u64) -> Result<u64> {
    let toml::Value::Integer(n) = val else {
        return Err(toml_err(line, format!("{key} must be an integer")));
    };
    let Ok(u) = u64::try_from(n) else {
        return Err(toml_err(line, format!("{key} must be >= {min}")));
    };
    if u < min {
        return Err(toml_err(line, format!("{key} must be >= {min}")));
    }
    Ok(u)
}

fn expect_string_list(
    key: &str,
    val: toml::Value,
    line: usize,
    no_whitespace: bool,
) -> Result<Vec<String>> {
    let toml::Value::Array(arr) = val else {
        return Err(toml_err(line, format!("{key} must be a list of strings")));
    };
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let toml::Value::String(s) = item else {
            return Err(toml_err(line, format!("{key} must be a list of strings")));
        };
        let trimmed = s.trim();
        if trimmed.is_empty() || (no_whitespace && trimmed.chars().any(char::is_whitespace)) {
            return Err(toml_err(
                line,
                format!("{key} entries must be non-empty without whitespace"),
            ));
        }
        out.push(trimmed.to_string());
    }
    Ok(out)
}
