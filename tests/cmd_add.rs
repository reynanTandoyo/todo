//! CLI integration tests for `todo add` (T-028; R-WRITE-1, R-ID-2, TEST-4, TEST-5).

mod common;

use std::collections::BTreeSet;
use std::fs;
use std::sync::{Arc, Barrier};
use std::thread;

use tempfile::TempDir;
use todo::doc::Document;

fn setup_proj(
    tmp: &TempDir,
    body: &str,
) -> std::io::Result<(std::path::PathBuf, std::path::PathBuf, std::path::PathBuf)> {
    let home = tmp.path().join("home");
    let work = tmp.path().join("work");
    fs::create_dir_all(&home)?;
    fs::create_dir_all(&work)?;
    let todo_file = work.join("TODO.md");
    fs::write(&todo_file, body)?;
    Ok((home, work, todo_file))
}

#[test]
fn add_sections_fields_duplicates_and_monotonic_ids() {
    let tmp = TempDir::new().unwrap();
    let initial = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
## 1. Now\n\n\
### T-001 · Existing open\n\
- status: open\n\
- owner: —\n\
- priority: P1\n\n\
## 2. Next Release\n\n\
### T-002 · Already finished\n\
- status: done\n\
- owner: alice\n\n\
## 3. Next Ideas\n\n";
    let (home, work, todo_file) = setup_proj(&tmp, initial).unwrap();
    let now = "2026-11-15T14:00:00+01:00";

    // Archive has T-005, so first added ID must be T-006!
    let archive_path = work.join("TODO-archive.md");
    fs::write(
        &archive_path,
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
## Archive\n\n\
### T-005 · Archived task\n\
- status: done\n\
- owner: alice\n",
    )
    .unwrap();

    // 1. Unknown section -> UNKNOWN_SECTION (exit 1) listing available sections.
    let before = fs::read(&todo_file).unwrap();
    let unk = common::run_todo(
        &home,
        &work,
        &[
            "--json",
            "add",
            "--title",
            "New task",
            "--priority",
            "P1",
            "--section",
            "Backlog",
            "--agent",
            "alice",
        ],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert_eq!(unk.status.code(), Some(1));
    let j_unk: serde_json::Value = serde_json::from_slice(&unk.stdout).unwrap();
    assert_eq!(j_unk["error"]["code"], "UNKNOWN_SECTION");
    let msg_unk = j_unk["error"]["message"].as_str().unwrap();
    assert!(msg_unk.contains("1. Now"), "msg: {msg_unk}");
    assert!(msg_unk.contains("2. Next Release"), "msg: {msg_unk}");
    assert!(msg_unk.contains("3. Next Ideas"), "msg: {msg_unk}");

    // 2. Ambiguous section ("Next") -> AMBIGUOUS_SECTION (exit 1) listing matches & sections.
    let amb = common::run_todo(
        &home,
        &work,
        &[
            "--json",
            "add",
            "--title",
            "New task",
            "--priority",
            "P1",
            "--section",
            "Next",
            "--agent",
            "alice",
        ],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert_eq!(amb.status.code(), Some(1));
    let j_amb: serde_json::Value = serde_json::from_slice(&amb.stdout).unwrap();
    assert_eq!(j_amb["error"]["code"], "AMBIGUOUS_SECTION");
    let msg_amb = j_amb["error"]["message"].as_str().unwrap();
    assert!(msg_amb.contains("2. Next Release"), "msg: {msg_amb}");
    assert!(msg_amb.contains("3. Next Ideas"), "msg: {msg_amb}");

    // 3. Omitting --section when multiple sections exist -> refused.
    let no_sec = common::run_todo(
        &home,
        &work,
        &[
            "--json",
            "add",
            "--title",
            "New task",
            "--priority",
            "P1",
            "--agent",
            "alice",
        ],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert_eq!(no_sec.status.code(), Some(1));

    // 4. Duplicate title of open task -> DUPLICATE_TITLE (exit 1).
    let dup = common::run_todo(
        &home,
        &work,
        &[
            "--json",
            "add",
            "--title",
            "existing open",
            "--priority",
            "P1",
            "--section",
            "Now",
            "--agent",
            "alice",
        ],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert_eq!(dup.status.code(), Some(1));
    let j_dup: serde_json::Value = serde_json::from_slice(&dup.stdout).unwrap();
    assert_eq!(j_dup["error"]["code"], "DUPLICATE_TITLE");

    // 5. Missing required priority -> MISSING_FIELD; invalid priority -> BAD_VALUE.
    let miss_p = common::run_todo(
        &home,
        &work,
        &[
            "--json",
            "add",
            "--title",
            "No prio",
            "--section",
            "Now",
            "--agent",
            "alice",
        ],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert_eq!(miss_p.status.code(), Some(1));
    let j_mp: serde_json::Value = serde_json::from_slice(&miss_p.stdout).unwrap();
    assert_eq!(j_mp["error"]["code"], "MISSING_FIELD");

    let bad_p = common::run_todo(
        &home,
        &work,
        &[
            "--json",
            "add",
            "--title",
            "Bad prio",
            "--priority",
            "P9",
            "--section",
            "Now",
            "--agent",
            "alice",
        ],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert_eq!(bad_p.status.code(), Some(1));
    let j_bp: serde_json::Value = serde_json::from_slice(&bad_p.stdout).unwrap();
    assert_eq!(j_bp["error"]["code"], "BAD_VALUE");

    // All refusals left TODO.md byte-identical.
    assert_eq!(fs::read(&todo_file).unwrap(), before);

    // 6. Successful add (with title matching a DONE task, allowed; partial section "Release"; --field).
    let ok1 = common::run_todo(
        &home,
        &work,
        &[
            "add",
            "--title",
            "Already finished",
            "--priority",
            "p0",
            "--type",
            "bug",
            "--area",
            "cli",
            "--section",
            "Release",
            "--depends",
            "T-001",
            "--files",
            "src/cli/add.rs",
            "--problem",
            "broken",
            "--fix",
            "fix it",
            "--accept",
            "passes",
            "--field",
            "reqs=R-WRITE-1",
            "--agent",
            "alice",
        ],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert!(
        ok1.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&ok1.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&ok1.stdout).trim(), "T-006: add ok");

    let after1 = fs::read_to_string(&todo_file).unwrap();
    assert!(
        after1.contains(
            "### T-006 · Already finished\n\
- status: open\n\
- owner: —\n\
- priority: P0 · type: bug · area: cli\n\
- depends: T-001\n\
- files: src/cli/add.rs\n\
- problem: broken\n\
- fix: fix it\n\
- accept: passes\n\
- reqs: R-WRITE-1\n\
- note (2026-11-15, alice): added\n"
        ),
        "unexpected file:\n{after1}"
    );

    // 7. Delete T-006 by hand from TODO.md; next add must get T-007 (not T-006!) per R-ID-2.
    let without_t6 = after1
        .split("### T-006 · Already finished")
        .next()
        .unwrap()
        .to_string()
        + "\n## 3. Next Ideas\n\n";
    fs::write(&todo_file, without_t6).unwrap();

    let ok2 = common::run_todo(
        &home,
        &work,
        &[
            "--json",
            "add",
            "--title",
            "After manual delete",
            "--priority",
            "P2",
            "--section",
            "Now",
            "--agent",
            "bob",
        ],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert!(ok2.status.success());
    let j_ok2: serde_json::Value = serde_json::from_slice(&ok2.stdout).unwrap();
    assert_eq!(j_ok2["schema"], "todo.add.v1");
    assert_eq!(j_ok2["data"]["id"], "T-007");
}

#[test]
fn twenty_parallel_adds_lose_nothing_and_get_distinct_ids() {
    let tmp = TempDir::new().unwrap();
    let initial = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
## Now\n\n\
### T-001 · Seed task\n\
- status: open\n\
- owner: —\n\
- priority: P1\n";
    let (home, work, todo_file) = setup_proj(&tmp, initial).unwrap();

    let n = 20usize;
    let barrier = Arc::new(Barrier::new(n));
    let mut handles = Vec::with_capacity(n);

    for i in 0..n {
        let home = home.clone();
        let work = work.clone();
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            barrier.wait();
            let title = format!("Parallel task {i:02}");
            let agent = format!("agent-{i:02}");
            let out = common::run_todo(
                &home,
                &work,
                &[
                    "--json",
                    "add",
                    "--title",
                    &title,
                    "--priority",
                    "P1",
                    "--agent",
                    &agent,
                ],
                &[],
            )
            .unwrap();
            assert!(
                out.status.success(),
                "worker {i} failed: status={:?}, stdout={}, stderr={}",
                out.status.code(),
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            let j: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
            j["data"]["id"].as_str().unwrap().to_string()
        }));
    }

    let mut returned_ids = BTreeSet::new();
    for h in handles {
        let id = h.join().unwrap();
        assert!(
            returned_ids.insert(id.clone()),
            "duplicate ID returned: {id}"
        );
    }
    assert_eq!(returned_ids.len(), n);

    let bytes = fs::read(&todo_file).unwrap();
    let doc = Document::parse_bytes(&bytes).unwrap();
    assert_eq!(doc.tasks.len(), n + 1);

    let doc_ids: BTreeSet<u64> = doc.tasks.iter().map(|t| t.id.number()).collect();
    assert_eq!(doc_ids.len(), n + 1);
    for expected in 1..=(n as u64 + 1) {
        assert!(doc_ids.contains(&expected), "missing ID {expected}");
    }
}
