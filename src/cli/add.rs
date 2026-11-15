//! `todo add` command: add a new task to a section with monotonic IDs (T-028; R-WRITE-1, R-ID-2).

use clap::Args as ClapArgs;

use crate::cli::{CommandKind, CommandMeta, Context};
use crate::doc::{
    CANONICAL_HEADING_SEPARATOR, CANONICAL_NOBODY_OWNER, Document, LineEdit, Status, TaskChangeIntent,
    TaskId, sanitize_trimmed_value,
};
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::{
    DocWritePlan, OpsConfig, build_task_json, edit_doc_with_state_and_write,
    matching_section_indices,
};
use crate::output::Output;
use crate::state::IdsState;

/// Metadata for the `add` command.
pub const META: CommandMeta = CommandMeta {
    name: "add",
    summary: "Add a new task to a section",
    kind: CommandKind::Write,
    details: "Adds a new task at the end of the target section using the next monotonic task ID across `TODO.md`, `TODO-archive.md`, and `ids.json`. Refuses duplicate titles among active tasks, missing required fields (default `priority`), and unknown or ambiguous sections.",
    examples: &[
        "todo add --title \"Fix login timeout\" --priority P1 --section Now --agent agent-1",
        "todo add --title \"Add metrics\" --priority P2 --type feature --area cli --agent coordinator",
    ],
};

/// Arguments for `todo add`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Title for the new task (required).
    #[arg(long, value_name = "TEXT")]
    pub title: String,

    /// Target section name (exact or unique substring match).
    #[arg(long, value_name = "NAME")]
    pub section: Option<String>,

    /// Priority (e.g. `P0`, `P1`, `P2`, `P3`).
    #[arg(long, value_name = "VALUE")]
    pub priority: Option<String>,

    /// Task type (e.g. `feature`, `bug`, `chore`).
    #[arg(long = "type", value_name = "VALUE")]
    pub task_type: Option<String>,

    /// Area (e.g. `cli`, `docs`).
    #[arg(long, value_name = "VALUE")]
    pub area: Option<String>,

    /// Dependencies (comma-separated task IDs).
    #[arg(long, value_name = "IDS")]
    pub depends: Option<String>,

    /// `files` field value.
    #[arg(long, value_name = "TEXT")]
    pub files: Option<String>,

    /// `problem` field value.
    #[arg(long, value_name = "TEXT")]
    pub problem: Option<String>,

    /// `fix` field value.
    #[arg(long, value_name = "TEXT")]
    pub fix: Option<String>,

    /// `accept` field value.
    #[arg(long, value_name = "TEXT")]
    pub accept: Option<String>,

    /// Additional field `key=value` (repeatable).
    #[arg(long = "field", value_name = "KEY=VALUE")]
    pub fields: Vec<String>,
}

/// Runs `todo add --title ...`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let clean_title = sanitize_trimmed_value(&args.title);
    if clean_title.is_empty() {
        return Err(TodoError::new(
            ErrorCode::BadValue,
            "--title cannot be empty",
        ));
    }

    let extra_fields = parse_extra_fields(&args.fields)?;
    let cfg = OpsConfig::default();

    // Validate priority (and required fields) before taking the lock.
    let priority_val = resolve_priority(args.priority.as_deref(), &extra_fields, &cfg)?;
    let type_val = resolve_opt(args.task_type.as_deref(), &extra_fields, "type");
    let area_val = resolve_opt(args.area.as_deref(), &extra_fields, "area");
    let depends_val = resolve_opt(args.depends.as_deref(), &extra_fields, "depends");
    let files_val = resolve_opt(args.files.as_deref(), &extra_fields, "files");
    let problem_val = resolve_opt(args.problem.as_deref(), &extra_fields, "problem");
    let fix_val = resolve_opt(args.fix.as_deref(), &extra_fields, "fix");
    let accept_val = resolve_opt(args.accept.as_deref(), &extra_fields, "accept");

    let cfg_clone = cfg.clone();
    let res = edit_doc_with_state_and_write(
        ctx,
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

            // 2. Resolve target section (R-WRITE-1).
            let section_idx = resolve_target_section(doc, args.section.as_deref(), &cfg)?;
            let section = &doc.sections[section_idx];

            // 3. Allocate next monotonic ID across TODO.md, archive, and ids.json (R-ID-2).
            let max_doc = doc.tasks.iter().map(|t| t.id.number()).max().unwrap_or(0);
            let max_arch = archive
                .iter()
                .flat_map(|a| a.tasks.iter())
                .map(|t| t.id.number())
                .max()
                .unwrap_or(0);
            let max_state = state.read_ids()?.map_or(0, |s| s.highest_id);
            let next_num = max_doc
                .max(max_arch)
                .max(max_state)
                .checked_add(1)
                .ok_or_else(|| TodoError::new(ErrorCode::WriteFailed, "task ID overflow"))?;
            let new_id = TaskId(next_num);
            let id_str = cfg.id_config.format_id(new_id);

            // Persist the new high-water mark while holding the write lock.
            state.write_ids(&IdsState {
                highest_id: next_num,
            })?;

            // 4. Build canonical lines for the new task (docs/format.md §10).
            let mut lines = Vec::new();
            lines.push(String::new());
            lines.push(format!(
                "### {id_str}{CANONICAL_HEADING_SEPARATOR}{clean_title}"
            ));
            lines.push("- status: open".to_string());
            lines.push(format!("- owner: {CANONICAL_NOBODY_OWNER}"));

            let mut combined_parts = Vec::new();
            if let Some(ref p) = priority_val {
                combined_parts.push(format!("priority: {p}"));
            }
            if let Some(ref t) = type_val {
                combined_parts.push(format!("type: {t}"));
            }
            if let Some(ref a) = area_val {
                combined_parts.push(format!("area: {a}"));
            }
            if !combined_parts.is_empty() {
                lines.push(format!("- {}", combined_parts.join(" \u{00b7} ")));
            }

            for (key, val_opt) in [
                ("depends", &depends_val),
                ("files", &files_val),
                ("problem", &problem_val),
                ("fix", &fix_val),
                ("accept", &accept_val),
            ] {
                if let Some(v) = val_opt {
                    lines.push(format!("- {key}: {v}"));
                }
            }

            for (k, v) in &extra_fields {
                if !is_known_add_key(k) {
                    lines.push(format!("- {k}: {v}"));
                }
            }

            let date = OpsConfig::format_note_date(now);
            lines.push(format!("- note ({date}, {agent}): added"));

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
        pairs.push((key, sanitize_trimmed_value(v)));
    }
    Ok(pairs)
}

fn is_known_add_key(key: &str) -> bool {
    matches!(
        key,
        "priority" | "type" | "area" | "depends" | "files" | "problem" | "fix" | "accept"
    )
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

fn resolve_priority(
    flag: Option<&str>,
    extra: &[(String, String)],
    cfg: &OpsConfig,
) -> Result<Option<String>> {
    let raw = resolve_opt(flag, extra, "priority");
    match raw {
        Some(val) => {
            let Some(canonical) = cfg
                .priorities
                .iter()
                .find(|p| p.eq_ignore_ascii_case(&val))
            else {
                return Err(TodoError::new(
                    ErrorCode::BadValue,
                    format!(
                        "invalid priority '{val}'; expected one of: {}",
                        cfg.priorities.join(", ")
                    ),
                ));
            };
            Ok(Some(canonical.clone()))
        }
        None => {
            if cfg
                .required_fields
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

fn resolve_target_section(
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

    let all_names: Vec<&str> = doc.sections.iter().map(|s| s.name.as_str()).collect();
    let all_joined = all_names.join(", ");

    let query = match section_flag {
        Some(s) => s,
        None => match cfg.default_section.as_deref() {
            Some(def) => def,
            None if doc.sections.len() == 1 => return Ok(0),
            None => {
                return Err(TodoError::new(
                    ErrorCode::UnknownSection,
                    format!(
                        "multiple sections exist; pass --section <name>. Available sections: {all_joined}"
                    ),
                ));
            }
        },
    };

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
