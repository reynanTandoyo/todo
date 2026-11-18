//! Migrations through the CLI (`docs/format.md` §18, R-MIG-1): the first write on an older file
//! migrates it under the lock, says so in one line on standard error and in the journal; `todo
//! migrate` does the same without another change; `--dry-run` prints the diff; read commands never
//! write; the archive is migrated in the same write; a restored old snapshot migrates again.

mod common;

use std::fs;

use tempfile::TempDir;

use common::{
    copy_fixture, count_tmp_files, parse_json, read_todo, run_in, set_read_only, stderr_text,
    stdout_text,
};

/// The running version, as Cargo.toml gives it.
const RUNNING: &str = env!("CARGO_PKG_VERSION");

const MARKER: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->";

fn stamp_line() -> String {
    format!("<!-- todo-tool: {RUNNING} -->")
}

/// Runs a write command that must succeed and returns the output.
fn ok(dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    let out = run_in(dir, args);
    assert!(
        out.status.success(),
        "{args:?} failed: {}",
        stderr_text(&out)
    );
    out
}

/// `text` with the format 1 line raised to 2 and the tool line added after it: the whole change a
/// migration makes to a file whose only other lines stay as they are.
fn migrated_text(text: &str, eol: &str) -> String {
    let format_1 = format!("<!-- todo-format: 1 -->{eol}");
    let format_2 = format!("<!-- todo-format: 2 -->{eol}<!-- todo-tool: {RUNNING} -->{eol}");
    assert!(text.contains(&format_1), "no format 1 line in {text:?}");
    text.replacen(&format_1, &format_2, 1)
}

#[test]
fn the_first_write_migrates_an_lf_file_and_says_so_in_one_stderr_line() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "migrate_format1_lf.md");
    let before = read_todo(dir.path());

    let out = ok(
        dir.path(),
        &["note", "T-001", "checked the wicks", "--agent", "bob"],
    );
    let stderr = stderr_text(&out);
    assert_eq!(stderr.lines().count(), 1, "stderr: {stderr:?}");
    assert_eq!(stderr.trim_end(), "migrated TODO.md: format 1 to 2");

    // The note is the only line that the write adds; everything else is the migration.
    let after = read_todo(dir.path());
    let without_note: String = after
        .split_inclusive('\n')
        .filter(|line| !line.contains("checked the wicks"))
        .collect();
    assert_eq!(without_note, migrated_text(&before, "\n"));
    assert!(after.contains("<!-- todo-format: 2 -->\n"));
    assert!(after.contains(&format!("{}\n", stamp_line())));
}

#[test]
fn the_first_write_migrates_a_crlf_and_byte_order_mark_file_with_every_other_byte_kept() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "migrate_format1_bom_crlf.md");
    let before = fs::read(dir.path().join("TODO.md")).expect("read");

    ok(
        dir.path(),
        &["note", "T-004", "compared two oils", "--agent", "bob"],
    );

    let after = fs::read(dir.path().join("TODO.md")).expect("read");
    assert_eq!(
        &after[..3],
        &[0xEF, 0xBB, 0xBF],
        "the byte-order mark stays first"
    );
    let before_text = String::from_utf8(before).expect("UTF-8");
    let after_text = String::from_utf8(after).expect("UTF-8");
    let without_note: String = after_text
        .split_inclusive('\n')
        .filter(|line| !line.contains("compared two oils"))
        .collect();
    assert_eq!(without_note, migrated_text(&before_text, "\r\n"));
}

#[test]
fn migrate_changes_only_the_format_and_tool_lines_and_a_second_run_changes_nothing() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "migrate_format1_crlf.md");
    let before = read_todo(dir.path());

    let out = ok(dir.path(), &["migrate", "--agent", "bob"]);
    assert_eq!(
        stdout_text(&out).trim_end(),
        "migrated TODO.md: format 1 to 2"
    );
    assert_eq!(read_todo(dir.path()), migrated_text(&before, "\r\n"));

    let migrated = read_todo(dir.path());
    let again = ok(dir.path(), &["migrate", "--agent", "bob"]);
    assert_eq!(
        stdout_text(&again).trim_end(),
        "TODO.md is already in format 2: nothing to migrate"
    );
    assert_eq!(
        read_todo(dir.path()),
        migrated,
        "a second run changes nothing"
    );
}

#[test]
fn migrate_dry_run_prints_the_diff_and_writes_nothing() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "migrate_format1_lf.md");
    let before = fs::read(dir.path().join("TODO.md")).expect("read");

    let out = ok(dir.path(), &["--dry-run", "migrate", "--agent", "bob"]);
    let shown = stdout_text(&out);
    assert!(shown.contains("-<!-- todo-format: 1 -->"), "{shown}");
    assert!(shown.contains("+<!-- todo-format: 2 -->"), "{shown}");
    assert!(shown.contains(&format!("+{}", stamp_line())), "{shown}");
    assert_eq!(fs::read(dir.path().join("TODO.md")).expect("read"), before);
    assert!(!dir.path().join(".todo").join("journal.jsonl").exists());
}

#[test]
fn a_file_without_a_format_line_gets_the_format_line_first_and_the_stamp_after_it() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "migrate_format1_no_format_line.md");
    let before = read_todo(dir.path());

    ok(dir.path(), &["migrate", "--agent", "bob"]);

    let expected = format!("<!-- todo-format: 2 -->\n{}\n{before}", stamp_line());
    assert_eq!(read_todo(dir.path()), expected);
}

#[test]
fn read_commands_read_an_older_file_and_never_write_it() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "migrate_format1_lf.md");
    let before = read_todo(dir.path());

    for args in [
        vec!["list"],
        vec!["--json", "list"],
        vec!["summary"],
        vec!["where"],
        vec!["show", "T-001"],
        vec!["next"],
        vec!["search", "wicks"],
    ] {
        let out = run_in(dir.path(), &args);
        assert!(
            out.status.success(),
            "{args:?} failed: {}",
            stderr_text(&out)
        );
        assert_eq!(read_todo(dir.path()), before, "{args:?} wrote TODO.md");
    }
    let where_out = stdout_text(&run_in(dir.path(), &["where"]));
    assert!(where_out.contains("Format version: 1"), "{where_out}");
}

#[test]
fn a_write_that_changes_nothing_does_not_migrate_the_file() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "migrate_format1_lf.md");
    let before = read_todo(dir.path());

    // T-001 is already a feature, so this changes no line (R-EDIT-6).
    ok(
        dir.path(),
        &["set", "T-001", "--type", "feature", "--agent", "bob"],
    );
    assert_eq!(read_todo(dir.path()), before);
}

#[test]
fn the_journal_entry_of_a_migrating_write_says_what_was_migrated() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "migrate_format1_lf.md");

    ok(
        dir.path(),
        &["note", "T-001", "checked the wicks", "--agent", "bob"],
    );

    let journal =
        fs::read_to_string(dir.path().join(".todo").join("journal.jsonl")).expect("journal");
    let last: serde_json::Value =
        serde_json::from_str(journal.lines().last().expect("an entry")).expect("JSON");
    assert_eq!(last["command"], "note");
    assert_eq!(last["migrated"], "TODO.md: format 1 to 2");
}

#[test]
fn migrate_writes_one_journal_entry_naming_the_migration_and_none_for_a_second_run() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "migrate_format1_lf.md");

    ok(dir.path(), &["migrate", "--agent", "bob"]);
    ok(dir.path(), &["migrate", "--agent", "bob"]);

    let journal =
        fs::read_to_string(dir.path().join(".todo").join("journal.jsonl")).expect("journal");
    let entries: Vec<serde_json::Value> = journal
        .lines()
        .map(|l| serde_json::from_str(l).expect("JSON"))
        .collect();
    assert_eq!(entries.len(), 1, "the second run writes nothing: {journal}");
    assert_eq!(entries[0]["command"], "migrate");
    assert_eq!(entries[0]["migrated"], "TODO.md: format 1 to 2");
    assert_eq!(entries[0]["tasks"], serde_json::json!([]));
}

#[test]
fn json_output_lists_each_migrated_file_with_its_versions() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "migrate_format1_lf.md");

    let out = ok(dir.path(), &["--json", "migrate", "--agent", "bob"]);
    let json = parse_json(&out.stdout);
    assert_eq!(json["ok"], true);
    assert_eq!(json["data"]["current_format"], 2);
    assert_eq!(json["data"]["changed"], true);
    assert_eq!(json["data"]["migrated"][0]["file"], "TODO.md");
    assert_eq!(json["data"]["migrated"][0]["from"], 1);
    assert_eq!(json["data"]["migrated"][0]["to"], 2);
}

/// An archive in format 1 that holds one old task.
fn format1_archive() -> String {
    format!(
        "{MARKER}\n<!-- todo-format: 1 -->\n# Archive\n\n## Done\n\n### T-000 · Old task\n\
         - status: done\n- owner: bob\n- note (2026-11-01, bob): done\n"
    )
}

#[test]
fn a_read_only_todo_md_stops_the_migration_before_the_archive_is_migrated() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "migrate_format1_lf.md");
    let todo = dir.path().join("TODO.md");
    let archive = dir.path().join("TODO-archive.md");
    let todo_before = read_todo(dir.path());
    let archive_before = format1_archive();
    fs::write(&archive, &archive_before).expect("archive");
    set_read_only(&todo, true);

    // Both a migrating `migrate` and a migrating write (`note`) are refused.
    let commands: [&[&str]; 2] = [
        &["migrate", "--yes", "--agent", "bob"],
        &["note", "T-001", "checked the wicks", "--agent", "bob"],
    ];
    let mut outs = Vec::new();
    for args in commands {
        outs.push(run_in(dir.path(), args));
    }
    set_read_only(&todo, false);

    for out in &outs {
        assert_eq!(out.status.code(), Some(4), "{}", stderr_text(out));
        assert!(
            stderr_text(out).contains("read-only"),
            "{}",
            stderr_text(out)
        );
    }
    assert_eq!(
        fs::read_to_string(&archive).expect("archive"),
        archive_before
    );
    assert_eq!(read_todo(dir.path()), todo_before);
    assert_eq!(count_tmp_files(dir.path()), 0);
}

/// TODO.md as another program leaves it while `migrate` waits to replace it.
#[cfg(feature = "test-hooks")]
const EXTERNAL_EDIT: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
# Edited by hand while migrate ran\n";

#[cfg(feature = "test-hooks")]
#[test]
fn migrate_puts_the_old_archive_back_when_todo_md_changes_before_it_is_replaced() {
    let tmp = TempDir::new().expect("tempdir");
    let home = tmp.path().join("home");
    let work = tmp.path().join("work");
    fs::create_dir_all(&home).expect("home");
    fs::create_dir_all(&work).expect("work");
    copy_fixture(&work, "migrate_format1_lf.md");
    let archive_before = format1_archive();
    fs::write(work.join("TODO-archive.md"), &archive_before).expect("archive");

    let ready = tmp.path().join("rename.ready");
    let ready_str = ready.to_string_lossy().into_owned();
    let child = common::spawn_todo(
        &home,
        &work,
        &["migrate", "--yes", "--agent", "bob"],
        &[
            ("TODO_TEST_BEFORE_RENAME_READY_FILE", ready_str.as_str()),
            ("TODO_TEST_BEFORE_RENAME_HOLD_MS", "1000"),
        ],
    )
    .expect("spawn migrate");
    common::wait_for_file(&ready, std::time::Duration::from_secs(10))
        .expect("migrate reached a rename");
    // The first pause is the replace of the migrated archive: TODO.md changes during it.
    fs::write(work.join("TODO.md"), EXTERNAL_EDIT).expect("external edit");
    let out = child.wait_with_output().expect("wait for migrate");

    assert_eq!(out.status.code(), Some(3), "{}", stderr_text(&out));
    assert_eq!(
        fs::read_to_string(work.join("TODO-archive.md")).expect("archive"),
        archive_before
    );
    assert_eq!(read_todo(&work), EXTERNAL_EDIT);
    assert_eq!(count_tmp_files(&work), 0);
}

#[test]
fn the_archive_is_migrated_in_the_same_write_as_todo_md() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "migrate_format1_lf.md");
    fs::write(
        dir.path().join("TODO-archive.md"),
        format!(
            "{MARKER}\n<!-- todo-format: 1 -->\n# Archive\n\n## Done\n\n### T-000 · Old task\n\
             - status: done\n- owner: bob\n- note (2026-11-01, bob): done\n"
        ),
    )
    .expect("archive");

    let out = ok(
        dir.path(),
        &["note", "T-001", "checked the wicks", "--agent", "bob"],
    );
    assert_eq!(
        stderr_text(&out).trim_end(),
        "migrated TODO.md: format 1 to 2; TODO-archive.md: format 1 to 2"
    );
    let archive = fs::read_to_string(dir.path().join("TODO-archive.md")).expect("archive");
    let head: Vec<&str> = archive.lines().take(3).collect();
    assert_eq!(
        head,
        [MARKER, "<!-- todo-format: 2 -->", stamp_line().as_str()]
    );
}

#[test]
fn archive_migrates_an_old_archive_in_the_same_write_as_it_moves_tasks() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "migrate_format1_lf.md");
    fs::write(
        dir.path().join("TODO-archive.md"),
        "<!-- todo-format: 1 -->\n# Archive\n\n## Done\n\n### T-000 · Old task\n\
         - status: done\n- owner: bob\n- note (2026-11-01, bob): done\n",
    )
    .expect("archive");

    let out = ok(dir.path(), &["archive", "--yes", "--agent", "bob"]);
    assert_eq!(
        stderr_text(&out).trim_end(),
        "migrated TODO.md: format 1 to 2; TODO-archive.md: format 1 to 2"
    );
    let archive = fs::read_to_string(dir.path().join("TODO-archive.md")).expect("archive");
    assert!(
        archive.starts_with("<!-- todo-format: 2 -->\n"),
        "{archive}"
    );
    assert!(archive.contains("### T-000 · Old task"));
    assert!(
        archive.contains("### T-003 · Light the porch lamp"),
        "{archive}"
    );
    assert!(read_todo(dir.path()).starts_with(&format!(
        "{MARKER}\n<!-- todo-format: 2 -->\n{}\n",
        stamp_line()
    )));
}

#[test]
fn restoring_a_snapshot_from_before_the_migration_works_and_the_next_write_migrates_it_again() {
    let dir = TempDir::new().expect("tempdir");
    copy_fixture(dir.path(), "migrate_format1_lf.md");
    let original = read_todo(dir.path());

    ok(
        dir.path(),
        &["note", "T-001", "checked the wicks", "--agent", "bob"],
    );
    let history = parse_json(&run_in(dir.path(), &["--json", "history"]).stdout);
    let number = history["data"]["snapshots"][0]["number"]
        .as_u64()
        .expect("snapshot number")
        .to_string();

    ok(dir.path(), &["restore", &number, "--yes", "--agent", "bob"]);
    let restored = read_todo(dir.path());
    assert!(restored.contains("<!-- todo-format: 1 -->\n"), "{restored}");
    assert!(!restored.contains("checked the wicks"));
    assert_eq!(
        restored.replacen(&format!("{}\n", stamp_line()), "", 1),
        original,
        "the snapshot is the file as it was before the migration"
    );

    ok(
        dir.path(),
        &["note", "T-001", "checked the wicks again", "--agent", "bob"],
    );
    assert!(read_todo(dir.path()).contains("<!-- todo-format: 2 -->\n"));
}
