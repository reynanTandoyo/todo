//! `todo scan` command: find `TODO:` and `FIXME:` comments and offer them as tasks (I-11).

use std::collections::HashSet;
use std::path::Path;
use std::process::Command as ProcessCommand;

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::cli::add::{add_batch_text, BatchAdded};
use crate::cli::{CommandKind, CommandMeta, Context, load_read};
use crate::doc::IdConfig;
use crate::error::{ErrorCode, Result, TodoError};
use crate::find::fs_path;
use crate::output::Output;

/// Metadata for the `scan` command.
pub const META: CommandMeta = CommandMeta {
    name: "scan",
    summary: "Find TODO: and FIXME: comments in the project",
    kind: CommandKind::Read,
    details: "Lists `TODO:` and `FIXME:` comments in the files of the folder that holds `TODO.md`, as `path:line: text` (paths use forward slashes). With git on `PATH` and the folder inside a repository, the files are those from `git ls-files --cached --others --exclude-standard`; otherwise the folder is walked, skipping hidden folders, `target` and `node_modules`. Skips `TODO.md`, the archive, files that are not UTF-8, files over 1 MB, comments that name a task ID, and comments whose text equals an existing task title (ignoring case). `--add` adds the rest as tasks in one locked write (`files: <path>:<line>`, title cut to 120 characters); with `--dry-run` nothing is written.",
    examples: &[
        "todo scan",
        "todo scan --json",
        "todo scan --add --section Now --priority P2 --agent coordinator",
        "todo scan --add --dry-run --agent coordinator",
    ],
};

/// Arguments for `todo scan`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Add the new comments as tasks in one locked write.
    #[arg(long)]
    pub add: bool,

    /// Section for the new tasks (exact or unique substring match); with `--add`.
    #[arg(long, value_name = "NAME", requires = "add")]
    pub section: Option<String>,

    /// Priority for the new tasks; with `--add`.
    #[arg(long, value_name = "PRIORITY", requires = "add")]
    pub priority: Option<String>,
}

/// Files larger than this are not scanned (1 MB).
const MAX_FILE_BYTES: u64 = 1_000_000;

/// Titles of new tasks are cut to this many characters (`files` keeps the location).
const TITLE_MAX_CHARS: usize = 120;

/// Markers that start a comment to find.
const MARKERS: [&str; 2] = ["TODO:", "FIXME:"];

/// Folders the walk never enters (hidden folders are skipped as well).
const SKIPPED_FOLDERS: [&str; 2] = ["target", "node_modules"];

/// One `TODO:` or `FIXME:` comment that is not known yet.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Candidate {
    /// Path relative to the folder holding `TODO.md`, with forward slashes.
    path: String,
    /// One-based line number.
    line: usize,
    /// The comment from the marker to the end of the line, without closing delimiters.
    text: String,
    /// The text after the marker; it becomes the task title.
    body: String,
}

#[derive(Serialize)]
struct FoundJson<'a> {
    path: &'a str,
    line: usize,
    text: &'a str,
}

#[derive(Serialize)]
struct ScanJson<'a> {
    count: usize,
    found: Vec<FoundJson<'a>>,
    /// Task IDs given to the new tasks; present only with `--add`.
    #[serde(skip_serializing_if = "Option::is_none")]
    added: Option<Vec<String>>,
}

/// One JSON line for `add --from`.
#[derive(Serialize)]
struct BatchLine<'a> {
    title: &'a str,
    priority: Option<&'a str>,
    files: String,
}

/// Runs `todo scan [--add]`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    if args.add {
        ctx.require_agent()?;
    }
    let (cfg, loaded) = load_read(ctx, out, false, true)?;
    let root = loaded
        .found
        .path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));

    let mut known: HashSet<String> = loaded
        .doc
        .tasks
        .iter()
        .map(|t| normalize(&t.title))
        .collect();
    if let Some(archive) = &loaded.archive {
        known.extend(archive.tasks.iter().map(|t| normalize(&t.title)));
    }

    let mut found = Vec::new();
    for rel in list_files(root, out) {
        if is_task_file(&rel, &cfg.archive_file) {
            continue;
        }
        let Some(content) = read_scannable(&root.join(&rel)) else {
            continue;
        };
        for (idx, line) in content.lines().enumerate() {
            let Some(comment) = marker_comment(line) else {
                continue;
            };
            if is_known(line, &comment, &known, &cfg.id_config) {
                continue;
            }
            found.push(Candidate {
                path: rel.clone(),
                line: idx + 1,
                text: comment.text.to_string(),
                body: comment.body.to_string(),
            });
        }
    }

    let picked = if args.add {
        pick_new(&found, out)
    } else {
        Vec::new()
    };
    let added = if args.add {
        let batch = if picked.is_empty() {
            BatchAdded {
                ids: Vec::new(),
                added: Vec::new(),
            }
        } else {
            let raw = batch_lines(&picked, args.priority.as_deref())?;
            add_batch_text(&raw, args.section.as_deref(), ctx, out)?
        };
        Some(batch)
    } else {
        None
    };

    if out.is_json() {
        let json = ScanJson {
            count: found.len(),
            found: found
                .iter()
                .map(|c| FoundJson {
                    path: &c.path,
                    line: c.line,
                    text: &c.text,
                })
                .collect(),
            added: added.as_ref().map(|a| a.ids.clone()),
        };
        out.emit_json_ok(META.name, &json);
        return Ok(());
    }

    if found.is_empty() {
        out.write_line("no new TODO: or FIXME: comments found");
        return Ok(());
    }
    match added {
        None => {
            for c in &found {
                out.write_line(&format!("{}:{}: {}", c.path, c.line, c.text));
            }
        }
        Some(batch) => {
            // The batch gives IDs in the order of the picked candidates.
            for (id, c) in batch.ids.iter().zip(picked) {
                out.write_line(&format!("{id}: add ok  {}:{}", c.path, c.line));
            }
        }
    }
    Ok(())
}

/// Chooses the candidates to add: the first comment of each title (ignoring case).
///
/// A later comment with the same title is skipped with a warning, because `add --from` refuses
/// a batch that repeats a title.
fn pick_new<'a>(found: &'a [Candidate], out: &mut Output) -> Vec<&'a Candidate> {
    let mut seen = HashSet::new();
    let mut picked = Vec::new();
    for c in found {
        if seen.insert(normalize(&title_of(&c.body))) {
            picked.push(c);
        } else {
            out.warn(format!(
                "{}:{}: skipped, a comment earlier in this scan has the same title",
                c.path, c.line
            ));
        }
    }
    picked
}

/// The JSON lines for `add --from` describing the candidates.
fn batch_lines(picked: &[&Candidate], priority: Option<&str>) -> Result<String> {
    let mut lines = Vec::with_capacity(picked.len());
    for c in picked {
        let entry = BatchLine {
            title: &title_of(&c.body),
            priority,
            files: format!("{}:{}", c.path, c.line),
        };
        let line = serde_json::to_string(&entry).map_err(|e| {
            TodoError::new(
                ErrorCode::BadValue,
                format!("could not build the task for {}:{}: {e}", c.path, c.line),
            )
        })?;
        lines.push(line);
    }
    Ok(lines.join("\n"))
}

/// The task title for a comment: its text, cut to [`TITLE_MAX_CHARS`] characters.
fn title_of(text: &str) -> String {
    text.chars().take(TITLE_MAX_CHARS).collect::<String>().trim_end().to_string()
}

/// Lower-cased, trimmed text for comparisons.
fn normalize(text: &str) -> String {
    text.trim().to_lowercase()
}

/// Whether `rel` is `TODO.md` or the archive file in the folder holding `TODO.md`.
fn is_task_file(rel: &str, archive_file: &str) -> bool {
    rel.eq_ignore_ascii_case("TODO.md") || rel.eq_ignore_ascii_case(archive_file)
}

/// A comment starting with a marker: the text from the marker on, and the text after the marker.
struct Comment<'a> {
    text: &'a str,
    body: &'a str,
}

/// Finds the first `TODO:` or `FIXME:` that is not part of a longer word.
///
/// Returns `None` when there is no marker or nothing follows it.
fn marker_comment(line: &str) -> Option<Comment<'_>> {
    let start = MARKERS
        .iter()
        .flat_map(|m| line.match_indices(m).map(|(i, _)| i))
        .filter(|&i| line[..i].chars().next_back().is_none_or(|c| !is_word_char(c)))
        .min()?;
    let text = strip_closers(&line[start..]);
    // The first colon is the marker's own colon.
    let body = text.split_once(':')?.1.trim();
    if body.is_empty() {
        return None;
    }
    Some(Comment { text, body })
}

/// Removes trailing `*/`, `-->` and `#}` delimiters and whitespace, repeatedly.
fn strip_closers(text: &str) -> &str {
    let mut rest = text.trim();
    loop {
        let stripped = rest
            .strip_suffix("*/")
            .or_else(|| rest.strip_suffix("-->"))
            .or_else(|| rest.strip_suffix("#}"));
        match stripped {
            Some(s) => rest = s.trim_end(),
            None => return rest,
        }
    }
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether the comment is already tracked: the line names a task ID, or the comment text or its
/// body equals a task title (ignoring case). Titles are compared as `--add` stores them, cut to
/// [`TITLE_MAX_CHARS`], so a comment added earlier is not offered again.
fn is_known(line: &str, comment: &Comment<'_>, known: &HashSet<String>, ids: &IdConfig) -> bool {
    names_task_id(line, &ids.prefix)
        || [comment.text, comment.body]
            .iter()
            .any(|t| known.contains(&normalize(t)) || known.contains(&normalize(&title_of(t))))
}

/// Whether `line` contains a task ID such as `T-081` (prefix ignoring case, digits after `-`).
fn names_task_id(line: &str, prefix: &str) -> bool {
    // ASCII lower-casing keeps byte offsets identical to `line`.
    let lower = line.to_ascii_lowercase();
    let pattern = format!("{}-", prefix.to_ascii_lowercase());
    for (i, _) in lower.match_indices(&pattern) {
        let digits_start = i + pattern.len();
        let digits = lower[digits_start..]
            .bytes()
            .take_while(u8::is_ascii_digit)
            .count();
        let before_ok = line[..i].chars().next_back().is_none_or(|c| !is_word_char(c));
        let after_ok = lower[digits_start + digits..]
            .chars()
            .next()
            .is_none_or(|c| !is_word_char(c));
        if before_ok && digits > 0 && after_ok {
            return true;
        }
    }
    false
}

/// The text of a file to scan, or `None` when it is too large, not a regular file or not UTF-8.
fn read_scannable(path: &Path) -> Option<String> {
    let p = fs_path(path);
    let meta = std::fs::metadata(&p).ok()?;
    if !meta.is_file() || meta.len() > MAX_FILE_BYTES {
        return None;
    }
    let bytes = std::fs::read(&p).ok()?;
    String::from_utf8(bytes).ok()
}

/// Relative paths (forward slashes, sorted) of the files to scan under `root`.
///
/// Uses git when it is on `PATH` and `root` is inside a repository (NFR-5); otherwise walks the
/// folder. Problems reading a folder become warnings.
fn list_files(root: &Path, out: &mut Output) -> Vec<String> {
    if let Some(files) = git_files(root) {
        return files;
    }
    let mut files = Vec::new();
    walk(root, "", &mut files, out);
    files.sort();
    files
}

/// Files git knows about in `root` (tracked, plus untracked and not ignored), or `None` when git
/// is not available or `root` is not in a repository.
fn git_files(root: &Path) -> Option<Vec<String>> {
    // Not on PATH (or no usable repository) means the folder walk is used instead.
    let output = ProcessCommand::new("git")
        .current_dir(fs_path(root).as_ref())
        .args(["ls-files", "-z", "--cached", "--others", "--exclude-standard"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let mut files: Vec<String> = output
        .stdout
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .filter_map(|s| String::from_utf8(s.to_vec()).ok())
        .collect();
    files.sort();
    files.dedup();
    Some(files)
}

/// Adds the files under `dir` to `files`, with paths relative to the root (`prefix` is `dir`'s
/// relative path). Skips hidden folders, `target`, `node_modules` and links.
fn walk(dir: &Path, prefix: &str, files: &mut Vec<String>, out: &mut Output) {
    let entries = match std::fs::read_dir(fs_path(dir).as_ref()) {
        Ok(entries) => entries,
        Err(e) => {
            out.warn(format!("could not read folder {}: {e}", dir.display()));
            return;
        }
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        let rel = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        if file_type.is_dir() {
            if name.starts_with('.') || SKIPPED_FOLDERS.contains(&name.as_str()) {
                continue;
            }
            walk(&entry.path(), &rel, files, out);
        } else if file_type.is_file() {
            files.push(rel);
        }
    }
}
