//! Integration tests for session-tagged claims, `release --session`, the `SessionEnd` hook
//! command and its install/uninstall (T-072; R-CLAIM-7).

mod common;

use std::io::Write;
use std::path::Path;
use std::process::{Output, Stdio};

use common::{clean_todo_command, read_text, run_todo, setup_proj, stderr_text};
use tempfile::TempDir;

const BODY: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
## Now\n\n\
### T-001 · First\n\
- status: open\n\
- owner: —\n\
- priority: P1 · difficulty: 2\n\
\n\
### T-002 · Second\n\
- status: open\n\
- owner: —\n\
- priority: P2 · difficulty: 2\n\
\n\
### T-003 · Third\n\
- status: open\n\
- owner: —\n\
- priority: P2 · difficulty: 2\n";

fn todo(home: &Path, work: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
    run_todo(home, work, args, env).unwrap()
}

#[test]
fn claim_writes_session_from_flag_then_todo_session_then_claude_variable() {
    let tmp = TempDir::new().unwrap();
    let (home, work, file) = setup_proj(&tmp, BODY);

    // --session wins over both variables.
    let out = todo(
        &home,
        &work,
        &["claim", "T-001", "--agent", "alice", "--session", "flag-1"],
        &[
            ("TODO_SESSION", "env-1"),
            ("CLAUDE_CODE_SESSION_ID", "claude-1"),
        ],
    );
    assert!(out.status.success(), "{}", stderr_text(&out));
    // TODO_SESSION wins over CLAUDE_CODE_SESSION_ID.
    let out = todo(
        &home,
        &work,
        &["claim", "T-002", "--agent", "alice"],
        &[
            ("TODO_SESSION", "env-1"),
            ("CLAUDE_CODE_SESSION_ID", "claude-1"),
        ],
    );
    assert!(out.status.success(), "{}", stderr_text(&out));
    // CLAUDE_CODE_SESSION_ID alone.
    let out = todo(
        &home,
        &work,
        &["claim", "T-003", "--agent", "alice"],
        &[("CLAUDE_CODE_SESSION_ID", "claude-1")],
    );
    assert!(out.status.success(), "{}", stderr_text(&out));

    let text = read_text(&file);
    // The session line sits right after `claimed`, before `priority`.
    for (id, session, prio) in [
        ("T-001", "flag-1", "P1"),
        ("T-002", "env-1", "P2"),
        ("T-003", "claude-1", "P2"),
    ] {
        let block = text
            .split("### ")
            .find(|b| b.starts_with(id))
            .unwrap_or_else(|| panic!("no block {id}"));
        let lines: Vec<&str> = block.lines().collect();
        assert_eq!(lines[1], "- status: claimed", "{block}");
        assert_eq!(lines[2], "- owner: alice", "{block}");
        assert!(lines[3].starts_with("- claimed: "), "{block}");
        assert_eq!(lines[4], format!("- session: {session}"), "{block}");
        assert_eq!(
            lines[5],
            format!("- priority: {prio} · difficulty: 2"),
            "{block}"
        );
    }

    // The JSON of a task shows the session under `fields`.
    let shown = todo(&home, &work, &["--json", "show", "T-001"], &[]);
    let json = common::parse_json(&shown.stdout);
    assert_eq!(json["data"]["fields"]["session"], "flag-1");
}

#[test]
fn claim_without_any_session_writes_no_session_field_and_reclaim_drops_an_old_one() {
    let tmp = TempDir::new().unwrap();
    let (home, work, file) = setup_proj(&tmp, BODY);

    let out = todo(&home, &work, &["claim", "T-001", "--agent", "alice"], &[]);
    assert!(out.status.success(), "{}", stderr_text(&out));
    assert!(!read_text(&file).contains("session"));

    // Tag it, then refresh the claim without a session: the stale tag goes away.
    let out = todo(
        &home,
        &work,
        &["claim", "T-001", "--agent", "alice", "--session", "s-1"],
        &[],
    );
    assert!(out.status.success());
    assert!(read_text(&file).contains("- session: s-1\n"));
    let out = todo(&home, &work, &["claim", "T-001", "--agent", "alice"], &[]);
    assert!(out.status.success());
    assert!(!read_text(&file).contains("session"));
}

#[test]
fn invalid_session_flag_is_a_usage_error_and_invalid_environment_value_is_ignored() {
    let tmp = TempDir::new().unwrap();
    let (home, work, file) = setup_proj(&tmp, BODY);

    let out = todo(
        &home,
        &work,
        &[
            "claim",
            "T-001",
            "--agent",
            "alice",
            "--session",
            "has space",
        ],
        &[],
    );
    assert_eq!(out.status.code(), Some(2), "{}", stderr_text(&out));
    assert_eq!(read_text(&file), BODY);

    let out = todo(
        &home,
        &work,
        &["claim", "T-001", "--agent", "alice"],
        &[("CLAUDE_CODE_SESSION_ID", "bad value!")],
    );
    assert!(out.status.success(), "{}", stderr_text(&out));
    assert!(!read_text(&file).contains("session"));
}

#[test]
fn next_claim_and_unblock_write_the_session_and_done_or_release_remove_it() {
    let tmp = TempDir::new().unwrap();
    let (home, work, file) = setup_proj(&tmp, BODY);
    let env = [("CLAUDE_CODE_SESSION_ID", "sess-9")];

    let out = todo(&home, &work, &["next", "--claim", "--agent", "bob"], &env);
    assert!(out.status.success(), "{}", stderr_text(&out));
    // T-001 has the highest priority (P1) and is claimed with the session.
    assert!(read_text(&file).contains("- claimed: "));
    assert_eq!(read_text(&file).matches("- session: sess-9\n").count(), 1);

    // block keeps the claim lines; unblock refreshes the claim and writes the session again.
    let out = todo(
        &home,
        &work,
        &["block", "T-001", "--note", "waiting", "--agent", "bob"],
        &env,
    );
    assert!(out.status.success(), "{}", stderr_text(&out));
    let out = todo(
        &home,
        &work,
        &["unblock", "T-001", "--agent", "bob", "--session", "sess-10"],
        &[],
    );
    assert!(out.status.success(), "{}", stderr_text(&out));
    let text = read_text(&file);
    assert!(text.contains("- session: sess-10\n"), "{text}");
    assert!(!text.contains("sess-9"), "{text}");

    // done removes the claim and with it the session.
    let out = todo(&home, &work, &["done", "T-001", "--agent", "bob"], &env);
    assert!(out.status.success(), "{}", stderr_text(&out));
    assert!(!read_text(&file).contains("session:"));

    // release removes it too.
    let out = todo(&home, &work, &["claim", "T-002", "--agent", "bob"], &env);
    assert!(out.status.success());
    assert!(read_text(&file).contains("- session: sess-9\n"));
    let out = todo(&home, &work, &["release", "T-002", "--agent", "bob"], &env);
    assert!(out.status.success());
    assert!(!read_text(&file).contains("session:"));
}

#[test]
fn release_session_releases_only_the_claims_of_that_session() {
    let tmp = TempDir::new().unwrap();
    let (home, work, file) = setup_proj(&tmp, BODY);

    for (task, agent, session) in [
        ("T-001", "alice", "s-a"),
        ("T-002", "bob", "s-a"),
        ("T-003", "alice", "s-b"),
    ] {
        let out = todo(
            &home,
            &work,
            &["claim", task, "--agent", agent, "--session", session],
            &[],
        );
        assert!(out.status.success(), "{}", stderr_text(&out));
    }
    // A blocked task of the session stays blocked but loses owner, claim and session.
    let out = todo(
        &home,
        &work,
        &["block", "T-002", "--note", "stuck", "--agent", "bob"],
        &[],
    );
    assert!(out.status.success(), "{}", stderr_text(&out));

    let out = todo(
        &home,
        &work,
        &[
            "--json",
            "release",
            "--session",
            "s-a",
            "--agent",
            "coordinator",
        ],
        &[("CLAUDE_CODE_SESSION_ID", "unrelated")],
    );
    assert!(out.status.success(), "{}", stderr_text(&out));
    let json = common::parse_json(&out.stdout);
    assert_eq!(json["data"]["count"], 2);

    let text = read_text(&file);
    let block = |id: &str| {
        text.split("### ")
            .find(|b| b.starts_with(id))
            .unwrap()
            .to_string()
    };
    let b1 = block("T-001");
    assert!(b1.contains("- status: open\n- owner: —\n"), "{b1}");
    assert!(!b1.contains("claimed:") && !b1.contains("session:"), "{b1}");
    assert!(
        b1.contains("- note (") && b1.contains("coordinator): released — session s-a ended\n"),
        "{b1}"
    );
    let b2 = block("T-002");
    assert!(b2.contains("- status: blocked\n- owner: —\n"), "{b2}");
    assert!(b2.contains("released — session s-a ended"), "{b2}");
    // The other session is untouched.
    let b3 = block("T-003");
    assert!(b3.contains("- status: claimed\n- owner: alice\n"), "{b3}");
    assert!(b3.contains("- session: s-b\n"), "{b3}");

    // Nothing left for s-a: still succeeds and releases nothing.
    let out = todo(
        &home,
        &work,
        &[
            "--json",
            "release",
            "--session",
            "s-a",
            "--agent",
            "coordinator",
        ],
        &[],
    );
    assert!(out.status.success());
    assert_eq!(common::parse_json(&out.stdout)["data"]["count"], 0);

    // --session cannot be combined with a task.
    let out = todo(
        &home,
        &work,
        &["release", "T-003", "--session", "s-b", "--agent", "alice"],
        &[],
    );
    assert_eq!(out.status.code(), Some(2));
}

fn session_end(home: &Path, work: &Path, stdin: &str, env: &[(&str, &str)]) -> Output {
    let mut cmd = clean_todo_command(home, work);
    cmd.arg("session-end")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for &(k, v) in env {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn session_end_hook_command_releases_the_claims_named_in_its_json() {
    let tmp = TempDir::new().unwrap();
    let (home, work, file) = setup_proj(&tmp, BODY);
    for (task, session) in [("T-001", "end-1"), ("T-002", "other")] {
        let out = todo(
            &home,
            &work,
            &["claim", task, "--agent", "alice", "--session", session],
            &[],
        );
        assert!(out.status.success());
    }

    // Sample payload as Claude Code sends it to a SessionEnd hook.
    let payload = r#"{"session_id":"end-1","transcript_path":"C:\\Users\\x\\t.jsonl","cwd":"C:\\proj","hook_event_name":"SessionEnd","reason":"other"}"#;
    let out = session_end(&home, &work, payload, &[]);
    assert!(out.status.success(), "{}", stderr_text(&out));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "T-001: released"
    );

    let text = read_text(&file);
    assert!(
        text.contains("session-end): released — session end-1 ended"),
        "{text}"
    );
    assert!(!text.contains("session: end-1"));
    assert!(text.contains("- session: other\n"));

    // Quiet no-ops: empty input, bad JSON, unknown session.
    let before = read_text(&file);
    for input in ["", "not json", r#"{"session_id":"nobody"}"#, "{}"] {
        let out = session_end(&home, &work, input, &[]);
        assert!(out.status.success(), "{input}: {}", stderr_text(&out));
        assert!(out.stdout.is_empty());
    }
    assert_eq!(read_text(&file), before);

    // Without a TODO.md the hook still exits 0.
    let empty = tmp.path().join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let out = session_end(&home, &empty, payload, &[]);
    assert!(out.status.success(), "{}", stderr_text(&out));
}

#[test]
fn hooks_install_adds_the_session_end_hook_and_uninstall_removes_it() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _file) = setup_proj(&tmp, BODY);
    let settings = work.join(".claude").join("settings.json");

    let out = todo(&home, &work, &["hooks", "install", "--yes"], &[]);
    assert!(out.status.success(), "{}", stderr_text(&out));
    let value: serde_json::Value = serde_json::from_str(&read_text(&settings)).unwrap();
    let entry = &value["hooks"]["SessionEnd"][0];
    assert_eq!(entry["_managed_by"], "todo");
    assert_eq!(entry["hooks"][0]["type"], "command");
    assert_eq!(entry["hooks"][0]["command"], "todo session-end");

    // A second install does not duplicate it.
    let out = todo(&home, &work, &["hooks", "install", "--yes"], &[]);
    assert!(out.status.success());
    let value: serde_json::Value = serde_json::from_str(&read_text(&settings)).unwrap();
    assert_eq!(value["hooks"]["SessionEnd"].as_array().unwrap().len(), 1);

    let out = todo(&home, &work, &["hooks", "uninstall", "--yes"], &[]);
    assert!(out.status.success(), "{}", stderr_text(&out));
    assert!(!read_text(&settings).contains("SessionEnd"));
}

#[test]
fn session_end_is_hidden_from_the_command_list() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _file) = setup_proj(&tmp, BODY);
    let out = todo(&home, &work, &["--help"], &[]);
    assert!(!String::from_utf8_lossy(&out.stdout).contains("session-end"));
}
