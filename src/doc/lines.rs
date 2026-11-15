//! Byte-exact line model, line edits, and byte-level write verification.
//!
//! Covers requirements R-EDIT-1, R-EDIT-2, R-EDIT-4, R-EDIT-5, R-EDIT-6, R-EDIT-7, and R-IO-4.

use std::fmt;
use std::ops::Range;

/// The UTF-8 byte-order mark (`U+FEFF`).
pub const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";

/// The line ending of a single line in a `TODO.md` document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineEnding {
    /// Unix line feed (`\n`).
    Lf,
    /// Windows carriage return + line feed (`\r\n`).
    Crlf,
    /// No line ending (only valid on the last line of a file).
    None,
}

impl LineEnding {
    /// Returns the raw byte representation of this line ending.
    #[must_use]
    pub const fn as_bytes(self) -> &'static [u8] {
        match self {
            Self::Lf => b"\n",
            Self::Crlf => b"\r\n",
            Self::None => b"",
        }
    }

    /// Returns the string representation of this line ending.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::Crlf => "\r\n",
            Self::None => "",
        }
    }
}

/// A single line in a document, preserving its exact line ending.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// Line text without its trailing `\n` or `\r\n` (and without any leading file BOM).
    pub text: String,
    /// The exact ending on this line.
    pub ending: LineEnding,
}

impl Line {
    /// Creates a new line with the given text and ending.
    #[must_use]
    pub fn new(text: impl Into<String>, ending: LineEnding) -> Self {
        Self {
            text: text.into(),
            ending,
        }
    }
}

/// Error returned when input bytes are not valid UTF-8 (R-IO-4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Utf8DecodeError {
    /// 0-based byte offset of the first invalid UTF-8 byte.
    pub byte_offset: usize,
    /// 1-based line number where the invalid UTF-8 byte appears.
    pub line: usize,
}

impl fmt::Display for Utf8DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid UTF-8 at byte {}", self.byte_offset)
    }
}

impl std::error::Error for Utf8DecodeError {}

/// Errors produced when constructing, applying, or verifying line edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    /// Input bytes contained invalid UTF-8.
    InvalidUtf8(Utf8DecodeError),
    /// An edit range had `start > end`.
    InvalidRange { start: usize, end: usize },
    /// An edit range exceeded the document's line count.
    RangeOutOfBounds {
        start: usize,
        end: usize,
        line_count: usize,
    },
    /// Two edits in the same batch overlapped.
    OverlappingEdits {
        first: Range<usize>,
        second: Range<usize>,
    },
    /// A single line provided to an edit contained `\n` or `\r`.
    NewlineInLine { line_text: String },
    /// An edit would leave the final line empty with no line ending, which cannot be
    /// represented on disk.
    EmptyUnterminatedLastLine,
    /// Post-edit verification failed (R-EDIT-7).
    VerificationFailed { reason: String },
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUtf8(err) => write!(f, "{err}"),
            Self::InvalidRange { start, end } => {
                write!(f, "invalid line edit range {start}..{end}")
            }
            Self::RangeOutOfBounds {
                start,
                end,
                line_count,
            } => write!(
                f,
                "line edit range {start}..{end} is out of bounds for {line_count} lines"
            ),
            Self::OverlappingEdits { first, second } => {
                write!(f, "overlapping line edits {first:?} and {second:?}")
            }
            Self::NewlineInLine { .. } => {
                write!(f, "edited line text must not contain line breaks")
            }
            Self::EmptyUnterminatedLastLine => {
                write!(f, "cannot leave an empty final line without a line ending")
            }
            Self::VerificationFailed { reason } => {
                write!(f, "edit verification failed: {reason}")
            }
        }
    }
}

impl std::error::Error for EditError {}

impl From<Utf8DecodeError> for EditError {
    fn from(value: Utf8DecodeError) -> Self {
        Self::InvalidUtf8(value)
    }
}

/// A whole-line edit expressed in 0-based line indices of the original document.
///
/// Replacing `i..i+1` with `[text]` replaces one line.
/// Replacing `i..i` with `lines` inserts before line `i` (or appends at `doc.len()`).
/// Replacing `a..b` with `[]` removes lines `a..b`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineEdit {
    /// 0-based line range in the original document to replace.
    pub range: Range<usize>,
    /// Replacement lines (each must be a single line without `\r` or `\n`).
    pub lines: Vec<String>,
}

impl LineEdit {
    /// Creates a splice edit replacing `range` with `lines`.
    #[must_use]
    pub fn splice(range: Range<usize>, lines: Vec<String>) -> Self {
        Self { range, lines }
    }

    /// Replaces a single line at `index` with `text`.
    #[must_use]
    pub fn replace_line(index: usize, text: impl Into<String>) -> Self {
        Self {
            range: index..index + 1,
            lines: vec![text.into()],
        }
    }

    /// Replaces `range` with `lines`.
    #[must_use]
    pub fn replace_range(range: Range<usize>, lines: Vec<String>) -> Self {
        Self { range, lines }
    }

    /// Inserts `lines` before 0-based line `index` (or at `doc.len()` to append).
    #[must_use]
    pub fn insert(index: usize, lines: Vec<String>) -> Self {
        Self {
            range: index..index,
            lines,
        }
    }

    /// Removes the half-open 0-based line range `range`.
    #[must_use]
    pub fn remove(range: Range<usize>) -> Self {
        Self {
            range,
            lines: Vec::new(),
        }
    }
}

/// Summary of an edit range used by byte-level verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedByteChange {
    /// 0-based line range in the original document meant to change.
    pub orig_range: Range<usize>,
    /// Replacement line texts expected in the edited document.
    pub new_lines: Vec<String>,
}

impl From<&LineEdit> for ExpectedByteChange {
    fn from(edit: &LineEdit) -> Self {
        Self {
            orig_range: edit.range.clone(),
            new_lines: edit.lines.clone(),
        }
    }
}

/// Result of applying [`LineEdit`]s to a [`LineDoc`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedLineEdits {
    /// The resulting line document.
    pub doc: LineDoc,
    /// Serialized bytes of `doc`.
    pub bytes: Vec<u8>,
    /// `true` if `bytes` differs from the original document's bytes (R-EDIT-6).
    pub changed: bool,
}

/// A byte-exact representation of a UTF-8 file split into lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineDoc {
    /// Whether the file began with a UTF-8 byte-order mark (`0xEF 0xBB 0xBF`).
    pub has_bom: bool,
    /// Lines in order, each preserving its exact ending.
    pub lines: Vec<Line>,
}

impl LineDoc {
    /// Decodes raw file bytes into a [`LineDoc`].
    ///
    /// Returns [`Utf8DecodeError`] with the exact byte offset and 1-based line number if the input
    /// is not valid UTF-8.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Utf8DecodeError> {
        let text = std::str::from_utf8(bytes).map_err(|err| {
            let byte_offset = err.valid_up_to();
            let line = bytes[..byte_offset].iter().filter(|&&b| b == b'\n').count() + 1;
            Utf8DecodeError { byte_offset, line }
        })?;

        let (has_bom, rest) = match text.strip_prefix('\u{FEFF}') {
            Some(after_bom) => (true, after_bom),
            None => (false, text),
        };

        let mut lines = Vec::new();
        for raw in rest.split_inclusive('\n') {
            if let Some(without_lf) = raw.strip_suffix('\n') {
                if let Some(without_crlf) = without_lf.strip_suffix('\r') {
                    lines.push(Line::new(without_crlf, LineEnding::Crlf));
                } else {
                    lines.push(Line::new(without_lf, LineEnding::Lf));
                }
            } else {
                lines.push(Line::new(raw, LineEnding::None));
            }
        }

        Ok(Self { has_bom, lines })
    }

    /// Serializes the document back into exact bytes (R-EDIT-2).
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let extra_bom = if self.has_bom { UTF8_BOM.len() } else { 0 };
        let capacity: usize = extra_bom
            + self
                .lines
                .iter()
                .map(|l| l.text.len() + l.ending.as_bytes().len())
                .sum::<usize>();
        let mut out = Vec::with_capacity(capacity);
        if self.has_bom {
            out.extend_from_slice(UTF8_BOM);
        }
        for line in &self.lines {
            out.extend_from_slice(line.text.as_bytes());
            out.extend_from_slice(line.ending.as_bytes());
        }
        out
    }

    /// Returns the number of lines in the document.
    #[must_use]
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Returns `true` if the document has no lines.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Returns the 0-based line at `index`, if in bounds.
    #[must_use]
    pub fn line(&self, index: usize) -> Option<&Line> {
        self.lines.get(index)
    }

    /// Returns the ending to use for newly added lines (`docs/format.md` §2).
    ///
    /// Uses `Crlf` if more lines end with CRLF than LF, and `Lf` on a tie or when no line has an
    /// ending.
    #[must_use]
    pub fn default_ending(&self) -> LineEnding {
        let mut lf_count = 0usize;
        let mut crlf_count = 0usize;
        for line in &self.lines {
            match line.ending {
                LineEnding::Lf => lf_count += 1,
                LineEnding::Crlf => crlf_count += 1,
                LineEnding::None => {}
            }
        }
        if crlf_count > lf_count {
            LineEnding::Crlf
        } else {
            LineEnding::Lf
        }
    }

    /// Applies one or more whole-line edits, returning the new document, bytes, and whether
    /// anything changed.
    ///
    /// Preserves BOM, existing line endings on untouched and 1-for-1 replaced lines, uses
    /// [`Self::default_ending`] for newly inserted lines, and handles appending after an
    /// unterminated last line per `docs/format.md` §2.
    pub fn apply_edits(&self, edits: &[LineEdit]) -> Result<AppliedLineEdits, EditError> {
        let sorted = validate_and_sort(edits, self.lines.len(), |e| &e.range, |e| &e.lines)?;
        let orig_bytes = self.to_bytes();

        if sorted.is_empty() {
            return Ok(AppliedLineEdits {
                doc: self.clone(),
                bytes: orig_bytes,
                changed: false,
            });
        }

        let default_ending = self.default_ending();
        let mut new_lines = Vec::with_capacity(self.lines.len());
        let mut cursor = 0usize;

        for edit in sorted {
            while cursor < edit.range.start {
                new_lines.push(self.lines[cursor].clone());
                cursor += 1;
            }

            if edit.range.is_empty() {
                append_inserted_lines(
                    &mut new_lines,
                    &edit.lines,
                    edit.range.start == self.lines.len(),
                    default_ending,
                );
            } else if !edit.lines.is_empty() {
                let last_replaced_ending = self.lines[edit.range.end - 1].ending;
                if edit.lines.len() == edit.range.len() {
                    for (offset, text) in edit.lines.iter().enumerate() {
                        let ending = self.lines[edit.range.start + offset].ending;
                        new_lines.push(Line::new(text.clone(), ending));
                    }
                } else {
                    for (idx, text) in edit.lines.iter().enumerate() {
                        let ending = if idx + 1 == edit.lines.len() {
                            last_replaced_ending
                        } else {
                            default_ending
                        };
                        new_lines.push(Line::new(text.clone(), ending));
                    }
                }
            }

            cursor = edit.range.end;
        }

        while cursor < self.lines.len() {
            new_lines.push(self.lines[cursor].clone());
            cursor += 1;
        }

        if let Some(last) = new_lines.last()
            && last.text.is_empty()
            && last.ending == LineEnding::None
        {
            return Err(EditError::EmptyUnterminatedLastLine);
        }

        let new_doc = Self {
            has_bom: self.has_bom,
            lines: new_lines,
        };
        let new_bytes = new_doc.to_bytes();
        let changed = new_bytes != orig_bytes;

        Ok(AppliedLineEdits {
            doc: new_doc,
            bytes: new_bytes,
            changed,
        })
    }

    /// Applies `edits` and verifies that every byte outside `edits` remained unchanged (R-EDIT-7).
    pub fn apply_and_verify_bytes(
        &self,
        edits: &[LineEdit],
    ) -> Result<AppliedLineEdits, EditError> {
        let orig_bytes = self.to_bytes();
        let applied = self.apply_edits(edits)?;
        verify_byte_edits(&orig_bytes, &applied.bytes, edits)?;
        Ok(applied)
    }
}

/// Converts any line breaks (`\r\n`, `\n`, `\r`) in `input` to single spaces (R-EDIT-5).
#[must_use]
pub fn sanitize_single_line(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            out.push(' ');
        } else if c == '\n' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

/// Converts line breaks to spaces and trims surrounding whitespace (`docs/format.md` §2, §10).
#[must_use]
pub fn sanitize_trimmed_value(input: &str) -> String {
    sanitize_single_line(input).trim().to_string()
}

/// Verifies that `edited_bytes` differs from `orig_bytes` only within the expected `edits`.
///
/// Proves that:
/// - Both documents are valid UTF-8 and have matching BOM presence.
/// - The line count matches `orig_len - removed + inserted`.
/// - Every line outside the edited ranges has identical raw bytes (including exact line endings),
///   accounting only for an unterminated final line gaining `default_ending` when new lines are
///   appended after it.
/// - Every line inside the edited ranges has text matching `edit.lines`.
pub fn verify_byte_edits(
    orig_bytes: &[u8],
    edited_bytes: &[u8],
    edits: &[LineEdit],
) -> Result<(), EditError> {
    let expected: Vec<ExpectedByteChange> = edits.iter().map(ExpectedByteChange::from).collect();
    verify_expected_byte_changes(orig_bytes, edited_bytes, &expected)
}

/// Same as [`verify_byte_edits`], accepting [`ExpectedByteChange`] slices directly.
pub fn verify_expected_byte_changes(
    orig_bytes: &[u8],
    edited_bytes: &[u8],
    changes: &[ExpectedByteChange],
) -> Result<(), EditError> {
    let orig_doc = LineDoc::from_bytes(orig_bytes)?;
    let edited_doc = LineDoc::from_bytes(edited_bytes)?;

    if orig_doc.has_bom != edited_doc.has_bom {
        return Err(EditError::VerificationFailed {
            reason: "UTF-8 BOM presence changed".to_string(),
        });
    }

    let sorted = validate_and_sort(
        changes,
        orig_doc.lines.len(),
        |c| &c.orig_range,
        |c| &c.new_lines,
    )?;

    let removed_total: usize = sorted.iter().map(|c| c.orig_range.len()).sum();
    let inserted_total: usize = sorted.iter().map(|c| c.new_lines.len()).sum();
    let expected_edited_len = orig_doc
        .lines
        .len()
        .saturating_sub(removed_total)
        .saturating_add(inserted_total);

    if edited_doc.lines.len() != expected_edited_len {
        return Err(EditError::VerificationFailed {
            reason: format!(
                "expected {expected_edited_len} lines after edit, got {}",
                edited_doc.lines.len()
            ),
        });
    }

    let orig_spans = line_byte_spans(orig_bytes, orig_doc.has_bom);
    let edited_spans = line_byte_spans(edited_bytes, edited_doc.has_bom);
    let default_ending = orig_doc.default_ending();

    let mut orig_cursor = 0usize;
    let mut edited_cursor = 0usize;

    for (idx, change) in sorted.iter().enumerate() {
        let appends_after_unterminated_eof = change.orig_range.is_empty()
            && change.orig_range.start == orig_doc.lines.len()
            && !change.new_lines.is_empty()
            && orig_doc
                .lines
                .last()
                .is_some_and(|l| l.ending == LineEnding::None)
            // Only the first non-empty EOF append triggers the transition from None -> default_ending.
            && sorted[..idx]
                .iter()
                .all(|prev| !(prev.orig_range.start == orig_doc.lines.len() && !prev.new_lines.is_empty()));

        verify_unchanged_gap(
            orig_bytes,
            edited_bytes,
            &orig_doc,
            &edited_doc,
            &orig_spans,
            &edited_spans,
            orig_cursor..change.orig_range.start,
            edited_cursor,
            appends_after_unterminated_eof,
            default_ending,
        )?;

        let gap_len = change.orig_range.start - orig_cursor;
        edited_cursor += gap_len;

        for (offset, expected_text) in change.new_lines.iter().enumerate() {
            let actual = &edited_doc.lines[edited_cursor + offset];
            if &actual.text != expected_text {
                return Err(EditError::VerificationFailed {
                    reason: format!(
                        "edited line {} text did not match intended output",
                        edited_cursor + offset + 1
                    ),
                });
            }
        }

        orig_cursor = change.orig_range.end;
        edited_cursor += change.new_lines.len();
    }

    verify_unchanged_gap(
        orig_bytes,
        edited_bytes,
        &orig_doc,
        &edited_doc,
        &orig_spans,
        &edited_spans,
        orig_cursor..orig_doc.lines.len(),
        edited_cursor,
        false,
        default_ending,
    )?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn verify_unchanged_gap(
    orig_bytes: &[u8],
    edited_bytes: &[u8],
    orig_doc: &LineDoc,
    edited_doc: &LineDoc,
    orig_spans: &[(usize, usize)],
    edited_spans: &[(usize, usize)],
    orig_gap: Range<usize>,
    edited_start: usize,
    allow_last_line_ending_gain: bool,
    default_ending: LineEnding,
) -> Result<(), EditError> {
    let gap_len = orig_gap.len();
    if gap_len == 0 {
        return Ok(());
    }

    let check_exact_end = if allow_last_line_ending_gain
        && orig_gap.end == orig_doc.lines.len()
        && orig_gap.end > orig_gap.start
    {
        orig_gap.end - 1
    } else {
        orig_gap.end
    };

    for offset in 0..(check_exact_end - orig_gap.start) {
        let oi = orig_gap.start + offset;
        let ei = edited_start + offset;
        let (os, oe) = orig_spans[oi];
        let (es, ee) = edited_spans[ei];
        if orig_bytes[os..oe] != edited_bytes[es..ee] {
            return Err(EditError::VerificationFailed {
                reason: format!("untouched line {} changed bytes", oi + 1),
            });
        }
    }

    if check_exact_end < orig_gap.end {
        let oi = orig_gap.end - 1;
        let ei = edited_start + (gap_len - 1);
        let orig_line = &orig_doc.lines[oi];
        let edited_line = &edited_doc.lines[ei];
        if orig_line.text != edited_line.text || edited_line.ending != default_ending {
            return Err(EditError::VerificationFailed {
                reason: format!(
                    "last line {} changed unexpectedly when appending at end of file",
                    oi + 1
                ),
            });
        }
    }

    Ok(())
}

fn append_inserted_lines(
    new_lines: &mut Vec<Line>,
    lines_to_insert: &[String],
    at_eof: bool,
    default_ending: LineEnding,
) {
    if lines_to_insert.is_empty() {
        return;
    }

    let unterminated_eof = at_eof
        && new_lines
            .last()
            .is_some_and(|last| last.ending == LineEnding::None);

    if unterminated_eof {
        if let Some(last) = new_lines.last_mut() {
            last.ending = default_ending;
        }
        for (idx, text) in lines_to_insert.iter().enumerate() {
            let ending = if idx + 1 == lines_to_insert.len() {
                LineEnding::None
            } else {
                default_ending
            };
            new_lines.push(Line::new(text.clone(), ending));
        }
    } else {
        for text in lines_to_insert {
            new_lines.push(Line::new(text.clone(), default_ending));
        }
    }
}

fn line_byte_spans(bytes: &[u8], has_bom: bool) -> Vec<(usize, usize)> {
    let mut pos = if has_bom { UTF8_BOM.len() } else { 0 };
    let mut spans = Vec::new();
    while pos < bytes.len() {
        let start = pos;
        match bytes[pos..].iter().position(|&b| b == b'\n') {
            Some(rel) => {
                pos += rel + 1;
            }
            None => {
                pos = bytes.len();
            }
        }
        spans.push((start, pos));
    }
    spans
}

fn validate_and_sort<'a, T>(
    items: &'a [T],
    line_count: usize,
    get_range: impl Fn(&T) -> &Range<usize>,
    get_lines: impl Fn(&T) -> &[String],
) -> Result<Vec<&'a T>, EditError> {
    for item in items {
        let range = get_range(item);
        if range.start > range.end {
            return Err(EditError::InvalidRange {
                start: range.start,
                end: range.end,
            });
        }
        if range.end > line_count {
            return Err(EditError::RangeOutOfBounds {
                start: range.start,
                end: range.end,
                line_count,
            });
        }
        for line in get_lines(item) {
            if line.contains('\n') || line.contains('\r') {
                return Err(EditError::NewlineInLine {
                    line_text: line.clone(),
                });
            }
        }
    }

    let mut sorted: Vec<&'a T> = items.iter().collect();
    sorted.sort_by_key(|item| {
        let r = get_range(item);
        (r.start, r.end)
    });

    for pair in sorted.windows(2) {
        let first = get_range(pair[0]);
        let second = get_range(pair[1]);
        if first.end > second.start {
            return Err(EditError::OverlappingEdits {
                first: first.clone(),
                second: second.clone(),
            });
        }
    }

    Ok(sorted)
}
