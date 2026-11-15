//! Automated tests that piped `todo` output is valid UTF-8 (`R-OUT-6`, T-006).

mod common;

use std::fs;

use tempfile::TempDir;

#[test]
fn piped_output_is_utf8_with_middot_emdash_and_japanese_text() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let work = tmp.path().join("work");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&work).unwrap();

    let fixture = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
## Now\n\n\
### T-001 · 日本語タスク · 確認 — テスト\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: feature · area: cli\n\
- note (2026-11-15, alice): メモ — UTF-8 テスト\n";
    fs::write(work.join("TODO.md"), fixture.as_bytes()).unwrap();

    // `todo list` piped output
    let list_out = common::run_todo(&home, &work, &["list", "--wide"], &[]).unwrap();
    assert!(list_out.status.success());
    let list_str = std::str::from_utf8(&list_out.stdout).expect("list stdout must be valid UTF-8");
    assert!(list_str.contains('\u{00b7}'), "missing middot: {list_str}");
    assert!(list_str.contains('\u{2014}'), "missing em-dash: {list_str}");
    assert!(
        list_str.contains("日本語タスク"),
        "missing Japanese text: {list_str}"
    );

    // `todo show` piped output
    let show_out = common::run_todo(&home, &work, &["show", "T-001"], &[]).unwrap();
    assert!(show_out.status.success());
    let show_str = std::str::from_utf8(&show_out.stdout).expect("show stdout must be valid UTF-8");
    assert!(show_str.contains("T-001 · 日本語タスク · 確認 — テスト"));
    assert!(show_str.contains("- owner: —"));
    assert!(show_str.contains("メモ — UTF-8 テスト"));

    // `todo --json list` piped output
    let json_out = common::run_todo(&home, &work, &["--json", "list"], &[]).unwrap();
    assert!(json_out.status.success());
    let json_str = std::str::from_utf8(&json_out.stdout).expect("json stdout must be valid UTF-8");
    let parsed: serde_json::Value = serde_json::from_str(json_str).unwrap();
    assert_eq!(
        parsed["data"]["tasks"][0]["title"],
        "日本語タスク · 確認 — テスト"
    );
}
