//! Canonical whole-line task edit builder (`R-EDIT-1`, `R-EDIT-5`, `R-EDIT-9`, `format.md` §8–§10).

use std::collections::BTreeMap;

use crate::doc::{
    CANONICAL_HEADING_SEPARATOR, CANONICAL_NOBODY_OWNER, COMBINED_PART_SEPARATOR, CombinedPart,
    Document, FieldLocation, LineEdit, Status, Task, TaskChangeIntent, canonical_field_rank,
    sanitize_trimmed_value,
};
use crate::ops::task_ops::steps_from_text;

/// A segment of a combined priority line being rebuilt: its key when it is a field, and its text as
/// it will be written.
type PartEntry = (Option<String>, String);

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
    /// Planned updates to combined priority line (`priority`, `difficulty`, `type`, `area`).
    priority_override: Option<Option<String>>,
    difficulty_override: Option<Option<String>>,
    type_override: Option<Option<String>>,
    area_override: Option<Option<String>>,
    /// Planned updates to other parts of the combined priority line, by key (`milestone`, `goal`,
    /// `why`, `depends`, ... and unknown parts); `None` removes the part.
    combined_part_overrides: BTreeMap<String, Option<String>>,
    /// Whole-range replacements of a multi-line field, such as the `steps` field and its lines
    /// (`R-DETAIL-1`), keyed by the first line of the range: a later change to the same field
    /// replaces an earlier one, as a later change to a single line does.
    block_edits: BTreeMap<usize, LineEdit>,
    /// The step lines of a `steps` field that has no line of its own yet, to be inserted in canonical
    /// order by `build`; a later `set_steps` replaces them and `unset_steps` drops them.
    steps_insert: Option<Vec<String>>,
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
            difficulty_override: None,
            type_override: None,
            area_override: None,
            combined_part_overrides: BTreeMap::new(),
            block_edits: BTreeMap::new(),
            steps_insert: None,
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
        self.set_line_field("status", self.task.status_line, status.as_str());
    }

    /// Sets `- owner: <agent>` (or `- owner: —` for nobody), updating or inserting in canonical order.
    pub fn set_owner(&mut self, owner: Option<&str>) {
        let val = match owner {
            Some(name) => sanitize_trimmed_value(name),
            None => CANONICAL_NOBODY_OWNER.to_string(),
        };
        self.set_line_field("owner", self.task.owner_line, &val);
    }

    /// Clears an existing owner to `- owner: —` if the task currently has an owner (`R-WRITE-3`).
    ///
    /// Leaves an unowned task's `- owner:` line untouched so no unrelated bytes change (`R-EDIT-9`).
    pub fn clear_owner_if_present(&mut self) {
        if self.task.owner.is_some() && self.task.owner_line.is_some() {
            self.set_line_field("owner", self.task.owner_line, CANONICAL_NOBODY_OWNER);
        }
    }

    /// Sets `- claimed: <timestamp>`, updating the existing line or inserting in canonical order.
    pub fn set_claimed(&mut self, timestamp: &str) {
        let clean = sanitize_trimmed_value(timestamp);
        self.set_line_field("claimed", self.task.claimed_line, &clean);
    }

    /// Removes the `- claimed:` line if present (`R-WRITE-9`, `docs/format.md` §10), and with it
    /// the `- session:` line, which only describes a claim (`R-CLAIM-7`).
    pub fn remove_claimed(&mut self) {
        self.unset_line_field("claimed", self.task.claimed_line);
        self.remove_session();
    }

    /// Writes `- session: <id>` for a new claim, or removes an existing `session` line when the
    /// claim has no session (`R-CLAIM-7`).
    pub fn set_session(&mut self, session: Option<&str>) {
        match session {
            Some(id) => self.set_field("session", id),
            None => self.remove_session(),
        }
    }

    fn remove_session(&mut self) {
        let existing = self.unknown_line("session");
        self.unset_line_field("session", existing);
    }

    /// Sets a field by key (`priority`, `difficulty`, `type`, `area`, known fields, or unknown fields),
    /// respecting the combined priority line rules of `docs/format.md` §8 and canonical order §10.
    pub fn set_field(&mut self, key: &str, value: &str) {
        let lower = key.trim().to_ascii_lowercase();
        // `steps` keeps its line breaks: each line of the value is one step (`R-DETAIL-1`).
        if lower == "steps" {
            self.set_steps(&steps_from_text(value));
            return;
        }
        let clean = sanitize_trimmed_value(value);
        match lower.as_str() {
            "priority" => self.priority_override = Some(Some(clean)),
            "difficulty" => self.difficulty_override = Some(Some(clean)),
            "type" => self.type_override = Some(Some(clean)),
            "area" => self.area_override = Some(Some(clean)),
            "status" => {
                if let Some(st) = Status::parse(&clean) {
                    self.set_status(st);
                }
            }
            "owner" => self.set_owner(if clean.is_empty() { None } else { Some(&clean) }),
            "claimed" => self.set_claimed(&clean),
            "milestone" => self.set_part_field("milestone", self.task.milestone_loc, &clean),
            "goal" => self.set_part_field("goal", self.task.goal_loc, &clean),
            "why" => self.set_line_field("why", self.task.why_line, &clean),
            "depends" => self.set_line_field("depends", self.task.depends_line, &clean),
            "files" => self.set_line_field("files", self.task.files_line, &clean),
            "read" => self.set_line_field("read", self.task.read_line, &clean),
            "problem" => self.set_line_field("problem", self.task.problem_line, &clean),
            "fix" => self.set_line_field("fix", self.task.fix_line, &clean),
            "accept" => self.set_line_field("accept", self.task.accept_line, &clean),
            "verify" => self.set_line_field("verify", self.task.verify_line, &clean),
            "final" => self.set_line_field("final", self.task.final_line, &clean),
            "attempts" => self.set_line_field("attempts", self.task.attempts_line, &clean),
            "verified" => self.set_line_field("verified", self.task.verified_line, &clean),
            "commit" => self.set_line_field("commit", self.task.commit_line, &clean),
            _ => {
                let existing = self.unknown_line(&lower);
                self.set_line_field(&lower, existing, &clean);
            }
        }
    }

    /// Removes a field by key (`docs/format.md` §8, §10).
    pub fn unset_field(&mut self, key: &str) {
        let lower = key.trim().to_ascii_lowercase();
        match lower.as_str() {
            "priority" => self.priority_override = Some(None),
            "difficulty" => self.difficulty_override = Some(None),
            "type" => self.type_override = Some(None),
            "area" => self.area_override = Some(None),
            "milestone" => self.unset_part_field("milestone", self.task.milestone_loc),
            "goal" => self.unset_part_field("goal", self.task.goal_loc),
            "claimed" => self.remove_claimed(),
            "why" => self.unset_line_field("why", self.task.why_line),
            "depends" => self.unset_line_field("depends", self.task.depends_line),
            "files" => self.unset_line_field("files", self.task.files_line),
            "read" => self.unset_line_field("read", self.task.read_line),
            "problem" => self.unset_line_field("problem", self.task.problem_line),
            "fix" => self.unset_line_field("fix", self.task.fix_line),
            "steps" => self.unset_steps(),
            "accept" => self.unset_line_field("accept", self.task.accept_line),
            "verify" => self.unset_line_field("verify", self.task.verify_line),
            "final" => self.unset_line_field("final", self.task.final_line),
            "attempts" => self.unset_line_field("attempts", self.task.attempts_line),
            "verified" => self.unset_line_field("verified", self.task.verified_line),
            "commit" => self.unset_line_field("commit", self.task.commit_line),
            _ => {
                let existing = self.unknown_line(&lower);
                self.unset_line_field(&lower, existing);
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
        if let Some(lines) = self.steps_insert.take() {
            self.queue_block_insert("steps", lines);
        }

        let mut edits = Vec::new();
        for (line, change) in self.line_changes {
            match change {
                Some(text) => edits.push(LineEdit::replace_line(line, text)),
                None => edits.push(LineEdit::remove(line..line + 1)),
            }
        }
        edits.extend(self.block_edits.into_values());

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

    /// Sets a field kept as one line (`status`, `owner`, `claimed`, `why`, `depends`, `files`, `problem`,
    /// `fix`, `accept`, `commit`, or an unknown part): a value on its own line is replaced, a value on the
    /// combined priority line is edited as a part of that line, and a missing field is added in canonical
    /// order (`docs/format.md` §8, §10). `existing_line` is where the field is now, if it is present.
    fn set_line_field(&mut self, key: &str, existing_line: Option<usize>, clean_val: &str) {
        match existing_line {
            Some(line) if self.on_priority_line(line) => self.set_combined_part(key, clean_val),
            Some(line) => self.replace_line_if_changed(line, format!("- {key}: {clean_val}")),
            None => self.queue_field_insert(key, format!("- {key}: {clean_val}")),
        }
    }

    /// Removes a field kept as one line: its own line, or its part of the combined priority line.
    fn unset_line_field(&mut self, key: &str, existing_line: Option<usize>) {
        match existing_line {
            Some(line) if self.on_priority_line(line) => {
                self.combined_part_overrides.insert(key.to_string(), None);
            }
            Some(line) => {
                self.line_changes.insert(line, None);
            }
            None => {}
        }
    }

    /// Sets the `steps` field to `steps`, one numbered line each (`docs/format.md` §8); an empty list
    /// removes the field. An own-line field is replaced together with its step lines, a `steps` part
    /// of the priority line moves below it, and a missing field is added in canonical order.
    pub fn set_steps(&mut self, steps: &[String]) {
        if steps.is_empty() {
            self.unset_steps();
            return;
        }
        let mut lines = vec!["- steps:".to_string()];
        lines.extend(
            steps
                .iter()
                .enumerate()
                .map(|(index, text)| format!("  {}. {text}", index + 1)),
        );
        if let (Some(_), Some(block)) = (self.task.steps_line, self.task.steps_block.clone()) {
            self.block_edits
                .insert(block.start, LineEdit::replace_range(block, lines));
            return;
        }
        // No line of its own: a `steps` part of the priority line goes, and the list is inserted.
        if let Some(line) = self.task.steps_line
            && self.on_priority_line(line)
        {
            self.combined_part_overrides
                .insert("steps".to_string(), None);
        }
        self.steps_insert = Some(lines);
    }

    /// Removes the `steps` field with all its step lines (`docs/format.md` §8, §10).
    pub fn unset_steps(&mut self) {
        self.steps_insert = None;
        match (self.task.steps_line, self.task.steps_block.clone()) {
            (Some(_), Some(block)) => {
                self.block_edits
                    .insert(block.start, LineEdit::remove(block));
            }
            (Some(line), _) if self.on_priority_line(line) => {
                self.combined_part_overrides
                    .insert("steps".to_string(), None);
            }
            _ => {}
        }
    }

    /// Queues `lines` as one field inserted in canonical order: they are kept in their order, and
    /// their insertion point is the one of a single line for `key`.
    fn queue_block_insert(&mut self, key: &str, lines: Vec<String>) {
        let insert_at = self.task.insert_line_for_field(key);
        let rank = canonical_field_rank(key);
        for text in lines {
            let seq = self.seq;
            self.seq += 1;
            self.field_inserts.push((insert_at, rank, seq, text));
        }
    }

    /// Sets the part `key` of the combined priority line to `value`. The line is split at ` · ` when it
    /// is read, so a value that contains the separator cannot stay on it: the part then moves to a line
    /// of its own, and the rest of the combined line is kept as it was.
    fn set_combined_part(&mut self, key: &str, value: &str) {
        if value.contains(COMBINED_PART_SEPARATOR) {
            self.combined_part_overrides.insert(key.to_string(), None);
            self.queue_field_insert(key, format!("- {key}: {value}"));
        } else {
            self.combined_part_overrides
                .insert(key.to_string(), Some(value.to_string()));
        }
    }

    /// The line of an unknown field or part `key`, if the task has one.
    fn unknown_line(&self, key: &str) -> Option<usize> {
        self.task
            .unknown_fields
            .iter()
            .find(|f| f.key == key)
            .map(|f| f.line)
    }

    /// Whether `line` is the task's combined priority line, whose parts are edited in place.
    fn on_priority_line(&self, line: usize) -> bool {
        self.task
            .priority_detail
            .as_ref()
            .is_some_and(|d| d.line == line)
    }

    /// Sets `milestone` or `goal` (`docs/format.md` §8, §17). A value on its own line is replaced
    /// in place, a value in the combined priority line is rewritten with that line, and a missing
    /// field is added in canonical order. An empty value removes the field.
    fn set_part_field(&mut self, key: &str, loc: Option<FieldLocation>, clean_val: &str) {
        if clean_val.is_empty() {
            self.unset_part_field(key, loc);
            return;
        }
        match loc {
            Some(FieldLocation::CombinedPriority(_)) => {
                self.combined_part_overrides
                    .insert(key.to_string(), Some(clean_val.to_string()));
            }
            Some(FieldLocation::OwnLine(line)) => {
                self.replace_line_if_changed(line, format!("- {key}: {clean_val}"));
            }
            None => self.queue_field_insert(key, format!("- {key}: {clean_val}")),
        }
    }

    /// Removes `milestone` or `goal`, from its own line or from the combined priority line.
    fn unset_part_field(&mut self, key: &str, loc: Option<FieldLocation>) {
        match loc {
            Some(FieldLocation::CombinedPriority(_)) => {
                self.combined_part_overrides.insert(key.to_string(), None);
            }
            Some(FieldLocation::OwnLine(line)) => {
                self.line_changes.insert(line, None);
            }
            None => {}
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

    /// Inserts a new `difficulty`, `type` or `area` part into the parts that follow the priority
    /// on a combined line: right after the nearest existing part that comes before it in the field
    /// order, or first when there is none (`docs/format.md` §8, §10).
    fn insert_combined_part(entries: &mut Vec<PartEntry>, key: &str, value: String) {
        let rank = canonical_field_rank(key);
        let at = entries
            .iter()
            .rposition(|(k, _)| k.as_deref().is_some_and(|k| canonical_field_rank(k) < rank))
            .map_or(0, |i| i + 1);
        entries.insert(at, (Some(key.to_string()), format!("{key}: {value}")));
    }

    /// Adds a new part to the combined line, or to `own_lines` when its value holds the separator
    /// (`docs/format.md` §8): a value that is split at the separator cannot stay on the line.
    fn place_new_part(
        entries: &mut Vec<PartEntry>,
        own_lines: &mut Vec<(String, String)>,
        key: &str,
        value: String,
    ) {
        if value.contains(COMBINED_PART_SEPARATOR) {
            own_lines.push((key.to_string(), value));
        } else {
            Self::insert_combined_part(entries, key, value);
        }
    }

    /// The value a field on the combined priority line has after this edit, or `None` when the edit
    /// removes it.
    fn combined_field_value(&self, key: &str, value: &str) -> Option<String> {
        let planned = match key {
            "difficulty" => self.difficulty_override.as_ref(),
            "type" => self.type_override.as_ref(),
            "area" => self.area_override.as_ref(),
            _ => self.combined_part_overrides.get(key),
        };
        match planned {
            Some(Some(new_value)) => Some(new_value.clone()),
            Some(None) => None,
            None => Some(value.to_string()),
        }
    }

    fn flush_priority_type_area(&mut self) {
        if self.priority_override.is_none()
            && self.difficulty_override.is_none()
            && self.type_override.is_none()
            && self.area_override.is_none()
            && self.combined_part_overrides.is_empty()
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

            let mut entries: Vec<PartEntry> = Vec::new();
            // Parts whose new value holds the separator: they get a line of their own (T-178).
            let mut own_lines: Vec<(String, String)> = Vec::new();

            let difficulty_in_combined = matches!(
                self.task.difficulty_loc,
                Some(FieldLocation::CombinedPriority(_))
            );
            let type_in_combined = matches!(
                self.task.task_type_loc,
                Some(FieldLocation::CombinedPriority(_))
            );
            let area_in_combined =
                matches!(self.task.area_loc, Some(FieldLocation::CombinedPriority(_)));

            // Every segment stays in its place: a field keeps its position unless an edit removes it,
            // and a segment that is not a field is kept as written (`docs/format.md` §8, `R-EDIT-1`).
            for part in &detail.parts {
                match part {
                    CombinedPart::Text(text) => entries.push((None, text.clone())),
                    CombinedPart::Field { key, value } => {
                        if let Some(new_value) = self.combined_field_value(key, value) {
                            if new_value.contains(COMBINED_PART_SEPARATOR) {
                                own_lines.push((key.clone(), new_value));
                            } else {
                                entries.push((Some(key.clone()), format!("{key}: {new_value}")));
                            }
                        }
                    }
                }
            }

            // Per docs/format.md §8: a new `difficulty`, `type` or `area` goes into the combined line
            // when the task has a priority line (unless priority itself is being removed).
            if !priority_is_removed {
                if !difficulty_in_combined
                    && self.task.difficulty_loc.is_none()
                    && let Some(Some(new_d)) = &self.difficulty_override
                {
                    Self::place_new_part(&mut entries, &mut own_lines, "difficulty", new_d.clone());
                }
                if !type_in_combined
                    && self.task.task_type_loc.is_none()
                    && let Some(Some(new_t)) = &self.type_override
                {
                    Self::place_new_part(&mut entries, &mut own_lines, "type", new_t.clone());
                }
                if !area_in_combined
                    && self.task.area_loc.is_none()
                    && let Some(Some(new_a)) = &self.area_override
                {
                    Self::place_new_part(&mut entries, &mut own_lines, "area", new_a.clone());
                }
            }

            if let Some(p) = new_priority {
                let mut segments = vec![p];
                segments.extend(entries.into_iter().map(|(_, text)| text));
                self.replace_line_if_changed(
                    detail.line,
                    format!("- priority: {}", segments.join(COMBINED_PART_SEPARATOR)),
                );
            } else {
                // Priority was unset: the priority line goes and its fields move to lines of their own.
                // Segments that are not fields are kept as written on one line in the same place, so
                // no text is lost (`R-EDIT-1`).
                let texts: Vec<&str> = entries
                    .iter()
                    .filter(|(key, _)| key.is_none())
                    .map(|(_, text)| text.as_str())
                    .collect();
                if texts.is_empty() {
                    self.line_changes.insert(detail.line, None);
                } else {
                    self.replace_line_if_changed(
                        detail.line,
                        format!("- {}", texts.join(COMBINED_PART_SEPARATOR)),
                    );
                }
                for (key, text) in entries {
                    if let Some(key) = key {
                        self.queue_field_insert(&key, format!("- {text}"));
                    }
                }
            }

            for (key, value) in own_lines {
                self.queue_field_insert(&key, format!("- {key}: {value}"));
            }

            // If `difficulty`, `type` or `area` lived on its own line, edit/remove it in place
            // (`docs/format.md` §15).
            if let Some(FieldLocation::OwnLine(line)) = self.task.difficulty_loc
                && let Some(ref ov) = self.difficulty_override
            {
                match ov {
                    Some(v) => self.replace_line_if_changed(line, format!("- difficulty: {v}")),
                    None => {
                        self.line_changes.insert(line, None);
                    }
                }
            }
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

            if let Some(ref ov) = self.difficulty_override {
                match self.task.difficulty_loc {
                    Some(FieldLocation::OwnLine(line)) => match ov {
                        Some(v) => self.replace_line_if_changed(line, format!("- difficulty: {v}")),
                        None => {
                            self.line_changes.insert(line, None);
                        }
                    },
                    _ => {
                        if let Some(v) = ov {
                            if adding_priority && !v.contains(COMBINED_PART_SEPARATOR) {
                                combined_parts.push(format!("difficulty: {v}"));
                            } else {
                                self.queue_field_insert("difficulty", format!("- difficulty: {v}"));
                            }
                        }
                    }
                }
            }

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
                            if adding_priority && !v.contains(COMBINED_PART_SEPARATOR) {
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
                            if adding_priority && !v.contains(COMBINED_PART_SEPARATOR) {
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
                    format!(
                        "- priority: {}",
                        combined_parts.join(COMBINED_PART_SEPARATOR)
                    ),
                );
            }
        }
    }
}
