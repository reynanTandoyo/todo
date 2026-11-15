//! Runtime configuration and time/archive helpers for task operations.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, FixedOffset, Utc};

use crate::doc::{IdConfig, Task};
use crate::lock::LockConfig;

/// Default `claim_ttl_hours` from requirements §5.14 (`R-CLAIM-4`).
pub const DEFAULT_CLAIM_TTL_HOURS: u64 = 24;

/// Default `archive_file` name from requirements §5.14 (`format.md` §14).
pub const DEFAULT_ARCHIVE_FILE: &str = "TODO-archive.md";

/// Effective configuration for task operations (defaults from requirements §5.14).
///
/// Built with defaults today (`R-CFG-1`); `todo.toml` loading (T-007) can populate this struct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpsConfig {
    /// Task ID prefix and zero-padding width (`R-ID-1`).
    pub id_config: IdConfig,
    /// Ordered priority names from most urgent to least urgent (`R-CFG-2`).
    pub priorities: Vec<String>,
    /// Default section name for `todo add` when `--section` is omitted (`R-WRITE-1`).
    pub default_section: Option<String>,
    /// Fields required by `todo add` (`R-WRITE-1`, default `["priority"]`).
    pub required_fields: Vec<String>,
    /// File name of the archive file next to `TODO.md` (`format.md` §14).
    pub archive_file: String,
    /// Number of hours before a claim becomes stale (`R-CLAIM-4`).
    pub claim_ttl_hours: u64,
    /// Write lock and manual lease timing settings (`R-LOCK-3`, `R-LOCK-6`).
    pub lock: LockConfig,
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
            default_section: None,
            required_fields: vec!["priority".to_string()],
            archive_file: DEFAULT_ARCHIVE_FILE.to_string(),
            claim_ttl_hours: DEFAULT_CLAIM_TTL_HOURS,
            lock: LockConfig::default(),
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
}
