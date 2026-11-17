//! `todo merge-driver`: a git merge driver that merges `TODO.md` task by task (`R-GIT-2`,
//! `R-GIT-3`).
//!
//! # How a merge works
//!
//! 1. The three inputs (`base`, `ours`, `theirs`) are parsed. If any of them cannot be trusted
//!    (not UTF-8, conflict markers, an open code fence, a repeated task ID, a repeated field,
//!    different or unknown format versions) the whole merge falls back to `git merge-file`.
//! 2. Every input is turned into a *skeleton*: its lines, with each task block replaced by one
//!    placeholder line that names the task ID. `git merge-file` merges the three skeletons, which
//!    decides section order, prose, and which tasks exist. A conflict between runs that hold only
//!    placeholders and blank lines (two sides added different tasks at the same spot) is resolved
//!    by keeping both sides, ours first. Any other conflict falls back to `git merge-file` on the
//!    original files.
//! 3. Each placeholder is replaced by the merged block of that task (see [`merge_block`]).
//!
//! # Judgement calls
//!
//! - Task IDs are recognised with the default prefix `T` (the driver does not read `todo.toml`).
//!   With another prefix no task is recognised and everything is merged as prose by git.
//! - A field is a whole `- key: value` line, so the combined priority line is one field.
//! - Notes are never deleted: the result is ours' notes followed by the notes only theirs has,
//!   except notes that were in the base and are missing on one side (those stay removed).
//! - Other lines inside a task block (prose, tables) are merged by `git merge-file` on the block
//!   when the two sides differ in them; a conflict there makes the whole block a conflict.
//! - Equality of lines ignores a missing final line ending, so a file whose last line had no
//!   newline in the base does not look changed on that account.
//! - A line without ending that is followed by another line gets the ending of the line before
//!   it, or LF. This is the only byte not taken from an input.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::sync::atomic::{AtomicU64, Ordering};

use clap::{Args as ClapArgs, Subcommand};
use serde::Serialize;

use crate::atomic::atomic_write_bytes;
use crate::cli::{CommandKind, CommandMeta, Context};
use crate::doc::{
    Document, Line, LineDoc, LineEnding, ProblemSeverity, SUPPORTED_FORMAT_VERSION, Task, TaskId,
};
use crate::error::{ErrorCode, Result, TodoError};
use crate::output::Output;

const DRIVER_NAME: &str = "todo task-level merge for TODO.md";
const DRIVER_COMMAND: &str = "todo merge-driver run %O %A %B %P";
const ATTR: &str = "merge=todo";
const DEFAULT_PATH: &str = "TODO.md";
const PLACEHOLDER_PREFIX: &str = "\u{1}todo-task:";

/// Metadata for the `merge-driver` command.
pub const META: CommandMeta = CommandMeta {
    name: "merge-driver",
    summary: "Git merge driver that merges TODO.md task by task",
    kind: CommandKind::Meta,
    details: "`merge-driver install` sets `merge.todo.name` and `merge.todo.driver` in this repository's git config and adds `TODO.md merge=todo` to `.gitattributes` (other lines stay as they are); `merge-driver uninstall` removes them; both can be repeated safely. `merge-driver run <base> <ours> <theirs> [<path>]` is what git calls: tasks changed on different sides merge cleanly, notes added on both sides are all kept (ours first), and a task whose same field changed differently on both sides, or that was added with the same ID on both sides, is written with `<<<<<<<` / `=======` / `>>>>>>>` markers around the whole task block and the command exits 1. Prose conflicts, unreadable input and different format versions are left to `git merge-file`. The result is written to <ours>. Needs `todo` on PATH when git runs it.",
    examples: &[
        "todo merge-driver install",
        "todo merge-driver install --path docs/TODO.md",
        "todo merge-driver uninstall",
        "todo merge-driver run base.md ours.md theirs.md TODO.md",
    ],
};

/// Arguments for `todo merge-driver`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// What to do.
    #[command(subcommand)]
    pub action: Action,
}

/// Subcommands under `todo merge-driver`.
#[derive(Debug, Subcommand)]
pub enum Action {
    /// Register the driver in git config and `.gitattributes`.
    Install(PathArgs),
    /// Remove the driver from git config and `.gitattributes`.
    Uninstall(PathArgs),
    /// Merge three versions of `TODO.md` (called by git with `%O %A %B %P`).
    Run(RunArgs),
}

/// The `.gitattributes` pattern to manage.
#[derive(Debug, ClapArgs)]
pub struct PathArgs {
    /// File pattern written to `.gitattributes`.
    #[arg(long, value_name = "PATH", default_value = DEFAULT_PATH)]
    pub path: String,
}

/// Arguments of `merge-driver run`.
#[derive(Debug, ClapArgs)]
pub struct RunArgs {
    /// Common ancestor version (`%O`).
    pub base: PathBuf,
    /// Our version (`%A`); the merge result is written here.
    pub ours: PathBuf,
    /// Their version (`%B`).
    pub theirs: PathBuf,
    /// Path of the file in the repository (`%P`); only informational.
    pub path: Option<String>,
}

#[derive(Serialize)]
struct InstallJson {
    action: &'static str,
    changed: bool,
    attributes_file: String,
    path: String,
}

#[derive(Serialize)]
struct RunJson {
    clean: bool,
    conflicts: usize,
    fallback: bool,
}

/// Runs `todo merge-driver`.
pub fn run(args: Args, _ctx: &Context, out: &mut Output) -> Result<()> {
    match args.action {
        Action::Install(a) => install(&a.path, out),
        Action::Uninstall(a) => uninstall(&a.path, out),
        Action::Run(a) => run_merge(&a, out),
    }
}

// ---------------------------------------------------------------------------------------------
// merge-driver run
// ---------------------------------------------------------------------------------------------

struct Merged {
    bytes: Vec<u8>,
    conflicts: usize,
    fallback: bool,
}

fn io_err(what: &str, path: &Path, e: &std::io::Error) -> TodoError {
    TodoError::new(
        ErrorCode::WriteFailed,
        format!("{what} {}: {e}", path.display()),
    )
}

fn run_merge(args: &RunArgs, out: &mut Output) -> Result<()> {
    let read = |p: &Path| fs::read(p).map_err(|e| io_err("could not read", p, &e));
    let base = read(&args.base)?;
    let ours = read(&args.ours)?;
    let theirs = read(&args.theirs)?;

    let merged = match structured_merge(&base, &ours, &theirs)? {
        Some(m) => m,
        None => {
            let (bytes, conflicts) = git_merge_file(&base, &ours, &theirs, "ours", "base", "theirs")?;
            Merged {
                bytes,
                conflicts,
                fallback: true,
            }
        }
    };
    fs::write(&args.ours, &merged.bytes).map_err(|e| io_err("could not write", &args.ours, &e))?;

    let label = args.path.as_deref().unwrap_or(DEFAULT_PATH);
    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &RunJson {
                clean: merged.conflicts == 0,
                conflicts: merged.conflicts,
                fallback: merged.fallback,
            },
        );
    }
    if merged.conflicts > 0 {
        return Err(TodoError::new(
            ErrorCode::BadValue,
            format!(
                "{label}: {} merge conflict(s) left in the file; resolve them and commit",
                merged.conflicts
            ),
        ));
    }
    Ok(())
}

/// Parses one input; `None` when it cannot be merged task by task.
fn parse_side(bytes: &[u8]) -> Option<Document> {
    let doc = Document::parse_bytes(bytes).ok()?;
    if doc
        .problems
        .iter()
        .any(|p| !matches!(p.severity, ProblemSeverity::Warning))
    {
        return None;
    }
    let mut seen = HashSet::new();
    if !doc.tasks.iter().all(|t| seen.insert(t.id)) {
        return None;
    }
    if doc.declared_format_version.is_some_and(|v| v != SUPPORTED_FORMAT_VERSION) {
        return None;
    }
    Some(doc)
}

fn same(a: &Line, b: &Line) -> bool {
    a.text == b.text
        && (a.ending == b.ending || a.ending == LineEnding::None || b.ending == LineEnding::None)
}

fn same_lines(a: &[Line], b: &[Line]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| same(x, y))
}

fn same_opt(a: Option<&Line>, b: Option<&Line>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => same(x, y),
        _ => false,
    }
}

fn is_placeholder(line: &Line) -> Option<u64> {
    line.text.strip_prefix(PLACEHOLDER_PREFIX)?.parse().ok()
}

/// The lines of `doc` with each task block replaced by one placeholder line.
fn skeleton(doc: &Document) -> Vec<Line> {
    let lines = &doc.line_doc.lines;
    let mut starts: HashMap<usize, &Task> = HashMap::new();
    for t in &doc.tasks {
        starts.insert(t.block_range.start, t);
    }
    let mut out = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        if let Some(t) = starts.get(&i) {
            out.push(Line::new(
                format!("{PLACEHOLDER_PREFIX}{}", t.id.number()),
                LineEnding::Lf,
            ));
            i = t.block_range.end.max(i + 1);
        } else {
            out.push(lines[i].clone());
            i += 1;
        }
    }
    out
}

fn lines_to_bytes(lines: &[Line]) -> Vec<u8> {
    LineDoc {
        has_bom: false,
        lines: lines.to_vec(),
    }
    .to_bytes()
}

/// Merges task by task; `None` means "use `git merge-file` on the whole files".
fn structured_merge(base: &[u8], ours: &[u8], theirs: &[u8]) -> Result<Option<Merged>> {
    let (Some(b), Some(o), Some(t)) = (parse_side(base), parse_side(ours), parse_side(theirs))
    else {
        return Ok(None);
    };
    if b.effective_format_version() != o.effective_format_version()
        || b.effective_format_version() != t.effective_format_version()
    {
        return Ok(None);
    }

    let (sb, so, st) = (skeleton(&b), skeleton(&o), skeleton(&t));
    let (merged_skel, skel_conflicts) = git_merge_file(
        &lines_to_bytes(&sb),
        &lines_to_bytes(&so),
        &lines_to_bytes(&st),
        "todo-ours",
        "todo-base",
        "todo-theirs",
    )?;
    let _ = skel_conflicts;
    let Ok(skel_doc) = LineDoc::from_bytes(&merged_skel) else {
        return Ok(None);
    };
    let Some(skel_lines) = resolve_skeleton_conflicts(&skel_doc.lines) else {
        return Ok(None);
    };

    let mut out: Vec<Line> = Vec::new();
    let mut seen: BTreeSet<u64> = BTreeSet::new();
    let mut conflicts = 0usize;
    for line in &skel_lines {
        let Some(n) = is_placeholder(line) else {
            out.push(line.clone());
            continue;
        };
        if !seen.insert(n) {
            return Ok(None);
        }
        let id = TaskId(n);
        match merge_block(block_of(&b, id), block_of(&o, id), block_of(&t, id))? {
            BlockOut::Lines(ls) => out.extend(ls),
            BlockOut::Removed => {}
            BlockOut::Conflict { ours, theirs } => {
                conflicts += 1;
                push_conflict(&mut out, &ours, &theirs);
            }
        }
    }
    fix_endings(&mut out);
    let doc = LineDoc {
        has_bom: o.line_doc.has_bom,
        lines: out,
    };
    Ok(Some(Merged {
        bytes: doc.to_bytes(),
        conflicts,
        fallback: false,
    }))
}

/// Replaces skeleton conflict hunks made only of placeholders and blank lines by both sides
/// (ours first, a placeholder already present in ours not repeated). `None` for any other conflict
/// or an unexpected marker layout.
fn resolve_skeleton_conflicts(lines: &[Line]) -> Option<Vec<Line>> {
    let mut out = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        if lines[i].text != "<<<<<<< todo-ours" {
            if lines[i].text.starts_with("=======") || lines[i].text.starts_with(">>>>>>> todo-") {
                return None;
            }
            out.push(lines[i].clone());
            i += 1;
            continue;
        }
        let mut j = i + 1;
        let mut ours = Vec::new();
        while j < lines.len() && lines[j].text != "=======" {
            ours.push(&lines[j]);
            j += 1;
        }
        j += 1;
        let mut theirs = Vec::new();
        while j < lines.len() && lines[j].text != ">>>>>>> todo-theirs" {
            theirs.push(&lines[j]);
            j += 1;
        }
        if j >= lines.len() {
            return None;
        }
        let ok = |l: &&Line| is_placeholder(l).is_some() || l.text.trim().is_empty();
        if !ours.iter().all(ok) || !theirs.iter().all(ok) {
            return None;
        }
        let in_ours: HashSet<u64> = ours.iter().filter_map(|l| is_placeholder(l)).collect();
        out.extend(ours.iter().map(|l| (*l).clone()));
        out.extend(
            theirs
                .iter()
                .filter(|l| is_placeholder(l).is_none_or(|n| !in_ours.contains(&n)))
                .map(|l| (*l).clone()),
        );
        i = j + 1;
    }
    Some(out)
}

/// Gives a line that lost its place at the end of the file an ending (see the module comment).
fn fix_endings(lines: &mut [Line]) {
    let mut prev = LineEnding::Lf;
    let last = lines.len().saturating_sub(1);
    for (i, l) in lines.iter_mut().enumerate() {
        if l.ending == LineEnding::None && i != last {
            l.ending = prev;
        }
        if l.ending != LineEnding::None {
            prev = l.ending;
        }
    }
}

fn push_conflict(out: &mut Vec<Line>, ours: &[Line], theirs: &[Line]) {
    let eol = ours
        .first()
        .or(theirs.first())
        .map_or(LineEnding::Lf, |l| match l.ending {
            LineEnding::None => LineEnding::Lf,
            e => e,
        });
    out.push(Line::new("<<<<<<< ours", eol));
    out.extend(ours.iter().cloned());
    out.push(Line::new("=======", eol));
    out.extend(theirs.iter().cloned());
    out.push(Line::new(">>>>>>> theirs", eol));
}

// ---------------------------------------------------------------------------------------------
// Merging one task block
// ---------------------------------------------------------------------------------------------

struct BlockRef<'a> {
    lines: &'a [Line],
    task: &'a Task,
}

fn block_of(d: &Document, id: TaskId) -> Option<BlockRef<'_>> {
    let task = d.task_by_id(id)?;
    Some(BlockRef {
        lines: d.line_doc.lines.get(task.block_range.clone())?,
        task,
    })
}

enum BlockOut {
    Lines(Vec<Line>),
    Removed,
    Conflict { ours: Vec<Line>, theirs: Vec<Line> },
}

fn merge_block(
    b: Option<BlockRef<'_>>,
    o: Option<BlockRef<'_>>,
    t: Option<BlockRef<'_>>,
) -> Result<BlockOut> {
    match (b, o, t) {
        (_, None, None) => Ok(BlockOut::Removed),
        (None, Some(x), None) | (None, None, Some(x)) => Ok(BlockOut::Lines(x.lines.to_vec())),
        // One side deleted the task. A surviving placeholder means the other side kept or changed
        // it: unchanged is dropped, changed is a conflict.
        (Some(b), Some(x), None) => Ok(if same_lines(b.lines, x.lines) {
            BlockOut::Removed
        } else {
            BlockOut::Conflict {
                ours: x.lines.to_vec(),
                theirs: Vec::new(),
            }
        }),
        (Some(b), None, Some(x)) => Ok(if same_lines(b.lines, x.lines) {
            BlockOut::Removed
        } else {
            BlockOut::Conflict {
                ours: Vec::new(),
                theirs: x.lines.to_vec(),
            }
        }),
        // The same new ID on both sides.
        (None, Some(o), Some(t)) => Ok(if same_lines(o.lines, t.lines) {
            BlockOut::Lines(o.lines.to_vec())
        } else {
            conflict_of(&o, &t)
        }),
        (Some(b), Some(o), Some(t)) => {
            if same_lines(o.lines, t.lines) || same_lines(b.lines, t.lines) {
                return Ok(BlockOut::Lines(o.lines.to_vec()));
            }
            if same_lines(b.lines, o.lines) {
                return Ok(BlockOut::Lines(t.lines.to_vec()));
            }
            if let Some(res) = merge_fields(&b, &o, &t) {
                return Ok(res);
            }
            textual_block_merge(&b, &o, &t)
        }
    }
}

fn conflict_of(o: &BlockRef<'_>, t: &BlockRef<'_>) -> BlockOut {
    BlockOut::Conflict {
        ours: o.lines.to_vec(),
        theirs: t.lines.to_vec(),
    }
}

#[derive(Clone, PartialEq, Eq)]
enum Kind {
    Heading,
    Field(String),
    Note,
    Other,
}

struct Parts<'a> {
    kinds: Vec<Kind>,
    fields: HashMap<&'a str, &'a Line>,
    field_order: Vec<&'a str>,
    notes: Vec<&'a Line>,
    others: Vec<&'a Line>,
    lines: &'a [Line],
}

/// Splits a block into heading, fields, notes and other lines; `None` when a field repeats.
fn parts<'a>(b: &BlockRef<'a>) -> Option<Parts<'a>> {
    let start = b.task.block_range.start;
    let mut kinds = vec![Kind::Other; b.lines.len()];
    if let Some(k) = kinds.first_mut() {
        *k = Kind::Heading;
    }
    let mut fields = HashMap::new();
    let mut field_order = Vec::new();
    for f in &b.task.field_lines {
        let idx = f.line.checked_sub(start)?;
        let line = b.lines.get(idx)?;
        if fields.insert(f.key.as_str(), line).is_some() {
            return None;
        }
        field_order.push(f.key.as_str());
        kinds[idx] = Kind::Field(f.key.clone());
    }
    let mut notes = Vec::new();
    for n in &b.task.notes {
        let idx = n.line.checked_sub(start)?;
        notes.push(b.lines.get(idx)?);
        kinds[idx] = Kind::Note;
    }
    let others = b
        .lines
        .iter()
        .zip(&kinds)
        .filter(|(_, k)| **k == Kind::Other)
        .map(|(l, _)| l)
        .collect();
    Some(Parts {
        kinds,
        fields,
        field_order,
        notes,
        others,
        lines: b.lines,
    })
}

/// Three-way choice of one value; the outer `None` is a conflict.
fn three<'a>(
    b: Option<&'a Line>,
    o: Option<&'a Line>,
    t: Option<&'a Line>,
) -> Option<Option<&'a Line>> {
    if same_opt(o, t) || same_opt(t, b) {
        Some(o)
    } else if same_opt(o, b) {
        Some(t)
    } else {
        None
    }
}

/// Field-by-field merge on the layout of ours. `None` means "not applicable, merge as text".
fn merge_fields(b: &BlockRef<'_>, o: &BlockRef<'_>, t: &BlockRef<'_>) -> Option<BlockOut> {
    let (pb, po, pt) = (parts(b)?, parts(o)?, parts(t)?);
    if po.others.len() != pt.others.len()
        || !po.others.iter().zip(&pt.others).all(|(x, y)| same(x, y))
    {
        return None;
    }
    if pb.others.len() != po.others.len()
        || !pb.others.iter().zip(&po.others).all(|(x, y)| same(x, y))
    {
        // Prose changed on both sides alike or on one side only: let git decide line by line.
        return None;
    }

    let conflict = || {
        Some(BlockOut::Conflict {
            ours: o.lines.to_vec(),
            theirs: t.lines.to_vec(),
        })
    };

    // Heading.
    let Some(Some(heading)) = three(pb.lines.first(), po.lines.first(), pt.lines.first()) else {
        return conflict();
    };

    let mut out: Vec<(Kind, Line)> = Vec::new();
    for (i, kind) in po.kinds.iter().enumerate() {
        match kind {
            Kind::Heading => out.push((Kind::Heading, heading.clone())),
            Kind::Field(key) => {
                let merged = three(
                    pb.fields.get(key.as_str()).copied(),
                    po.fields.get(key.as_str()).copied(),
                    pt.fields.get(key.as_str()).copied(),
                );
                match merged {
                    None => return conflict(),
                    Some(None) => {}
                    Some(Some(l)) => out.push((kind.clone(), l.clone())),
                }
            }
            Kind::Note | Kind::Other => out.push((kind.clone(), o.lines[i].clone())),
        }
    }

    // Fields only theirs has: added by theirs (kept) or deleted by ours (conflict if changed).
    for (pos, key) in pt.field_order.iter().enumerate() {
        if po.fields.contains_key(key) {
            continue;
        }
        match three(
            pb.fields.get(key).copied(),
            None,
            pt.fields.get(key).copied(),
        ) {
            None => return conflict(),
            Some(None) => {}
            Some(Some(l)) => {
                // After the nearest earlier field of theirs that is in the result, else after the
                // heading.
                let at = pt.field_order[..pos]
                    .iter()
                    .rev()
                    .find_map(|k| {
                        out.iter()
                            .rposition(|(kk, _)| *kk == Kind::Field((*k).to_string()))
                    })
                    .unwrap_or(0);
                out.insert(at + 1, (Kind::Field((*key).to_string()), l.clone()));
            }
        }
    }

    // Notes only theirs has (not in ours, not in the base): after ours' last note, else at the end.
    let new_notes: Vec<&Line> = pt
        .notes
        .iter()
        .filter(|n| {
            !po.notes.iter().any(|x| same(x, n)) && !pb.notes.iter().any(|x| same(x, n))
        })
        .copied()
        .collect();
    if !new_notes.is_empty() {
        let at = out
            .iter()
            .rposition(|(k, _)| *k == Kind::Note)
            .map_or(out.len(), |p| p + 1);
        for (k, n) in new_notes.into_iter().enumerate() {
            out.insert(at + k, (Kind::Note, n.clone()));
        }
    }
    Some(BlockOut::Lines(out.into_iter().map(|(_, l)| l).collect()))
}

/// Last resort for a block both sides changed outside the field model: `git merge-file` on it.
fn textual_block_merge(
    b: &BlockRef<'_>,
    o: &BlockRef<'_>,
    t: &BlockRef<'_>,
) -> Result<BlockOut> {
    let (bytes, conflicts) = git_merge_file(
        &lines_to_bytes(b.lines),
        &lines_to_bytes(o.lines),
        &lines_to_bytes(t.lines),
        "ours",
        "base",
        "theirs",
    )?;
    if conflicts == 0
        && let Ok(doc) = LineDoc::from_bytes(&bytes)
    {
        return Ok(BlockOut::Lines(doc.lines));
    }
    Ok(conflict_of(o, t))
}

// ---------------------------------------------------------------------------------------------
// git
// ---------------------------------------------------------------------------------------------

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Runs `git merge-file -p` on three byte strings; returns the merged bytes and the number of
/// conflicts (git's exit status).
fn git_merge_file(
    base: &[u8],
    ours: &[u8],
    theirs: &[u8],
    label_ours: &str,
    label_base: &str,
    label_theirs: &str,
) -> Result<(Vec<u8>, usize)> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let dir = std::env::temp_dir().join(format!(
        "todo-merge-{}-{nanos}-{}",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).map_err(|e| io_err("could not create", &dir, &e))?;
    let result = (|| {
        let paths = [("ours", ours), ("base", base), ("theirs", theirs)];
        for (name, bytes) in paths {
            let p = dir.join(name);
            fs::write(&p, bytes).map_err(|e| io_err("could not write", &p, &e))?;
        }
        let output = ProcessCommand::new("git")
            .args(["merge-file", "-p", "-L", label_ours, "-L", label_base, "-L", label_theirs])
            .arg(dir.join("ours"))
            .arg(dir.join("base"))
            .arg(dir.join("theirs"))
            .output()
            .map_err(|e| {
                TodoError::new(
                    ErrorCode::WriteFailed,
                    format!("could not run `git merge-file`: {e}"),
                )
            })?;
        match output.status.code() {
            Some(c) if (0..128).contains(&c) => {
                Ok((output.stdout, usize::try_from(c).unwrap_or(1)))
            }
            _ => Err(TodoError::new(
                ErrorCode::WriteFailed,
                format!(
                    "`git merge-file` failed: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ),
            )),
        }
    })();
    let _ = fs::remove_dir_all(&dir);
    result
}

fn git_in(dir: Option<&Path>, args: &[&str]) -> Result<std::process::Output> {
    let mut cmd = ProcessCommand::new("git");
    if let Some(d) = dir {
        cmd.current_dir(d);
    }
    cmd.args(args).output().map_err(|e| {
        TodoError::new(
            ErrorCode::BadValue,
            format!("could not run git: {e} (is git on PATH?)"),
        )
    })
}

// ---------------------------------------------------------------------------------------------
// install / uninstall
// ---------------------------------------------------------------------------------------------

fn repo_root() -> Result<PathBuf> {
    let o = git_in(None, &["rev-parse", "--show-toplevel"])?;
    if !o.status.success() {
        return Err(TodoError::new(
            ErrorCode::BadValue,
            "not inside a git repository",
        ));
    }
    let text = String::from_utf8_lossy(&o.stdout);
    // git prints `C:/x/y` on Windows; rebuilding from components gives native separators.
    Ok(Path::new(text.trim_end_matches(['\r', '\n']))
        .components()
        .collect())
}

fn config_get(root: &Path, key: &str) -> Result<Option<String>> {
    let o = git_in(Some(root), &["config", "--local", "--get", key])?;
    Ok(o.status
        .success()
        .then(|| String::from_utf8_lossy(&o.stdout).trim().to_string()))
}

fn config_set(root: &Path, key: &str, value: &str) -> Result<bool> {
    if config_get(root, key)?.as_deref() == Some(value) {
        return Ok(false);
    }
    let o = git_in(Some(root), &["config", "--local", key, value])?;
    if !o.status.success() {
        return Err(TodoError::new(
            ErrorCode::WriteFailed,
            format!(
                "git config {key} failed: {}",
                String::from_utf8_lossy(&o.stderr).trim()
            ),
        ));
    }
    Ok(true)
}

fn check_pattern(path: &str) -> Result<()> {
    if path.is_empty() || path.contains(char::is_whitespace) {
        return Err(TodoError::new(
            ErrorCode::Usage,
            "--path must be a non-empty pattern without spaces",
        ));
    }
    Ok(())
}

fn attr_tokens(line: &Line) -> Vec<&str> {
    line.text.split_whitespace().collect()
}

fn has_attr_for(line: &Line, path: &str) -> bool {
    let t = attr_tokens(line);
    t.first() == Some(&path) && t[1..].contains(&ATTR)
}

fn is_exact_attr_line(line: &Line, path: &str) -> bool {
    attr_tokens(line) == [path, ATTR]
}

fn read_attributes(file: &Path) -> Result<Option<LineDoc>> {
    match fs::read(file) {
        Ok(bytes) => LineDoc::from_bytes(&bytes).map(Some).map_err(|e| {
            TodoError::new(
                ErrorCode::BadValue,
                format!("{} is not valid UTF-8: {e}", file.display()),
            )
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_err("could not read", file, &e)),
    }
}

fn install(path: &str, out: &mut Output) -> Result<()> {
    check_pattern(path)?;
    let root = repo_root()?;
    let mut changed = config_set(&root, "merge.todo.name", DRIVER_NAME)?;
    changed |= config_set(&root, "merge.todo.driver", DRIVER_COMMAND)?;

    let file = root.join(".gitattributes");
    let mut doc = read_attributes(&file)?.unwrap_or(LineDoc {
        has_bom: false,
        lines: Vec::new(),
    });
    if !doc.lines.iter().any(|l| has_attr_for(l, path)) {
        let eol = doc.default_ending();
        if let Some(last) = doc.lines.last_mut()
            && last.ending == LineEnding::None
        {
            last.ending = eol;
        }
        doc.lines.push(Line::new(format!("{path} {ATTR}"), eol));
        atomic_write_bytes(&file, &doc.to_bytes())?;
        changed = true;
    }
    report(out, "install", changed, &file, path);
    Ok(())
}

fn uninstall(path: &str, out: &mut Output) -> Result<()> {
    check_pattern(path)?;
    let root = repo_root()?;
    let mut changed = false;
    let o = git_in(Some(&root), &["config", "--local", "--get-regexp", r"^merge\.todo\."])?;
    if o.status.success() {
        // Exit status 128 only means the section is already gone.
        let _ = git_in(Some(&root), &["config", "--local", "--remove-section", "merge.todo"])?;
        changed = true;
    }

    let file = root.join(".gitattributes");
    if let Some(mut doc) = read_attributes(&file)? {
        let before = doc.lines.len();
        doc.lines.retain(|l| !is_exact_attr_line(l, path));
        if doc.lines.len() != before {
            if doc.lines.is_empty() {
                fs::remove_file(&file).map_err(|e| io_err("could not remove", &file, &e))?;
            } else {
                atomic_write_bytes(&file, &doc.to_bytes())?;
            }
            changed = true;
        }
    }
    report(out, "uninstall", changed, &file, path);
    Ok(())
}

fn report(out: &mut Output, action: &'static str, changed: bool, file: &Path, path: &str) {
    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &InstallJson {
                action,
                changed,
                attributes_file: file.display().to_string(),
                path: path.to_string(),
            },
        );
    } else if changed {
        out.write_line(&format!("merge driver {action} ok ({path})"));
    } else {
        out.write_line(&format!("merge driver {action}: already up to date ({path})"));
    }
}
