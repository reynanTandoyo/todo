//! The project files that carry todo's installed content (`R-MIG-2`): the managed rules block in
//! `TODO.md`, `AGENTS.md` and `CLAUDE.md`, the Claude Code skill and the Claude Code hooks.
//!
//! A file is out of date when `migrate --yes` would change it. Only files that already exist and
//! already carry todo's content are changed; nothing is created here. Each file changes only
//! inside the parts todo owns: the lines between the rules markers, the whole skill file, and the
//! managed hook entries.

use std::path::{Path, PathBuf};

use serde::Serialize;
use similar::TextDiff;

use super::hooks_file::{HookEdit, apply_hook_edits, outdated_hook_entries};
use super::{
    ManagedBlockStatus, check_managed_block_in_text, render_claude_skill, upsert_managed_block,
};
use crate::atomic::{
    ReadFile, atomic_replace, atomic_write_bytes, read_file_with_snapshot, validate_utf8,
};
use crate::error::Result;
use crate::find::fs_path;

/// The Claude Code skill, relative to the project folder (`R-AGENT-5`).
pub const SKILL_REL_PATH: &str = ".claude/skills/todo/SKILL.md";

/// The Claude Code settings that hold the hooks, relative to the project folder (`R-AGENT-4`).
pub const SETTINGS_REL_PATH: &str = ".claude/settings.json";

/// Where `hooks install` keeps the settings file as it was before its change.
pub const BACKUP_REL_PATH: &str = ".claude/settings.json.bak";

/// The agent files besides `TODO.md` that may carry a rules block, in the folder of `TODO.md`.
const RULES_FILES: [&str; 2] = ["AGENTS.md", "CLAUDE.md"];

/// What kind of project file an update changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    /// The `<!-- todo:begin -->` ... `<!-- todo:end -->` rules block (`R-AGENT-3`).
    RulesBlock,
    /// The Claude Code skill (`R-AGENT-5`).
    Skill,
    /// The managed Claude Code hook entries (`R-AGENT-4`).
    Hooks,
}

impl ItemKind {
    /// What the item is, as people read it: `agent rules block`, `Claude Code skill`, or
    /// `Claude Code hooks (SessionStart, PreToolUse)` with the events that are updated.
    #[must_use]
    pub fn describe(self, events: &[&str]) -> String {
        match self {
            Self::RulesBlock => "agent rules block".to_string(),
            Self::Skill => "Claude Code skill".to_string(),
            Self::Hooks => format!("Claude Code hooks ({})", events.join(", ")),
        }
    }
}

/// One installed project file that is out of date, with what `migrate --yes` would write to it.
#[derive(Debug)]
pub struct ProjectUpdate {
    /// What the file holds.
    pub kind: ItemKind,
    /// The file as people name it: `TODO.md`, `AGENTS.md`, `.claude/settings.json`.
    pub file: String,
    /// Where the file is.
    pub path: PathBuf,
    /// The file as it was read; the write is refused if it has changed since.
    pub old: ReadFile,
    /// The bytes it gets.
    pub new_bytes: Vec<u8>,
    /// The hook events whose entries change (hooks only).
    pub events: Vec<&'static str>,
    /// Where the file is copied first (hooks only).
    pub backup: Option<PathBuf>,
}

impl ProjectUpdate {
    /// The unified diff from the file's current text to the text it gets.
    #[must_use]
    pub fn diff(&self) -> String {
        let old_text = String::from_utf8_lossy(&self.old.bytes);
        let new_text = String::from_utf8_lossy(&self.new_bytes);
        TextDiff::from_lines(old_text.as_ref(), new_text.as_ref())
            .unified_diff()
            .header(&format!("a/{}", self.file), &format!("b/{}", self.file))
            .to_string()
    }
}

/// The out-of-date project files, and the warnings for files that could not be checked.
#[derive(Debug, Default)]
pub struct ProjectPlan {
    /// The updates `migrate --yes` would make, in the order: rules blocks, skill, hooks.
    pub updates: Vec<ProjectUpdate>,
    /// Files that could not be read or parsed, and so were left alone.
    pub warnings: Vec<String>,
}

/// The project folder for `todo_path` (`TODO.md`): the folder that holds it. A bare file name is in
/// the current folder. Every agent file, the skill and the hooks are kept here (`R-AGENT-3`).
#[must_use]
pub fn project_dir_of(todo_path: &Path) -> PathBuf {
    match todo_path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

/// Plans the updates for the project folder of `todo_path`: the folder that holds `TODO.md`.
#[must_use]
pub fn plan(todo_path: &Path) -> ProjectPlan {
    let dir = project_dir_of(todo_path);
    let dir = dir.as_path();
    let todo_name = todo_path.file_name().map_or_else(
        || "TODO.md".to_string(),
        |n| n.to_string_lossy().into_owned(),
    );

    let mut plan = ProjectPlan::default();
    let mut rules: Vec<(String, PathBuf)> = vec![(todo_name.clone(), todo_path.to_path_buf())];
    for name in RULES_FILES {
        if name != todo_name {
            rules.push((name.to_string(), dir.join(name)));
        }
    }
    for (label, path) in rules {
        collect(&mut plan, rules_block_update(&path, &label));
    }
    collect(&mut plan, skill_update(dir));
    collect(&mut plan, hooks_update(dir));
    plan
}

/// Adds one file's result to the plan: an update, nothing, or a warning.
fn collect(plan: &mut ProjectPlan, result: std::result::Result<Option<ProjectUpdate>, String>) {
    match result {
        Ok(Some(update)) => plan.updates.push(update),
        Ok(None) => {}
        Err(warning) => plan.warnings.push(warning),
    }
}

/// One line for `todo summary` naming the out-of-date files and `todo migrate --yes`, or `None`
/// when nothing is out of date (`R-MIG-2`). Files that cannot be checked are not named.
#[must_use]
pub fn out_of_date_line(todo_path: &Path) -> Option<String> {
    let names: Vec<String> = plan(todo_path)
        .updates
        .into_iter()
        .map(|u| u.file)
        .collect();
    if names.is_empty() {
        return None;
    }
    Some(format!(
        "project files out of date ({}): run todo migrate --yes",
        names.join(", ")
    ))
}

/// The bytes of `bytes` with the managed rules block brought up to date, or `None` when the file
/// has no block, the block is current, or nothing would change (`R-AGENT-3`). `label` names the
/// file in errors.
pub fn rules_block_fix(bytes: &[u8], label: &str) -> Result<Option<Vec<u8>>> {
    let text = validate_utf8(bytes, label)?;
    if !matches!(
        check_managed_block_in_text(text),
        ManagedBlockStatus::Outdated { .. }
    ) {
        return Ok(None);
    }
    let new_bytes = upsert_managed_block(Some(bytes), label)?;
    Ok((new_bytes.as_slice() != bytes).then_some(new_bytes))
}

/// Writes one planned update: the backup first (hooks), then the file, guarded by the snapshot
/// it was planned from (`R-IO-2`).
pub fn apply(update: &ProjectUpdate) -> Result<()> {
    if let Some(backup) = &update.backup {
        atomic_write_bytes(backup, &update.old.bytes)?;
    }
    atomic_replace(&update.path, &update.old.snapshot, &update.new_bytes)
}

/// Reads an existing file, or `None` when there is no such file. Errors become warnings.
fn read_existing(path: &Path, label: &str) -> std::result::Result<Option<ReadFile>, String> {
    if !fs_path(path).is_file() {
        return Ok(None);
    }
    read_file_with_snapshot(path)
        .map(Some)
        .map_err(|e| format!("{label}: {}; not checked", e.message))
}

fn update(
    kind: ItemKind,
    file: &str,
    path: PathBuf,
    old: ReadFile,
    new_bytes: Vec<u8>,
) -> ProjectUpdate {
    ProjectUpdate {
        kind,
        file: file.to_string(),
        path,
        old,
        new_bytes,
        events: Vec::new(),
        backup: None,
    }
}

fn rules_block_update(
    path: &Path,
    label: &str,
) -> std::result::Result<Option<ProjectUpdate>, String> {
    let Some(old) = read_existing(path, label)? else {
        return Ok(None);
    };
    let new_bytes = rules_block_fix(&old.bytes, label)
        .map_err(|e| format!("{label}: {}; not checked", e.message))?;
    Ok(new_bytes.map(|new_bytes| {
        update(
            ItemKind::RulesBlock,
            label,
            path.to_path_buf(),
            old,
            new_bytes,
        )
    }))
}

fn skill_update(dir: &Path) -> std::result::Result<Option<ProjectUpdate>, String> {
    let path = dir.join(SKILL_REL_PATH);
    let Some(old) = read_existing(&path, SKILL_REL_PATH)? else {
        return Ok(None);
    };
    let text = std::str::from_utf8(&old.bytes)
        .map_err(|_| format!("{SKILL_REL_PATH}: not valid UTF-8; not checked"))?;
    // Compared as `agents check` compares rules text: line endings do not count.
    let wanted = render_claude_skill();
    if text.replace("\r\n", "\n") == wanted {
        return Ok(None);
    }
    Ok(Some(update(
        ItemKind::Skill,
        SKILL_REL_PATH,
        path,
        old,
        wanted.into_bytes(),
    )))
}

fn hooks_update(dir: &Path) -> std::result::Result<Option<ProjectUpdate>, String> {
    let path = dir.join(SETTINGS_REL_PATH);
    let Some(old) = read_existing(&path, SETTINGS_REL_PATH)? else {
        return Ok(None);
    };
    let text = std::str::from_utf8(&old.bytes)
        .map_err(|_| format!("{SETTINGS_REL_PATH}: not valid UTF-8; hooks not checked"))?;
    let edits: Vec<HookEdit> = outdated_hook_entries(text)
        .map_err(|e| format!("{SETTINGS_REL_PATH}: {e}; hooks not checked"))?;
    if edits.is_empty() {
        return Ok(None);
    }
    let new_text = apply_hook_edits(text, &edits)
        .ok_or_else(|| format!("{SETTINGS_REL_PATH}: could not place the hooks; not changed"))?;
    // The new text must parse, and must have no managed entry left to change.
    let left = outdated_hook_entries(&new_text)
        .map_err(|e| format!("{SETTINGS_REL_PATH}: {e}; not changed"))?;
    if !left.is_empty() {
        return Err(format!(
            "{SETTINGS_REL_PATH}: hooks did not update; not changed"
        ));
    }
    let mut item = update(
        ItemKind::Hooks,
        SETTINGS_REL_PATH,
        path,
        old,
        new_text.into_bytes(),
    );
    item.events = edits.iter().map(|e| e.event).collect();
    item.backup = Some(dir.join(BACKUP_REL_PATH));
    Ok(Some(item))
}
