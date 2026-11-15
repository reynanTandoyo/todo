//! Integration tests for `todo show` (T-021; R-READ-2, R-DEP-2).

mod common;

use common::{parse_json, read_todo, stderr_text, stdout_text};

use std::fs;
use std::path::Path;

use tempfile::TempDir;

fn copy_clean_fixture(work: &Path) -> std::io::Result<()> {
    let fixture = fs::read("tests/fixtures/clean_basic.md")?;
    fs::write(work.join("TODO.md"), fixture)
}

#[test]
fn show_text_output_equals_task_lines_in_file_and_accepts_flexible_ids() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let work = tmp.path().join("work");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&work).unwrap();
    copy_clean_fixture(&work).unwrap();

    let file_text = read_todo(&work);
    let file_lines: Vec<&str> = file_text.lines().collect();
    // T-001 occupies 1-based lines 9..=24 (0-based 8..24).
    let expected = format!("{}\n", file_lines[8..24].join("\n"));

    for query in ["T-001", "t1", "1", "T-01"] {
        let out = common::run_todo(&home, &work, &["show", query], &[]).unwrap();
        assert!(
            out.status.success(),
            "show {query} failed: {}",
            stderr_text(&out)
        );
        let stdout = String::from_utf8(out.stdout).unwrap();
        assert_eq!(stdout, expected, "mismatch for query `{query}`");
    }
}

#[cfg(feature = "test-hooks")]
#[test]
fn show_json_includes_unknown_fields_notes_lines_dependencies_and_dependents() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let work = tmp.path().join("work");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&work).unwrap();
    copy_clean_fixture(&work).unwrap();

    // T-001 has unknown field `custom_tag`, 2 notes, lines 9..24, and T-002 depends on it.
    let out1 = common::run_todo(
        &home,
        &work,
        &["--json", "show", "T-001"],
        &[("TODO_TEST_NOW", "2026-11-15T15:00:00+01:00")],
    )
    .unwrap();
    assert!(out1.status.success());
    let j1: serde_json::Value = parse_json(&out1.stdout);
    assert_eq!(j1["schema"], "todo.show.v1");
    assert_eq!(j1["ok"], true);
    assert_eq!(j1["data"]["id"], "T-001");
    assert_eq!(j1["data"]["section"], "1. Now");
    assert_eq!(j1["data"]["archived"], false);
    assert_eq!(j1["data"]["line_start"], 9);
    assert_eq!(j1["data"]["line_end"], 24);
    assert_eq!(j1["data"]["fields"]["custom_tag"], "morning-shift");
    assert_eq!(j1["data"]["notes"].as_array().unwrap().len(), 2);
    assert_eq!(j1["data"]["notes"][0]["date"], "2026-11-15");
    assert_eq!(j1["data"]["notes"][0]["agent"], "coordinator");
    assert_eq!(j1["data"]["notes"][0]["text"], "added");
    assert_eq!(j1["data"]["notes"][0]["line"], 19);
    assert_eq!(j1["data"]["dependents"].as_array().unwrap().len(), 1);
    assert_eq!(j1["data"]["dependents"][0]["id"], "T-002");
    assert_eq!(j1["data"]["dependents"][0]["status"], "open");

    // T-002 depends on T-001 (which is claimed -> not satisfied).
    let out2 = common::run_todo(
        &home,
        &work,
        &["--json", "show", "T-002"],
        &[("TODO_TEST_NOW", "2026-11-15T15:00:00+01:00")],
    )
    .unwrap();
    assert!(out2.status.success());
    let j2: serde_json::Value = parse_json(&out2.stdout);
    assert_eq!(j2["data"]["dependencies"].as_array().unwrap().len(), 1);
    assert_eq!(j2["data"]["dependencies"][0]["id"], "T-001");
    assert_eq!(j2["data"]["dependencies"][0]["exists"], true);
    assert_eq!(j2["data"]["dependencies"][0]["status"], "claimed");
    assert_eq!(j2["data"]["dependencies"][0]["satisfied"], false);
}

#[test]
fn show_finds_archived_tasks_and_archived_dependencies() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let work = tmp.path().join("work");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&work).unwrap();

    fs::write(
        work.join("TODO.md"),
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n## Now\n\n\
         ### T-002 · Follows archived work\n- status: open\n- depends: T-001, T-999\n",
    )
    .unwrap();
    fs::write(
        work.join("TODO-archive.md"),
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n## Now\n\n\
         ### T-001 · Archived step\n- status: done\n- owner: alice\n",
    )
    .unwrap();

    let arch_show = common::run_todo(&home, &work, &["--json", "show", "T-001"], &[]).unwrap();
    assert!(arch_show.status.success());
    let aj: serde_json::Value = parse_json(&arch_show.stdout);
    assert_eq!(aj["data"]["archived"], true);
    assert_eq!(aj["data"]["status"], "done");
    assert_eq!(aj["data"]["dependents"][0]["id"], "T-002");

    let active_show = common::run_todo(&home, &work, &["--json", "show", "T-002"], &[]).unwrap();
    assert!(active_show.status.success());
    let tj: serde_json::Value = parse_json(&active_show.stdout);
    assert_eq!(tj["data"]["dependencies"][0]["id"], "T-001");
    assert_eq!(tj["data"]["dependencies"][0]["archived"], true);
    assert_eq!(tj["data"]["dependencies"][0]["status"], "done");
    assert_eq!(tj["data"]["dependencies"][0]["satisfied"], true);
    assert_eq!(tj["data"]["dependencies"][1]["id"], "T-999");
    assert_eq!(tj["data"]["dependencies"][1]["exists"], false);
    assert_eq!(tj["data"]["dependencies"][1]["satisfied"], false);
}

#[test]
fn show_missing_task_returns_not_found_and_unsafe_file_still_warns() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let work = tmp.path().join("work");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&work).unwrap();
    copy_clean_fixture(&work).unwrap();

    let missing = common::run_todo(&home, &work, &["--json", "show", "T-999"], &[]).unwrap();
    assert_eq!(missing.status.code(), Some(1));
    let mj: serde_json::Value = parse_json(&missing.stdout);
    assert_eq!(mj["error"]["code"], "NOT_FOUND");
    assert_eq!(mj["error"]["task"], "T-999");

    // Unsafe file (conflict marker) still allows show with a warning.
    fs::write(
        work.join("TODO.md"),
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n## Now\n\n\
         ### T-001 · Readable despite marker\n- status: open\n\n<<<<<<< HEAD\n",
    )
    .unwrap();
    let unsafe_show = common::run_todo(&home, &work, &["show", "T-001"], &[]).unwrap();
    assert!(unsafe_show.status.success());
    assert!(stdout_text(&unsafe_show).contains("Readable despite marker"));
    assert!(stderr_text(&unsafe_show).contains("warning: TODO.md:"));
}
