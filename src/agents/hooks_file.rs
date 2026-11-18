//! The todo-managed Claude Code hooks in `.claude/settings.json` (`R-AGENT-4`, `R-MIG-2`).
//!
//! `todo migrate --yes` rewrites a managed hook entry in place and changes no other byte of the
//! file. `serde_json` does not keep where a value sits in the text, so this module reads the
//! positions of the entries with a small scanner, after the text has been parsed as JSON.

use std::ops::Range;

use serde::Serialize;
use serde_json::ser::{PrettyFormatter, Serializer};
use serde_json::{Value, json};

/// Key that marks a hook entry as added by todo.
pub(crate) const MANAGED_KEY: &str = "_managed_by";

/// Value of [`MANAGED_KEY`] on entries added by todo.
pub(crate) const MANAGED_VAL: &str = "todo";

/// The hook events that todo manages, in the order `hooks install` adds them.
pub(crate) const HOOK_EVENTS: [&str; 3] = ["SessionStart", "PreToolUse", "SessionEnd"];

/// The entry that `hooks install` writes for each managed event, in [`HOOK_EVENTS`] order.
#[must_use]
pub(crate) fn canonical_entries() -> Vec<(&'static str, Value)> {
    vec![
        (
            "SessionStart",
            json!({
                MANAGED_KEY: MANAGED_VAL,
                "hooks": [{"type": "command", "command": "todo summary"}]
            }),
        ),
        (
            "PreToolUse",
            json!({
                MANAGED_KEY: MANAGED_VAL,
                "matcher": "Edit|Write|MultiEdit",
                "hooks": [{"type": "command", "command": "todo guard"}]
            }),
        ),
        (
            "SessionEnd",
            json!({
                MANAGED_KEY: MANAGED_VAL,
                "hooks": [{"type": "command", "command": "todo session-end"}]
            }),
        ),
    ]
}

/// Whether a hook entry was added by todo.
#[must_use]
pub(crate) fn is_managed_hook(val: &Value) -> bool {
    val.as_object()
        .and_then(|m| m.get(MANAGED_KEY))
        .and_then(Value::as_str)
        == Some(MANAGED_VAL)
}

/// The indentation of the first indented line of `text`, or two spaces when there is none.
#[must_use]
pub(crate) fn detect_indent(text: &str) -> String {
    for line in text.lines() {
        let ws: String = line
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();
        if !ws.is_empty() {
            return ws;
        }
    }
    "  ".to_string()
}

/// A managed hook entry that differs from the canonical one, and the text that replaces it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HookEdit {
    /// The event the entry belongs to.
    pub event: &'static str,
    /// Byte range of the entry in the file, from its `{` to its `}`.
    pub range: Range<usize>,
    /// The canonical entry, formatted to sit at its place in the file.
    pub text: String,
}

/// Finds the managed hook entries of `text` that differ from what `hooks install` writes.
///
/// Only events whose managed entry is already in the file are returned: a missing event is not
/// installed here (`R-MIG-2`). Empty text has no hooks. Errors name what could not be read.
pub(crate) fn outdated_hook_entries(text: &str) -> Result<Vec<HookEdit>, String> {
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    let root: Value = serde_json::from_str(text).map_err(|e| format!("not valid JSON: {e}"))?;
    if !root.is_object() {
        return Err("the root is not a JSON object".to_string());
    }
    let layout = || "unexpected layout; hooks not checked".to_string();

    let bytes = text.as_bytes();
    let root_members = object_members(bytes, skip_ws(bytes, 0)).ok_or_else(layout)?;
    let Some(hooks) = last_member(&root_members, "hooks") else {
        return Ok(Vec::new());
    };
    if bytes.get(hooks.start) != Some(&b'{') {
        return Ok(Vec::new());
    }
    let hook_members = object_members(bytes, hooks.start).ok_or_else(layout)?;

    let indent = detect_indent(text);
    let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut edits = Vec::new();
    for (event, canonical) in canonical_entries() {
        let Some(member) = last_member(&hook_members, event) else {
            continue;
        };
        if bytes.get(member.start) != Some(&b'[') {
            continue;
        }
        let elements = array_elements(bytes, member.start).ok_or_else(layout)?;
        let Some((start, end, value)) = managed_element(text, &elements)? else {
            continue;
        };
        let range = start..end;
        if value == canonical {
            continue;
        }
        edits.push(HookEdit {
            event,
            range,
            text: render_value(&canonical, &indent, eol, 3)?,
        });
    }
    Ok(edits)
}

/// The text with each edit's range replaced by its text. `None` if the edits overlap or do not
/// fall on the text.
#[must_use]
pub(crate) fn apply_hook_edits(text: &str, edits: &[HookEdit]) -> Option<String> {
    let splices: Vec<Splice> = edits
        .iter()
        .map(|e| Splice {
            range: e.range.clone(),
            text: e.text.clone(),
        })
        .collect();
    splice_text(text, &splices)
}

/// Replaces the bytes in `range` by `text`. An empty range inserts.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Splice {
    range: Range<usize>,
    text: String,
}

/// The text with each splice applied. Splices at the same place keep the order they are given in.
/// `None` if they overlap or do not fall on the text.
fn splice_text(text: &str, splices: &[Splice]) -> Option<String> {
    let mut sorted: Vec<&Splice> = splices.iter().collect();
    sorted.sort_by_key(|s| s.range.start);
    let mut out = String::with_capacity(text.len());
    let mut pos = 0;
    for splice in sorted {
        if splice.range.start < pos || splice.range.end < splice.range.start {
            return None;
        }
        out.push_str(text.get(pos..splice.range.start)?);
        out.push_str(&splice.text);
        pos = splice.range.end;
    }
    out.push_str(text.get(pos..)?);
    Some(out)
}

/// The settings text with the managed hook entries installed, for `hooks install` (`R-AGENT-4`).
///
/// Only the bytes of the entries that change are touched, as `migrate --yes` does: an outdated
/// managed entry is replaced where it sits, and a missing entry, event array or `hooks` object is
/// inserted at the end of the object or array it belongs to. Every other byte, including key
/// order and inline formatting, stays as it is. Empty text becomes a new settings object. Errors
/// name what cannot be installed; nothing is changed then.
pub(crate) fn install_hook_text(text: &str) -> Result<String, String> {
    let text = if text.trim().is_empty() { "{}\n" } else { text };
    let root: Value = serde_json::from_str(text).map_err(|e| format!("not valid JSON: {e}"))?;
    let root_map = root
        .as_object()
        .ok_or("the root of settings.json must be a JSON object")?;

    let indent = detect_indent(text);
    let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let bytes = text.as_bytes();
    let layout = || "unexpected layout; hooks not changed".to_string();
    let root_open = skip_ws(bytes, 0);
    let root_members = object_members(bytes, root_open).ok_or_else(layout)?;

    let mut splices = Vec::new();
    match root_map.get("hooks") {
        None => {
            // The members sit at depth 1, the events at depth 2 (`hooks` is a member of the root).
            let mut events = Vec::new();
            for (event, entry) in canonical_entries() {
                events.push(member_text(
                    event,
                    &Value::Array(vec![entry]),
                    &indent,
                    eol,
                    2,
                )?);
            }
            let hooks = block(&events, '{', '}', &indent, eol, 2);
            let item = format!("\"hooks\": {hooks}");
            let last_end = root_members.last().map(|m| m.end);
            splices.push(
                insert_items(bytes, root_open, last_end, &[item], &indent, eol, 1)
                    .ok_or_else(layout)?,
            );
        }
        Some(Value::Object(hook_map)) => {
            let hooks = last_member(&root_members, "hooks").ok_or_else(layout)?;
            let hook_members = object_members(bytes, hooks.start).ok_or_else(layout)?;
            let mut missing = Vec::new();
            for (event, entry) in canonical_entries() {
                match hook_map.get(event) {
                    None => missing.push(member_text(
                        event,
                        &Value::Array(vec![entry]),
                        &indent,
                        eol,
                        2,
                    )?),
                    Some(Value::Array(_)) => {
                        let member = last_member(&hook_members, event).ok_or_else(layout)?;
                        splices.extend(install_event(
                            bytes,
                            text,
                            member.start,
                            &entry,
                            &indent,
                            eol,
                        )?);
                    }
                    Some(_) => {
                        return Err(format!(
                            "`hooks.{event}` in settings.json must be a JSON array"
                        ));
                    }
                }
            }
            if !missing.is_empty() {
                let last_end = hook_members.last().map(|m| m.end);
                splices.push(
                    insert_items(bytes, hooks.start, last_end, &missing, &indent, eol, 2)
                        .ok_or_else(layout)?,
                );
            }
        }
        Some(_) => {
            return Err("`hooks` in settings.json must be a JSON object".to_string());
        }
    }
    splice_text(text, &splices).ok_or_else(layout)
}

/// The splices that bring one event's array (opening at `open`) up to date: the managed entry is
/// replaced if it differs, or the canonical entry is added at the end when there is none.
fn install_event(
    bytes: &[u8],
    text: &str,
    open: usize,
    canonical: &Value,
    indent: &str,
    eol: &str,
) -> Result<Vec<Splice>, String> {
    let layout = || "unexpected layout; hooks not changed".to_string();
    let elements = array_elements(bytes, open).ok_or_else(layout)?;
    let rendered = render_value(canonical, indent, eol, 3)?;
    match managed_element(text, &elements)? {
        Some((_, _, value)) if value == *canonical => Ok(Vec::new()),
        Some((start, end, _)) => Ok(vec![Splice {
            range: start..end,
            text: rendered,
        }]),
        None => {
            let last_end = elements.last().map(|e| e.1);
            let splice = insert_items(bytes, open, last_end, &[rendered], indent, eol, 3)
                .ok_or_else(layout)?;
            Ok(vec![splice])
        }
    }
}

/// The first managed entry among the array elements at `elements` (byte ranges in `text`).
fn managed_element(
    text: &str,
    elements: &[(usize, usize)],
) -> Result<Option<(usize, usize, Value)>, String> {
    for &(start, end) in elements {
        let slice = text
            .get(start..end)
            .ok_or_else(|| "unexpected layout; hooks not changed".to_string())?;
        let value: Value =
            serde_json::from_str(slice).map_err(|e| format!("not valid JSON: {e}"))?;
        if is_managed_hook(&value) {
            return Ok(Some((start, end, value)));
        }
    }
    Ok(None)
}

/// The splice that adds `items` to the object or array that opens at `open`, with the items at
/// `depth` levels of indentation. After its last item (`last_end`) when it has one, otherwise in
/// place of its empty brackets. `None` when there is nothing to add or the brackets are missing.
fn insert_items(
    bytes: &[u8],
    open: usize,
    last_end: Option<usize>,
    items: &[String],
    indent: &str,
    eol: &str,
    depth: usize,
) -> Option<Splice> {
    if items.is_empty() {
        return None;
    }
    let close = value_end(bytes, open)?.checked_sub(1)?;
    match last_end {
        Some(end) => Some(Splice {
            range: end..end,
            text: format!(",{}", joined(items, indent, eol, depth)),
        }),
        None => Some(Splice {
            range: open..close + 1,
            text: block(
                items,
                char::from(*bytes.get(open)?),
                char::from(*bytes.get(close)?),
                indent,
                eol,
                depth,
            ),
        }),
    }
}

/// `items` as the whole contents of a new object or array, with its brackets: the items at
/// `depth` levels, the closing bracket one level less.
fn block(
    items: &[String],
    open: char,
    close: char,
    indent: &str,
    eol: &str,
    depth: usize,
) -> String {
    format!(
        "{open}{}{eol}{}{close}",
        joined(items, indent, eol, depth),
        indent.repeat(depth.saturating_sub(1))
    )
}

/// `items`, each on its own line at `depth` levels, separated by commas (no comma after the last).
fn joined(items: &[String], indent: &str, eol: &str, depth: usize) -> String {
    let pad = indent.repeat(depth);
    items
        .iter()
        .map(|item| format!("{eol}{pad}{item}"))
        .collect::<Vec<_>>()
        .join(",")
}

/// One object member as text: its quoted key, then its value formatted at `depth` levels.
fn member_text(
    key: &str,
    value: &Value,
    indent: &str,
    eol: &str,
    depth: usize,
) -> Result<String, String> {
    let key = serde_json::to_string(key).map_err(|e| format!("could not format a key: {e}"))?;
    Ok(format!(
        "{key}: {}",
        render_value(value, indent, eol, depth)?
    ))
}

/// Formats `value` as pretty JSON in the file's indentation. The value starts on a line `depth`
/// levels deep, so every line after the first is indented that far.
fn render_value(value: &Value, indent: &str, eol: &str, depth: usize) -> Result<String, String> {
    let mut buf = Vec::new();
    let formatter = PrettyFormatter::with_indent(indent.as_bytes());
    let mut ser = Serializer::with_formatter(&mut buf, formatter);
    value
        .serialize(&mut ser)
        .map_err(|e| format!("could not format a hook entry: {e}"))?;
    let rendered =
        String::from_utf8(buf).map_err(|e| format!("could not format a hook entry: {e}"))?;
    let pad = format!("{eol}{}", indent.repeat(depth));
    Ok(rendered.split('\n').collect::<Vec<_>>().join(&pad))
}

/// The settings text with the managed hook entries removed, for `hooks uninstall` (`R-AGENT-4`).
///
/// Only the managed entries go, and every other byte stays, as `hooks install` keeps them: a
/// managed entry is cut out of its event's array; an event whose entries are all managed is cut out
/// of `hooks` with its key; and `hooks` itself goes when nothing else is left in it. An event array
/// the user left empty stays. Text without managed entries comes back as it is, and blank text
/// stays blank. Errors name what cannot be read; nothing is changed then.
pub(crate) fn uninstall_hook_text(text: &str) -> Result<String, String> {
    if text.trim().is_empty() {
        return Ok(text.to_string());
    }
    let root: Value = serde_json::from_str(text).map_err(|e| format!("not valid JSON: {e}"))?;
    if !root.is_object() {
        return Ok(text.to_string());
    }
    let bytes = text.as_bytes();
    let layout = || "unexpected layout; hooks not changed".to_string();
    let root_open = skip_ws(bytes, 0);
    let root_members = object_members(bytes, root_open).ok_or_else(layout)?;
    let Some(hooks_index) = last_index(&root_members, "hooks") else {
        return Ok(text.to_string());
    };
    let hooks_open = root_members[hooks_index].start;
    if bytes.get(hooks_open) != Some(&b'{') {
        return Ok(text.to_string());
    }
    let hook_members = object_members(bytes, hooks_open).ok_or_else(layout)?;

    let mut splices = Vec::new();
    // Per member of `hooks`: whether the whole event (a member) goes.
    let mut removed = vec![false; hook_members.len()];
    for event in HOOK_EVENTS {
        let Some(index) = last_index(&hook_members, event) else {
            continue;
        };
        let open = hook_members[index].start;
        if bytes.get(open) != Some(&b'[') {
            continue;
        }
        let elements = array_elements(bytes, open).ok_or_else(layout)?;
        let managed = managed_flags(text, &elements)?;
        if !managed.contains(&true) {
            continue;
        }
        if managed.iter().all(|m| *m) {
            removed[index] = true;
        } else {
            splices.extend(remove_items(bytes, open, &elements, &managed).ok_or_else(layout)?);
        }
    }

    if removed.contains(&true) && removed.iter().all(|r| *r) {
        // Nothing else is in `hooks`, so the object goes with the events in it. No entry inside it
        // is cut on its own in this case, so the splices above are empty.
        let spans: Vec<(usize, usize)> =
            root_members.iter().map(|m| (m.key_start, m.end)).collect();
        let mut root_removed = vec![false; root_members.len()];
        root_removed[hooks_index] = true;
        splices = remove_items(bytes, root_open, &spans, &root_removed).ok_or_else(layout)?;
    } else if removed.contains(&true) {
        let spans: Vec<(usize, usize)> =
            hook_members.iter().map(|m| (m.key_start, m.end)).collect();
        splices.extend(remove_items(bytes, hooks_open, &spans, &removed).ok_or_else(layout)?);
    }
    splice_text(text, &splices).ok_or_else(layout)
}

/// Whether each array element (its byte range in `text`) is a managed hook entry.
fn managed_flags(text: &str, elements: &[(usize, usize)]) -> Result<Vec<bool>, String> {
    elements
        .iter()
        .map(|&(start, end)| {
            let slice = text
                .get(start..end)
                .ok_or_else(|| "unexpected layout; hooks not changed".to_string())?;
            let value: Value =
                serde_json::from_str(slice).map_err(|e| format!("not valid JSON: {e}"))?;
            Ok(is_managed_hook(&value))
        })
        .collect()
}

/// The splices that remove the items flagged in `remove` from the object or array that opens at
/// `open`. `spans` are the byte ranges of the items, in order. A run of removed items is cut
/// together with the comma that joins it to a neighbour that stays, so the items that stay keep
/// their text. When every item is flagged, the whole contents go. `None` if the layout is unexpected.
fn remove_items(
    bytes: &[u8],
    open: usize,
    spans: &[(usize, usize)],
    remove: &[bool],
) -> Option<Vec<Splice>> {
    if remove.len() != spans.len() {
        return None;
    }
    if !remove.contains(&true) {
        return Some(Vec::new());
    }
    if remove.iter().all(|r| *r) {
        let close = value_end(bytes, open)?.checked_sub(1)?;
        return Some(vec![Splice {
            range: open + 1..close,
            text: String::new(),
        }]);
    }
    let mut splices = Vec::new();
    let mut i = 0;
    while i < spans.len() {
        if !remove[i] {
            i += 1;
            continue;
        }
        let first = i;
        while i + 1 < spans.len() && remove[i + 1] {
            i += 1;
        }
        let last = i;
        let range = if last + 1 < spans.len() {
            // The next item stays: cut up to its start.
            spans[first].0..spans[last + 1].0
        } else {
            // The last item goes: cut from the end of the item before it, which stays.
            spans[first.checked_sub(1)?].1..spans[last].1
        };
        splices.push(Splice {
            range,
            text: String::new(),
        });
        i += 1;
    }
    Some(splices)
}

/// One member of a JSON object: its key, where the member starts (at its key), and where its value
/// starts and ends.
struct Member {
    key: String,
    key_start: usize,
    start: usize,
    end: usize,
}

/// The last member named `key`, which is the one serde_json keeps when a key repeats.
fn last_member<'a>(members: &'a [Member], key: &str) -> Option<&'a Member> {
    members.iter().rev().find(|m| m.key == key)
}

/// The position in `members` of the last member named `key`.
fn last_index(members: &[Member], key: &str) -> Option<usize> {
    members.iter().rposition(|m| m.key == key)
}

fn skip_ws(b: &[u8], mut i: usize) -> usize {
    while matches!(b.get(i), Some(b' ' | b'\t' | b'\r' | b'\n')) {
        i += 1;
    }
    i
}

/// The index just past the string that starts at `i` (a `"`), or `None` if it never ends.
fn string_end(b: &[u8], i: usize) -> Option<usize> {
    let mut j = i + 1;
    loop {
        match b.get(j)? {
            b'\\' => j += 2,
            b'"' => return Some(j + 1),
            _ => j += 1,
        }
    }
}

/// The index just past the JSON value that starts at `i`, or `None` if there is none.
fn value_end(b: &[u8], i: usize) -> Option<usize> {
    match b.get(i)? {
        b'"' => string_end(b, i),
        b'{' | b'[' => {
            let mut depth = 0usize;
            let mut j = i;
            loop {
                match b.get(j)? {
                    b'"' => {
                        j = string_end(b, j)?;
                        continue;
                    }
                    b'{' | b'[' => depth += 1,
                    b'}' | b']' => {
                        depth = depth.checked_sub(1)?;
                        if depth == 0 {
                            return Some(j + 1);
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
        }
        _ => {
            let mut j = i;
            while let Some(c) = b.get(j) {
                if matches!(c, b',' | b'}' | b']' | b' ' | b'\t' | b'\r' | b'\n') {
                    break;
                }
                j += 1;
            }
            (j > i).then_some(j)
        }
    }
}

/// The members of the object that starts at `start` (a `{`), with each value's position.
fn object_members(b: &[u8], start: usize) -> Option<Vec<Member>> {
    if b.get(start) != Some(&b'{') {
        return None;
    }
    let mut members = Vec::new();
    let mut i = skip_ws(b, start + 1);
    if b.get(i) == Some(&b'}') {
        return Some(members);
    }
    loop {
        if b.get(i) != Some(&b'"') {
            return None;
        }
        let key_start = i;
        let key_end = string_end(b, i)?;
        let key: String = serde_json::from_slice(b.get(i..key_end)?).ok()?;
        i = skip_ws(b, key_end);
        if b.get(i) != Some(&b':') {
            return None;
        }
        i = skip_ws(b, i + 1);
        let end = value_end(b, i)?;
        members.push(Member {
            key,
            key_start,
            start: i,
            end,
        });
        i = skip_ws(b, end);
        match b.get(i)? {
            b',' => i = skip_ws(b, i + 1),
            b'}' => return Some(members),
            _ => return None,
        }
    }
}

/// The positions of the elements of the array that starts at `start` (a `[`).
fn array_elements(b: &[u8], start: usize) -> Option<Vec<(usize, usize)>> {
    if b.get(start) != Some(&b'[') {
        return None;
    }
    let mut items = Vec::new();
    let mut i = skip_ws(b, start + 1);
    if b.get(i) == Some(&b']') {
        return Some(items);
    }
    loop {
        let end = value_end(b, i)?;
        items.push((i, end));
        i = skip_ws(b, end);
        match b.get(i)? {
            b',' => i = skip_ws(b, i + 1),
            b']' => return Some(items),
            _ => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A settings file whose `SessionStart` array holds `session_start`. The `env` value has
    /// brackets and an escaped quote inside a string, which the scanner must step over.
    fn settings(session_start: &str) -> String {
        format!(
            "{{\n  \"env\": {{\"A\": \"}}]\\\"{{\"}},\n  \"hooks\": {{\n    \"SessionStart\": [\n      {session_start}\n    ]\n  }}\n}}\n"
        )
    }

    #[test]
    fn canonical_entries_are_the_three_events_in_order() {
        let events: Vec<&str> = canonical_entries().iter().map(|(e, _)| *e).collect();
        assert_eq!(events, HOOK_EVENTS);
    }

    #[test]
    fn a_current_entry_is_not_an_edit_whatever_its_formatting() {
        let text = settings(
            "{\"hooks\": [{\"command\": \"todo summary\", \"type\": \"command\"}], \"_managed_by\": \"todo\"}",
        );
        assert!(outdated_hook_entries(&text).expect("scan").is_empty());
    }

    #[test]
    fn an_old_entry_is_replaced_and_nothing_else_moves() {
        let old = "{\"_managed_by\": \"todo\", \"hooks\": [{\"type\": \"command\", \"command\": \"todo summ\"}]}";
        let user = "{\"matcher\": \"Bash\", \"hooks\": [{\"type\": \"command\", \"command\": \"echo \\\"}]\"}]}";
        let text = settings(&format!("{user},\n      {old}"));
        let edits = outdated_hook_entries(&text).expect("scan");
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].event, "SessionStart");
        let fixed = apply_hook_edits(&text, &edits).expect("apply");
        assert!(fixed.starts_with(text.get(..text.find(old).expect("old")).expect("prefix")));
        assert!(fixed.contains(user));
        assert!(fixed.contains("todo summary"));
        assert!(!fixed.contains("todo summ\""));
        assert!(outdated_hook_entries(&fixed).expect("rescan").is_empty());
        assert!(serde_json::from_str::<Value>(&fixed).is_ok());
    }

    #[test]
    fn an_event_without_a_managed_entry_is_not_listed() {
        let text = "{\"hooks\": {\"SessionEnd\": [{\"command\": \"other\"}]}}";
        assert!(outdated_hook_entries(text).expect("scan").is_empty());
    }

    #[test]
    fn empty_text_and_missing_hooks_have_nothing_to_do() {
        assert!(outdated_hook_entries("").expect("scan").is_empty());
        assert!(
            outdated_hook_entries("{\"env\": {}}")
                .expect("scan")
                .is_empty()
        );
    }

    #[test]
    fn invalid_json_is_reported() {
        assert!(outdated_hook_entries("{\"hooks\": ").is_err());
    }

    #[test]
    fn crlf_files_get_crlf_inside_the_replacement() {
        let old = "{\"_managed_by\": \"todo\", \"hooks\": [{\"type\": \"command\", \"command\": \"todo summ\"}]}";
        let text = format!(
            "{{\r\n  \"hooks\": {{\r\n    \"SessionStart\": [\r\n      {old}\r\n    ]\r\n  }}\r\n}}\r\n"
        );
        let edits = outdated_hook_entries(&text).expect("scan");
        let fixed = apply_hook_edits(&text, &edits).expect("apply");
        assert!(fixed.contains("\r\n      }"));
        assert!(!fixed.replace("\r\n", "").contains('\n'));
        assert!(outdated_hook_entries(&fixed).expect("rescan").is_empty());
    }

    #[test]
    fn strings_with_brackets_and_escapes_do_not_confuse_the_scanner() {
        let b = br#"{"a": "x\"]}", "b": [1, {"c": "]"}]}"#;
        let root = object_members(b, 0).expect("members");
        assert_eq!(root.len(), 2);
        assert_eq!(root[1].key, "b");
        assert_eq!(b.get(root[1].start), Some(&b'['));
        assert_eq!(root[1].end, b.len() - 1);
    }

    #[test]
    fn installing_into_empty_text_writes_the_three_events_and_is_then_current() {
        let fresh = install_hook_text("").expect("install");
        assert!(fresh.ends_with("}\n"), "{fresh}");
        let value: Value = serde_json::from_str(&fresh).expect("json");
        for event in HOOK_EVENTS {
            assert!(value["hooks"][event].is_array(), "{event}: {fresh}");
        }
        assert!(outdated_hook_entries(&fresh).expect("scan").is_empty());
        assert_eq!(install_hook_text(&fresh).expect("again"), fresh);
    }

    #[test]
    fn a_current_file_comes_back_unchanged_whatever_its_formatting() {
        let text = "{\"hooks\":{\"SessionStart\":[{\"_managed_by\":\"todo\",\"hooks\":[{\"command\":\"todo summary\",\"type\":\"command\"}]}],\"PreToolUse\":[{\"_managed_by\":\"todo\",\"matcher\":\"Edit|Write|MultiEdit\",\"hooks\":[{\"type\":\"command\",\"command\":\"todo guard\"}]}],\"SessionEnd\":[{\"_managed_by\":\"todo\",\"hooks\":[{\"type\":\"command\",\"command\":\"todo session-end\"}]}]}}";
        assert_eq!(install_hook_text(text).expect("install"), text);
    }

    #[test]
    fn install_keeps_key_order_inline_formatting_and_escapes() {
        let text = "{\"zeta\": 1,\"hooks\":{\"PreToolUse\":[{\"matcher\":\"Bash\",\"hooks\":[{\"type\":\"command\",\"command\":\"echo \\u00e9\"}]}]},\"alpha\":[1,2]}\n";
        let fixed = install_hook_text(text).expect("install");
        assert!(
            fixed.starts_with("{\"zeta\": 1,\"hooks\":{\"PreToolUse\":[{\"matcher\":\"Bash\""),
            "{fixed}"
        );
        assert!(fixed.ends_with("},\"alpha\":[1,2]}\n"), "{fixed}");
        assert!(fixed.contains("echo \\u00e9"), "{fixed}");
        let value: Value = serde_json::from_str(&fixed).expect("json");
        assert_eq!(
            value["hooks"]["PreToolUse"].as_array().map(Vec::len),
            Some(2)
        );
        assert!(outdated_hook_entries(&fixed).expect("scan").is_empty());
    }

    #[test]
    fn an_outdated_managed_entry_is_replaced_where_it_sits_and_the_rest_is_kept() {
        let old = "{\"_managed_by\": \"todo\", \"hooks\": [{\"type\": \"command\", \"command\": \"todo summ\"}]}";
        let user = "{\"matcher\": \"Bash\", \"hooks\": [{\"type\": \"command\", \"command\": \"echo \\\"}]\"}]}";
        let text = settings(&format!("{user},\n      {old}"));
        let fixed = install_hook_text(&text).expect("install");
        let before = text.find(old).expect("old entry");
        assert!(fixed.starts_with(text.get(..before).expect("prefix")));
        assert!(fixed.contains(user));
        assert!(fixed.contains("todo summary"));
        assert!(!fixed.contains("todo summ\""));
        assert!(fixed.ends_with("\n  }\n}\n"), "{fixed}");
        assert!(outdated_hook_entries(&fixed).expect("rescan").is_empty());
    }

    #[test]
    fn a_missing_hooks_object_is_added_after_the_last_setting_in_the_file_style() {
        let text = "{\r\n    \"env\": {\"A\": 1}\r\n}\r\n";
        let fixed = install_hook_text(text).expect("install");
        assert!(
            fixed.starts_with("{\r\n    \"env\": {\"A\": 1},\r\n    \"hooks\": {\r\n"),
            "{fixed}"
        );
        assert!(fixed.ends_with("\r\n}\r\n"), "{fixed}");
        assert!(!fixed.replace("\r\n", "").contains('\n'), "{fixed}");
        assert!(outdated_hook_entries(&fixed).expect("scan").is_empty());
    }

    #[test]
    fn empty_hooks_and_empty_event_arrays_are_filled() {
        let text = "{\"hooks\": {\"SessionEnd\": []}}";
        let fixed = install_hook_text(text).expect("install");
        assert!(
            fixed.starts_with("{\"hooks\": {\"SessionEnd\": [\n"),
            "{fixed}"
        );
        let value: Value = serde_json::from_str(&fixed).expect("json");
        assert_eq!(
            value["hooks"]["SessionEnd"].as_array().map(Vec::len),
            Some(1)
        );
        assert!(value["hooks"]["SessionStart"].is_array());
        assert!(outdated_hook_entries(&fixed).expect("scan").is_empty());

        let empty = install_hook_text("{\"hooks\": {}}").expect("install");
        let value: Value = serde_json::from_str(&empty).expect("json");
        for event in HOOK_EVENTS {
            assert!(value["hooks"][event].is_array(), "{event}: {empty}");
        }
    }

    #[test]
    fn shapes_that_cannot_hold_the_entries_are_refused() {
        assert!(install_hook_text("[]").is_err());
        assert!(install_hook_text("{\"hooks\": []}").is_err());
        assert!(install_hook_text("{\"hooks\": {\"SessionEnd\": {}}}").is_err());
        assert!(install_hook_text("{\"hooks\": ").is_err());
    }

    /// `text` with the managed entries installed, then removed again.
    fn installed_then_removed(text: &str) -> String {
        let installed = install_hook_text(text).expect("install");
        uninstall_hook_text(&installed).expect("uninstall")
    }

    #[test]
    fn uninstall_undoes_install_byte_for_byte() {
        for text in [
            "{\n  \"zeta\": 1,\n  \"alpha\": true\n}\n",
            "{\r\n    \"env\": {\"A\": 1}\r\n}\r\n",
            "{\"zeta\": 1,\"hooks\":{\"PreToolUse\":[{\"matcher\":\"Bash\",\"hooks\":[{\"type\":\"command\",\"command\":\"echo \\u00e9\"}]}]},\"alpha\":[1,2]}\n",
            "{\n  \"hooks\": {\n    \"PreToolUse\": [\n      {\"matcher\": \"Bash\"}\n    ]\n  }\n}\n",
        ] {
            assert_eq!(installed_then_removed(text), text);
        }
    }

    #[test]
    fn uninstall_cuts_only_the_managed_entries_and_keeps_the_rest_as_written() {
        let text = "{\n  \"hooks\": {\n    \"PreToolUse\": [\n      {\"matcher\": \"Bash\"},\n      {\"_managed_by\": \"todo\", \"matcher\": \"Edit|Write|MultiEdit\"}\n    ],\n    \"SessionStart\": [\n      {\"_managed_by\": \"todo\"}\n    ]\n  },\n  \"env\": {}\n}\n";
        let expected = "{\n  \"hooks\": {\n    \"PreToolUse\": [\n      {\"matcher\": \"Bash\"}\n    ]\n  },\n  \"env\": {}\n}\n";
        assert_eq!(uninstall_hook_text(text).expect("uninstall"), expected);
    }

    #[test]
    fn a_run_of_managed_entries_between_kept_ones_is_cut_with_its_commas() {
        let text = "{\"hooks\": {\"PreToolUse\": [{\"a\": 1}, {\"_managed_by\": \"todo\"}, {\"_managed_by\": \"todo\", \"b\": 2}, {\"c\": 3}]}}";
        let expected = "{\"hooks\": {\"PreToolUse\": [{\"a\": 1}, {\"c\": 3}]}}";
        assert_eq!(uninstall_hook_text(text).expect("uninstall"), expected);
    }

    #[test]
    fn an_outdated_managed_entry_goes_and_the_other_events_stay() {
        let text = "{\"hooks\": {\"SessionEnd\": [{\"_managed_by\": \"todo\", \"hooks\": [{\"command\": \"todo session-end old\"}]}], \"Stop\": [{\"hooks\": []}]}, \"env\": 1}";
        let expected = "{\"hooks\": {\"Stop\": [{\"hooks\": []}]}, \"env\": 1}";
        assert_eq!(uninstall_hook_text(text).expect("uninstall"), expected);
    }

    #[test]
    fn text_without_managed_entries_comes_back_unchanged() {
        for text in [
            "",
            "  \n",
            "{\"hooks\": {\"SessionEnd\": [], \"Stop\": [{\"command\": \"x\"}]}}",
            "{\"env\": {}}",
            "[1, 2]",
        ] {
            assert_eq!(uninstall_hook_text(text).expect("uninstall"), text);
        }
    }

    #[test]
    fn uninstall_refuses_invalid_json() {
        assert!(uninstall_hook_text("{\"hooks\": ").is_err());
    }
}
