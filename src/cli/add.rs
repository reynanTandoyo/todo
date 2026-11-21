//! `todo add` command: add a new task to a section with monotonic IDs (T-028; R-WRITE-1, R-ID-2).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Read as _;
use std::path::Path;

use clap::Args as ClapArgs;
use serde::{Deserialize, Serialize};

use crate::cli::{CommandKind, CommandMeta, Context, resolve_opt_text_arg, resolve_text_arg};
use crate::doc::{
    Document, EntryKind, LineEdit, Status, TaskChangeIntent, TaskId, sanitize_trimmed_value,
};
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::{
    DocWritePlan, NewTask, OpsConfig, StepsInput, TaskJson, build_task_json,
    edit_doc_with_found_and_state_and_write, highest_task_number, matching_section_indices, milestone,
    steps_from_text,
};
use crate::output::Output;
use crate::state::IdsState;

/// Metadata for the `add` command.
pub const META: CommandMeta = CommandMeta {
    name: "add",
    summary: "Add a new task to a section",
    kind: CommandKind::Write,
    details: "Adds a new task at the end of the target section using the next monotonic task ID across `TODO.md`, `TODO-archive.md`, and `ids.json`. Refuses duplicate titles among active tasks, missing required fields (default `priority` and `difficulty` for a task; a milestone, question or idea needs neither), and unknown or ambiguous sections.",
    examples: &[
        "todo add --title \"Fix login timeout\" --priority P1 --difficulty 2 --section Now --agent agent-1",
        "todo add --title \"Add metrics\" --priority P2 --difficulty 3 --type feature --area cli --agent coordinator",
    ],
};

/// Arguments for `todo add`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Title for the new task (required unless `--from` is used).
    #[arg(long, value_name = "TEXT")]
    pub title: Option<String>,

    /// Add multiple tasks from JSON lines in a file or `-` (standard input).
    #[arg(long, value_name = "FILE|-")]
    pub from: Option<String>,

    /// Target section name (exact or unique substring match).
    #[arg(long, value_name = "NAME")]
    pub section: Option<String>,

    /// Priority (e.g. `P0`, `P1`, `P2`, `P3`).
    #[arg(long, value_name = "VALUE")]
    pub priority: Option<String>,

    /// The task's level, one of the configured `difficulties` (default `1` to `5`; `todo help add`).
    #[arg(long, value_name = "VALUE")]
    pub difficulty: Option<String>,

    /// Task type (e.g. `feature`, `bug`, `chore`).
    #[arg(long = "type", value_name = "VALUE")]
    pub task_type: Option<String>,

    /// Area (e.g. `cli`, `docs`).
    #[arg(long, value_name = "VALUE")]
    pub area: Option<String>,

    /// Parent milestone: the ID or title of the milestone this entry belongs to.
    #[arg(long, value_name = "TASK")]
    pub milestone: Option<String>,

    /// Goal of a milestone; a milestone needs one.
    #[arg(long, value_name = "TEXT")]
    pub goal: Option<String>,

    /// `why` field value: what the task is for.
    #[arg(long, value_name = "TEXT")]
    pub why: Option<String>,

    /// `summary` field value: what the entry is about, in plain words for people (R-WRITE-19).
    #[arg(long, value_name = "TEXT")]
    pub summary: Option<String>,

    /// Dependencies (comma-separated task IDs).
    #[arg(long, value_name = "IDS")]
    pub depends: Option<String>,

    /// `files` field value.
    #[arg(long, value_name = "TEXT")]
    pub files: Option<String>,

    /// `read` field value: paths to look at but not change, separated by commas.
    #[arg(long, value_name = "TEXT")]
    pub read: Option<String>,

    /// `problem` field value.
    #[arg(long, value_name = "TEXT")]
    pub problem: Option<String>,

    /// `fix` field value.
    #[arg(long, value_name = "TEXT")]
    pub fix: Option<String>,

    /// `steps` field: one step per line of the text.
    #[arg(long, value_name = "TEXT")]
    pub steps: Option<String>,

    /// `accept` field value.
    #[arg(long, value_name = "TEXT")]
    pub accept: Option<String>,

    /// `verify` field value: one command line that checks the task.
    #[arg(long, value_name = "TEXT")]
    pub verify: Option<String>,

    /// Additional field `key=value` (repeatable).
    #[arg(long = "field", value_name = "KEY=VALUE")]
    pub fields: Vec<String>,
}

/// Runs `todo add --title ...` or `todo add --from <file|->`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    if let Some(ref from_src) = args.from {
        if args.title.is_some() {
            return Err(TodoError::new(
                ErrorCode::Usage,
                "cannot combine --title and --from",
            ));
        }
        return run_batch_from(from_src, args.section.as_deref(), ctx, out);
    }

    let Some(ref title_arg) = args.title else {
        return Err(TodoError::new(
            ErrorCode::Usage,
            "missing --title <TEXT> or --from <FILE|->",
        ));
    };

    let raw_title = resolve_text_arg(title_arg)?;
    let clean_title = sanitize_trimmed_value(&raw_title);
    if clean_title.is_empty() {
        return Err(TodoError::new(
            ErrorCode::BadValue,
            "--title cannot be empty",
        ));
    }

    let extra_fields = parse_extra_fields(&args.fields)?;
    let (found, cfg) = ctx.load_config_with_found()?;

    // Validate priority, type, area, and required fields before taking the lock.
    let type_val = resolve_allowed(
        "type",
        args.task_type.as_deref(),
        &extra_fields,
        &cfg.types,
    )?;
    let required = required_for_kind(&cfg.required_fields, type_val.as_deref());
    let priority_val =
        resolve_priority(args.priority.as_deref(), &extra_fields, &cfg, required)?;
    let difficulty_val = resolve_allowed(
        "difficulty",
        args.difficulty.as_deref(),
        &extra_fields,
        &cfg.difficulties,
    )?;
    let area_val = resolve_allowed("area", args.area.as_deref(), &extra_fields, &cfg.areas)?;
    let milestone_val = resolve_opt(args.milestone.as_deref(), &extra_fields, "milestone");
    let goal_arg = resolve_opt_text_arg(args.goal.as_deref())?;
    let goal_val = resolve_opt(goal_arg.as_deref(), &extra_fields, "goal");
    milestone::check_new_entry_fields(
        type_val.as_deref(),
        milestone_val.as_deref(),
        goal_val.as_deref(),
    )?;
    let why_arg = resolve_opt_text_arg(args.why.as_deref())?;
    let summary_arg = resolve_opt_text_arg(args.summary.as_deref())?;
    let files_arg = resolve_opt_text_arg(args.files.as_deref())?;
    let read_arg = resolve_opt_text_arg(args.read.as_deref())?;
    let problem_arg = resolve_opt_text_arg(args.problem.as_deref())?;
    let fix_arg = resolve_opt_text_arg(args.fix.as_deref())?;
    let steps_arg = resolve_opt_text_arg(args.steps.as_deref())?;
    let accept_arg = resolve_opt_text_arg(args.accept.as_deref())?;
    let verify_arg = resolve_opt_text_arg(args.verify.as_deref())?;
    let why_val = resolve_opt(why_arg.as_deref(), &extra_fields, "why");
    let summary_val = resolve_opt(summary_arg.as_deref(), &extra_fields, "summary");
    let depends_val = resolve_opt(args.depends.as_deref(), &extra_fields, "depends");
    let files_val = resolve_opt(files_arg.as_deref(), &extra_fields, "files");
    let read_val = resolve_opt(read_arg.as_deref(), &extra_fields, "read");
    let problem_val = resolve_opt(problem_arg.as_deref(), &extra_fields, "problem");
    let fix_val = resolve_opt(fix_arg.as_deref(), &extra_fields, "fix");
    let steps_val = resolve_steps(steps_arg.as_deref(), &extra_fields);
    let accept_val = resolve_opt(accept_arg.as_deref(), &extra_fields, "accept");
    let verify_val = resolve_opt(verify_arg.as_deref(), &extra_fields, "verify");
    check_required_fields(
        required,
        RequiredValues {
            priority: priority_val.as_deref(),
            difficulty: difficulty_val.as_deref(),
            task_type: type_val.as_deref(),
            area: area_val.as_deref(),
            why: why_val.as_deref(),
            summary: summary_val.as_deref(),
            depends: depends_val.as_deref(),
            files: files_val.as_deref(),
            read: read_val.as_deref(),
            problem: problem_val.as_deref(),
            fix: fix_val.as_deref(),
            steps: !steps_val.is_empty(),
            accept: accept_val.as_deref(),
            verify: verify_val.as_deref(),
        },
        &extra_fields,
    )?;

    let cfg_clone = cfg.clone();
    let res = edit_doc_with_found_and_state_and_write(
        ctx,
        found,
        &cfg,
        META.name,
        out,
        |doc, archive, state, _label, agent, now| {
            // 1. Check duplicate title among active (non-done, non-dropped) tasks (R-WRITE-1).
            if let Some(existing) = doc.tasks.iter().find(|t| {
                !matches!(t.status, Status::Done | Status::Dropped)
                    && t.title.trim().eq_ignore_ascii_case(&clean_title)
            }) {
                let existing_id = cfg.id_config.format_id(existing.id);
                return Err(TodoError::new(
                    ErrorCode::DuplicateTitle,
                    format!("task {existing_id} already has title {clean_title:?}"),
                )
                .with_task(existing_id));
            }

            // 2. The parent milestone must exist and be a milestone (R-MILE-1).
            let milestone_id = match milestone_val.as_deref() {
                Some(value) => Some(milestone::check_parent(doc, &cfg, None, value)?),
                None => None,
            };

            // 3. Resolve target section (R-WRITE-1).
            let section_idx = resolve_target_section(doc, args.section.as_deref(), &cfg)?;
            let section = &doc.sections[section_idx];

            // 4. Allocate next monotonic ID across TODO.md, archive, and ids.json (R-ID-2).
            let max_state = state.read_ids()?.map_or(0, |s| s.highest_id);
            let next_num = highest_task_number(doc, archive, max_state)
                .checked_add(1)
                .ok_or_else(|| TodoError::new(ErrorCode::WriteFailed, "task ID overflow"))?;
            let new_id = TaskId(next_num);
            let id_str = cfg.id_config.format_id(new_id);

            // Persist the new high-water mark while holding the write lock (unless --dry-run).
            if !ctx.dry_run {
                state.write_ids(&IdsState {
                    highest_id: next_num,
                })?;
            }

            // 5. Build canonical lines for the new task (docs/format.md §10).
            let date = OpsConfig::format_note_date(now);
            let lines = NewTask {
                title: &clean_title,
                priority: priority_val.as_deref(),
                difficulty: difficulty_val.as_deref(),
                task_type: type_val.as_deref(),
                area: area_val.as_deref(),
                milestone: milestone_id.as_deref(),
                goal: goal_val.as_deref(),
                why: why_val.as_deref(),
                summary: summary_val.as_deref(),
                depends: depends_val.as_deref(),
                files: files_val.as_deref(),
                read: read_val.as_deref(),
                problem: problem_val.as_deref(),
                fix: fix_val.as_deref(),
                steps: &steps_val,
                accept: accept_val.as_deref(),
                verify: verify_val.as_deref(),
                extra: &extra_fields,
            }
            .lines(&id_str, &date, agent);

            let edit = LineEdit::insert(section.content_end, lines);
            let intent = TaskChangeIntent::add_one(new_id);
            let arch_owned = archive.cloned();
            let cfg_owned = cfg_clone.clone();
            let err_id = id_str.clone();

            Ok(DocWritePlan::Edits {
                edits: vec![edit],
                intent,
                build_value: Box::new(move |new_doc| {
                    let added = new_doc.task_by_id(new_id).ok_or_else(|| {
                        TodoError::new(
                            ErrorCode::WriteFailed,
                            format!("added task {err_id} missing after verification"),
                        )
                    })?;
                    let task_json = build_task_json(
                        new_doc,
                        added,
                        arch_owned.as_ref(),
                        false,
                        now,
                        &cfg_owned,
                    );
                    Ok((err_id, task_json))
                }),
            })
        },
    )?;

    if out.is_json() {
        out.emit_json_ok(META.name, &res.value.1);
    } else {
        out.write_line(&format!("{}: add ok", res.value.0));
    }
    Ok(())
}

fn parse_extra_fields(raw: &[String]) -> Result<Vec<(String, String)>> {
    let mut pairs = Vec::with_capacity(raw.len());
    for item in raw {
        let Some((k, v)) = item.split_once('=') else {
            return Err(TodoError::new(
                ErrorCode::Usage,
                format!("invalid --field {item:?}: expected KEY=VALUE"),
            ));
        };
        let key = k.trim().to_ascii_lowercase();
        if key.is_empty() {
            return Err(TodoError::new(
                ErrorCode::Usage,
                format!("invalid --field {item:?}: key cannot be empty"),
            ));
        }
        if matches!(key.as_str(), "status" | "owner" | "claimed" | "note" | "title") {
            return Err(TodoError::new(
                ErrorCode::BadValue,
                format!("cannot set `{key}` via --field on add"),
            ));
        }
        let resolved_v = resolve_text_arg(v)?;
        // `steps` keeps its line breaks: each line is one step.
        let value = if key == "steps" {
            resolved_v.trim().to_string()
        } else {
            sanitize_trimmed_value(&resolved_v)
        };
        pairs.push((key, value));
    }
    Ok(pairs)
}

/// The steps of a new task: `--steps` when given, else a `steps` field (`R-DETAIL-1`).
fn resolve_steps(flag: Option<&str>, extra: &[(String, String)]) -> Vec<String> {
    let text = flag.or_else(|| {
        extra
            .iter()
            .rev()
            .find(|(k, _)| k == "steps")
            .map(|(_, v)| v.as_str())
    });
    text.map_or_else(Vec::new, steps_from_text)
}

fn resolve_opt(
    flag: Option<&str>,
    extra: &[(String, String)],
    key: &str,
) -> Option<String> {
    let raw = flag.or_else(|| {
        extra
            .iter()
            .rev()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    })?;
    let clean = sanitize_trimmed_value(raw);
    if clean.is_empty() { None } else { Some(clean) }
}

/// The required fields that apply to an entry of this `type`: `required_fields` for a task, and none
/// for a milestone, question or idea, which need neither a priority nor a level (R-LEVEL-1).
fn required_for_kind<'a>(required_fields: &'a [String], task_type: Option<&str>) -> &'a [String] {
    if EntryKind::from_type(task_type) == EntryKind::Task {
        required_fields
    } else {
        &[]
    }
}

fn resolve_priority(
    flag: Option<&str>,
    extra: &[(String, String)],
    cfg: &OpsConfig,
    required: &[String],
) -> Result<Option<String>> {
    let raw = resolve_opt(flag, extra, "priority");
    match raw {
        Some(val) => Ok(Some(cfg.canonical_priority(&val)?)),
        None => {
            if required
                .iter()
                .any(|f| f.eq_ignore_ascii_case("priority"))
            {
                return Err(TodoError::new(
                    ErrorCode::MissingField,
                    format!(
                        "missing required field `priority`; pass --priority <{}>",
                        cfg.priorities.join("|")
                    ),
                ));
            }
            Ok(None)
        }
    }
}

fn resolve_allowed(
    key: &str,
    flag: Option<&str>,
    extra: &[(String, String)],
    allowed: &[String],
) -> Result<Option<String>> {
    let Some(val) = resolve_opt(flag, extra, key) else {
        return Ok(None);
    };
    if allowed.is_empty() {
        return Ok(Some(val));
    }
    let Some(canonical) = allowed.iter().find(|a| a.eq_ignore_ascii_case(&val)) else {
        return Err(TodoError::new(
            ErrorCode::BadValue,
            format!("invalid {key} '{val}'; expected one of: {}", allowed.join(", ")),
        ));
    };
    Ok(Some(canonical.clone()))
}

/// The values of the fields that `required_fields` can name, as the caller resolved them.
#[derive(Clone, Copy)]
struct RequiredValues<'a> {
    priority: Option<&'a str>,
    difficulty: Option<&'a str>,
    task_type: Option<&'a str>,
    area: Option<&'a str>,
    why: Option<&'a str>,
    summary: Option<&'a str>,
    depends: Option<&'a str>,
    files: Option<&'a str>,
    read: Option<&'a str>,
    problem: Option<&'a str>,
    fix: Option<&'a str>,
    /// Whether the task has at least one step.
    steps: bool,
    accept: Option<&'a str>,
    verify: Option<&'a str>,
}

fn check_required_fields(
    required: &[String],
    values: RequiredValues<'_>,
    extra: &[(String, String)],
) -> Result<()> {
    for req in required {
        let present = match req.to_ascii_lowercase().as_str() {
            "priority" => values.priority.is_some(),
            "difficulty" => values.difficulty.is_some(),
            "type" => values.task_type.is_some(),
            "area" => values.area.is_some(),
            "why" => values.why.is_some(),
            "summary" => values.summary.is_some(),
            "depends" => values.depends.is_some(),
            "files" => values.files.is_some(),
            "read" => values.read.is_some(),
            "problem" => values.problem.is_some(),
            "fix" => values.fix.is_some(),
            "steps" => values.steps,
            "accept" => values.accept.is_some(),
            "verify" => values.verify.is_some(),
            other => extra
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case(other) && !v.trim().is_empty()),
        };
        if !present {
            return Err(TodoError::new(
                ErrorCode::MissingField,
                format!("missing required field `{req}`"),
            ));
        }
    }
    Ok(())
}

pub(crate) fn resolve_target_section(
    doc: &Document,
    section_flag: Option<&str>,
    cfg: &OpsConfig,
) -> Result<usize> {
    if doc.sections.is_empty() {
        return Err(TodoError::new(
            ErrorCode::UnknownSection,
            "TODO.md has no `## ` sections to add a task to",
        ));
    }

    let query = match section_flag {
        Some(s) => s,
        None => match cfg.default_section.as_deref() {
            Some(def) => def,
            None if doc.sections.len() == 1 => return Ok(0),
            None => {
                let all_joined = section_names_joined(doc);
                return Err(TodoError::new(
                    ErrorCode::UnknownSection,
                    format!(
                        "multiple sections exist; pass --section <name>. Available sections: {all_joined}"
                    ),
                ));
            }
        },
    };

    find_section(doc, query)
}

/// The index of the one `## ` section that `query` names (exact name or unique substring).
///
/// Unknown and ambiguous queries fail with `UnknownSection` / `AmbiguousSection` and list the
/// available sections. Shared by `todo add`, `todo move` and `todo apply`.
pub(crate) fn find_section(doc: &Document, query: &str) -> Result<usize> {
    let all_joined = section_names_joined(doc);
    let matches = matching_section_indices(&doc.sections, query);
    match matches.as_slice() {
        [] => Err(TodoError::new(
            ErrorCode::UnknownSection,
            format!("unknown section '{query}'; available sections: {all_joined}"),
        )),
        [only] => Ok(*only),
        many => {
            let matched_names: Vec<&str> =
                many.iter().map(|&i| doc.sections[i].name.as_str()).collect();
            Err(TodoError::new(
                ErrorCode::AmbiguousSection,
                format!(
                    "section '{query}' is ambiguous (matches: {}); available sections: {all_joined}",
                    matched_names.join(", ")
                ),
            ))
        }
    }
}

/// Section names of `doc` separated by `, ` (for error messages).
fn section_names_joined(doc: &Document) -> String {
    doc.sections
        .iter()
        .map(|s| s.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Checks the `@ref` milestones of a batch before anything is written (R-MILE-1): each names an
/// entry of the same batch that is a milestone, and no chain of references leads back to itself.
fn check_batch_milestones(entries: &[ValidatedBatchEntry]) -> Result<()> {
    for entry in entries {
        let Some(target) = entry.milestone.as_deref().and_then(|m| m.strip_prefix('@')) else {
            continue;
        };
        let Some(parent) = entries
            .iter()
            .find(|e| e.ref_name.as_deref() == Some(target))
        else {
            return Err(TodoError::new(
                ErrorCode::BadValue,
                format!("unknown milestone reference @{target}"),
            ));
        };
        if EntryKind::from_type(parent.task_type.as_deref()) != EntryKind::Milestone {
            return Err(TodoError::new(
                ErrorCode::BadValue,
                format!("@{target} is not a milestone, so it cannot hold tasks"),
            ));
        }
    }
    for entry in entries {
        let Some(start) = entry.ref_name.as_deref() else {
            continue;
        };
        let mut current = start;
        for _ in 0..=entries.len() {
            let next = entries
                .iter()
                .find(|e| e.ref_name.as_deref() == Some(current))
                .and_then(|e| e.milestone.as_deref().and_then(|m| m.strip_prefix('@')));
            let Some(next) = next else {
                break;
            };
            if next == start {
                return Err(TodoError::new(
                    ErrorCode::BadValue,
                    format!("@{start} would be its own ancestor through its milestone"),
                ));
            }
            current = next;
        }
    }
    Ok(())
}

/// Adds the batch line number to a refusal, as the other `--from` errors do (`R-OUT-4`).
pub(crate) fn at_batch_line(mut err: TodoError, line_no: usize) -> TodoError {
    err.message = format!("--from line {line_no}: {}", err.message);
    err.with_line(line_no)
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum DependsInput {
    One(String),
    Many(Vec<String>),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BatchAddEntry {
    #[serde(rename = "ref")]
    ref_name: Option<String>,
    title: String,
    section: Option<String>,
    priority: Option<String>,
    difficulty: Option<String>,
    #[serde(rename = "type")]
    task_type: Option<String>,
    area: Option<String>,
    milestone: Option<String>,
    goal: Option<String>,
    why: Option<String>,
    summary: Option<String>,
    depends: Option<DependsInput>,
    files: Option<String>,
    read: Option<String>,
    problem: Option<String>,
    fix: Option<String>,
    steps: Option<StepsInput>,
    accept: Option<String>,
    verify: Option<String>,
    #[serde(default)]
    fields: BTreeMap<String, String>,
}

pub(crate) struct ValidatedBatchEntry {
    pub(crate) ref_name: Option<String>,
    pub(crate) title: String,
    pub(crate) section: Option<String>,
    pub(crate) priority: Option<String>,
    pub(crate) difficulty: Option<String>,
    pub(crate) task_type: Option<String>,
    pub(crate) area: Option<String>,
    /// Parent milestone as given: an ID, or `@ref` naming an entry of the same batch (R-MILE-1).
    pub(crate) milestone: Option<String>,
    pub(crate) goal: Option<String>,
    pub(crate) why: Option<String>,
    pub(crate) summary: Option<String>,
    pub(crate) depends_tokens: Vec<String>,
    pub(crate) files: Option<String>,
    pub(crate) read: Option<String>,
    pub(crate) problem: Option<String>,
    pub(crate) fix: Option<String>,
    pub(crate) steps: Vec<String>,
    pub(crate) accept: Option<String>,
    pub(crate) verify: Option<String>,
    pub(crate) extra: Vec<(String, String)>,
    /// Line of the batch input the entry came from (1-based), for refusals.
    pub(crate) line_no: usize,
}

#[derive(Serialize)]
struct BatchAddJson {
    ids: Vec<String>,
    added: Vec<TaskJson>,
    count: usize,
}

pub(crate) fn read_batch_input(src: &str) -> Result<String> {
    if src == "-" {
        let mut buf = Vec::new();
        std::io::stdin().read_to_end(&mut buf).map_err(|e| {
            TodoError::new(
                ErrorCode::BadValue,
                format!("failed to read standard input: {e}"),
            )
        })?;
        let text = String::from_utf8(buf).map_err(|e| {
            TodoError::new(
                ErrorCode::BadValue,
                format!("standard input is not valid UTF-8: {e}"),
            )
        })?;
        return Ok(text.strip_prefix('\u{feff}').unwrap_or(&text).to_string());
    }
    let path = Path::new(src);
    let bytes = std::fs::read(crate::find::fs_path(path).as_ref()).map_err(|e| {
        TodoError::new(
            ErrorCode::NoFile,
            format!("could not read {}: {e}", path.display()),
        )
        .with_file(path.display().to_string())
    })?;
    let text = String::from_utf8(bytes).map_err(|e| {
        TodoError::new(
            ErrorCode::BadValue,
            format!("file {} is not valid UTF-8: {e}", path.display()),
        )
        .with_file(path.display().to_string())
    })?;
    Ok(text.strip_prefix('\u{feff}').unwrap_or(&text).to_string())
}

fn run_batch_from(
    from_src: &str,
    default_section_flag: Option<&str>,
    ctx: &Context,
    out: &mut Output,
) -> Result<()> {
    let raw = read_batch_input(from_src)?;
    let res = add_batch_text(&raw, default_section_flag, ctx, out)?;
    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &BatchAddJson {
                count: res.ids.len(),
                ids: res.ids,
                added: res.added,
            },
        );
    } else {
        for id in &res.ids {
            out.write_line(&format!("{id}: add ok"));
        }
    }
    Ok(())
}

/// The IDs given to the tasks of a batch (in entry order) and their JSON.
pub(crate) struct BatchAdded {
    pub ids: Vec<String>,
    pub added: Vec<TaskJson>,
}

/// Adds the tasks given as JSON lines in one locked write, all or none (R-WRITE-13).
///
/// Shared by `todo add --from` and `todo scan --add`. Prints nothing; the caller reports the result.
pub(crate) fn add_batch_text(
    raw: &str,
    default_section_flag: Option<&str>,
    ctx: &Context,
    out: &mut Output,
) -> Result<BatchAdded> {
    let (found, cfg) = ctx.load_config_with_found()?;

    let entries = parse_batch_entries(raw, default_section_flag, &cfg)?;
    check_batch_milestones(&entries)?;

    let cfg_clone = cfg.clone();
    let res = edit_doc_with_found_and_state_and_write(
        ctx,
        found,
        &cfg,
        META.name,
        out,
        |doc, archive, state, _label, agent, now| {
            // 1. Check duplicate titles against existing active tasks (R-WRITE-1).
            check_batch_titles(doc, &entries, &cfg)?;

            // 2. Allocate monotonic IDs for the entire batch.
            let max_state = state.read_ids()?.map_or(0, |s| s.highest_id);
            let mut next_num = highest_task_number(doc, archive, max_state);

            let mut allocated_ids = Vec::with_capacity(entries.len());
            let mut ref_to_id = HashMap::new();
            for entry in &entries {
                next_num = next_num
                    .checked_add(1)
                    .ok_or_else(|| TodoError::new(ErrorCode::WriteFailed, "task ID overflow"))?;
                let id = TaskId(next_num);
                let id_str = cfg.id_config.format_id(id);
                if let Some(ref r) = entry.ref_name {
                    ref_to_id.insert(r.clone(), id_str.clone());
                }
                allocated_ids.push((id, id_str));
            }

            // 3. Resolve sections and `@ref` dependencies before writing `ids.json`.
            let date = OpsConfig::format_note_date(now);
            let mut grouped_lines: BTreeMap<usize, Vec<String>> = BTreeMap::new();
            let mut added_ids = BTreeSet::new();
            let mut ordered_ids = Vec::with_capacity(entries.len());

            for (entry, (new_id, id_str)) in entries.iter().zip(allocated_ids.iter()) {
                // A `@ref` parent is an entry of this batch (checked before the lock); any other
                // parent must already be a milestone in TODO.md (R-MILE-1).
                let milestone_id = match entry.milestone.as_deref() {
                    Some(value) => match value.strip_prefix('@') {
                        Some(name) => Some(ref_to_id.get(name).cloned().ok_or_else(|| {
                            TodoError::new(
                                ErrorCode::BadValue,
                                format!("unknown milestone reference {value:?}"),
                            )
                        })?),
                        None => Some(milestone::check_parent(doc, &cfg, None, value)?),
                    },
                    None => None,
                };
                let sec_idx = resolve_target_section(doc, entry.section.as_deref(), &cfg)?;
                let insert_at = doc.sections[sec_idx].content_end;

                let depends_val = resolve_batch_depends(&entry.depends_tokens, &ref_to_id)?;
                let lines = entry_lines(
                    entry,
                    milestone_id.as_deref(),
                    depends_val.as_deref(),
                    id_str,
                    &date,
                    agent,
                );
                grouped_lines.entry(insert_at).or_default().extend(lines);

                added_ids.insert(*new_id);
                ordered_ids.push((*new_id, id_str.clone()));
            }

            // All validations succeeded: persist the new high-water mark under the lock (unless --dry-run).
            if !ctx.dry_run {
                state.write_ids(&IdsState {
                    highest_id: next_num,
                })?;
            }

            let edits: Vec<LineEdit> = grouped_lines
                .into_iter()
                .map(|(idx, lines)| LineEdit::insert(idx, lines))
                .collect();
            let intent = TaskChangeIntent {
                added_ids,
                ..TaskChangeIntent::default()
            };
            let arch_owned = archive.cloned();
            let cfg_owned = cfg_clone.clone();

            Ok(DocWritePlan::Edits {
                edits,
                intent,
                build_value: Box::new(move |new_doc| {
                    let mut ids = Vec::with_capacity(ordered_ids.len());
                    let mut added = Vec::with_capacity(ordered_ids.len());
                    for (id, id_str) in ordered_ids {
                        let task = new_doc.task_by_id(id).ok_or_else(|| {
                            TodoError::new(
                                ErrorCode::WriteFailed,
                                format!("added task {id_str} missing after verification"),
                            )
                        })?;
                        ids.push(id_str);
                        added.push(build_task_json(
                            new_doc,
                            task,
                            arch_owned.as_ref(),
                            false,
                            now,
                            &cfg_owned,
                        ));
                    }
                    Ok((ids, added))
                }),
            })
        },
    )?;

    Ok(BatchAdded {
        ids: res.value.0,
        added: res.value.1,
    })
}

/// Parses the JSON lines of a batch into validated entries (R-WRITE-13), before anything is locked.
/// Shared by `todo add --from` and `todo split --from`; `default_section_flag` names the section of
/// an entry that gives none.
pub(crate) fn parse_batch_entries(
    raw: &str,
    default_section_flag: Option<&str>,
    cfg: &OpsConfig,
) -> Result<Vec<ValidatedBatchEntry>> {
    let mut entries = Vec::new();
    let mut seen_refs = HashMap::new();
    let mut seen_titles = Vec::<String>::new();

    for (line_idx, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let line_no = line_idx + 1;
        let parsed: BatchAddEntry = serde_json::from_str(trimmed).map_err(|e| {
            TodoError::new(
                ErrorCode::BadValue,
                format!("--from line {line_no}: invalid JSON: {e}"),
            )
            .with_line(line_no)
        })?;

        let title = sanitize_trimmed_value(&parsed.title);
        if title.is_empty() {
            return Err(TodoError::new(
                ErrorCode::BadValue,
                format!("--from line {line_no}: title cannot be empty"),
            )
            .with_line(line_no));
        }
        if seen_titles.iter().any(|t| t.eq_ignore_ascii_case(&title)) {
            return Err(TodoError::new(
                ErrorCode::DuplicateTitle,
                format!("--from line {line_no}: duplicate title {title:?} in batch"),
            )
            .with_line(line_no));
        }
        seen_titles.push(title.clone());

        let ref_name = match parsed.ref_name {
            Some(r) => {
                let clean = r.trim().to_string();
                if clean.is_empty() || clean.chars().any(char::is_whitespace) || clean.contains(',')
                {
                    return Err(TodoError::new(
                        ErrorCode::BadValue,
                        format!("--from line {line_no}: invalid ref {r:?}"),
                    )
                    .with_line(line_no));
                }
                if seen_refs.insert(clean.clone(), line_no).is_some() {
                    return Err(TodoError::new(
                        ErrorCode::BadValue,
                        format!("--from line {line_no}: duplicate ref {clean:?}"),
                    )
                    .with_line(line_no));
                }
                Some(clean)
            }
            None => None,
        };

        let mut extra = Vec::new();
        for (k, v) in parsed.fields {
            let key = k.trim().to_ascii_lowercase();
            if key.is_empty()
                || matches!(key.as_str(), "status" | "owner" | "claimed" | "note" | "title")
            {
                return Err(TodoError::new(
                    ErrorCode::BadValue,
                    format!("--from line {line_no}: cannot set field `{k}`"),
                )
                .with_line(line_no));
            }
            // `steps` keeps its line breaks: each line is one step.
            let value = if key == "steps" {
                v.trim().to_string()
            } else {
                sanitize_trimmed_value(&v)
            };
            extra.push((key, value));
        }

        let task_type = resolve_allowed("type", parsed.task_type.as_deref(), &extra, &cfg.types)?;
        let required = required_for_kind(&cfg.required_fields, task_type.as_deref());
        let priority = resolve_priority(parsed.priority.as_deref(), &extra, cfg, required)?;
        let difficulty = resolve_allowed(
            "difficulty",
            parsed.difficulty.as_deref(),
            &extra,
            &cfg.difficulties,
        )?;
        let area = resolve_allowed("area", parsed.area.as_deref(), &extra, &cfg.areas)?;
        let parent = parsed
            .milestone
            .as_deref()
            .map(sanitize_trimmed_value)
            .filter(|s| !s.is_empty())
            .or_else(|| resolve_opt(None, &extra, "milestone"));
        let goal = parsed
            .goal
            .as_deref()
            .map(sanitize_trimmed_value)
            .filter(|s| !s.is_empty())
            .or_else(|| resolve_opt(None, &extra, "goal"));
        milestone::check_new_entry_fields(
            task_type.as_deref(),
            parent.as_deref(),
            goal.as_deref(),
        )
        .map_err(|e| at_batch_line(e, line_no))?;
        let why = parsed
            .why
            .as_deref()
            .map(sanitize_trimmed_value)
            .filter(|s| !s.is_empty())
            .or_else(|| resolve_opt(None, &extra, "why"));
        let summary = parsed
            .summary
            .as_deref()
            .map(sanitize_trimmed_value)
            .filter(|s| !s.is_empty())
            .or_else(|| resolve_opt(None, &extra, "summary"));
        let files = parsed
            .files
            .as_deref()
            .map(sanitize_trimmed_value)
            .filter(|s| !s.is_empty())
            .or_else(|| resolve_opt(None, &extra, "files"));
        let read = parsed
            .read
            .as_deref()
            .map(sanitize_trimmed_value)
            .filter(|s| !s.is_empty())
            .or_else(|| resolve_opt(None, &extra, "read"));
        let problem = parsed
            .problem
            .as_deref()
            .map(sanitize_trimmed_value)
            .filter(|s| !s.is_empty())
            .or_else(|| resolve_opt(None, &extra, "problem"));
        let fix = parsed
            .fix
            .as_deref()
            .map(sanitize_trimmed_value)
            .filter(|s| !s.is_empty())
            .or_else(|| resolve_opt(None, &extra, "fix"));
        let steps = match &parsed.steps {
            Some(input) => steps_from_text(&input.text()),
            None => resolve_steps(None, &extra),
        };
        let accept = parsed
            .accept
            .as_deref()
            .map(sanitize_trimmed_value)
            .filter(|s| !s.is_empty())
            .or_else(|| resolve_opt(None, &extra, "accept"));
        let verify = parsed
            .verify
            .as_deref()
            .map(sanitize_trimmed_value)
            .filter(|s| !s.is_empty())
            .or_else(|| resolve_opt(None, &extra, "verify"));

        let depends_tokens: Vec<String> = match parsed.depends {
            Some(DependsInput::One(s)) => s
                .split(',')
                .map(sanitize_trimmed_value)
                .filter(|t| !t.is_empty())
                .collect(),
            Some(DependsInput::Many(items)) => items
                .into_iter()
                .map(|t| sanitize_trimmed_value(&t))
                .filter(|t| !t.is_empty())
                .collect(),
            None => resolve_opt(None, &extra, "depends")
                .map(|s| {
                    s.split(',')
                        .map(sanitize_trimmed_value)
                        .filter(|t| !t.is_empty())
                        .collect()
                })
                .unwrap_or_default(),
        };

        let depends_summary = if depends_tokens.is_empty() {
            None
        } else {
            Some(depends_tokens.join(", "))
        };
        check_required_fields(
            required,
            RequiredValues {
                priority: priority.as_deref(),
                difficulty: difficulty.as_deref(),
                task_type: task_type.as_deref(),
                area: area.as_deref(),
                why: why.as_deref(),
                summary: summary.as_deref(),
                depends: depends_summary.as_deref(),
                files: files.as_deref(),
                read: read.as_deref(),
                problem: problem.as_deref(),
                fix: fix.as_deref(),
                steps: !steps.is_empty(),
                accept: accept.as_deref(),
                verify: verify.as_deref(),
            },
            &extra,
        )?;

        let section = parsed
            .section
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .or_else(|| default_section_flag.map(str::to_string));

        entries.push(ValidatedBatchEntry {
            ref_name,
            title,
            section,
            priority,
            difficulty,
            task_type,
            area,
            milestone: parent,
            goal,
            why,
            summary,
            depends_tokens,
            files,
            read,
            problem,
            fix,
            steps,
            accept,
            verify,
            extra,
            line_no,
        });
    }

    if entries.is_empty() {
        return Err(TodoError::new(
            ErrorCode::BadValue,
            "no tasks found in --from input",
        ));
    }

    Ok(entries)
}

/// Refuses a batch title that an active task already has (R-WRITE-1), as `add` does.
pub(crate) fn check_batch_titles(
    doc: &Document,
    entries: &[ValidatedBatchEntry],
    cfg: &OpsConfig,
) -> Result<()> {
    for entry in entries {
        if let Some(existing) = doc.tasks.iter().find(|t| {
            !matches!(t.status, Status::Done | Status::Dropped)
                && t.title.trim().eq_ignore_ascii_case(&entry.title)
        }) {
            let existing_id = cfg.id_config.format_id(existing.id);
            return Err(TodoError::new(
                ErrorCode::DuplicateTitle,
                format!("task {existing_id} already has title {:?}", entry.title),
            )
            .with_task(existing_id));
        }
    }
    Ok(())
}

/// The `depends` value of a batch entry: its tokens, each `@ref` replaced by the ID of the entry it
/// names (R-WRITE-13). `None` when the entry has no dependencies.
pub(crate) fn resolve_batch_depends(
    tokens: &[String],
    ref_to_id: &HashMap<String, String>,
) -> Result<Option<String>> {
    let mut resolved_deps = Vec::with_capacity(tokens.len());
    for tok in tokens {
        if let Some(rname) = tok.strip_prefix('@') {
            let Some(target_id) = ref_to_id.get(rname) else {
                return Err(TodoError::new(
                    ErrorCode::BadValue,
                    format!("unknown dependency reference {tok:?}"),
                ));
            };
            resolved_deps.push(target_id.clone());
        } else {
            resolved_deps.push(tok.clone());
        }
    }
    if resolved_deps.is_empty() {
        Ok(None)
    } else {
        Ok(Some(resolved_deps.join(", ")))
    }
}

/// The lines of a batch entry as a new task under `milestone`, with its resolved `depends` value.
pub(crate) fn entry_lines(
    entry: &ValidatedBatchEntry,
    milestone: Option<&str>,
    depends: Option<&str>,
    id_str: &str,
    date: &str,
    agent: &str,
) -> Vec<String> {
    NewTask {
        title: &entry.title,
        priority: entry.priority.as_deref(),
        difficulty: entry.difficulty.as_deref(),
        task_type: entry.task_type.as_deref(),
        area: entry.area.as_deref(),
        milestone,
        goal: entry.goal.as_deref(),
        why: entry.why.as_deref(),
        summary: entry.summary.as_deref(),
        depends,
        files: entry.files.as_deref(),
        read: entry.read.as_deref(),
        problem: entry.problem.as_deref(),
        fix: entry.fix.as_deref(),
        steps: &entry.steps,
        accept: entry.accept.as_deref(),
        verify: entry.verify.as_deref(),
        extra: &entry.extra,
    }
    .lines(id_str, date, agent)
}
