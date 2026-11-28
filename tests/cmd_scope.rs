//! CLI tests for file scopes (R-SCOPE-1, R-READ-3, R-CLAIM-2, TEST-5): `next`, `next --claim`,
//! `claim`, `show --json`, `list --ready` and `check` with the `files` field. Every fixture is
//! invented and written into a temporary folder.

// The helpers panic when a command or a file is wrong, which is how a test fails, as in the other
// test files (AGENTS.md "Test rules").
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

mod common;

use std::collections::HashSet;
use std::fs;
use std::process::Output;

use chrono::Local;
use tempfile::TempDir;

use common::{parse_json, run_parallel_todos, run_todo, setup_proj, stderr_text, stdout_text};
use todo::doc::{Document, Status};
use todo::ops::scope::{parse_files, scopes_overlap};

const MARKER: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->";

/// A claim time of now, written as `todo claim` writes it (R-IO-6): the claim is fresh.
fn fresh_claim() -> String {
    Local::now().format("%Y-%m-%dT%H:%M%:z").to_string()
}

/// One task block. `owner` is `—` for an open task. `claimed` and `files` are written when given.
fn block(
    id: u32,
    title: &str,
    status: &str,
    owner: &str,
    priority: &str,
    claimed: Option<&str>,
    files: Option<&str>,
) -> String {
    // Every task has a level (R-LEVEL-1): `next` hands out only leveled tasks, and these tests are
    // about files, not levels.
    let mut text = format!(
        "### T-{id:03} · {title}\n- status: {status}\n- owner: {owner}\n- priority: {priority} · difficulty: 2\n"
    );
    if let Some(claimed) = claimed {
        text.push_str(&format!("- claimed: {claimed}\n"));
    }
    if let Some(files) = files {
        text.push_str(&format!("- files: {files}\n"));
    }
    text.push('\n');
    text
}

/// A TODO.md with the marker, one section and the given task blocks.
fn body(blocks: &[String]) -> String {
    format!("{MARKER}\n## Queue\n\n{}", blocks.concat())
}

/// Runs `todo` with `args` in the project of `home` and `work`.
fn run(home: &std::path::Path, work: &std::path::Path, args: &[&str]) -> Output {
    run_todo(home, work, args, &[]).expect("run todo")
}

#[test]
fn next_hands_out_no_task_whose_files_another_agent_holds() {
    let tmp = TempDir::new().unwrap();
    let claimed = fresh_claim();
    let (home, work, _todo) = setup_proj(
        &tmp,
        &body(&[
            block(
                1,
                "Shader",
                "claimed",
                "alice",
                "P1",
                Some(&claimed),
                Some("src/shader.c"),
            ),
            block(
                2,
                "Shader tests",
                "open",
                "—",
                "P0",
                None,
                Some("src/shader.c, tests/shader_test.c"),
            ),
            block(3, "Docs", "open", "—", "P2", None, Some("docs/")),
        ]),
    );

    let out = run(&home, &work, &["--json", "--agent", "bob", "next"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let json = parse_json(&out.stdout);
    assert_eq!(json["data"]["task"]["id"], "T-003", "{json}");
    assert_eq!(json["data"]["claimed"], false);
}

#[test]
fn next_claim_passes_over_files_held_by_others_in_turn() {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo_file) = setup_proj(
        &tmp,
        &body(&[
            block(1, "Shader", "open", "—", "P0", None, Some("src/a.c")),
            block(
                2,
                "Shader part two",
                "open",
                "—",
                "P1",
                None,
                Some("src/a.c"),
            ),
            block(3, "Loader", "open", "—", "P2", None, Some("src/b.c")),
        ]),
    );

    let first = run(
        &home,
        &work,
        &["--json", "--agent", "alice", "next", "--claim"],
    );
    assert_eq!(parse_json(&first.stdout)["data"]["task"]["id"], "T-001");

    // T-002 shares src/a.c with alice's T-001, so bob gets T-003.
    let second = run(
        &home,
        &work,
        &["--json", "--agent", "bob", "next", "--claim"],
    );
    assert!(second.status.success(), "stderr: {}", stderr_text(&second));
    assert_eq!(parse_json(&second.stdout)["data"]["task"]["id"], "T-003");

    // Nothing else is free: T-002 still waits for src/a.c.
    let third = run(
        &home,
        &work,
        &["--json", "--agent", "carol", "next", "--claim"],
    );
    assert!(third.status.success(), "stderr: {}", stderr_text(&third));
    assert!(parse_json(&third.stdout)["data"]["task"].is_null());

    let doc = Document::parse_bytes(&fs::read(&todo_file).unwrap()).unwrap();
    let held: Vec<&str> = doc
        .tasks
        .iter()
        .filter(|t| t.status == Status::Claimed)
        .map(|t| t.title.as_str())
        .collect();
    assert_eq!(held, ["Shader", "Loader"]);
}

#[test]
fn own_claims_do_not_hold_back_the_same_agent() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo) = setup_proj(
        &tmp,
        &body(&[
            block(1, "Shader", "open", "—", "P0", None, Some("src/a.c")),
            block(
                2,
                "Shader part two",
                "open",
                "—",
                "P1",
                None,
                Some("src/a.c"),
            ),
        ]),
    );

    let first = run(&home, &work, &["--agent", "alice", "claim", "T-001"]);
    assert!(first.status.success(), "stderr: {}", stderr_text(&first));
    // alice's own claim on T-001 does not refuse her second claim on the same file.
    let second = run(&home, &work, &["--agent", "alice", "claim", "T-002"]);
    assert!(second.status.success(), "stderr: {}", stderr_text(&second));
    // Refreshing her claim on T-001 takes no new files, so it is not refused either.
    let refresh = run(&home, &work, &["--agent", "alice", "claim", "T-001"]);
    assert!(
        refresh.status.success(),
        "stderr: {}",
        stderr_text(&refresh)
    );
}

#[test]
fn tasks_without_files_are_handed_out_as_before() {
    let tmp = TempDir::new().unwrap();
    let claimed = fresh_claim();
    let (home, work, _todo) = setup_proj(
        &tmp,
        &body(&[
            block(1, "Held", "claimed", "alice", "P0", Some(&claimed), None),
            block(2, "Free one", "open", "—", "P1", None, None),
            block(3, "Free two", "open", "—", "P1", None, None),
        ]),
    );

    let out = run(
        &home,
        &work,
        &["--json", "--agent", "bob", "next", "--claim"],
    );
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    assert_eq!(parse_json(&out.stdout)["data"]["task"]["id"], "T-002");
}

#[test]
fn a_stale_claim_holds_no_files() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo) = setup_proj(
        &tmp,
        &body(&[
            // alice's claim is years old, so it is stale (R-CLAIM-4) and holds nothing.
            block(
                1,
                "Old claim",
                "claimed",
                "alice",
                "P2",
                Some("2020-01-01T00:00+00:00"),
                Some("src/a.c"),
            ),
            block(2, "Shader", "open", "—", "P0", None, Some("src/a.c")),
        ]),
    );

    let out = run(
        &home,
        &work,
        &["--json", "--agent", "bob", "next", "--claim"],
    );
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    assert_eq!(parse_json(&out.stdout)["data"]["task"]["id"], "T-002");
}

#[test]
fn a_blocked_task_with_a_fresh_claim_holds_its_files() {
    let tmp = TempDir::new().unwrap();
    let claimed = fresh_claim();
    let (home, work, _todo) = setup_proj(
        &tmp,
        &body(&[
            block(
                1,
                "Blocked shader",
                "blocked",
                "alice",
                "P1",
                Some(&claimed),
                Some("src/a.c"),
            ),
            block(2, "Shader", "open", "—", "P0", None, Some("src/a.c")),
        ]),
    );

    // alice still owns the blocked T-001 and may have uncommitted edits, so T-002 is held back.
    let out = run(&home, &work, &["--json", "--agent", "bob", "next"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    assert!(parse_json(&out.stdout)["data"]["task"].is_null());
}

#[test]
fn claim_is_refused_with_files_busy_naming_the_task_in_the_way() {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo_file) = setup_proj(
        &tmp,
        &body(&[
            block(1, "Shader", "open", "—", "P1", None, Some("src/a.c")),
            block(
                2,
                "Shader part two",
                "open",
                "—",
                "P1",
                None,
                Some("src/a.c"),
            ),
        ]),
    );
    let first = run(&home, &work, &["--agent", "alice", "claim", "T-001"]);
    assert!(first.status.success(), "stderr: {}", stderr_text(&first));
    let before = fs::read(&todo_file).unwrap();

    let out = run(
        &home,
        &work,
        &["--json", "--agent", "bob", "claim", "T-002"],
    );
    assert_eq!(out.status.code(), Some(1), "stdout: {}", stdout_text(&out));
    let json = parse_json(&out.stdout);
    assert_eq!(json["error"]["code"], "FILES_BUSY", "{json}");
    assert_eq!(json["error"]["task"], "T-002");
    assert_eq!(json["error"]["files_busy"][0], "T-001");
    let message = json["error"]["message"].as_str().unwrap();
    assert!(message.contains("T-001"), "{message}");
    assert!(message.contains("--force"), "{message}");
    // A refusal writes nothing.
    assert_eq!(fs::read(&todo_file).unwrap(), before);
}

#[test]
fn claim_with_force_takes_the_files_and_records_the_override() {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo_file) = setup_proj(
        &tmp,
        &body(&[
            block(1, "Shader", "open", "—", "P1", None, Some("src/a.c")),
            block(
                2,
                "Shader part two",
                "open",
                "—",
                "P1",
                None,
                Some("src/a.c"),
            ),
        ]),
    );
    let first = run(&home, &work, &["--agent", "alice", "claim", "T-001"]);
    assert!(first.status.success(), "stderr: {}", stderr_text(&first));

    let out = run(
        &home,
        &work,
        &["claim", "T-002", "--agent", "bob", "--force"],
    );
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let text = fs::read_to_string(&todo_file).unwrap();
    assert!(
        text.contains("forced claim (files held by T-001)"),
        "the override note is missing:\n{text}"
    );
    let doc = Document::parse_bytes(text.as_bytes()).unwrap();
    let t2 = doc
        .tasks
        .iter()
        .find(|t| t.title == "Shader part two")
        .unwrap();
    assert_eq!(t2.status, Status::Claimed);
    assert_eq!(t2.owner.as_deref(), Some("bob"));
}

#[test]
fn forced_takeover_of_a_live_claim_still_notes_the_files_others_hold() {
    let tmp = TempDir::new().unwrap();
    let claimed = fresh_claim();
    let (home, work, todo_file) = setup_proj(
        &tmp,
        &body(&[
            block(
                1,
                "Shader",
                "claimed",
                "alice",
                "P1",
                Some(&claimed),
                Some("src/a.rs"),
            ),
            block(
                2,
                "Shader part two",
                "claimed",
                "bob",
                "P1",
                Some(&claimed),
                Some("src/a.rs"),
            ),
        ]),
    );

    // alice's claim of T-001 is fresh, so carol may take it only with --force. The files are held
    // by bob's T-002, so the takeover still records that override in T-001.
    let out = run(
        &home,
        &work,
        &["claim", "T-001", "--agent", "carol", "--force"],
    );
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let text = fs::read_to_string(&todo_file).unwrap();
    assert!(
        text.contains("forced claim (files held by T-002)"),
        "the override note is missing:\n{text}"
    );
    let doc = Document::parse_bytes(text.as_bytes()).unwrap();
    let t1 = doc.tasks.iter().find(|t| t.title == "Shader").unwrap();
    assert_eq!(t1.status, Status::Claimed);
    assert_eq!(t1.owner.as_deref(), Some("carol"));
}

#[test]
fn a_task_without_files_is_claimed_while_others_hold_files() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo) = setup_proj(
        &tmp,
        &body(&[
            block(1, "Shader", "open", "—", "P1", None, Some("src/a.c")),
            block(2, "Notes", "open", "—", "P1", None, None),
        ]),
    );
    let first = run(&home, &work, &["--agent", "alice", "claim", "T-001"]);
    assert!(first.status.success(), "stderr: {}", stderr_text(&first));

    let out = run(&home, &work, &["--agent", "bob", "claim", "T-002"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
}

#[test]
fn a_blocked_task_with_a_stale_claim_holds_no_files() {
    let tmp = TempDir::new().unwrap();
    // alice's claim is 25 hours old, past the 24 hours of claim_ttl_hours, so it is stale and the
    // blocked task holds nothing (R-CLAIM-4).
    let stale = (Local::now() - chrono::Duration::hours(25))
        .format("%Y-%m-%dT%H:%M%:z")
        .to_string();
    let (home, work, todo_file) = setup_proj(
        &tmp,
        &body(&[
            block(
                1,
                "Blocked shader",
                "blocked",
                "alice",
                "P1",
                Some(&stale),
                Some("src/a.rs"),
            ),
            block(2, "Shader", "open", "—", "P0", None, Some("src/a.rs")),
        ]),
    );

    let out = run(&home, &work, &["--agent", "bob", "claim", "T-002"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let doc = Document::parse_bytes(&fs::read(&todo_file).unwrap()).unwrap();
    let t2 = doc.tasks.iter().find(|t| t.title == "Shader").unwrap();
    assert_eq!(t2.status, Status::Claimed);
    assert_eq!(t2.owner.as_deref(), Some("bob"));
}

#[test]
fn a_blocked_task_without_an_owner_holds_no_files() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo) = setup_proj(
        &tmp,
        &body(&[
            block(
                1,
                "Blocked shader",
                "blocked",
                "—",
                "P1",
                None,
                Some("src/a.rs"),
            ),
            block(2, "Shader", "open", "—", "P0", None, Some("src/a.rs")),
        ]),
    );

    let out = run(&home, &work, &["--agent", "bob", "claim", "T-002"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
}

#[test]
fn a_blocked_task_without_an_owner_holds_no_files_even_with_a_fresh_claim_time() {
    let tmp = TempDir::new().unwrap();
    let claimed = fresh_claim();
    // The claim time is fresh, so only the missing owner keeps the task from holding its files.
    let (home, work, _todo) = setup_proj(
        &tmp,
        &body(&[
            block(
                1,
                "Blocked shader",
                "blocked",
                "—",
                "P1",
                Some(&claimed),
                Some("src/a.rs"),
            ),
            block(2, "Shader", "open", "—", "P0", None, Some("src/a.rs")),
        ]),
    );

    let out = run(&home, &work, &["--agent", "bob", "claim", "T-002"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
}

#[test]
fn files_busy_names_a_blocked_holder_with_its_owner() {
    let tmp = TempDir::new().unwrap();
    let claimed = fresh_claim();
    let (home, work, todo_file) = setup_proj(
        &tmp,
        &body(&[
            block(
                1,
                "Blocked shader",
                "blocked",
                "alice",
                "P1",
                Some(&claimed),
                Some("src/a.rs"),
            ),
            block(2, "Shader", "open", "—", "P0", None, Some("src/a.rs")),
        ]),
    );
    let before = fs::read(&todo_file).unwrap();

    let out = run(
        &home,
        &work,
        &["--json", "--agent", "bob", "claim", "T-002"],
    );
    assert_eq!(out.status.code(), Some(1), "stdout: {}", stdout_text(&out));
    let json = parse_json(&out.stdout);
    assert_eq!(json["error"]["code"], "FILES_BUSY", "{json}");
    assert_eq!(json["error"]["files_busy"][0], "T-001");
    let message = json["error"]["message"].as_str().unwrap();
    assert!(
        message.contains("T-001 (blocked, owned by alice)"),
        "{message}"
    );
    // A refusal writes nothing.
    assert_eq!(fs::read(&todo_file).unwrap(), before);
}

#[test]
fn files_busy_names_a_claimed_holder_with_its_owner() {
    let tmp = TempDir::new().unwrap();
    let claimed = fresh_claim();
    let (home, work, todo_file) = setup_proj(
        &tmp,
        &body(&[
            block(
                1,
                "Shader",
                "claimed",
                "alice",
                "P1",
                Some(&claimed),
                Some("src/a.rs"),
            ),
            block(
                2,
                "Shader part two",
                "open",
                "—",
                "P0",
                None,
                Some("src/a.rs"),
            ),
        ]),
    );
    let before = fs::read(&todo_file).unwrap();

    let out = run(
        &home,
        &work,
        &["--json", "--agent", "bob", "claim", "T-002"],
    );
    assert_eq!(out.status.code(), Some(1), "stdout: {}", stdout_text(&out));
    let json = parse_json(&out.stdout);
    assert_eq!(json["error"]["code"], "FILES_BUSY", "{json}");
    assert_eq!(json["error"]["files_busy"][0], "T-001");
    let message = json["error"]["message"].as_str().unwrap();
    assert!(message.contains("T-001 (claimed by alice)"), "{message}");
    assert_eq!(fs::read(&todo_file).unwrap(), before);
}

#[test]
fn refreshing_an_own_live_claim_takes_no_new_files_after_another_forced_them() {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo_file) = setup_proj(
        &tmp,
        &body(&[
            block(1, "Shader", "open", "—", "P1", None, Some("src/a.rs")),
            block(
                2,
                "Shader part two",
                "open",
                "—",
                "P1",
                None,
                Some("src/a.rs"),
            ),
        ]),
    );
    let first = run(&home, &work, &["--agent", "alice", "claim", "T-001"]);
    assert!(first.status.success(), "stderr: {}", stderr_text(&first));
    let forced = run(
        &home,
        &work,
        &["claim", "T-002", "--agent", "bob", "--force"],
    );
    assert!(forced.status.success(), "stderr: {}", stderr_text(&forced));
    let before = fs::read_to_string(&todo_file).unwrap();
    assert_eq!(before.matches("forced claim").count(), 1, "{before}");

    // bob's forced claim of T-002 now holds src/a.rs as well. alice refreshes her own live claim on
    // T-001, which takes no new files, so it is not refused and adds no note.
    let refresh = run(&home, &work, &["--agent", "alice", "claim", "T-001"]);
    assert!(
        refresh.status.success(),
        "stderr: {}",
        stderr_text(&refresh)
    );
    let after = fs::read_to_string(&todo_file).unwrap();
    assert_eq!(after.matches("forced claim").count(), 1, "{after}");
    let doc = Document::parse_bytes(after.as_bytes()).unwrap();
    let t1 = doc.tasks.iter().find(|t| t.title == "Shader").unwrap();
    assert_eq!(t1.status, Status::Claimed);
    assert_eq!(t1.owner.as_deref(), Some("alice"));
}

#[test]
fn show_json_lists_the_other_claims_that_hold_the_files() {
    let tmp = TempDir::new().unwrap();
    let claimed = fresh_claim();
    let (home, work, _todo) = setup_proj(
        &tmp,
        &body(&[
            block(
                1,
                "Alice shader",
                "claimed",
                "alice",
                "P1",
                Some(&claimed),
                Some("src/a.c"),
            ),
            block(2, "Open shader", "open", "—", "P1", None, Some("src/a.c")),
            block(
                3,
                "Carol shader",
                "claimed",
                "carol",
                "P1",
                Some(&claimed),
                Some("src/a.c"),
            ),
        ]),
    );

    let open = run(&home, &work, &["--json", "show", "T-002"]);
    assert!(open.status.success(), "stderr: {}", stderr_text(&open));
    let json = parse_json(&open.stdout);
    assert_eq!(json["data"]["ready"], false, "{json}");
    let busy = json["data"]["files_busy"].as_array().unwrap();
    let ids: Vec<&str> = busy.iter().map(|b| b["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["T-001", "T-003"]);
    assert_eq!(busy[1]["owner"], "carol");
    assert_eq!(busy[1]["status"], "claimed");

    // For alice's own T-001, only carol's claim is in the way: her own claim is not listed.
    let own = run(&home, &work, &["--json", "show", "T-001"]);
    let json = parse_json(&own.stdout);
    let ids: Vec<&str> = json["data"]["files_busy"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["T-003"]);
}

#[test]
fn list_ready_marks_the_tasks_held_back_by_files() {
    let tmp = TempDir::new().unwrap();
    let claimed = fresh_claim();
    let (home, work, _todo) = setup_proj(
        &tmp,
        &body(&[
            block(
                1,
                "Alice shader",
                "claimed",
                "alice",
                "P1",
                Some(&claimed),
                Some("src/a.c"),
            ),
            block(2, "Open shader", "open", "—", "P1", None, Some("src/a.c")),
            block(3, "Docs", "open", "—", "P2", None, Some("docs/")),
        ]),
    );

    let text = run(&home, &work, &["list", "--ready"]);
    assert!(text.status.success(), "stderr: {}", stderr_text(&text));
    let out = stdout_text(&text);
    let shader_line = out.lines().find(|l| l.contains("T-002")).unwrap();
    assert!(
        shader_line.contains("[held back: files shared with T-001]"),
        "{out}"
    );
    let docs_line = out.lines().find(|l| l.contains("T-003")).unwrap();
    assert!(!docs_line.contains("held back"), "{out}");

    let json = run(&home, &work, &["--json", "list", "--ready"]);
    let json = parse_json(&json.stdout);
    let tasks = json["data"]["tasks"].as_array().unwrap();
    let held = tasks.iter().find(|t| t["id"] == "T-002").unwrap();
    assert_eq!(held["ready"], false);
    assert_eq!(held["files_busy"][0]["id"], "T-001");
    let docs = tasks.iter().find(|t| t["id"] == "T-003").unwrap();
    assert_eq!(docs["ready"], true);
    assert_eq!(docs["files_busy"], serde_json::json!([]));
}

#[test]
fn check_warns_about_files_entries_that_leave_the_folder() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo) = setup_proj(
        &tmp,
        &body(&[block(
            1,
            "Odd paths",
            "open",
            "—",
            "P1",
            None,
            Some("/etc/a.c, ../other/b.c, src/ok.c"),
        )]),
    );

    // A warning does not change the exit code (R-CHECK-1).
    let out = run(&home, &work, &["check"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let err = stderr_text(&out);
    assert!(
        err.contains("files entry \"/etc/a.c\" is absolute"),
        "{err}"
    );
    assert!(
        err.contains("files entry \"../other/b.c\" leaves the folder of TODO.md"),
        "{err}"
    );
    assert!(!err.contains("src/ok.c"), "{err}");
}

#[test]
fn ten_parallel_next_claim_with_pairwise_shared_files_hands_out_one_task() {
    let tmp = TempDir::new().unwrap();
    let blocks: Vec<String> = (1..=10)
        .map(|i| {
            block(
                i,
                &format!("Shared item {i}"),
                "open",
                "—",
                "P1",
                None,
                Some("shared/x.c"),
            )
        })
        .collect();
    let (home, work, todo_file) = setup_proj(&tmp, &body(&blocks));

    let invocations: Vec<Vec<String>> = (0..10)
        .map(|i| {
            vec![
                "--json".to_string(),
                "next".to_string(),
                "--claim".to_string(),
                "--agent".to_string(),
                format!("worker-{i:02}"),
            ]
        })
        .collect();
    let outputs = run_parallel_todos(&home, &work, &invocations).unwrap();

    let mut handed_out = 0;
    for (i, out) in outputs.iter().enumerate() {
        assert!(
            out.status.success(),
            "worker-{i:02} failed ({:?}): stdout={} stderr={}",
            out.status.code(),
            stdout_text(out),
            stderr_text(out)
        );
        if !parse_json(&out.stdout)["data"]["task"].is_null() {
            handed_out += 1;
        }
    }
    assert_eq!(
        handed_out, 1,
        "every other task shares src/x.c with the claimed one"
    );

    let doc = Document::parse_bytes(&fs::read(&todo_file).unwrap()).unwrap();
    let claimed = doc
        .tasks
        .iter()
        .filter(|t| t.status == Status::Claimed)
        .count();
    assert_eq!(claimed, 1);
}

#[test]
fn ten_parallel_next_claim_hands_out_tasks_that_never_overlap() {
    let tmp = TempDir::new().unwrap();
    // Four groups of tasks, one file scope per group. A folder pattern is one of the groups.
    let groups = ["g1/a.c", "g2/b.c", "g3/c.c", "g4/*.c"];
    let mut blocks = Vec::new();
    let mut id = 0;
    for files in groups {
        for _ in 0..4 {
            id += 1;
            blocks.push(block(
                id,
                &format!("Item {id}"),
                "open",
                "—",
                "P1",
                None,
                Some(files),
            ));
        }
    }
    let (home, work, todo_file) = setup_proj(&tmp, &body(&blocks));

    let invocations: Vec<Vec<String>> = (0..10)
        .map(|i| {
            vec![
                "--json".to_string(),
                "next".to_string(),
                "--claim".to_string(),
                "--agent".to_string(),
                format!("worker-{i:02}"),
            ]
        })
        .collect();
    let outputs = run_parallel_todos(&home, &work, &invocations).unwrap();
    for (i, out) in outputs.iter().enumerate() {
        assert!(
            out.status.success(),
            "worker-{i:02} failed ({:?}): stderr={}",
            out.status.code(),
            stderr_text(out)
        );
    }

    let doc = Document::parse_bytes(&fs::read(&todo_file).unwrap()).unwrap();
    let claimed: Vec<&str> = doc
        .tasks
        .iter()
        .filter(|t| t.status == Status::Claimed)
        .map(|t| t.files.as_deref().unwrap())
        .collect();
    // Each group yields exactly one claim: the four groups never share a file.
    let distinct: HashSet<&str> = claimed.iter().copied().collect();
    assert_eq!(
        claimed.len(),
        groups.len(),
        "one task per group: {claimed:?}"
    );
    assert_eq!(distinct.len(), groups.len(), "{claimed:?}");
    for (i, a) in claimed.iter().enumerate() {
        for b in &claimed[i + 1..] {
            assert!(
                !scopes_overlap(&parse_files(a), &parse_files(b)),
                "{a} and {b} were both handed out"
            );
        }
    }
}

/// The IDs in `data.next_ready` of `todo --json summary`.
fn summary_next_ready_ids(home: &std::path::Path, work: &std::path::Path) -> Vec<String> {
    let out = run(home, work, &["--json", "summary"]);
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    parse_json(&out.stdout)["data"]["next_ready"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn summary_next_ready_leaves_out_the_tasks_whose_files_another_agent_holds() {
    let tmp = TempDir::new().unwrap();
    let claimed = fresh_claim();
    let (home, work, _todo) = setup_proj(
        &tmp,
        &body(&[
            block(
                1,
                "Shader",
                "claimed",
                "alice",
                "P1",
                Some(&claimed),
                Some("src/a.c"),
            ),
            block(
                2,
                "Shader part two",
                "open",
                "—",
                "P0",
                None,
                Some("src/a.c"),
            ),
            block(3, "Docs", "open", "—", "P2", None, Some("docs/")),
        ]),
    );

    // `next` skips T-002 because alice's claim holds src/a.c, so the summary must not list it.
    let next = run(&home, &work, &["--json", "--agent", "bob", "next"]);
    assert_eq!(parse_json(&next.stdout)["data"]["task"]["id"], "T-003");
    assert_eq!(summary_next_ready_ids(&home, &work), ["T-003"]);

    let text = run(&home, &work, &["summary"]);
    assert!(text.status.success(), "stderr: {}", stderr_text(&text));
    let out = stdout_text(&text);
    assert!(out.contains("T-003"), "{out}");
    assert!(!out.contains("T-002"), "{out}");
}

#[test]
fn summary_next_ready_keeps_a_task_whose_only_holder_is_stale() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo) = setup_proj(
        &tmp,
        &body(&[
            // alice's claim is years old: it is stale and holds nothing (R-CLAIM-4).
            block(
                1,
                "Old claim",
                "claimed",
                "alice",
                "P2",
                Some("2020-01-01T00:00+00:00"),
                Some("src/a.c"),
            ),
            block(2, "Shader", "open", "—", "P0", None, Some("src/a.c")),
        ]),
    );

    assert_eq!(summary_next_ready_ids(&home, &work), ["T-002", "T-001"]);
}

/// `T-001` (open) and `T-002` (open, depends on `T-001`, names `dependent_files`), plus `extra`.
/// Both tasks have a level (R-LEVEL-1): `next` and the newly ready list hand out only leveled tasks,
/// and this fixture is about files, not levels.
fn finishing_body(dependent_files: &str, extra: &[String]) -> String {
    let mut blocks = format!(
        "### T-001 · First\n- status: open\n- owner: —\n- priority: P1 · difficulty: 2\n\n\
         ### T-002 · Second\n- status: open\n- owner: —\n- priority: P1 · difficulty: 2\n- depends: T-001\n- files: {dependent_files}\n\n"
    );
    blocks.push_str(&extra.concat());
    format!("{MARKER}\n<!-- todo-format: 3 -->\n## Queue\n\n{blocks}")
}

/// The `T-…` IDs in `data.newly_ready` of a `--json` write, empty when the key is left out.
fn newly_ready_ids(out: &Output) -> Vec<String> {
    parse_json(&out.stdout)["data"]["newly_ready"]
        .as_array()
        .map(|list| {
            list.iter()
                .map(|r| r["id"].as_str().unwrap().to_string())
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn done_leaves_out_a_newly_ready_task_whose_files_another_agent_holds() {
    let tmp = TempDir::new().unwrap();
    let holder = block(
        3,
        "Holder",
        "claimed",
        "carol",
        "P2",
        Some(&fresh_claim()),
        Some("src/a.c"),
    );
    let (home, work, _todo) = setup_proj(&tmp, &finishing_body("src/a.c", &[holder]));

    let done = run(
        &home,
        &work,
        &["--json", "--agent", "alice", "done", "T-001"],
    );
    assert!(done.status.success(), "stderr: {}", stderr_text(&done));
    // T-002 has no dependency left, but carol's live claim holds src/a.c, so it is not newly ready.
    assert!(newly_ready_ids(&done).is_empty(), "{}", stdout_text(&done));
}

#[test]
fn done_text_names_no_now_ready_task_whose_files_another_agent_holds() {
    let tmp = TempDir::new().unwrap();
    let holder = block(
        3,
        "Holder",
        "claimed",
        "carol",
        "P2",
        Some(&fresh_claim()),
        Some("src/a.c"),
    );
    let (home, work, _todo) = setup_proj(&tmp, &finishing_body("src/a.c", &[holder]));

    let text = run(&home, &work, &["--agent", "alice", "done", "T-001"]);
    assert!(text.status.success(), "stderr: {}", stderr_text(&text));
    assert!(
        !stdout_text(&text).contains("now ready"),
        "{}",
        stdout_text(&text)
    );
}

#[test]
fn done_lists_a_newly_ready_task_when_nothing_holds_its_files() {
    let tmp = TempDir::new().unwrap();
    // carol's claim names another file, so it does not hold the dependent's src/a.c.
    let other = block(
        3,
        "Elsewhere",
        "claimed",
        "carol",
        "P2",
        Some(&fresh_claim()),
        Some("docs/x.md"),
    );
    let (home, work, _todo) = setup_proj(&tmp, &finishing_body("src/a.c", &[other]));

    let done = run(
        &home,
        &work,
        &["--json", "--agent", "alice", "done", "T-001"],
    );
    assert!(done.status.success(), "stderr: {}", stderr_text(&done));
    assert_eq!(newly_ready_ids(&done), ["T-002"]);
}

#[test]
fn done_text_names_a_now_ready_task_when_nothing_holds_its_files() {
    let tmp = TempDir::new().unwrap();
    let other = block(
        3,
        "Elsewhere",
        "claimed",
        "carol",
        "P2",
        Some(&fresh_claim()),
        Some("docs/x.md"),
    );
    let (home, work, _todo) = setup_proj(&tmp, &finishing_body("src/a.c", &[other]));

    let text = run(&home, &work, &["--agent", "alice", "done", "T-001"]);
    assert!(text.status.success(), "stderr: {}", stderr_text(&text));
    assert!(
        stdout_text(&text).contains("now ready: T-002"),
        "{}",
        stdout_text(&text)
    );
}

#[test]
fn drop_applies_the_same_file_scope_to_newly_ready_tasks() {
    let tmp = TempDir::new().unwrap();
    let holder = block(
        3,
        "Holder",
        "claimed",
        "carol",
        "P2",
        Some(&fresh_claim()),
        Some("src/a.c"),
    );
    let (home, work, _todo) = setup_proj(&tmp, &finishing_body("src/a.c", &[holder]));

    let dropped = run(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "alice",
            "drop",
            "T-001",
            "--note",
            "not needed",
        ],
    );
    assert!(
        dropped.status.success(),
        "stderr: {}",
        stderr_text(&dropped)
    );
    assert!(
        newly_ready_ids(&dropped).is_empty(),
        "{}",
        stdout_text(&dropped)
    );
}
