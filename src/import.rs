//! Conversion of Markdown checkbox task lists into `TODO.md` format version 1 (`R-IMP-1`).
//!
//! [`convert_checkbox`] is a pure function: bytes in, bytes out. It never touches the disk; the
//! command in [`crate::cli::import`] does the reading, the preview and the writing.
//!
//! Only whole lines change. Every source line that is not a converted checkbox line (or one of its
//! `YYYY-MM-DD agent: text` sub-bullets) is copied with its own line ending; the converter only
//! adds the format line at the top, the new task lines, and a blank line before a task heading.

use chrono::{DateTime, FixedOffset, NaiveDate};
use serde::Serialize;
use serde_json::json;

use crate::cli::check::{CheckProblem, check_dependencies, check_tasks};
use crate::doc::{
    Document, Line, LineDoc, LineEnding, Problem, Status, TaskId, is_valid_agent_name,
    sanitize_trimmed_value,
};
use crate::error::{ErrorCode, ExitCode, Result, TodoError};
use crate::ops::OpsConfig;

/// What the converter needs besides the source bytes.
#[derive(Debug, Clone)]
pub struct ImportEnv<'a> {
    /// Settings of the target folder (ID prefix and width, priorities, claim lifetime).
    pub cfg: &'a OpsConfig,
    /// Agent written on notes and claims that the source does not attribute (default `import`).
    pub agent: &'a str,
    /// The moment of the import: the date of fallback notes and the time of imported claims.
    pub now: DateTime<FixedOffset>,
    /// File name used in `todo check` problem messages.
    pub label: &'a str,
}

/// One task produced by [`convert_checkbox`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConvertedTask {
    /// Canonical ID, such as `T-001`.
    pub id: String,
    /// Task title.
    pub title: String,
    /// Status written: `open`, `claimed`, `blocked` or `done`.
    pub status: &'static str,
    /// 1-based line of the checkbox line in the source.
    pub source_line: usize,
}

/// The result of a successful conversion.
#[derive(Debug, Clone)]
pub struct Converted {
    /// The converted file.
    pub bytes: Vec<u8>,
    /// Number of top-level checkbox lines under `##` sections that were converted.
    pub checkbox_lines: usize,
    /// Number of tasks found when the converted file is parsed again (equal to `checkbox_lines`).
    pub tasks: usize,
    /// The tasks, in file order.
    pub converted: Vec<ConvertedTask>,
}

struct Suffixes {
    added: Option<(String, String)>,
    claimed: Option<(String, String)>,
    done: Option<(String, String, Option<String>)>,
}

enum Tail {
    Note(String),
    Text(Line),
}

struct Builder {
    id: String,
    mark: char,
    title: String,
    description: Option<String>,
    suffixes: Suffixes,
    question: bool,
    source_line: usize,
    tail: Vec<Tail>,
}

/// Converts a Markdown checkbox list to format version 1, then proves the result with the same
/// rules as `todo check` and by comparing the task count with the number of converted lines.
///
/// Claimed tasks get the time of the import as their claim time (a claim dated in the past would
/// be stale and fail `todo check`); the original date stays in a note.
///
/// Returns `BAD_VALUE` (exit 1) when nothing could be converted, the counts differ, or the result
/// has problems; `UNSAFE_FILE` (exit 4) when the source is not valid UTF-8 or can't be written
/// safely (unclosed code fence, conflict markers, a newer format version).
pub fn convert_checkbox(source: &[u8], env: &ImportEnv<'_>) -> Result<Converted> {
    let src = LineDoc::from_bytes(source).map_err(|e| {
        TodoError::new(
            ErrorCode::UnsafeFile,
            format!("{}:{}: not valid UTF-8", env.label, e.line),
        )
    })?;
    let src_doc = Document::from_line_doc(src.clone(), env.cfg.id_config.clone());
    if let Some(p) = src_doc
        .problems
        .iter()
        .find(|p| Problem::blocks_all_writes(p))
    {
        return Err(TodoError::new(
            ErrorCode::UnsafeFile,
            format!("{}:{}: {}", env.label, p.line, p.message),
        ));
    }

    let ending = src.default_ending();
    let mut out: Vec<Line> = Vec::with_capacity(src.len() + 16);
    let mut region = Region::Normal;
    let mut in_section = false;
    let mut section_is_question = false;
    let mut current: Option<Builder> = None;
    let mut converted: Vec<ConvertedTask> = Vec::new();
    let mut next_number: u64 = 0;

    for (index, line) in src.lines.iter().enumerate() {
        if region.advance(&line.text) {
            flush(&mut current, &mut out, &mut converted, env, ending);
            out.push(line.clone());
            continue;
        }
        if let Some(name) = line.text.strip_prefix("## ") {
            flush(&mut current, &mut out, &mut converted, env, ending);
            in_section = true;
            let lower = name.to_lowercase();
            section_is_question = lower.contains("decision") || lower.contains("question");
            out.push(line.clone());
            continue;
        }
        if in_section && let Some((mark, body)) = parse_checkbox(&line.text) {
            flush(&mut current, &mut out, &mut converted, env, ending);
            next_number += 1;
            let (rest, suffixes) = split_suffixes(body);
            let (title, description) = split_title(rest);
            current = Some(Builder {
                id: env.cfg.id_config.format_id(TaskId(next_number)),
                mark,
                title,
                description,
                suffixes,
                question: section_is_question,
                source_line: index + 1,
                tail: Vec::new(),
            });
            continue;
        }
        if let Some(b) = current.as_mut()
            && line.text.starts_with([' ', '\t'])
            && !line.text.trim().is_empty()
        {
            match parse_sub_note(&line.text) {
                Some(note) => b.tail.push(Tail::Note(note)),
                None => b.tail.push(Tail::Text(line.clone())),
            }
            continue;
        }
        flush(&mut current, &mut out, &mut converted, env, ending);
        out.push(line.clone());
    }
    flush(&mut current, &mut out, &mut converted, env, ending);

    if converted.is_empty() {
        return Err(TodoError::new(
            ErrorCode::BadValue,
            format!(
                "{}: no checkbox tasks found: expected top-level `- [ ]`, `- [~]`, `- [!]` or \
                 `- [x]` lines under `## ` sections (checkboxes in the preamble, in code fences \
                 or indented stay text)",
                env.label
            ),
        ));
    }

    if src_doc.declared_format_version.is_none() {
        out.insert(0, Line::new("<!-- todo-format: 1 -->", ending));
    }
    if src
        .lines
        .last()
        .is_some_and(|l| l.ending == LineEnding::None)
        && let Some(last) = out.last_mut()
    {
        last.ending = LineEnding::None;
    }
    let bytes = LineDoc {
        has_bom: src.has_bom,
        lines: out,
    }
    .to_bytes();

    let checkbox_lines = converted.len();
    let doc =
        Document::parse_bytes_with_config(&bytes, env.cfg.id_config.clone()).map_err(|e| {
            TodoError::new(
                ErrorCode::UnsafeFile,
                format!("converted text is invalid: {e}"),
            )
        })?;
    let mut problems: Vec<CheckProblem> = doc
        .problems
        .iter()
        .map(|p| CheckProblem {
            file: env.label.to_string(),
            line: p.line,
            message: p.message.clone(),
        })
        .collect();
    check_tasks(&doc, None, env.cfg, env.label, env.now, &mut problems);
    check_dependencies(&doc, None, env.label, &mut problems);
    if doc.tasks.len() != checkbox_lines {
        problems.push(CheckProblem {
            file: env.label.to_string(),
            line: 1,
            message: format!(
                "converted {checkbox_lines} checkbox lines but the result has {} tasks",
                doc.tasks.len()
            ),
        });
    }
    if let Some(first) = problems.first() {
        return Err(TodoError::new(
            ErrorCode::BadValue,
            format!(
                "the converted text fails `todo check` ({} problem(s)); first: {}:{}: {}",
                problems.len(),
                first.file,
                first.line,
                first.message
            ),
        )
        .with_exit_code(ExitCode::Refused)
        .with_detail("problems", json!(problems)));
    }

    Ok(Converted {
        bytes,
        checkbox_lines,
        tasks: doc.tasks.len(),
        converted,
    })
}

/// Tracks fenced code blocks and HTML comments (`docs/format.md` §3).
#[derive(Clone, Copy)]
enum Region {
    Normal,
    Fence { ch: char, len: usize },
    Comment,
}

impl Region {
    /// Updates the state with `line`; returns `true` when the line belongs to an ignored region
    /// (including the lines that open or close it).
    fn advance(&mut self, line: &str) -> bool {
        match *self {
            Self::Fence { ch, len } => {
                let t = line.trim_end_matches(' ');
                let body = t.trim_start_matches(' ');
                if t.len() - body.len() <= 3 && body.len() >= len && body.chars().all(|c| c == ch) {
                    *self = Self::Normal;
                }
                true
            }
            Self::Comment => {
                if line.contains("-->") {
                    *self = Self::Normal;
                }
                true
            }
            Self::Normal => {
                let body = line.trim_start_matches(' ');
                if line.len() - body.len() > 3 {
                    return false;
                }
                for ch in ['`', '~'] {
                    let n = body.chars().take_while(|&c| c == ch).count();
                    if n >= 3 {
                        *self = Self::Fence { ch, len: n };
                        return true;
                    }
                }
                if let Some(after) = body.strip_prefix("<!--") {
                    if !after.contains("-->") {
                        *self = Self::Comment;
                    }
                    return true;
                }
                false
            }
        }
    }
}

/// Splits `- [m] body` into the mark and the body; the mark must be ` `, `~`, `!`, `x` or `X`.
fn parse_checkbox(line: &str) -> Option<(char, &str)> {
    let rest = line.strip_prefix("- [")?;
    let mut chars = rest.chars();
    let mark = chars.next()?;
    if !matches!(mark, ' ' | '~' | '!' | 'x' | 'X') {
        return None;
    }
    let rest = chars.as_str().strip_prefix(']')?;
    if rest.is_empty() {
        return Some((mark, ""));
    }
    Some((mark, rest.strip_prefix(' ')?.trim()))
}

fn valid_date(s: &str) -> bool {
    s.len() == 10 && NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok()
}

type Suffix = (&'static str, String, String, Option<String>);

/// Parses `added: agent, date`, `claimed: agent, date` or `done: agent, date[, commit]`.
fn parse_suffix(inner: &str) -> Option<Suffix> {
    let (kind, args) = inner.split_once(':')?;
    let kind = match kind.trim() {
        "added" => "added",
        "claimed" => "claimed",
        "done" => "done",
        _ => return None,
    };
    let parts: Vec<&str> = args.split(',').map(str::trim).collect();
    if parts.len() < 2 || !is_valid_agent_name(parts[0]) || !valid_date(parts[1]) {
        return None;
    }
    let commit = if parts.len() == 2 {
        None
    } else if kind == "done" {
        Some(parts[2..].join(", ")).filter(|c| !c.is_empty())
    } else {
        return None;
    };
    Some((kind, parts[0].to_string(), parts[1].to_string(), commit))
}

/// Removes trailing `(added: …)`, `(claimed: …)` and `(done: …)` groups from `body`.
fn split_suffixes(body: &str) -> (&str, Suffixes) {
    let mut s = Suffixes {
        added: None,
        claimed: None,
        done: None,
    };
    let mut rest = body.trim_end();
    while rest.ends_with(')')
        && let Some(open) = rest.rfind('(')
        && let Some((kind, agent, date, commit)) = parse_suffix(&rest[open + 1..rest.len() - 1])
    {
        let repeated = match kind {
            "added" => s.added.replace((agent, date)).is_some(),
            "claimed" => s.claimed.replace((agent, date)).is_some(),
            _ => s.done.replace((agent, date, commit)).is_some(),
        };
        if repeated {
            break;
        }
        rest = rest[..open].trim_end();
    }
    (rest, s)
}

/// Title and description of a checkbox body: the bold part, else the text before ` — `.
fn split_title(body: &str) -> (String, Option<String>) {
    let body = body.trim();
    if let Some(start) = body.find("**")
        && let Some(len) = body[start + 2..].find("**")
    {
        let title = body[start + 2..start + 2 + len].trim();
        if !title.is_empty() {
            let before = body[..start].trim();
            let after = body[start + 2 + len + 2..]
                .trim_start_matches(|c: char| {
                    c.is_whitespace() || matches!(c, '—' | '–' | '-' | ':')
                })
                .trim();
            let description = match (before.is_empty(), after.is_empty()) {
                (true, true) => None,
                (true, false) => Some(after.to_string()),
                (false, true) => Some(before.to_string()),
                (false, false) => Some(format!("{before} {after}")),
            };
            return (
                sanitize_trimmed_value(title),
                description.map(|d| sanitize_trimmed_value(&d)),
            );
        }
    }
    match body.split_once(" — ") {
        Some((title, rest)) if !title.trim().is_empty() => {
            let rest = rest.trim();
            (
                sanitize_trimmed_value(title),
                (!rest.is_empty()).then(|| sanitize_trimmed_value(rest)),
            )
        }
        _ if body.is_empty() => ("(untitled)".to_string(), None),
        _ => (sanitize_trimmed_value(body), None),
    }
}

/// Turns `  - YYYY-MM-DD agent: text` into a note line.
fn parse_sub_note(line: &str) -> Option<String> {
    let rest = line.trim_start().strip_prefix("- ")?;
    let (date, rest) = rest.split_once(' ')?;
    let (agent, text) = rest.split_once(": ")?;
    let text = text.trim();
    if !valid_date(date) || !is_valid_agent_name(agent) || text.is_empty() {
        return None;
    }
    Some(format!("- note ({date}, {agent}): {text}"))
}

/// Writes the task being built (heading, fields, notes, kept indented lines) and records it.
fn flush(
    current: &mut Option<Builder>,
    out: &mut Vec<Line>,
    converted: &mut Vec<ConvertedTask>,
    env: &ImportEnv<'_>,
    ending: LineEnding,
) {
    let Some(b) = current.take() else {
        return;
    };
    let status = match b.mark {
        '~' => Status::Claimed,
        '!' => Status::Blocked,
        'x' | 'X' => Status::Done,
        _ => Status::Open,
    };
    let status_text = match status {
        Status::Claimed => "claimed",
        Status::Blocked => "blocked",
        Status::Done => "done",
        _ => "open",
    };
    let mut lines: Vec<String> = vec![
        format!("### {} · {}", b.id, b.title),
        format!("- status: {status_text}"),
    ];
    let mut notes: Vec<String> = Vec::new();
    if let Some((agent, date)) = &b.suffixes.added {
        notes.push(format!("- note ({date}, {agent}): added"));
    }
    if status == Status::Claimed {
        let (owner, claim_date) = match &b.suffixes.claimed {
            Some((agent, date)) => (agent.as_str(), Some(date.as_str())),
            None => (env.agent, None),
        };
        lines.push(format!("- owner: {owner}"));
        lines.push(format!(
            "- claimed: {}",
            env.now.format("%Y-%m-%dT%H:%M%:z")
        ));
        if let Some(date) = claim_date {
            notes.push(format!(
                "- note ({date}, {owner}): claimed — imported from a checkbox list; claim time is the import time"
            ));
        }
    } else {
        lines.push("- owner: —".to_string());
    }
    if b.question {
        lines.push("- type: question".to_string());
    }
    if let Some((agent, date, commit)) = &b.suffixes.done {
        notes.push(format!("- note ({date}, {agent}): done"));
        if let Some(commit) = commit {
            lines.push(format!("- commit: {}", sanitize_trimmed_value(commit)));
        }
    }
    if let Some(d) = &b.description {
        lines.push(format!("- description: {d}"));
    }
    let has_sub_notes = b.tail.iter().any(|t| matches!(t, Tail::Note(_)));
    if status == Status::Blocked && notes.is_empty() && !has_sub_notes {
        notes.push(format!(
            "- note ({}, {}): blocked — imported from a checkbox list",
            env.now.format("%Y-%m-%d"),
            env.agent
        ));
    }

    if out.last().is_some_and(|l| !l.text.trim().is_empty()) {
        out.push(Line::new("", ending));
    }
    for text in lines.into_iter().chain(notes) {
        out.push(Line::new(text, ending));
    }
    for t in b.tail {
        match t {
            Tail::Note(n) => out.push(Line::new(n, ending)),
            Tail::Text(l) => out.push(l),
        }
    }
    converted.push(ConvertedTask {
        id: b.id,
        title: b.title,
        status: status_text,
        source_line: b.source_line,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_is_the_bold_part_else_the_text_before_the_dash() {
        assert_eq!(
            split_title("**Fix it** — click runs twice"),
            ("Fix it".into(), Some("click runs twice".into()))
        );
        assert_eq!(
            split_title("Do this — because"),
            ("Do this".into(), Some("because".into()))
        );
        assert_eq!(split_title("Just text"), ("Just text".into(), None));
        assert_eq!(split_title("**Only bold**"), ("Only bold".into(), None));
        assert_eq!(
            split_title("Big **title** rest"),
            ("title".into(), Some("Big rest".into()))
        );
        assert_eq!(split_title(""), ("(untitled)".into(), None));
    }

    #[test]
    fn suffixes_are_removed_from_the_end_only() {
        let (rest, s) =
            split_suffixes("Text (note) (added: a, 2026-11-01) (done: b, 2026-11-02, abc, def)");
        assert_eq!(rest, "Text (note)");
        assert_eq!(s.added, Some(("a".into(), "2026-11-01".into())));
        assert_eq!(
            s.done,
            Some(("b".into(), "2026-11-02".into(), Some("abc, def".into())))
        );
        // Not a suffix: bad date, bad kind, claimed with a commit.
        for text in [
            "x (added: a, 2026-13-01)",
            "x (fixed: a, 2026-11-01)",
            "x (claimed: a, 2026-11-01, abc)",
        ] {
            assert_eq!(split_suffixes(text).0, text);
        }
    }

    #[test]
    fn checkbox_marks_are_limited_to_the_four_markers() {
        assert_eq!(parse_checkbox("- [ ] a"), Some((' ', "a")));
        assert_eq!(parse_checkbox("- [~] a"), Some(('~', "a")));
        assert_eq!(parse_checkbox("- [!] a"), Some(('!', "a")));
        assert_eq!(parse_checkbox("- [x] a"), Some(('x', "a")));
        assert_eq!(parse_checkbox("- [?] a"), None);
        assert_eq!(parse_checkbox("- [x]a"), None);
        assert_eq!(parse_checkbox("  - [ ] a"), None);
        assert_eq!(parse_checkbox("* [ ] a"), None);
    }

    #[test]
    fn sub_notes_need_date_agent_and_text() {
        assert_eq!(
            parse_sub_note("  - 2026-11-11 bob: did it"),
            Some("- note (2026-11-11, bob): did it".into())
        );
        assert_eq!(parse_sub_note("  - see also"), None);
        assert_eq!(parse_sub_note("  - 2026-11-11 bob:"), None);
    }
}
