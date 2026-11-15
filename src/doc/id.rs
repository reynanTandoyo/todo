//! Task IDs, flexible user-input parsing (R-ID-3), and canonical formatting (R-ID-1).

use std::fmt;

/// Default task ID prefix.
pub const DEFAULT_ID_PREFIX: &str = "T";

/// Default zero-padding width for canonical task IDs.
pub const DEFAULT_ID_WIDTH: usize = 3;

/// A numeric task identifier (independent of leading zeros and prefix casing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(pub u64);

impl TaskId {
    /// Returns the underlying numeric ID.
    #[must_use]
    pub const fn number(self) -> u64 {
        self.0
    }

    /// Formats this ID canonically using `config` (e.g. `T-007`, `T-1000`).
    #[must_use]
    pub fn format(self, config: &IdConfig) -> String {
        config.format_id(self)
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}-{:0width$}",
            DEFAULT_ID_PREFIX,
            self.0,
            width = DEFAULT_ID_WIDTH
        )
    }
}

/// Configuration for parsing and formatting task IDs (R-ID-1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdConfig {
    /// Prefix before `-` (default `"T"`). Matched ignoring case; written in upper case.
    pub prefix: String,
    /// Minimum digit width when formatting canonical IDs (default `3`).
    pub width: usize,
}

impl Default for IdConfig {
    fn default() -> Self {
        Self {
            prefix: DEFAULT_ID_PREFIX.to_string(),
            width: DEFAULT_ID_WIDTH,
        }
    }
}

impl IdConfig {
    /// Creates an [`IdConfig`] with the given prefix and zero-padding width.
    #[must_use]
    pub fn new(prefix: impl Into<String>, width: usize) -> Self {
        Self {
            prefix: prefix.into(),
            width,
        }
    }

    /// Formats `id` in canonical form: uppercase prefix, hyphen, and zero-padded number.
    ///
    /// Numbers wider than `self.width` simply grow longer (e.g. `T-1000`).
    #[must_use]
    pub fn format_id(&self, id: TaskId) -> String {
        format!(
            "{}-{:0width$}",
            self.prefix.to_uppercase(),
            id.0,
            width = self.width
        )
    }

    /// Parses user-supplied task ID input such as `12`, `t12`, `T-12`, or `T-012` (R-ID-3).
    #[must_use]
    pub fn parse_user_id(&self, input: &str) -> Option<TaskId> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return None;
        }

        if let Some(id) = parse_ascii_digits(trimmed) {
            return Some(id);
        }

        let after_prefix = strip_prefix_ignore_case(trimmed, &self.prefix)?;
        let digits = after_prefix.strip_prefix('-').unwrap_or(after_prefix);
        parse_ascii_digits(digits)
    }

    /// Parses a file-format task ID (`<prefix>-<digits>`, prefix matched ignoring case).
    ///
    /// Unlike [`Self::parse_user_id`], this requires both the prefix and the hyphen, per
    /// `docs/format.md` §6.
    #[must_use]
    pub fn parse_heading_id(&self, input: &str) -> Option<TaskId> {
        let trimmed = input.trim();
        let after_prefix = strip_prefix_ignore_case(trimmed, &self.prefix)?;
        let digits = after_prefix.strip_prefix('-')?;
        parse_ascii_digits(digits)
    }

    /// Checks whether `text` starts with `self.prefix` (ignoring case) followed by digits or
    /// `-` + digits, used to detect probable-mistake `### ` headings (`docs/format.md` §6).
    #[must_use]
    pub fn looks_like_prefixed_id(&self, text: &str) -> bool {
        let Some(after_prefix) = strip_prefix_ignore_case(text, &self.prefix) else {
            return false;
        };
        let digits_start = after_prefix.strip_prefix('-').unwrap_or(after_prefix);
        digits_start
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_digit())
    }
}

pub(crate) fn strip_prefix_ignore_case<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if prefix.is_empty() {
        return Some(s);
    }
    let mut s_chars = s.char_indices();
    let mut p_chars = prefix.chars();
    let mut matched_end = 0usize;

    for pc in &mut p_chars {
        let (_, sc) = s_chars.next()?;
        if !sc.to_lowercase().eq(pc.to_lowercase()) {
            return None;
        }
        matched_end += sc.len_utf8();
    }

    Some(&s[matched_end..])
}

fn parse_ascii_digits(s: &str) -> Option<TaskId> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse::<u64>().ok().map(TaskId)
}
