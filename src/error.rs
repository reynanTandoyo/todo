//! Errors with stable codes and exit codes (T-004; requirements R-OUT-2, R-OUT-3).

use std::collections::BTreeMap;
use std::fmt;

use serde::Serialize;
use serde_json::Value;

/// Process exit codes defined by R-OUT-2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ExitCode {
    /// Command succeeded (`0`).
    Ok = 0,
    /// Refused by a rule (`1`: claimed by someone else, bad transition, no/ambiguous match, check found problems).
    Refused = 1,
    /// Wrong usage (`2`: bad arguments, missing `--agent` / `TODO_AGENT`).
    Usage = 2,
    /// Busy, try again (`3`: lock or lease held, or file changed during write).
    Busy = 3,
    /// File problem (`4`: not found, unsafe to write, read-only, write failed).
    FileError = 4,
    /// Reserved for "a check couldn't run" (`5`).
    CheckError = 5,
}

impl ExitCode {
    /// Returns the numeric exit code as `i32`.
    #[must_use]
    pub const fn as_i32(self) -> i32 {
        self as i32
    }

    /// Returns the numeric exit code as `u8`.
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

impl From<ExitCode> for std::process::ExitCode {
    fn from(code: ExitCode) -> Self {
        Self::from(code.as_u8())
    }
}

/// Stable error codes defined by R-OUT-3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    /// `TODO.md` (or the specified file) could not be found.
    NoFile,
    /// Write command invoked without `--agent` or `TODO_AGENT`.
    NoAgent,
    /// Invalid command-line arguments or options.
    Usage,
    /// Requested task or item was not found.
    NotFound,
    /// Selector matched more than one task.
    Ambiguous,
    /// Task is already claimed by another agent.
    AlreadyClaimed,
    /// Task is owned by another agent and `--force` was not provided.
    NotOwner,
    /// Requested status transition is not allowed from the current status.
    BadTransition,
    /// Another task already has the same title.
    DuplicateTitle,
    /// Named section does not exist in `TODO.md`.
    UnknownSection,
    /// Section name matched more than one section.
    AmbiguousSection,
    /// A required field is missing.
    MissingField,
    /// A field or setting value is invalid.
    BadValue,
    /// Lock is currently held by another process.
    Busy,
    /// Manual edit lease is currently held by another agent.
    LeaseHeld,
    /// Target file changed on disk between read and write.
    ChangedDuringWrite,
    /// File is unsafe to write (e.g. invalid UTF-8, conflict markers, unclosed fence/comment, unknown format version).
    UnsafeFile,
    /// Target file is read-only.
    ReadOnly,
    /// Writing or replacing the file failed.
    WriteFailed,
}

impl ErrorCode {
    /// Returns the stable `SCREAMING_SNAKE_CASE` string for this error code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoFile => "NO_FILE",
            Self::NoAgent => "NO_AGENT",
            Self::Usage => "USAGE",
            Self::NotFound => "NOT_FOUND",
            Self::Ambiguous => "AMBIGUOUS",
            Self::AlreadyClaimed => "ALREADY_CLAIMED",
            Self::NotOwner => "NOT_OWNER",
            Self::BadTransition => "BAD_TRANSITION",
            Self::DuplicateTitle => "DUPLICATE_TITLE",
            Self::UnknownSection => "UNKNOWN_SECTION",
            Self::AmbiguousSection => "AMBIGUOUS_SECTION",
            Self::MissingField => "MISSING_FIELD",
            Self::BadValue => "BAD_VALUE",
            Self::Busy => "BUSY",
            Self::LeaseHeld => "LEASE_HELD",
            Self::ChangedDuringWrite => "CHANGED_DURING_WRITE",
            Self::UnsafeFile => "UNSAFE_FILE",
            Self::ReadOnly => "READ_ONLY",
            Self::WriteFailed => "WRITE_FAILED",
        }
    }

    /// Returns the default [`ExitCode`] for this error code per R-OUT-2.
    #[must_use]
    pub const fn default_exit_code(self) -> ExitCode {
        match self {
            Self::NoAgent | Self::Usage => ExitCode::Usage,
            Self::NotFound
            | Self::Ambiguous
            | Self::AlreadyClaimed
            | Self::NotOwner
            | Self::BadTransition
            | Self::DuplicateTitle
            | Self::UnknownSection
            | Self::AmbiguousSection
            | Self::MissingField
            | Self::BadValue => ExitCode::Refused,
            Self::Busy | Self::LeaseHeld | Self::ChangedDuringWrite => ExitCode::Busy,
            Self::NoFile | Self::UnsafeFile | Self::ReadOnly | Self::WriteFailed => {
                ExitCode::FileError
            }
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Optional structured details attached to a [`TodoError`] (R-OUT-1).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ErrorDetails {
    /// Task ID related to the error (e.g. `"T-012"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    /// Owner agent name (e.g. for `ALREADY_CLAIMED` or `NOT_OWNER`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// Candidate task IDs or section names (e.g. for `AMBIGUOUS`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<String>,
    /// 1-based line number where a problem occurred.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// File path related to the error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Additional structured fields.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// The unified error type for `todo`.
///
/// Carries a stable [`ErrorCode`], a plain-English message, an [`ExitCode`], and optional
/// structured [`ErrorDetails`] for `--json` output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodoError {
    /// Stable error code.
    pub code: ErrorCode,
    /// Human-readable error message.
    pub message: String,
    /// Process exit code.
    pub exit_code: ExitCode,
    /// Optional structured details serialized into the JSON `error` object.
    pub details: Box<ErrorDetails>,
}

impl TodoError {
    /// Creates a new [`TodoError`] with the default exit code for `code`.
    #[must_use]
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            exit_code: code.default_exit_code(),
            details: Box::new(ErrorDetails::default()),
        }
    }

    /// Overrides the exit code.
    #[must_use]
    pub const fn with_exit_code(mut self, exit_code: ExitCode) -> Self {
        self.exit_code = exit_code;
        self
    }

    /// Attaches a task ID detail.
    #[must_use]
    pub fn with_task(mut self, task: impl Into<String>) -> Self {
        self.details.task = Some(task.into());
        self
    }

    /// Attaches an owner detail.
    #[must_use]
    pub fn with_owner(mut self, owner: impl Into<String>) -> Self {
        self.details.owner = Some(owner.into());
        self
    }

    /// Attaches candidate IDs or names.
    #[must_use]
    pub fn with_candidates(mut self, candidates: Vec<String>) -> Self {
        self.details.candidates = candidates;
        self
    }

    /// Attaches a 1-based line number.
    #[must_use]
    pub const fn with_line(mut self, line: usize) -> Self {
        self.details.line = Some(line);
        self
    }

    /// Attaches a file path.
    #[must_use]
    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        self.details.file = Some(file.into());
        self
    }

    /// Attaches an arbitrary extra key-value detail.
    #[must_use]
    pub fn with_detail(mut self, key: impl Into<String>, value: Value) -> Self {
        self.details.extra.insert(key.into(), value);
        self
    }
}

impl fmt::Display for TodoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.details.file, self.details.line) {
            (Some(file), Some(line)) => write!(f, "{file}:{line}: {}", self.message),
            _ => f.write_str(&self.message),
        }
    }
}

impl std::error::Error for TodoError {}

/// Convenience `Result` alias using [`TodoError`].
pub type Result<T> = std::result::Result<T, TodoError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_error_codes_have_stable_strings_and_exit_codes() {
        let cases = [
            (ErrorCode::NoFile, "NO_FILE", ExitCode::FileError),
            (ErrorCode::NoAgent, "NO_AGENT", ExitCode::Usage),
            (ErrorCode::Usage, "USAGE", ExitCode::Usage),
            (ErrorCode::NotFound, "NOT_FOUND", ExitCode::Refused),
            (ErrorCode::Ambiguous, "AMBIGUOUS", ExitCode::Refused),
            (
                ErrorCode::AlreadyClaimed,
                "ALREADY_CLAIMED",
                ExitCode::Refused,
            ),
            (ErrorCode::NotOwner, "NOT_OWNER", ExitCode::Refused),
            (
                ErrorCode::BadTransition,
                "BAD_TRANSITION",
                ExitCode::Refused,
            ),
            (
                ErrorCode::DuplicateTitle,
                "DUPLICATE_TITLE",
                ExitCode::Refused,
            ),
            (
                ErrorCode::UnknownSection,
                "UNKNOWN_SECTION",
                ExitCode::Refused,
            ),
            (
                ErrorCode::AmbiguousSection,
                "AMBIGUOUS_SECTION",
                ExitCode::Refused,
            ),
            (ErrorCode::MissingField, "MISSING_FIELD", ExitCode::Refused),
            (ErrorCode::BadValue, "BAD_VALUE", ExitCode::Refused),
            (ErrorCode::Busy, "BUSY", ExitCode::Busy),
            (ErrorCode::LeaseHeld, "LEASE_HELD", ExitCode::Busy),
            (
                ErrorCode::ChangedDuringWrite,
                "CHANGED_DURING_WRITE",
                ExitCode::Busy,
            ),
            (ErrorCode::UnsafeFile, "UNSAFE_FILE", ExitCode::FileError),
            (ErrorCode::ReadOnly, "READ_ONLY", ExitCode::FileError),
            (ErrorCode::WriteFailed, "WRITE_FAILED", ExitCode::FileError),
        ];

        for (code, expected_str, expected_exit) in cases {
            assert_eq!(code.as_str(), expected_str);
            assert_eq!(code.to_string(), expected_str);
            assert_eq!(code.default_exit_code(), expected_exit);
            let err = TodoError::new(code, "test");
            assert_eq!(err.exit_code, expected_exit);
        }
    }

    #[test]
    fn display_formats_file_and_line_per_r_out_4() {
        let err = TodoError::new(ErrorCode::UnsafeFile, "unclosed code fence")
            .with_file("TODO.md")
            .with_line(42);
        assert_eq!(err.to_string(), "TODO.md:42: unclosed code fence");
    }

    #[test]
    fn exit_codes_match_r_out_2() {
        assert_eq!(ExitCode::Ok.as_i32(), 0);
        assert_eq!(ExitCode::Refused.as_i32(), 1);
        assert_eq!(ExitCode::Usage.as_i32(), 2);
        assert_eq!(ExitCode::Busy.as_i32(), 3);
        assert_eq!(ExitCode::FileError.as_i32(), 4);
        assert_eq!(ExitCode::CheckError.as_i32(), 5);
    }
}
