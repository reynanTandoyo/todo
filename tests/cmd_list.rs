//! Integration tests for `todo list` (T-020; R-READ-1, R-READ-4).

mod common;

use std::fs;
use std::path::Path;

use tempfile::TempDir;

fn copy_clean_fixture(work: &Path) -> std::io::Result<()> {
    let fixture = fs::read("tests/fixtures/clean_basic.md")?;
    fs::write(work.join("TODO.md"), fixture)
}

#[test]
fn list_hides_done_and_dropped_by_default_and_prints_count() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let work = tmp.path().join("work");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&work).unwrap();
    copy_clean_fixture(&work).unwrap();

    let out = common::run_todo(
        &home,
        &work,
        &["list"],
        &[("TODO_TEST_NOW", "2026-11-15T15:00:00+01:00")],
    )
    .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("T-001"), "stdout:\n{stdout}");
    assert!(stdout.contains("T-002"), "stdout:\n{stdout}");
    assert!(stdout.contains("T-003"), "stdout:\n{stdout}");
    assert!(stdout.contains("T-006"), "stdout:\n{stdout}");
    assert!(!stdout.contains("T-004"), "done task leaked:\n{stdout}");
    assert!(!stdout.contains("T-005"), "dropped task leaked:\n{stdout}");
    assert!(stdout.contains("4 tasks"), "stdout:\n{stdout}");
}

#[test]
fn list_status_filter_supports_comma_list_all_and_rejects_invalid() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let work = tmp.path().join("work");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&work).unwrap();
    copy_clean_fixture(&work).unwrap();

    let out_done_drop = common::run_todo(
        &home,
        &work,
        &["--json", "list", "--status", "done,dropped"],
        &[],
    )
    .unwrap();
    assert!(out_done_drop.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&out_done_drop.stdout).unwrap();
    assert_eq!(parsed["data"]["count"], 2);
    let ids: Vec<&str> = parsed["data"]["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["T-004", "T-005"]);

    let out_all =
        common::run_todo(&home, &work, &["--json", "list", "--status", "all"], &[]).unwrap();
    assert!(out_all.status.success());
    let parsed_all: serde_json::Value = serde_json::from_slice(&out_all.stdout).unwrap();
    assert_eq!(parsed_all["data"]["count"], 6);

    let bad =
        common::run_todo(&home, &work, &["--json", "list", "--status", "bogus"], &[]).unwrap();
    assert_eq!(bad.status.code(), Some(1));
    let bad_json: serde_json::Value = serde_json::from_slice(&bad.stdout).unwrap();
    assert_eq!(bad_json["error"]["code"], "BAD_VALUE");
}

#[test]
fn list_attribute_and_where_filters_work() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let work = tmp.path().join("work");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&work).unwrap();
    copy_clean_fixture(&work).unwrap();

    // --priority
    let p0 = common::run_todo(&home, &work, &["--json", "list", "--priority", "p0"], &[]).unwrap();
    let p0_json: serde_json::Value = serde_json::from_slice(&p0.stdout).unwrap();
    assert_eq!(p0_json["data"]["count"], 1);
    assert_eq!(p0_json["data"]["tasks"][0]["id"], "T-001");

    // --section (ignores leading number `1.` and `2)`)
    let sec =
        common::run_todo(&home, &work, &["--json", "list", "--section", "later"], &[]).unwrap();
    let sec_json: serde_json::Value = serde_json::from_slice(&sec.stdout).unwrap();
    assert_eq!(sec_json["data"]["count"], 1);
    assert_eq!(sec_json["data"]["tasks"][0]["id"], "T-003");

    // --type
    let ty = common::run_todo(
        &home,
        &work,
        &["--json", "list", "--type", "MAINTENANCE"],
        &[],
    )
    .unwrap();
    let ty_json: serde_json::Value = serde_json::from_slice(&ty.stdout).unwrap();
    assert_eq!(ty_json["data"]["count"], 1);
    assert_eq!(ty_json["data"]["tasks"][0]["id"], "T-002");

    // --area
    let area = common::run_todo(
        &home,
        &work,
        &["--json", "list", "--area", "greenhouse"],
        &[],
    )
    .unwrap();
    let area_json: serde_json::Value = serde_json::from_slice(&area.stdout).unwrap();
    assert_eq!(area_json["data"]["count"], 1);
    assert_eq!(area_json["data"]["tasks"][0]["id"], "T-001");

    // --owner
    let owner = common::run_todo(
        &home,
        &work,
        &["--json", "list", "--owner", "GARDENER-BOT"],
        &[],
    )
    .unwrap();
    let owner_json: serde_json::Value = serde_json::from_slice(&owner.stdout).unwrap();
    assert_eq!(owner_json["data"]["count"], 1);
    assert_eq!(owner_json["data"]["tasks"][0]["id"], "T-001");

    // --owner none (nobody)
    let unowned =
        common::run_todo(&home, &work, &["--json", "list", "--owner", "none"], &[]).unwrap();
    let unowned_json: serde_json::Value = serde_json::from_slice(&unowned.stdout).unwrap();
    assert_eq!(unowned_json["data"]["count"], 3);

    // --where repeatable, unknown field, case-insensitive
    let where_out = common::run_todo(
        &home,
        &work,
        &[
            "--json",
            "list",
            "--where",
            "custom_tag=MORNING-SHIFT",
            "--where",
            "area=Greenhouse",
        ],
        &[],
    )
    .unwrap();
    let where_json: serde_json::Value = serde_json::from_slice(&where_out.stdout).unwrap();
    assert_eq!(where_json["data"]["count"], 1);
    assert_eq!(where_json["data"]["tasks"][0]["id"], "T-001");

    // invalid --where exits 2
    let bad_where = common::run_todo(
        &home,
        &work,
        &["--json", "list", "--where", "noequals"],
        &[],
    )
    .unwrap();
    assert_eq!(bad_where.status.code(), Some(2));
}

#[cfg(feature = "test-hooks")]
#[test]
fn list_mine_stale_and_ready_filters_work() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let work = tmp.path().join("work");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&work).unwrap();

    // T-001 claimed at 09:30+01:00; T-002 open depends on T-001; T-006 open question.
    // Add T-007 open ready task and T-008 open task depending on archived T-099.
    let mut text = fs::read_to_string("tests/fixtures/clean_basic.md").unwrap();
    text.push_str(
        "\n### T-007 · Ready open task\n- status: open\n- owner: —\n- priority: P1\n\
         \n### T-008 · Depends on archived task\n- status: open\n- owner: —\n- depends: T-099\n",
    );
    fs::write(work.join("TODO.md"), text).unwrap();
    fs::write(
        work.join("TODO-archive.md"),
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n## Archive\n\n\
         ### T-099 · Archived prerequisite\n- status: done\n- owner: helper-1\n",
    )
    .unwrap();

    // --mine requires agent
    let no_agent = common::run_todo(&home, &work, &["--json", "list", "--mine"], &[]).unwrap();
    assert_eq!(no_agent.status.code(), Some(2));

    let mine = common::run_todo(
        &home,
        &work,
        &["--json", "list", "--mine", "--agent", "gardener-bot"],
        &[("TODO_TEST_NOW", "2026-11-15T15:00:00+01:00")],
    )
    .unwrap();
    assert!(mine.status.success());
    let mine_json: serde_json::Value = serde_json::from_slice(&mine.stdout).unwrap();
    assert_eq!(mine_json["data"]["count"], 1);
    assert_eq!(mine_json["data"]["tasks"][0]["id"], "T-001");

    // At 15:00 same day, T-001 is NOT stale; ready tasks are T-007 and T-008.
    let ready_fresh = common::run_todo(
        &home,
        &work,
        &["--json", "list", "--ready"],
        &[("TODO_TEST_NOW", "2026-11-15T15:00:00+01:00")],
    )
    .unwrap();
    let ready_json: serde_json::Value = serde_json::from_slice(&ready_fresh.stdout).unwrap();
    let ready_ids: Vec<&str> = ready_json["data"]["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap())
        .collect();
    assert_eq!(ready_ids, vec!["T-007", "T-008"]);

    let stale_fresh = common::run_todo(
        &home,
        &work,
        &["--json", "list", "--stale"],
        &[("TODO_TEST_NOW", "2026-11-15T15:00:00+01:00")],
    )
    .unwrap();
    let stale_fresh_json: serde_json::Value = serde_json::from_slice(&stale_fresh.stdout).unwrap();
    assert_eq!(stale_fresh_json["data"]["count"], 0);

    // After 25 hours, T-001 IS stale and therefore also appears in --ready.
    let stale_old = common::run_todo(
        &home,
        &work,
        &["--json", "list", "--stale"],
        &[("TODO_TEST_NOW", "2026-11-16T10:31:00+01:00")],
    )
    .unwrap();
    let stale_old_json: serde_json::Value = serde_json::from_slice(&stale_old.stdout).unwrap();
    assert_eq!(stale_old_json["data"]["count"], 1);
    assert_eq!(stale_old_json["data"]["tasks"][0]["id"], "T-001");

    let ready_old = common::run_todo(
        &home,
        &work,
        &["--json", "list", "--ready"],
        &[("TODO_TEST_NOW", "2026-11-16T10:31:00+01:00")],
    )
    .unwrap();
    let ready_old_json: serde_json::Value = serde_json::from_slice(&ready_old.stdout).unwrap();
    let ready_old_ids: Vec<&str> = ready_old_json["data"]["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap())
        .collect();
    assert_eq!(ready_old_ids, vec!["T-001", "T-007", "T-008"]);
}

#[test]
fn list_archive_reads_archive_when_present_and_empty_when_absent() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let work = tmp.path().join("work");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&work).unwrap();
    copy_clean_fixture(&work).unwrap();

    let absent = common::run_todo(&home, &work, &["--json", "list", "--archive"], &[]).unwrap();
    assert!(absent.status.success());
    let absent_json: serde_json::Value = serde_json::from_slice(&absent.stdout).unwrap();
    assert_eq!(absent_json["data"]["archive"], true);
    assert_eq!(absent_json["data"]["count"], 0);

    fs::write(
        work.join("TODO-archive.md"),
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n## Done\n\n\
         ### T-090 · Archived done task\n- status: done\n- owner: alice\n\
         ### T-091 · Archived dropped task\n- status: dropped\n- owner: —\n",
    )
    .unwrap();

    let present = common::run_todo(&home, &work, &["--json", "list", "--archive"], &[]).unwrap();
    assert!(present.status.success());
    let present_json: serde_json::Value = serde_json::from_slice(&present.stdout).unwrap();
    assert_eq!(present_json["data"]["archive"], true);
    assert_eq!(present_json["data"]["count"], 2);
}

#[test]
fn list_shortens_titles_unless_wide_and_warns_on_unsafe_files() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let work = tmp.path().join("work");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&work).unwrap();

    fs::write(
        work.join("TODO.md"),
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
         ## Tasks\n\n\
         ### T-001 · A very long title that should definitely be truncated when terminal width is narrow\n\
         - status: open\n\n\
         <<<<<<< HEAD\n",
    )
    .unwrap();

    let narrow = common::run_todo(&home, &work, &["list"], &[("COLUMNS", "50")]).unwrap();
    assert!(narrow.status.success());
    let narrow_out = String::from_utf8_lossy(&narrow.stdout);
    let narrow_err = String::from_utf8_lossy(&narrow.stderr);
    assert!(narrow_err.contains("warning: TODO.md:"));
    let first_line = narrow_out.lines().next().unwrap();
    assert!(first_line.chars().count() <= 50, "line: {first_line}");
    assert!(first_line.ends_with('…'), "line: {first_line}");

    let wide = common::run_todo(&home, &work, &["list", "--wide"], &[("COLUMNS", "50")]).unwrap();
    assert!(wide.status.success());
    let wide_out = String::from_utf8_lossy(&wide.stdout);
    assert!(wide_out.contains("when terminal width is narrow"));
}

#[cfg(feature = "test-hooks")]
#[test]
fn list_json_matches_snapshot() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let work = tmp.path().join("work");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&work).unwrap();
    copy_clean_fixture(&work).unwrap();

    let out = common::run_todo(
        &home,
        &work,
        &["--json", "list"],
        &[("TODO_TEST_NOW", "2026-11-15T15:00:00+01:00")],
    )
    .unwrap();
    assert!(out.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    insta::assert_json_snapshot!("cmd_list_json", parsed);
}
