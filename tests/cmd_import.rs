//! Integration tests for `todo import --from checkbox` (T-081; R-IMP-1).

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{assert_todo, fixture, parse_json};
use serde_json::Value;
use tempfile::TempDir;

/// Copies the invented CRLF fixture into `dir` and returns its path.
fn source_in(dir: &Path) -> PathBuf {
    let path = dir.join("my notes.md");
    fs::write(&path, fixture("import_checkbox_crlf.md")).unwrap();
    path
}

fn import(home: &Path, dir: &Path, extra: &[&str]) -> std::process::Output {
    assert_todo(home)
        .current_dir(dir)
        .arg("import")
        .args(["--from", "checkbox"])
        .args(extra)
        .output()
        .unwrap()
}

fn lines_with_endings(bytes: &[u8]) -> Vec<String> {
    String::from_utf8(bytes.to_vec())
        .unwrap()
        .split_inclusive('\n')
        .map(str::to_string)
        .collect()
}

#[test]
fn preview_prints_converted_text_and_writes_nothing() {
    let tmp = TempDir::new().unwrap();
    let src = source_in(tmp.path());
    let before = fs::read(&src).unwrap();
    let target = tmp.path().join("new TODO.md");

    let out = import(
        tmp.path(),
        tmp.path(),
        &[src.to_str().unwrap(), "--out", target.to_str().unwrap()],
    );
    assert!(out.status.success(), "{out:?}");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.starts_with("<!-- todo-format: 3 -->"), "{stdout}");
    assert!(stdout.contains("### T-001 · Add export"));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("converted 7 checkbox lines into 7 tasks"),
        "{stderr}"
    );
    assert!(stderr.contains("nothing written"), "{stderr}");
    assert!(!target.exists(), "preview must not create the target");
    assert_eq!(
        fs::read(&src).unwrap(),
        before,
        "source must stay unchanged"
    );
}

#[test]
fn preview_of_existing_target_shows_a_diff() {
    let tmp = TempDir::new().unwrap();
    let src = source_in(tmp.path());
    let before = fs::read(&src).unwrap();

    // Default target is the source itself.
    let out = import(tmp.path(), tmp.path(), &[src.to_str().unwrap()]);
    assert!(out.status.success(), "{out:?}");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("--- a/my notes.md"), "{stdout}");
    assert!(stdout.contains("+### T-001 · Add export"), "{stdout}");
    assert!(stdout.contains("-- [ ] **Add export**"), "{stdout}");
    assert_eq!(fs::read(&src).unwrap(), before);
    assert!(!tmp.path().join("my notes.md.pre-import").exists());
}

#[test]
fn yes_converts_in_place_keeps_backup_and_passes_check() {
    let tmp = TempDir::new().unwrap();
    let src = source_in(tmp.path());
    let before = fs::read(&src).unwrap();

    let out = import(tmp.path(), tmp.path(), &[src.to_str().unwrap(), "--yes"]);
    assert!(out.status.success(), "{out:?}");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("my notes.md.pre-import"), "{stdout}");
    assert!(stdout.contains("converted 7 checkbox lines into 7 tasks"));

    let backup = tmp.path().join("my notes.md.pre-import");
    assert_eq!(fs::read(&backup).unwrap(), before, "backup is the old file");
    assert_ne!(fs::read(&src).unwrap(), before);

    let check = assert_todo(tmp.path())
        .current_dir(tmp.path())
        .args(["--file", src.to_str().unwrap(), "check"])
        .output()
        .unwrap();
    assert!(check.status.success(), "todo check failed: {check:?}");

    // A second in-place import must not overwrite the backup.
    fs::write(&src, &before).unwrap();
    let again = import(tmp.path(), tmp.path(), &[src.to_str().unwrap(), "--yes"]);
    assert_eq!(again.status.code(), Some(4), "{again:?}");
    assert_eq!(fs::read(&backup).unwrap(), before);
    assert_eq!(fs::read(&src).unwrap(), before, "refused: nothing written");
}

#[test]
fn converted_fields_follow_the_rules() {
    let tmp = TempDir::new().unwrap();
    let src = source_in(tmp.path());
    let target = tmp.path().join("TODO.md");
    let out = import(
        tmp.path(),
        tmp.path(),
        &[
            src.to_str().unwrap(),
            "--out",
            target.to_str().unwrap(),
            "--yes",
        ],
    );
    assert!(out.status.success(), "{out:?}");
    assert!(src.exists(), "the source stays when --out differs");
    assert!(
        !tmp.path().join("TODO.md.pre-import").exists(),
        "no backup for a new target"
    );

    let shown = assert_todo(tmp.path())
        .current_dir(tmp.path())
        .args([
            "--json",
            "--file",
            target.to_str().unwrap(),
            "list",
            "--status",
            "all",
        ])
        .output()
        .unwrap();
    let json: Value = parse_json(&shown.stdout);
    let tasks = json["data"]["tasks"].as_array().unwrap();
    let status_of = |id: &str| {
        tasks
            .iter()
            .find(|t| t["id"] == id)
            .map(|t| t["status"].as_str().unwrap().to_string())
            .unwrap()
    };
    assert_eq!(status_of("T-001"), "open");
    assert_eq!(status_of("T-002"), "claimed");
    assert_eq!(status_of("T-003"), "blocked");
    assert_eq!(status_of("T-004"), "done");
    assert_eq!(status_of("T-005"), "done");
    assert_eq!(status_of("T-006"), "open");

    let text = fs::read_to_string(&target).unwrap().replace("\r\n", "\n");
    // Bold part is the title, the rest is the description.
    assert!(text.contains("### T-001 · Add export\n- status: open\n- owner: —\n- description: writes the list to a file\n"));
    // Claimed: owner from the suffix, original date kept in a note, sub-bullet became a note.
    assert!(text.contains("- owner: agent-ui-1\n- claimed: "));
    assert!(text.contains("- note (2026-11-11, agent-ui-1): reproduced on the second click\n"));
    assert!(text.contains("  - see also the notes in the old tracker\n"));
    // Added note; done note with and without commit.
    assert!(text.contains("- note (2026-11-01, coordinator): added\n"));
    // Blocked: `added` is not a block reason, so the import puts the checkbox list's reason on the
    // line after it (T-142). The first `added` note of 2026-11-01 is the blocked task's.
    let added = text
        .find("- note (2026-11-01, coordinator): added\n")
        .expect("the blocked task's added note");
    let next_line = text[added..]
        .lines()
        .nth(1)
        .expect("a line after the added note");
    assert!(
        next_line.ends_with("): blocked — imported from a checkbox list"),
        "{next_line}"
    );
    assert!(text.contains("- commit: abc1234\n"));
    assert!(text.contains("- note (2026-11-05, agent-doc-1): done\n"));
    assert!(text.contains("### T-005 · Tidy the build scripts\n- status: done\n- owner: —\n- description: no commit known\n- note (2026-11-06, agent-build): done\n"));
    // No bold, no dash: whole text is the title, no description.
    assert!(
        text.contains("### T-006 · Plain line without bold or dash\n- status: open\n- owner: —\n")
    );
    // Decision section gets type: question.
    assert!(text.contains(
        "### T-007 · Keep the old export format?\n- status: open\n- owner: —\n- type: question\n"
    ));
}

#[test]
fn non_task_lines_are_kept_byte_for_byte_and_crlf_is_kept() {
    let tmp = TempDir::new().unwrap();
    let src = source_in(tmp.path());
    let source_bytes = fs::read(&src).unwrap();
    let target = tmp.path().join("TODO.md");
    let out = import(
        tmp.path(),
        tmp.path(),
        &[
            src.to_str().unwrap(),
            "--out",
            target.to_str().unwrap(),
            "--yes",
        ],
    );
    assert!(out.status.success(), "{out:?}");
    let converted = fs::read(&target).unwrap();

    // Every `\n` is part of a `\r\n`: the fixture's line endings carry over to new lines.
    for (i, b) in converted.iter().enumerate() {
        if *b == b'\n' {
            assert_eq!(converted[i - 1], b'\r', "bare LF at byte {i}");
        }
    }

    // All source lines except the converted checkbox lines and the one note sub-bullet appear
    // unchanged, with their endings, in the same order.
    let converted_lines = lines_with_endings(&converted);
    let mut cursor = 0usize;
    let mut kept = 0usize;
    for line in lines_with_endings(&source_bytes) {
        let text = line.trim_end();
        let is_task_line = matches!(
            text,
            t if t.starts_with("- [ ] **Add export**")
                || t.starts_with("- [~] **Fix save button**")
                || t.starts_with("- [!] **Upgrade parser**")
                || t.starts_with("- [x] **Write README**")
                || t.starts_with("- [x] Tidy the build scripts")
                || t.starts_with("- [ ] Plain line")
                || t.starts_with("- [ ] **Keep the old export format?**")
                || t.starts_with("  - 2026-11-11 agent-ui-1:")
        );
        if is_task_line {
            continue;
        }
        let pos = converted_lines[cursor..]
            .iter()
            .position(|l| *l == line)
            .unwrap_or_else(|| panic!("line lost or changed: {line:?}"));
        cursor += pos + 1;
        kept += 1;
    }
    assert!(kept >= 15, "kept only {kept} lines");
    // Fences and preamble examples are text, not tasks.
    let text = String::from_utf8(converted).unwrap();
    assert!(text.contains("- [ ] checkbox inside a code fence\r\n"));
    assert!(text.contains("- [ ] this checkbox in the preamble is only an example\r\n"));
    assert!(text.contains("  - [ ] an indented sub-checkbox stays text\r\n"));
}

#[test]
fn json_output_reports_counts_and_the_preview() {
    let tmp = TempDir::new().unwrap();
    let src = source_in(tmp.path());
    let target = tmp.path().join("out.md");
    let out = assert_todo(tmp.path())
        .current_dir(tmp.path())
        .args(["--json", "import", "--from", "checkbox"])
        .args([src.to_str().unwrap(), "--out", target.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let json: Value = parse_json(&out.stdout);
    assert_eq!(json["schema"], "todo.import.v1");
    assert_eq!(json["data"]["written"], false);
    assert_eq!(json["data"]["checkbox_lines"], 7);
    assert_eq!(json["data"]["tasks"], 7);
    assert_eq!(json["data"]["problems"], 0);
    assert!(json["data"]["text"].as_str().unwrap().contains("T-007"));
    assert_eq!(json["data"]["converted"][1]["status"], "claimed");
    assert!(!target.exists());
}

#[test]
fn list_without_checkbox_tasks_is_refused() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("plain.md");
    fs::write(
        &src,
        "# Plan\n\n- [ ] only in the preamble\n\n## Later\n\nprose only\n",
    )
    .unwrap();
    let out = import(tmp.path(), tmp.path(), &[src.to_str().unwrap(), "--yes"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("no checkbox tasks found"), "{stderr}");
    assert_eq!(
        fs::read_to_string(&src).unwrap(),
        "# Plan\n\n- [ ] only in the preamble\n\n## Later\n\nprose only\n"
    );
}

#[test]
fn other_source_formats_are_rejected_by_usage() {
    let tmp = TempDir::new().unwrap();
    let src = source_in(tmp.path());
    let out = assert_todo(tmp.path())
        .current_dir(tmp.path())
        .args(["import", "--from", "todotxt", src.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2), "{out:?}");
}

#[test]
fn last_line_without_ending_stays_without_ending() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("l.md");
    fs::write(&src, "# T\n## A\n- [x] **One** (done: bob, 2026-11-01)").unwrap();
    let out = import(tmp.path(), tmp.path(), &[src.to_str().unwrap(), "--yes"]);
    assert!(out.status.success(), "{out:?}");
    let text = fs::read_to_string(&src).unwrap();
    assert!(text.ends_with("- note (2026-11-01, bob): done"), "{text:?}");
    let stamp = format!("<!-- todo-tool: {} -->", env!("CARGO_PKG_VERSION"));
    assert!(text.starts_with(&format!(
        "<!-- todo-format: 3 -->\n{stamp}\n# T\n## A\n\n### T-001 · One\n"
    )));
}
