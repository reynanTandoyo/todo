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
        let mut managed = None;
        for (start, end) in elements {
            let value: Value = serde_json::from_str(text.get(start..end).ok_or_else(layout)?)
                .map_err(|e| format!("not valid JSON: {e}"))?;
            if is_managed_hook(&value) {
                managed = Some((start..end, value));
                break;
            }
        }
        let Some((range, value)) = managed else {
            continue;
        };
        if value == canonical {
            continue;
        }
        edits.push(HookEdit {
            event,
            range,
            text: render_entry(&canonical, &indent, eol)?,
        });
    }
    Ok(edits)
}

/// The text with each edit's range replaced by its text. `None` if the edits overlap or do not
/// fall on the text.
#[must_use]
pub(crate) fn apply_hook_edits(text: &str, edits: &[HookEdit]) -> Option<String> {
    let mut sorted: Vec<&HookEdit> = edits.iter().collect();
    sorted.sort_by_key(|e| e.range.start);
    let mut out = String::with_capacity(text.len());
    let mut pos = 0;
    for edit in sorted {
        if edit.range.start < pos {
            return None;
        }
        out.push_str(text.get(pos..edit.range.start)?);
        out.push_str(&edit.text);
        pos = edit.range.end;
    }
    out.push_str(text.get(pos..)?);
    Some(out)
}

/// Formats a hook entry with the file's indentation. The entry sits three levels deep (the root
/// object, `hooks`, and the event's array), so every line after the first is indented that far.
fn render_entry(entry: &Value, indent: &str, eol: &str) -> Result<String, String> {
    let mut buf = Vec::new();
    let formatter = PrettyFormatter::with_indent(indent.as_bytes());
    let mut ser = Serializer::with_formatter(&mut buf, formatter);
    entry
        .serialize(&mut ser)
        .map_err(|e| format!("could not format a hook entry: {e}"))?;
    let rendered =
        String::from_utf8(buf).map_err(|e| format!("could not format a hook entry: {e}"))?;
    let pad = format!("{eol}{}", indent.repeat(3));
    Ok(rendered.split('\n').collect::<Vec<_>>().join(&pad))
}

/// One member of a JSON object: its key, and where its value starts.
struct Member {
    key: String,
    start: usize,
}

/// The last member named `key`, which is the one serde_json keeps when a key repeats.
fn last_member<'a>(members: &'a [Member], key: &str) -> Option<&'a Member> {
    members.iter().rev().find(|m| m.key == key)
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
        let key_end = string_end(b, i)?;
        let key: String = serde_json::from_slice(b.get(i..key_end)?).ok()?;
        i = skip_ws(b, key_end);
        if b.get(i) != Some(&b':') {
            return None;
        }
        i = skip_ws(b, i + 1);
        let end = value_end(b, i)?;
        members.push(Member { key, start: i });
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
    }
}
