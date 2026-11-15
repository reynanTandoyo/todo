//! Parser and domain model for `TODO.md` format version 1 (`docs/format.md`).

use std::ops::Range;

use chrono::{DateTime, FixedOffset, NaiveDate};

use crate::doc::id::{FastU64Map, IdConfig, TaskId, strip_prefix_ignore_case};
use crate::doc::lines::{LineDoc, Utf8DecodeError};

/// Supported format version of `TODO.md`.
pub const SUPPORTED_FORMAT_VERSION: u32 = 1;

/// Canonical field order for inserting missing fields (`docs/format.md` §10).
pub const CANONICAL_FIELD_ORDER: &[&str] = &[
    "status", "owner", "claimed", "priority", "type", "area", "depends", "files", "problem", "fix",
    "accept", "commit",
];

/// Accepted heading separators between `<ID>` and `<title>` (`docs/format.md` §6),
/// canonical separator first.
pub const TASK_HEADING_SEPARATORS: &[&str] = &[" · ", " - ", " — ", " – ", ": "];

/// Canonical heading separator written by the tool.
pub const CANONICAL_HEADING_SEPARATOR: &str = " · ";

/// Canonical "nobody" owner value written by the tool.
pub const CANONICAL_NOBODY_OWNER: &str = "—";

/// Managed block begin marker (`docs/format.md` §12).
pub const MANAGED_BLOCK_BEGIN: &str = "<!-- todo:begin -->";

/// Managed block end marker (`docs/format.md` §12).
pub const MANAGED_BLOCK_END: &str = "<!-- todo:end -->";

/// The five allowed task statuses (`docs/format.md` §8, decision D2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status {
    Open,
    Claimed,
    Blocked,
    Done,
    Dropped,
}

impl Status {
    /// Parses an exact lowercase status name from `TODO.md`.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "open" => Some(Self::Open),
            "claimed" => Some(Self::Claimed),
            "blocked" => Some(Self::Blocked),
            "done" => Some(Self::Done),
            "dropped" => Some(Self::Dropped),
            _ => None,
        }
    }

    /// Returns the canonical lowercase string for this status.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Claimed => "claimed",
            Self::Blocked => "blocked",
            Self::Done => "done",
            Self::Dropped => "dropped",
        }
    }

    /// Returns `true` if this status counts as finished (`done` or `dropped`).
    #[must_use]
    pub const fn is_finished(self) -> bool {
        matches!(self, Self::Done | Self::Dropped)
    }
}

/// Severity / write-blocking scope of a document problem (`docs/format.md` §6, §13).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProblemSeverity {
    /// Blocks all write commands (`UNSAFE_FILE`, exit 4).
    BlockAllWrites,
    /// Blocks writes to the affected task (`UNSAFE_FILE`, exit 4).
    BlockTask(TaskId),
    /// Non-blocking warning (reported by `todo check` and read warnings).
    Warning,
}

/// Specific kind of problem detected while parsing a `TODO.md` document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProblemKind {
    /// Input bytes were not valid UTF-8 (R-IO-4).
    InvalidUtf8 { byte_offset: usize },
    /// Git conflict marker outside ignored regions (`docs/format.md` §13).
    ConflictMarker,
    /// Fenced code block was still open at EOF (`docs/format.md` §3, §13).
    UnclosedCodeFence,
    /// HTML comment was still open at EOF (`docs/format.md` §3, §13).
    UnclosedHtmlComment,
    /// Format line named an unsupported or unparseable version (`docs/format.md` §4, §13).
    UnknownFormatVersion { raw: String },
    /// Two tasks shared the same numeric ID (`docs/format.md` §13).
    DuplicateTaskId { id: TaskId, first_line: usize },
    /// A task repeated a field (`docs/format.md` §8, §13).
    RepeatedField {
        id: TaskId,
        field: String,
        first_line: usize,
    },
    /// A task had a `status` value outside the five valid statuses (`docs/format.md` §8, §13).
    UnknownStatus { id: TaskId, value: String },
    /// A `### ` heading looked like a task ID but violated §6 grammar.
    HeadingProbableMistake,
}

/// A problem or warning found in a `TODO.md` document (`R-EDIT-8`, `R-OUT-4`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// 1-based line number in the file.
    pub line: usize,
    /// Whether this blocks all writes, blocks one task, or is a warning.
    pub severity: ProblemSeverity,
    /// Machine-inspectable problem classification.
    pub kind: ProblemKind,
    /// Plain-English description (without `file:line: ` prefix).
    pub message: String,
}

impl Problem {
    /// Formats this problem as `<file>:<line>: <message>` (R-OUT-4).
    #[must_use]
    pub fn format_with_path(&self, file: &str) -> String {
        format!("{file}:{}: {}", self.line, self.message)
    }

    /// Returns `true` if this problem blocks all writes to the file.
    #[must_use]
    pub const fn blocks_all_writes(&self) -> bool {
        matches!(self.severity, ProblemSeverity::BlockAllWrites)
    }

    /// Returns `true` if this problem blocks writes to `task_id`.
    #[must_use]
    pub fn blocks_task(&self, task_id: TaskId) -> bool {
        match self.severity {
            ProblemSeverity::BlockAllWrites => true,
            ProblemSeverity::BlockTask(id) => id == task_id,
            ProblemSeverity::Warning => false,
        }
    }
}

impl From<&Utf8DecodeError> for Problem {
    fn from(err: &Utf8DecodeError) -> Self {
        Self {
            line: err.line,
            severity: ProblemSeverity::BlockAllWrites,
            kind: ProblemKind::InvalidUtf8 {
                byte_offset: err.byte_offset,
            },
            message: err.to_string(),
        }
    }
}

/// Where a field value is stored in a task block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldLocation {
    /// Dedicated `- key: value` line (0-based line index).
    OwnLine(usize),
    /// Part of a combined `- priority: ... · key: value` line (0-based line index).
    CombinedPriority(usize),
}

impl FieldLocation {
    /// Returns the 0-based line index where this field resides.
    #[must_use]
    pub const fn line(self) -> usize {
        match self {
            Self::OwnLine(line) | Self::CombinedPriority(line) => line,
        }
    }
}

/// A single `- key: value` entry inside a task block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldEntry {
    /// Field key (e.g. `status`, `custom_tag`).
    pub key: String,
    /// Trimmed field value.
    pub value: String,
    /// 0-based line index in the document.
    pub line: usize,
}

/// Parsed combined priority line (`- priority: P1 · type: bug · area: ui`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombinedPriorityLine {
    /// 0-based line index of the `- priority: ...` line.
    pub line: usize,
    /// Priority value (first segment before ` · `), `None` if empty.
    pub priority: Option<String>,
    /// Additional `key: value` segments on the same line, in order.
    pub extra_parts: Vec<(String, String)>,
}

/// A parsed dependency reference from `- depends: ...`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyRef {
    /// Raw comma-separated token as written (trimmed).
    pub raw: String,
    /// Parsed task ID if valid according to [`IdConfig`].
    pub id: Option<TaskId>,
}

/// A dated note line (`- note (YYYY-MM-DD, agent): text`, `docs/format.md` §9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// Raw date string as written (`YYYY-MM-DD`).
    pub date_raw: String,
    /// Parsed calendar date.
    pub date: NaiveDate,
    /// Agent name inside the parentheses.
    pub agent: String,
    /// Trimmed note body after `):`.
    pub text: String,
    /// 0-based line index in the document.
    pub line: usize,
}

/// A `## <name>` section (`docs/format.md` §5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    /// Section name (trimmed text after `## `).
    pub name: String,
    /// 0-based line index of the `## ` heading.
    pub heading_line: usize,
    /// 0-based half-open line range of the entire section (to the next `## ` or EOF).
    pub range: Range<usize>,
    /// 0-based line index immediately after the section's last non-blank line.
    pub content_end: usize,
}

impl Section {
    /// Returns the section name with any leading `12.` or `3)` stripped (`docs/format.md` §5).
    #[must_use]
    pub fn normalized_name(&self) -> &str {
        strip_section_number(&self.name)
    }
}

/// Result of looking up a section by name (`docs/format.md` §5, R-WRITE-1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SectionLookup {
    /// Exactly one section matched; contains its index in [`Document::sections`].
    Found(usize),
    /// No section matched.
    NotFound,
    /// Multiple sections matched; contains candidate indices in [`Document::sections`].
    Ambiguous(Vec<usize>),
}

/// Location of a `<!-- todo:begin -->` ... `<!-- todo:end -->` managed block (`docs/format.md` §12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManagedBlock {
    /// 0-based line index of `<!-- todo:begin -->`.
    pub begin_line: usize,
    /// 0-based line index of `<!-- todo:end -->`.
    pub end_line: usize,
}

/// A parsed `### <ID><sep><title>` task block (`docs/format.md` §6–§9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    /// Numeric task ID.
    pub id: TaskId,
    /// Raw ID token as written in the heading (e.g. `t-7`, `T-007`).
    pub raw_id: String,
    /// Separator as written in the heading (e.g. `" · "`, `": "`).
    pub separator: String,
    /// Trimmed task title.
    pub title: String,
    /// 0-based line index of the `### ` heading.
    pub heading_line: usize,
    /// 0-based half-open line range of the task block (trailing blank lines excluded).
    pub block_range: Range<usize>,
    /// Index into [`Document::sections`], or `None` if the task appears before any `## ` section.
    pub section: Option<usize>,

    /// Effective status (`Open` when `- status:` is omitted or unparseable).
    pub status: Status,
    /// Raw `- status:` value if the field line is present.
    pub status_raw: Option<String>,
    /// 0-based line index of `- status:`, if present.
    pub status_line: Option<usize>,

    /// Owner agent name (`None` when omitted or any "nobody" form: `—`, `-`, `none`, empty).
    pub owner: Option<String>,
    /// Raw `- owner:` value if the field line is present.
    pub owner_raw: Option<String>,
    /// 0-based line index of `- owner:`, if present.
    pub owner_line: Option<usize>,

    /// Raw `- claimed:` timestamp string, if present.
    pub claimed: Option<String>,
    /// 0-based line index of `- claimed:`, if present.
    pub claimed_line: Option<usize>,

    /// Priority value (from standalone `- priority:` or first part of combined priority line).
    pub priority: Option<String>,
    /// Combined priority line details, if `- priority:` is present.
    pub priority_detail: Option<CombinedPriorityLine>,

    /// Task type (`type` field or combined priority segment).
    pub task_type: Option<String>,
    /// Where `type` is defined, if present.
    pub task_type_loc: Option<FieldLocation>,

    /// Task area (`area` field or combined priority segment).
    pub area: Option<String>,
    /// Where `area` is defined, if present.
    pub area_loc: Option<FieldLocation>,

    /// Parsed dependency references from `- depends:`.
    pub depends: Vec<DependencyRef>,
    /// Raw `- depends:` value, if present.
    pub depends_raw: Option<String>,
    /// 0-based line index of `- depends:`, if present.
    pub depends_line: Option<usize>,

    /// `- files:` value and line index.
    pub files: Option<String>,
    pub files_line: Option<usize>,

    /// `- problem:` value and line index.
    pub problem: Option<String>,
    pub problem_line: Option<usize>,

    /// `- fix:` value and line index.
    pub fix: Option<String>,
    pub fix_line: Option<usize>,

    /// `- accept:` value and line index.
    pub accept: Option<String>,
    pub accept_line: Option<usize>,

    /// Parsed commit hashes from `- commit:`.
    pub commit: Vec<String>,
    /// Raw `- commit:` value, if present.
    pub commit_raw: Option<String>,
    /// 0-based line index of `- commit:`, if present.
    pub commit_line: Option<usize>,

    /// Unknown `- key: value` fields kept as written, in order of appearance.
    pub unknown_fields: Vec<FieldEntry>,
    /// Every `- key: value` line present in the block, in order of appearance.
    pub field_lines: Vec<FieldEntry>,
    /// Dated notes in order of appearance.
    pub notes: Vec<Note>,
}

impl Task {
    /// Returns the parsed `claimed` timestamp, accepting `YYYY-MM-DDTHH:MM±HH:MM` and RFC 3339
    /// (`R-IO-6`, `docs/format.md` §8).
    #[must_use]
    pub fn parse_claimed_at(&self) -> Option<DateTime<FixedOffset>> {
        let raw = self.claimed.as_deref()?;
        DateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M%:z")
            .or_else(|_| DateTime::parse_from_rfc3339(raw))
            .ok()
    }

    /// Returns the numeric IDs of all valid parsed dependencies.
    #[must_use]
    pub fn dependency_ids(&self) -> Vec<TaskId> {
        self.depends.iter().filter_map(|d| d.id).collect()
    }

    /// Returns the 0-based line index where a missing field `key` should be inserted according to
    /// the canonical field order (`docs/format.md` §10).
    #[must_use]
    pub fn insert_line_for_field(&self, key: &str) -> usize {
        let target_rank = canonical_field_rank(key);

        // Build effective (rank, line) pairs for all existing fields on this task.
        // Note: `type` and `area` living on a combined priority line share its line index.
        let mut best: Option<(usize, usize)> = None;
        let is_other_field = target_rank == CANONICAL_FIELD_ORDER.len();
        for entry in &self.field_lines {
            let rank = canonical_field_rank(&entry.key);
            if rank < target_rank || (is_other_field && rank == target_rank) {
                let replace = match best {
                    None => true,
                    Some((best_rank, best_line)) => {
                        rank > best_rank || (rank == best_rank && entry.line > best_line)
                    }
                };
                if replace {
                    best = Some((rank, entry.line));
                }
            }
        }

        // Also check combined priority `type`/`area` if target is after them (e.g. `area` or `depends`).
        if let Some(ref detail) = self.priority_detail {
            for (part_key, _) in &detail.extra_parts {
                let rank = canonical_field_rank(part_key);
                if rank < target_rank {
                    let replace = match best {
                        None => true,
                        Some((best_rank, best_line)) => {
                            rank > best_rank || (rank == best_rank && detail.line > best_line)
                        }
                    };
                    if replace {
                        best = Some((rank, detail.line));
                    }
                }
            }
        }

        match best {
            Some((_, line)) => line + 1,
            None => self.heading_line + 1,
        }
    }

    /// Returns the 0-based line index where a new `- note (...)` should be appended
    /// (`docs/format.md` §9: after the last line of the block).
    #[must_use]
    pub const fn note_insert_line(&self) -> usize {
        self.block_range.end
    }

    /// Looks up a field value by key (case-insensitive) for `--where key=value` filtering,
    /// covering both known fields and unknown `fields` entries.
    #[must_use]
    pub fn field_value(&self, key: &str) -> Option<&str> {
        let lower = key.to_ascii_lowercase();
        match lower.as_str() {
            "status" => Some(self.status.as_str()),
            "owner" => self.owner.as_deref(),
            "claimed" => self.claimed.as_deref(),
            "priority" => self.priority.as_deref(),
            "type" => self.task_type.as_deref(),
            "area" => self.area.as_deref(),
            "depends" => self.depends_raw.as_deref(),
            "files" => self.files.as_deref(),
            "problem" => self.problem.as_deref(),
            "fix" => self.fix.as_deref(),
            "accept" => self.accept.as_deref(),
            "commit" => self.commit_raw.as_deref(),
            _ => self
                .unknown_fields
                .iter()
                .find(|e| e.key == lower)
                .map(|e| e.value.as_str()),
        }
    }
}

/// Returns the 0-based position of `key` in [`CANONICAL_FIELD_ORDER`], or
/// `CANONICAL_FIELD_ORDER.len()` for other fields.
#[must_use]
pub fn canonical_field_rank(key: &str) -> usize {
    CANONICAL_FIELD_ORDER
        .iter()
        .position(|&k| k == key)
        .unwrap_or(CANONICAL_FIELD_ORDER.len())
}

/// Strips a leading section number such as `12.` or `3)` and following spaces (`docs/format.md` §5).
#[must_use]
pub fn strip_section_number(name: &str) -> &str {
    let trimmed = name.trim();
    let bytes = trimmed.as_bytes();
    let mut digits_end = 0usize;
    while digits_end < bytes.len() && bytes[digits_end].is_ascii_digit() {
        digits_end += 1;
    }
    if digits_end == 0 || digits_end >= bytes.len() {
        return trimmed;
    }
    if matches!(bytes[digits_end], b'.' | b')') {
        trimmed[digits_end + 1..].trim_start()
    } else {
        trimmed
    }
}

/// Returns `true` if `name` is a valid agent name (`docs/format.md` §11):
/// 1 to 64 characters from `A-Z a-z 0-9 . _ -`.
#[must_use]
pub fn is_valid_agent_name(name: &str) -> bool {
    let len = name.len();
    (1..=64).contains(&len)
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// Compares two agent names ignoring ASCII case (`docs/format.md` §11).
#[must_use]
pub fn agent_names_equal(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// Returns `true` if `owner_value` represents "nobody" (`—`, `-`, `none`, or empty).
#[must_use]
pub fn is_nobody_owner(owner_value: &str) -> bool {
    let t = owner_value.trim();
    t.is_empty() || t == CANONICAL_NOBODY_OWNER || t == "-" || t.eq_ignore_ascii_case("none")
}

/// A parsed `TODO.md` document combining the byte-exact [`LineDoc`] and parsed structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// Byte-exact lines and BOM state.
    pub line_doc: LineDoc,
    /// ID prefix and width configuration used to parse this document.
    pub id_config: IdConfig,
    /// Format version declared by `<!-- todo-format: N -->` in the preamble, if present.
    pub declared_format_version: Option<u32>,
    /// 0-based line index of the format line, if present.
    pub format_line: Option<usize>,
    /// 0-based half-open line range of the preamble (before first `## ` or task `### `).
    pub preamble_range: Range<usize>,
    /// Managed block (`<!-- todo:begin -->` .. `<!-- todo:end -->`), if present.
    pub managed_block: Option<ManagedBlock>,
    /// Sections in order of appearance.
    pub sections: Vec<Section>,
    /// Tasks in order of appearance.
    pub tasks: Vec<Task>,
    /// Problems and warnings discovered while parsing.
    pub problems: Vec<Problem>,
    /// O(1) index from task ID to first index in `tasks`.
    id_index: FastU64Map<TaskId, usize>,
}

impl Document {
    /// Parses raw bytes using the default [`IdConfig`] (`T`, width 3).
    pub fn parse_bytes(bytes: &[u8]) -> Result<Self, Utf8DecodeError> {
        Self::parse_bytes_with_config(bytes, IdConfig::default())
    }

    /// Parses raw bytes using the provided [`IdConfig`].
    pub fn parse_bytes_with_config(
        bytes: &[u8],
        id_config: IdConfig,
    ) -> Result<Self, Utf8DecodeError> {
        let line_doc = LineDoc::from_bytes(bytes)?;
        Ok(Self::from_line_doc(line_doc, id_config))
    }

    /// Parses an already decoded [`LineDoc`] using `id_config`.
    #[must_use]
    pub fn from_line_doc(line_doc: LineDoc, id_config: IdConfig) -> Self {
        parse_line_doc(line_doc, id_config)
    }

    /// Serializes the underlying [`LineDoc`] back to exact bytes.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        self.line_doc.to_bytes()
    }

    /// Returns the effective format version (`1` when the format line is omitted).
    #[must_use]
    pub fn effective_format_version(&self) -> u32 {
        self.declared_format_version
            .unwrap_or(SUPPORTED_FORMAT_VERSION)
    }

    /// Returns `true` if any file-wide problem blocks all writes (`UNSAFE_FILE`).
    #[must_use]
    pub fn blocks_all_writes(&self) -> bool {
        self.problems.iter().any(Problem::blocks_all_writes)
    }

    /// Returns `true` if a file-wide or task-specific problem blocks writes to `task_id`.
    #[must_use]
    pub fn blocks_task_write(&self, task_id: TaskId) -> bool {
        self.problems.iter().any(|p| p.blocks_task(task_id))
    }

    /// Returns all problems that block writing to `task_id` (both file-wide and task-specific).
    #[must_use]
    pub fn blocking_problems_for_task(&self, task_id: TaskId) -> Vec<&Problem> {
        self.problems
            .iter()
            .filter(|p| p.blocks_task(task_id))
            .collect()
    }

    /// Checks for duplicate task IDs across this document and an archive document
    /// (`docs/format.md` §13).
    #[must_use]
    pub fn check_archive_duplicates(&self, archive: &Self) -> Vec<Problem> {
        let mut archive_first_line: FastU64Map<TaskId, usize> =
            FastU64Map::with_capacity_and_hasher(archive.tasks.len(), Default::default());
        for task in &archive.tasks {
            archive_first_line
                .entry(task.id)
                .or_insert(task.heading_line + 1);
        }

        let mut out = Vec::new();
        for task in &self.tasks {
            if let Some(&arch_line) = archive_first_line.get(&task.id) {
                let id_str = self.id_config.format_id(task.id);
                out.push(Problem {
                    line: task.heading_line + 1,
                    severity: ProblemSeverity::BlockTask(task.id),
                    kind: ProblemKind::DuplicateTaskId {
                        id: task.id,
                        first_line: arch_line,
                    },
                    message: format!("task ID {id_str} also exists in archive (line {arch_line})"),
                });
            }
        }
        out
    }

    /// Returns the highest numeric task ID present in this document, if any (`R-ID-2`).
    #[must_use]
    pub fn highest_id(&self) -> Option<u64> {
        self.tasks.iter().map(|t| t.id.number()).max()
    }

    /// Finds a task by exact numeric ID.
    #[must_use]
    pub fn task_by_id(&self, id: TaskId) -> Option<&Task> {
        self.id_index.get(&id).and_then(|&idx| self.tasks.get(idx))
    }

    /// Returns the section name for `task`, if it belongs to a section.
    #[must_use]
    pub fn task_section_name(&self, task: &Task) -> Option<&str> {
        task.section
            .and_then(|idx| self.sections.get(idx))
            .map(|s| s.name.as_str())
    }

    /// Matches a section name according to `docs/format.md` §5 and R-WRITE-1:
    /// exact match ignoring case and leading numbers (`12.`, `3)`) first, otherwise a unique
    /// case-insensitive substring match.
    #[must_use]
    pub fn find_section(&self, query: &str) -> SectionLookup {
        let q_norm = strip_section_number(query).to_lowercase();
        let q_full = query.trim().to_lowercase();
        if q_norm.is_empty() && q_full.is_empty() {
            return SectionLookup::NotFound;
        }

        let exact: Vec<usize> = self
            .sections
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                s.name.trim().to_lowercase() == q_full
                    || s.normalized_name().to_lowercase() == q_norm
            })
            .map(|(idx, _)| idx)
            .collect();

        if exact.len() == 1 {
            return SectionLookup::Found(exact[0]);
        }
        if exact.len() > 1 {
            return SectionLookup::Ambiguous(exact);
        }

        let partial: Vec<usize> = self
            .sections
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                (!q_norm.is_empty() && s.normalized_name().to_lowercase().contains(&q_norm))
                    || (!q_full.is_empty() && s.name.to_lowercase().contains(&q_full))
            })
            .map(|(idx, _)| idx)
            .collect();

        match partial.len() {
            0 => SectionLookup::NotFound,
            1 => SectionLookup::Found(partial[0]),
            _ => SectionLookup::Ambiguous(partial),
        }
    }

    /// Returns `true` if `task` has all dependencies satisfied (every dependency exists in `self`
    /// — or `archive` if provided — and is `done` or `dropped`; `R-DEP-1`).
    #[must_use]
    pub fn dependencies_satisfied(&self, task: &Task, archive: Option<&Self>) -> bool {
        for dep in &task.depends {
            let Some(dep_id) = dep.id else {
                return false;
            };
            let target = self
                .task_by_id(dep_id)
                .or_else(|| archive.and_then(|a| a.task_by_id(dep_id)));
            match target {
                Some(t) if t.status.is_finished() => {}
                _ => return false,
            }
        }
        true
    }
}

/// Inspects raw bytes and returns all problems/warnings, handling invalid UTF-8 uniformly
/// (`R-EDIT-8`, `R-OUT-4`).
#[must_use]
pub fn inspect_problems(bytes: &[u8], id_config: IdConfig) -> Vec<Problem> {
    match Document::parse_bytes_with_config(bytes, id_config) {
        Ok(doc) => doc.problems,
        Err(utf8_err) => vec![Problem::from(&utf8_err)],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RegionState {
    Normal,
    InFence {
        ch: char,
        len: usize,
        open_line: usize,
    },
    InComment {
        open_line: usize,
    },
}

fn parse_line_doc(line_doc: LineDoc, id_config: IdConfig) -> Document {
    let line_count = line_doc.len();
    let mut ignored = vec![false; line_count];
    let mut problems = Vec::new();
    let mut state = RegionState::Normal;

    let mut declared_format_version: Option<u32> = None;
    let mut format_line: Option<usize> = None;
    let mut managed_begin: Option<usize> = None;
    let mut managed_end: Option<usize> = None;
    let mut seen_first_section_or_task = false;

    for (idx, line) in line_doc.lines.iter().enumerate() {
        let text = &line.text;
        let line_no = idx + 1;

        match state {
            RegionState::InFence { ch, len, .. } => {
                ignored[idx] = true;
                if is_closing_fence(text, ch, len) {
                    state = RegionState::Normal;
                }
                continue;
            }
            RegionState::InComment { .. } => {
                ignored[idx] = true;
                if text.contains("-->") {
                    state = RegionState::Normal;
                }
                continue;
            }
            RegionState::Normal => {}
        }

        // Fast skip for normal prose, blank lines, and task field/note lines that cannot start
        // any format line, managed block, fence, HTML comment, conflict marker, or heading.
        if matches!(
            text.as_bytes().first().copied(),
            None | Some(b'-' | b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'*' | b'+')
        ) {
            continue;
        }

        // Check format line and managed-block markers before treating HTML comments as ignored
        // (`docs/format.md` §3, §4, §12).
        if !seen_first_section_or_task && let Some(raw_ver) = parse_format_line(text) {
            ignored[idx] = true;
            if format_line.is_none() {
                format_line = Some(idx);
            }
            match raw_ver.parse::<u32>() {
                Ok(v) if v == SUPPORTED_FORMAT_VERSION => {
                    if declared_format_version.is_none() {
                        declared_format_version = Some(v);
                    }
                }
                Ok(v) => {
                    if declared_format_version.is_none() {
                        declared_format_version = Some(v);
                    }
                    problems.push(Problem {
                        line: line_no,
                        severity: ProblemSeverity::BlockAllWrites,
                        kind: ProblemKind::UnknownFormatVersion { raw: raw_ver },
                        message: format!(
                            "unsupported format version {v} (supported: {SUPPORTED_FORMAT_VERSION})"
                        ),
                    });
                }
                Err(_) => {
                    problems.push(Problem {
                        line: line_no,
                        severity: ProblemSeverity::BlockAllWrites,
                        kind: ProblemKind::UnknownFormatVersion { raw: raw_ver.clone() },
                        message: format!(
                            "invalid format version '{raw_ver}' (supported: {SUPPORTED_FORMAT_VERSION})"
                        ),
                    });
                }
            }
            continue;
        }

        let trimmed = text.trim();
        if trimmed == MANAGED_BLOCK_BEGIN {
            ignored[idx] = true;
            if managed_begin.is_none() {
                managed_begin = Some(idx);
            }
            continue;
        }
        if trimmed == MANAGED_BLOCK_END {
            ignored[idx] = true;
            if managed_begin.is_some() && managed_end.is_none() {
                managed_end = Some(idx);
            }
            continue;
        }

        // Check opening of fenced code block.
        if let Some((ch, len)) = parse_opening_fence(text) {
            ignored[idx] = true;
            state = RegionState::InFence {
                ch,
                len,
                open_line: line_no,
            };
            continue;
        }

        // Check opening of HTML comment.
        if let Some(after_open) = strip_html_comment_open(text) {
            ignored[idx] = true;
            if !after_open.contains("-->") {
                state = RegionState::InComment { open_line: line_no };
            }
            continue;
        }

        // Check git conflict markers outside ignored regions (`docs/format.md` §13).
        if is_conflict_marker(text) {
            problems.push(Problem {
                line: line_no,
                severity: ProblemSeverity::BlockAllWrites,
                kind: ProblemKind::ConflictMarker,
                message: "git conflict marker found".to_string(),
            });
        }

        // Track when the first section or task heading appears so format lines only apply in preamble.
        if !seen_first_section_or_task
            && (text.starts_with("## ") || parse_task_heading(text, &id_config).is_some())
        {
            seen_first_section_or_task = true;
        }
    }

    match state {
        RegionState::InFence { open_line, .. } => {
            problems.push(Problem {
                line: open_line,
                severity: ProblemSeverity::BlockAllWrites,
                kind: ProblemKind::UnclosedCodeFence,
                message: "unclosed fenced code block".to_string(),
            });
        }
        RegionState::InComment { open_line } => {
            problems.push(Problem {
                line: open_line,
                severity: ProblemSeverity::BlockAllWrites,
                kind: ProblemKind::UnclosedHtmlComment,
                message: "unclosed HTML comment".to_string(),
            });
        }
        RegionState::Normal => {}
    }

    let managed_block = match (managed_begin, managed_end) {
        (Some(begin_line), Some(end_line)) if begin_line < end_line => Some(ManagedBlock {
            begin_line,
            end_line,
        }),
        _ => None,
    };

    // Second pass: sections, task headings, and task raw block end lines.
    struct PendingTask {
        start_idx: usize,
        section: Option<usize>,
        heading: ParsedHeading,
        raw_end: usize,
    }

    let mut sections: Vec<Section> = Vec::new();
    let mut task_starts: Vec<PendingTask> = Vec::new();
    let mut first_heading_line: Option<usize> = None;

    for (idx, line) in line_doc.lines.iter().enumerate() {
        if ignored[idx] {
            continue;
        }
        let text = &line.text;
        if let Some(name) = text.strip_prefix("## ") {
            if let Some(prev_task) = task_starts.last_mut()
                && prev_task.raw_end == line_count
            {
                prev_task.raw_end = idx;
            }
            if first_heading_line.is_none() {
                first_heading_line = Some(idx);
            }
            if let Some(prev) = sections.last_mut() {
                prev.range.end = idx;
            }
            sections.push(Section {
                name: name.trim().to_string(),
                heading_line: idx,
                range: idx..line_count,
                content_end: idx + 1,
            });
        } else if text.starts_with("### ") {
            if let Some(prev_task) = task_starts.last_mut()
                && prev_task.raw_end == line_count
            {
                prev_task.raw_end = idx;
            }
            match parse_task_heading(text, &id_config) {
                Some(parsed_heading) => {
                    if first_heading_line.is_none() {
                        first_heading_line = Some(idx);
                    }
                    let sec_idx = sections.len().checked_sub(1);
                    task_starts.push(PendingTask {
                        start_idx: idx,
                        section: sec_idx,
                        heading: parsed_heading,
                        raw_end: line_count,
                    });
                }
                None => {
                    let after_hashes = text.strip_prefix("### ").unwrap_or("");
                    if id_config.looks_like_prefixed_id(after_hashes.trim_start()) {
                        problems.push(Problem {
                            line: idx + 1,
                            severity: ProblemSeverity::Warning,
                            kind: ProblemKind::HeadingProbableMistake,
                            message: "heading looks like a task ID but does not match task heading syntax"
                                .to_string(),
                        });
                    }
                }
            }
        }
    }

    // Compute `content_end` for every section (line right after last non-blank line in range).
    for section in &mut sections {
        let mut end = section.range.end;
        while end > section.heading_line + 1 && line_doc.lines[end - 1].text.trim().is_empty() {
            end -= 1;
        }
        section.content_end = end;
    }

    let preamble_range = 0..first_heading_line.unwrap_or(line_count);

    // Parse each task block and track duplicate task IDs.
    let mut tasks = Vec::with_capacity(task_starts.len());
    let mut seen_ids: FastU64Map<TaskId, usize> =
        FastU64Map::with_capacity_and_hasher(task_starts.len(), Default::default());
    let mut id_index: FastU64Map<TaskId, usize> =
        FastU64Map::with_capacity_and_hasher(task_starts.len(), Default::default());

    for pending in task_starts {
        let start_idx = pending.start_idx;
        let sec_idx = pending.section;
        let heading = pending.heading;

        // Trailing blank lines don't belong to the task block (`docs/format.md` §7).
        let mut block_end = pending.raw_end;
        while block_end > start_idx + 1 && line_doc.lines[block_end - 1].text.trim().is_empty() {
            block_end -= 1;
        }

        let block_range = start_idx..block_end;
        let line_no = start_idx + 1;
        if let Some(&first_line) = seen_ids.get(&heading.id) {
            let formatted_id = id_config.format_id(heading.id);
            problems.push(Problem {
                line: line_no,
                severity: ProblemSeverity::BlockTask(heading.id),
                kind: ProblemKind::DuplicateTaskId {
                    id: heading.id,
                    first_line,
                },
                message: format!(
                    "duplicate task ID {formatted_id} (first seen on line {first_line})"
                ),
            });
        } else {
            seen_ids.insert(heading.id, line_no);
        }

        let task = parse_task_block(
            &line_doc,
            &ignored,
            &id_config,
            start_idx,
            block_range,
            sec_idx,
            heading,
            &mut problems,
        );
        let task_idx = tasks.len();
        id_index.entry(task.id).or_insert(task_idx);
        tasks.push(task);
    }

    problems.sort_by_key(|p| p.line);

    Document {
        line_doc,
        id_config,
        declared_format_version,
        format_line,
        preamble_range,
        managed_block,
        sections,
        tasks,
        problems,
        id_index,
    }
}

#[derive(Debug, Clone)]
struct ParsedHeading {
    id: TaskId,
    raw_id: String,
    separator: String,
    title: String,
}

fn parse_task_heading(line: &str, id_config: &IdConfig) -> Option<ParsedHeading> {
    let rest = line.strip_prefix("### ")?;
    let after_prefix = strip_prefix_ignore_case(rest, &id_config.prefix)?;
    let after_hyphen = after_prefix.strip_prefix('-')?;

    let digit_len = after_hyphen
        .bytes()
        .take_while(|b| b.is_ascii_digit())
        .count();
    if digit_len == 0 {
        return None;
    }

    let matched_prefix_len = rest.len() - after_prefix.len();
    let id_len = matched_prefix_len + 1 + digit_len;
    let raw_id = rest[..id_len].to_string();
    let id = id_config.parse_heading_id(&raw_id)?;
    let after_id = &rest[id_len..];

    for &sep in TASK_HEADING_SEPARATORS {
        if let Some(raw_title) = after_id.strip_prefix(sep) {
            let title = raw_title.trim();
            if title.is_empty() {
                return None;
            }
            return Some(ParsedHeading {
                id,
                raw_id,
                separator: sep.to_string(),
                title: title.to_string(),
            });
        }
    }

    None
}

#[allow(clippy::too_many_arguments)]
fn parse_task_block(
    line_doc: &LineDoc,
    ignored: &[bool],
    id_config: &IdConfig,
    heading_line: usize,
    block_range: Range<usize>,
    section: Option<usize>,
    heading: ParsedHeading,
    problems: &mut Vec<Problem>,
) -> Task {
    let mut task = Task {
        id: heading.id,
        raw_id: heading.raw_id,
        separator: heading.separator,
        title: heading.title,
        heading_line,
        block_range: block_range.clone(),
        section,
        status: Status::Open,
        status_raw: None,
        status_line: None,
        owner: None,
        owner_raw: None,
        owner_line: None,
        claimed: None,
        claimed_line: None,
        priority: None,
        priority_detail: None,
        task_type: None,
        task_type_loc: None,
        area: None,
        area_loc: None,
        depends: Vec::new(),
        depends_raw: None,
        depends_line: None,
        files: None,
        files_line: None,
        problem: None,
        problem_line: None,
        fix: None,
        fix_line: None,
        accept: None,
        accept_line: None,
        commit: Vec::new(),
        commit_raw: None,
        commit_line: None,
        unknown_fields: Vec::new(),
        field_lines: Vec::with_capacity(block_range.end.saturating_sub(heading_line + 1)),
        notes: Vec::new(),
    };

    let mut seen_known = [0usize; 12];
    let mut seen_unknown: Vec<(String, usize)> = Vec::new();

    let check_and_mark_seen = |key: &str,
                               line_no: usize,
                               seen_known: &mut [usize; 12],
                               seen_unknown: &mut Vec<(String, usize)>|
     -> Option<usize> {
        if let Some(slot) = known_field_index(key) {
            let prev = seen_known[slot];
            if prev != 0 {
                Some(prev)
            } else {
                seen_known[slot] = line_no;
                None
            }
        } else if let Some(&(_, prev)) = seen_unknown.iter().find(|(k, _)| k == key) {
            Some(prev)
        } else {
            seen_unknown.push((key.to_string(), line_no));
            None
        }
    };

    for (idx, &is_ignored) in ignored
        .iter()
        .enumerate()
        .take(block_range.end)
        .skip(heading_line + 1)
    {
        if is_ignored {
            continue;
        }
        let text = &line_doc.lines[idx].text;
        let line_no = idx + 1;

        if let Some(note) = parse_note_line(text, idx) {
            task.notes.push(note);
            continue;
        }

        if let Some((key_str, value_str)) = parse_field_line(text) {
            let owned_val = value_str.to_string();
            task.field_lines.push(FieldEntry {
                key: key_str.to_string(),
                value: owned_val.clone(),
                line: idx,
            });

            if let Some(first_line) =
                check_and_mark_seen(key_str, line_no, &mut seen_known, &mut seen_unknown)
            {
                problems.push(Problem {
                    line: line_no,
                    severity: ProblemSeverity::BlockTask(task.id),
                    kind: ProblemKind::RepeatedField {
                        id: task.id,
                        field: key_str.to_string(),
                        first_line,
                    },
                    message: format!(
                        "repeated field '{key_str}' on {} (first seen on line {first_line})",
                        id_config.format_id(task.id)
                    ),
                });
                continue;
            }

            match key_str {
                "status" => {
                    task.status_line = Some(idx);
                    match Status::parse(value_str) {
                        Some(s) => {
                            task.status = s;
                            task.status_raw = Some(owned_val);
                        }
                        None => {
                            task.status_raw = Some(owned_val.clone());
                            problems.push(Problem {
                                line: line_no,
                                severity: ProblemSeverity::BlockTask(task.id),
                                kind: ProblemKind::UnknownStatus {
                                    id: task.id,
                                    value: owned_val.clone(),
                                },
                                message: format!(
                                    "unknown status '{owned_val}' on {}",
                                    id_config.format_id(task.id)
                                ),
                            });
                        }
                    }
                }
                "owner" => {
                    task.owner_line = Some(idx);
                    if is_nobody_owner(value_str) {
                        task.owner_raw = Some(owned_val);
                        task.owner = None;
                    } else {
                        task.owner_raw = Some(owned_val.clone());
                        task.owner = Some(owned_val);
                    }
                }
                "claimed" => {
                    task.claimed = if owned_val.is_empty() {
                        None
                    } else {
                        Some(owned_val)
                    };
                    task.claimed_line = Some(idx);
                }
                "priority" => {
                    let detail = parse_combined_priority(idx, value_str);
                    task.priority = detail.priority.clone();
                    for (part_key, part_val) in &detail.extra_parts {
                        if let Some(first_line) = check_and_mark_seen(
                            part_key,
                            line_no,
                            &mut seen_known,
                            &mut seen_unknown,
                        ) {
                            problems.push(Problem {
                                line: line_no,
                                severity: ProblemSeverity::BlockTask(task.id),
                                kind: ProblemKind::RepeatedField {
                                    id: task.id,
                                    field: part_key.clone(),
                                    first_line,
                                },
                                message: format!(
                                    "repeated field '{part_key}' on {} (first seen on line {first_line})",
                                    id_config.format_id(task.id)
                                ),
                            });
                            continue;
                        }
                        match part_key.as_str() {
                            "type" => {
                                task.task_type = non_empty_string(part_val);
                                task.task_type_loc = Some(FieldLocation::CombinedPriority(idx));
                            }
                            "area" => {
                                task.area = non_empty_string(part_val);
                                task.area_loc = Some(FieldLocation::CombinedPriority(idx));
                            }
                            _ => {
                                task.unknown_fields.push(FieldEntry {
                                    key: part_key.clone(),
                                    value: part_val.clone(),
                                    line: idx,
                                });
                            }
                        }
                    }
                    task.priority_detail = Some(detail);
                }
                "type" => {
                    task.task_type = if owned_val.is_empty() {
                        None
                    } else {
                        Some(owned_val)
                    };
                    task.task_type_loc = Some(FieldLocation::OwnLine(idx));
                }
                "area" => {
                    task.area = if owned_val.is_empty() {
                        None
                    } else {
                        Some(owned_val)
                    };
                    task.area_loc = Some(FieldLocation::OwnLine(idx));
                }
                "depends" => {
                    task.depends = parse_depends_list(value_str, id_config);
                    task.depends_raw = Some(owned_val);
                    task.depends_line = Some(idx);
                }
                "files" => {
                    task.files = if owned_val.is_empty() {
                        None
                    } else {
                        Some(owned_val)
                    };
                    task.files_line = Some(idx);
                }
                "problem" => {
                    task.problem = if owned_val.is_empty() {
                        None
                    } else {
                        Some(owned_val)
                    };
                    task.problem_line = Some(idx);
                }
                "fix" => {
                    task.fix = if owned_val.is_empty() {
                        None
                    } else {
                        Some(owned_val)
                    };
                    task.fix_line = Some(idx);
                }
                "accept" => {
                    task.accept = if owned_val.is_empty() {
                        None
                    } else {
                        Some(owned_val)
                    };
                    task.accept_line = Some(idx);
                }
                "commit" => {
                    task.commit = parse_comma_list(value_str);
                    task.commit_raw = Some(owned_val);
                    task.commit_line = Some(idx);
                }
                _ => {
                    task.unknown_fields.push(FieldEntry {
                        key: key_str.to_string(),
                        value: owned_val,
                        line: idx,
                    });
                }
            }
        }
    }

    task
}

const fn known_field_index(key: &str) -> Option<usize> {
    if key.len() < 3 || key.len() > 8 {
        return None;
    }
    match key.as_bytes() {
        b"status" => Some(0),
        b"owner" => Some(1),
        b"claimed" => Some(2),
        b"priority" => Some(3),
        b"type" => Some(4),
        b"area" => Some(5),
        b"depends" => Some(6),
        b"files" => Some(7),
        b"problem" => Some(8),
        b"fix" => Some(9),
        b"accept" => Some(10),
        b"commit" => Some(11),
        _ => None,
    }
}

fn non_empty_string(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn parse_field_line(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix("- ")?;
    let (key, after_colon) = rest.split_once(':')?;
    if !is_valid_field_key(key) {
        return None;
    }
    if after_colon.is_empty() {
        return Some((key, ""));
    }
    let val = after_colon.strip_prefix(' ')?;
    Some((key, val.trim()))
}

fn is_valid_field_key(key: &str) -> bool {
    let mut bytes = key.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}

fn parse_combined_priority(line: usize, raw: &str) -> CombinedPriorityLine {
    let mut parts = raw.split(" · ");
    let first = parts.next().unwrap_or("").trim();
    let priority = if first.is_empty() {
        None
    } else {
        Some(first.to_string())
    };
    let mut extra_parts = Vec::new();
    for part in parts {
        let p = part.trim();
        if let Some((k, v)) = p.split_once(':') {
            let key = k.trim();
            if is_valid_field_key(key) {
                extra_parts.push((key.to_string(), v.trim().to_string()));
            }
        }
    }
    CombinedPriorityLine {
        line,
        priority,
        extra_parts,
    }
}

fn parse_depends_list(raw: &str, id_config: &IdConfig) -> Vec<DependencyRef> {
    parse_comma_list(raw)
        .into_iter()
        .map(|token| {
            let id = id_config.parse_user_id(&token);
            DependencyRef { raw: token, id }
        })
        .collect()
}

fn parse_comma_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_note_line(line: &str, line_idx: usize) -> Option<Note> {
    let after_open = line.strip_prefix("- note (")?;
    let (inside, after_paren) = after_open.split_once(')')?;
    let after_colon = after_paren.strip_prefix(':')?;
    let text = if after_colon.is_empty() {
        ""
    } else {
        after_colon.strip_prefix(' ')?.trim()
    };

    let (date_part, agent_part) = inside.split_once(',')?;
    let date_raw = date_part.trim();
    let agent = agent_part.trim();
    if agent.is_empty() {
        return None;
    }
    let date = parse_ascii_ymd(date_raw)?;

    Some(Note {
        date_raw: date_raw.to_string(),
        date,
        agent: agent.to_string(),
        text: text.to_string(),
        line: line_idx,
    })
}

fn parse_ascii_ymd(s: &str) -> Option<NaiveDate> {
    let b = s.as_bytes();
    if b.len() == 10 && b[4] == b'-' && b[7] == b'-' {
        let d = |i: usize| -> Option<u32> {
            let v = b[i].wrapping_sub(b'0');
            if v < 10 { Some(v as u32) } else { None }
        };
        let year = (d(0)? * 1000 + d(1)? * 100 + d(2)? * 10 + d(3)?) as i32;
        let month = d(5)? * 10 + d(6)?;
        let day = d(8)? * 10 + d(9)?;
        return NaiveDate::from_ymd_opt(year, month, day);
    }
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

fn strip_up_to_3_spaces(line: &str) -> Option<&str> {
    let spaces = line.bytes().take_while(|&b| b == b' ').count();
    if spaces <= 3 {
        Some(&line[spaces..])
    } else {
        None
    }
}

fn parse_opening_fence(line: &str) -> Option<(char, usize)> {
    let rest = strip_up_to_3_spaces(line)?;
    let first = rest.chars().next()?;
    if first != '`' && first != '~' {
        return None;
    }
    let len = rest.chars().take_while(|&c| c == first).count();
    if len >= 3 { Some((first, len)) } else { None }
}

fn is_closing_fence(line: &str, fence_char: char, min_len: usize) -> bool {
    let Some(rest) = strip_up_to_3_spaces(line) else {
        return false;
    };
    let trimmed = rest.trim_end_matches(' ');
    let count = trimmed.chars().count();
    count >= min_len && trimmed.chars().all(|c| c == fence_char)
}

fn strip_html_comment_open(line: &str) -> Option<&str> {
    let rest = strip_up_to_3_spaces(line)?;
    rest.strip_prefix("<!--")
}

fn parse_format_line(line: &str) -> Option<String> {
    let rest = strip_up_to_3_spaces(line)?.trim_end_matches(' ');
    let inner = rest.strip_prefix("<!--")?.strip_suffix("-->")?.trim();
    if inner.contains("-->") {
        return None;
    }
    let version = inner.strip_prefix("todo-format:")?.trim();
    Some(version.to_string())
}

fn is_conflict_marker(line: &str) -> bool {
    line.starts_with("<<<<<<< ")
        || line == "======="
        || line.starts_with("======= ")
        || line.starts_with(">>>>>>> ")
}
