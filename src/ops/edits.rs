//! Canonical whole-line task edit builder (`R-EDIT-1`, `R-EDIT-5`, `R-EDIT-9`, `format.md` §8–§10).

use std::collections::BTreeMap;

use crate::doc::{
    CANONICAL_HEADING_SEPARATOR, CANONICAL_NOBODY_OWNER, Document, FieldLocation, LineEdit, Status,
    Task, TaskChangeIntent, canonical_field_rank, sanitize_trimmed_value,
};

/// Builds non-overlapping, canonical [`LineEdit`]s for a single [`Task`] while keeping every
/// untouched byte identical (`R-EDIT-1`, `R-EDIT-9`, `docs/format.md` §8–§10).
#[derive(Debug)]
pub struct TaskEditBuilder<'a> {
    doc: &'a Document,
    task: &'a Task,
    /// Planned 0-based line modifications (`Some(new_text)` = replace, `None` = remove).
    line_changes: BTreeMap<usize, Option<String>>,
    /// Missing fields to insert (`(insert_line, rank, seq, full_line_text)`).
    field_inserts: Vec<(usize, usize, usize, String)>,
    /// Notes to append at `task.note_insert_line()`.
    notes_to_append: Vec<String>,
    /// Planned updates to combined priority line (`priority`, `type`, `area`).
    priority_override: Option<Option<String>>,
    type_override: Option<Option<String>>,
    area_override: Option<Option<String>>,
    seq: usize,
}

impl<'a> TaskEditBuilder<'a> {
    /// Creates a new [`TaskEditBuilder`] targeting `task` in `doc`.
    #[must_use]
    pub fn new(doc: &'a Document, task: &'a Task) -> Self {
        Self {
            doc,
            task,
            line_changes: BTreeMap::new(),
            field_inserts: Vec::new(),
            notes_to_append: Vec::new(),
            priority_override: None,
            type_override: None,
            area_override: None,
            seq: 0,
        }
    }

    /// Updates the task's title, rewriting the heading line in canonical form (`### T-007 · <title>`).
    pub fn set_title(&mut self, new_title: &str) {
        let clean = sanitize_trimmed_value(new_title);
        let id_str = self.doc.id_config.format_id(self.task.id);
        let line_text = format!("### {id_str}{CANONICAL_HEADING_SEPARATOR}{clean}");
        self.replace_line_if_changed(self.task.heading_line, line_text);
    }

    /// Sets `- status: <status>`, updating the existing line or inserting in canonical order (§10).
    pub fn set_status(&mut self, status: Status) {
        let text = format!("- status: {}", status.as_str());
        match self.task.status_line {
            Some(line) => self.replace_line_if_changed(line, text),
            None => self.queue_field_insert("status", text),
        }
    }

    /// Sets `- owner: <agent>` (or `- owner: —` for nobody), updating or inserting in canonical order.
    pub fn set_owner(&mut self, owner: Option<&str>) {
        let val = match owner {
            Some(name) => sanitize_trimmed_value(name),
            None => CANONICAL_NOBODY_OWNER.to_string(),
        };
        let text = format!("- owner: {val}");
        match self.task.owner_line {
            Some(line) => self.replace_line_if_changed(line, text),
            None => self.queue_field_insert("owner", text),
        }
    }

    /// Clears an existing owner to `- owner: —` if the task currently has an owner (`R-WRITE-3`).
    ///
    /// Leaves an unowned task's `- owner:` line untouched so no unrelated bytes change (`R-EDIT-9`).
    pub fn clear_owner_if_present(&mut self) {
        if self.task.owner.is_some()
            && let Some(line) = self.task.owner_line
        {
            self.replace_line_if_changed(line, format!("- owner: {CANONICAL_NOBODY_OWNER}"));
        }
    }

    /// Sets `- claimed: <timestamp>`, updating the existing line or inserting in canonical order.
    pub fn set_claimed(&mut self, timestamp: &str) {
        let clean = sanitize_trimmed_value(timestamp);
        let text = format!("- claimed: {clean}");
        match self.task.claimed_line {
            Some(line) => self.replace_line_if_changed(line, text),
            None => self.queue_field_insert("claimed", text),
        }
    }

    /// Removes the `- claimed:` line if present (`R-WRITE-9`, `docs/format.md` §10).
    pub fn remove_claimed(&mut self) {
        if let Some(line) = self.task.claimed_line {
            self.line_changes.insert(line, None);
        }
    }

    /// Sets a field by key (`priority`, `type`, `area`, known fields, or unknown fields),
    /// respecting the combined priority line rules of `docs/format.md` §8 and canonical order §10.
    pub fn set_field(&mut self, key: &str, value: &str) {
        let clean = sanitize_trimmed_value(value);
        let lower = key.trim().to_ascii_lowercase();
        match lower.as_str() {
            "priority" => self.priority_override = Some(Some(clean)),
            "type" => self.type_override = Some(Some(clean)),
            "area" => self.area_override = Some(Some(clean)),
            "status" => {
                if let Some(st) = Status::parse(&clean) {
                    self.set_status(st);
                }
            }
            "owner" => self.set_owner(if clean.is_empty() { None } else { Some(&clean) }),
            "claimed" => self.set_claimed(&clean),
            "depends" => self.set_simple_field("depends", self.task.depends_line, &clean),
            "files" => self.set_simple_field("files", self.task.files_line, &clean),
            "problem" => self.set_simple_field("problem", self.task.problem_line, &clean),
            "fix" => self.set_simple_field("fix", self.task.fix_line, &clean),
            "accept" => self.set_simple_field("accept", self.task.accept_line, &clean),
            "commit" => self.set_simple_field("commit", self.task.commit_line, &clean),
            _ => {
                let existing = self
                    .task
                    .unknown_fields
                    .iter()
                    .find(|f| f.key == lower)
                    .map(|f| f.line);
                if let Some(line) = existing {
                    if let Some(ref detail) = self.task.priority_detail
                        && detail.line == line
                    {
                        // Leave combined-priority extra unknown parts untouched unless needed.
                        self.queue_field_insert_owned(lower.clone(), format!("- {lower}: {clean}"));
                    } else {
                        self.replace_line_if_changed(line, format!("- {lower}: {clean}"));
                    }
                } else {
                    self.queue_field_insert_owned(lower.clone(), format!("- {lower}: {clean}"));
                }
            }
        }
    }

    /// Removes a field by key (`docs/format.md` §8, §10).
    pub fn unset_field(&mut self, key: &str) {
        let lower = key.trim().to_ascii_lowercase();
        match lower.as_str() {
            "priority" => self.priority_override = Some(None),
            "type" => self.type_override = Some(None),
            "area" => self.area_override = Some(None),
            "claimed" => self.remove_claimed(),
            "depends" => self.remove_if_some(self.task.depends_line),
            "files" => self.remove_if_some(self.task.files_line),
            "problem" => self.remove_if_some(self.task.problem_line),
            "fix" => self.remove_if_some(self.task.fix_line),
            "accept" => self.remove_if_some(self.task.accept_line),
            "commit" => self.remove_if_some(self.task.commit_line),
            _ => {
                if let Some(entry) = self.task.unknown_fields.iter().find(|f| f.key == lower)
                    && self
                        .task
                        .priority_detail
                        .as_ref()
                        .is_none_or(|d| d.line != entry.line)
                {
                    self.line_changes.insert(entry.line, None);
                }
            }
        }
    }

    /// Appends a dated note after the last line of the task block (`docs/format.md` §9).
    pub fn add_note(&mut self, date: &str, agent: &str, text: &str) {
        let clean = sanitize_trimmed_value(text);
        self.notes_to_append
            .push(format!("- note ({date}, {agent}): {clean}"));
    }

    /// Finalizes the planned edits into sorted, non-overlapping [`LineEdit`]s and the matching
    /// [`TaskChangeIntent`] for `R-EDIT-7` verification.
    #[must_use]
    pub fn build(mut self) -> (Vec<LineEdit>, TaskChangeIntent) {
        self.flush_priority_type_area();

        let mut edits = Vec::new();
        for (line, change) in self.line_changes {
            match change {
                Some(text) => edits.push(LineEdit::replace_line(line, text)),
                None => edits.push(LineEdit::remove(line..line + 1)),
            }
        }

        // Group insertions by line index; fields stay in canonical field rank order, and appended
        // notes always go after any fields inserted at the end of the block.
        let mut grouped_inserts: BTreeMap<usize, Vec<String>> = BTreeMap::new();
        self.field_inserts
            .sort_by_key(|(line, rank, seq, _)| (*line, *rank, *seq));
        for (line, _, _, text) in self.field_inserts {
            grouped_inserts.entry(line).or_default().push(text);
        }
        if !self.notes_to_append.is_empty() {
            grouped_inserts
                .entry(self.task.note_insert_line())
                .or_default()
                .extend(self.notes_to_append);
        }

        for (line, lines) in grouped_inserts {
            edits.push(LineEdit::insert(line, lines));
        }

        edits.sort_by_key(|e| (e.range.start, e.range.end));
        (edits, TaskChangeIntent::modify_one(self.task.id))
    }

    fn set_simple_field(&mut self, key: &str, existing_line: Option<usize>, clean_val: &str) {
        let text = format!("- {key}: {clean_val}");
        match existing_line {
            Some(line) => self.replace_line_if_changed(line, text),
            None => self.queue_field_insert_owned(key.to_string(), text),
        }
    }

    fn remove_if_some(&mut self, line: Option<usize>) {
        if let Some(idx) = line {
            self.line_changes.insert(idx, None);
        }
    }

    fn replace_line_if_changed(&mut self, line: usize, new_text: String) {
        if self
            .doc
            .line_doc
            .line(line)
            .is_some_and(|existing| existing.text == new_text)
        {
            return;
        }
        self.line_changes.insert(line, Some(new_text));
    }

    fn queue_field_insert(&mut self, key: &str, text: String) {
        let insert_at = self.task.insert_line_for_field(key);
        let rank = canonical_field_rank(key);
        let seq = self.seq;
        self.seq += 1;
        self.field_inserts.push((insert_at, rank, seq, text));
    }

    fn queue_field_insert_owned(&mut self, key: String, text: String) {
        self.queue_field_insert(&key, text);
    }

    fn flush_priority_type_area(&mut self) {
        if self.priority_override.is_none()
            && self.type_override.is_none()
            && self.area_override.is_none()
        {
            return;
        }

        let has_priority_line = self.task.priority_detail.is_some();
        let priority_is_removed = matches!(self.priority_override, Some(None));

        if let Some(ref detail) = self.task.priority_detail {
            // Reconstruct combined priority line if priority isn't being completely deleted, or if
            // extra parts live on it.
            let new_priority = match &self.priority_override {
                Some(Some(p)) => Some(p.clone()),
                Some(None) => None,
                None => detail.priority.clone(),
            };

            let mut parts: Vec<String> = Vec::new();
            let mut extra_entries: Vec<(String, String)> = Vec::new();

            let type_in_combined = matches!(
                self.task.task_type_loc,
                Some(FieldLocation::CombinedPriority(_))
            );
            let area_in_combined =
                matches!(self.task.area_loc, Some(FieldLocation::CombinedPriority(_)));

            for (k, v) in &detail.extra_parts {
                match k.as_str() {
                    "type" => match &self.type_override {
                        Some(Some(new_t)) => {
                            extra_entries.push(("type".to_string(), new_t.clone()))
                        }
                        Some(None) => {}
                        None => extra_entries.push((k.clone(), v.clone())),
                    },
                    "area" => match &self.area_override {
                        Some(Some(new_a)) => {
                            extra_entries.push(("area".to_string(), new_a.clone()))
                        }
                        Some(None) => {}
                        None => extra_entries.push((k.clone(), v.clone())),
                    },
                    _ => extra_entries.push((k.clone(), v.clone())),
                }
            }

            // Per docs/format.md §8: a new `type` or `area` goes into the combined line when the
            // task has a priority line (unless priority itself is being removed).
            if !priority_is_removed {
                if !type_in_combined
                    && self.task.task_type_loc.is_none()
                    && let Some(Some(new_t)) = &self.type_override
                {
                    extra_entries.push(("type".to_string(), new_t.clone()));
                }
                if !area_in_combined
                    && self.task.area_loc.is_none()
                    && let Some(Some(new_a)) = &self.area_override
                {
                    extra_entries.push(("area".to_string(), new_a.clone()));
                }
            }

            if let Some(p) = new_priority {
                parts.push(p);
                for (k, v) in extra_entries {
                    parts.push(format!("{k}: {v}"));
                }
                self.replace_line_if_changed(
                    detail.line,
                    format!("- priority: {}", parts.join(" · ")),
                );
            } else {
                // Priority was unset: remove priority line and preserve remaining extra parts on
                // their own lines.
                self.line_changes.insert(detail.line, None);
                for (k, v) in extra_entries {
                    self.queue_field_insert(&k, format!("- {k}: {v}"));
                }
            }

            // If `type` or `area` lived on its own line, edit/remove it in place (`docs/format.md` §15).
            if let Some(FieldLocation::OwnLine(line)) = self.task.task_type_loc
                && let Some(ref ov) = self.type_override
            {
                match ov {
                    Some(v) => self.replace_line_if_changed(line, format!("- type: {v}")),
                    None => {
                        self.line_changes.insert(line, None);
                    }
                }
            }
            if let Some(FieldLocation::OwnLine(line)) = self.task.area_loc
                && let Some(ref ov) = self.area_override
            {
                match ov {
                    Some(v) => self.replace_line_if_changed(line, format!("- area: {v}")),
                    None => {
                        self.line_changes.insert(line, None);
                    }
                }
            }
        } else {
            // Task did not have a `- priority:` line.
            let mut combined_parts: Vec<String> = Vec::new();
            if let Some(Some(p)) = &self.priority_override {
                combined_parts.push(p.clone());
            }

            let adding_priority = matches!(self.priority_override, Some(Some(_)));

            if let Some(ref ov) = self.type_override {
                match self.task.task_type_loc {
                    Some(FieldLocation::OwnLine(line)) => match ov {
                        Some(v) => self.replace_line_if_changed(line, format!("- type: {v}")),
                        None => {
                            self.line_changes.insert(line, None);
                        }
                    },
                    _ => {
                        if let Some(v) = ov {
                            if adding_priority {
                                combined_parts.push(format!("type: {v}"));
                            } else {
                                self.queue_field_insert("type", format!("- type: {v}"));
                            }
                        }
                    }
                }
            }

            if let Some(ref ov) = self.area_override {
                match self.task.area_loc {
                    Some(FieldLocation::OwnLine(line)) => match ov {
                        Some(v) => self.replace_line_if_changed(line, format!("- area: {v}")),
                        None => {
                            self.line_changes.insert(line, None);
                        }
                    },
                    _ => {
                        if let Some(v) = ov {
                            if adding_priority {
                                combined_parts.push(format!("area: {v}"));
                            } else {
                                self.queue_field_insert("area", format!("- area: {v}"));
                            }
                        }
                    }
                }
            }

            if !has_priority_line && !combined_parts.is_empty() {
                self.queue_field_insert(
                    "priority",
                    format!("- priority: {}", combined_parts.join(" · ")),
                );
            }
        }
    }
}
