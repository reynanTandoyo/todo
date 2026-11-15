use std::fs;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn todo_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_todo"))
}

#[test]
fn summary_json_matches_snapshot() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("TODO.md"),
        "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 1 -->
# Summary Fixture

## Tasks

### T-001 · Ready critical task
- status: open
- owner: —
- priority: P0

### T-002 · Claimed fresh task
- status: claimed
- owner: alice
- priority: P1
- claimed: 2026-11-15T18:00+01:00

### T-003 · Claimed stale task
- status: claimed
- owner: bob
- priority: P1
- claimed: 2026-11-13T10:00+01:00

### T-004 · Blocked task
- status: blocked
- owner: —
- priority: P2
- note (2026-11-15, alice): blocked — waiting on API key

### T-005 · Done task
- status: done
- owner: —
- priority: P2

### T-006 · Dropped task
- status: dropped
- owner: —
- priority: P3

## Questions

### T-007 · Should we ship v0.2 today?
- status: open
- owner: —
- priority: P1
- type: question
",
    )
    .expect("write TODO.md");

    let out = todo_bin()
        .current_dir(dir.path())
        .env("TODO_TEST_NOW", "2026-11-15T20:00:00+01:00")
        .args(["summary", "--json"])
        .output()
        .expect("run summary --json");
    assert!(
        out.status.success(),
        "summary --json failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let json: Value = serde_json::from_slice(&out.stdout).expect("parse json");
    insta::assert_json_snapshot!("summary_json", json);
}

#[test]
fn summary_on_5000_tasks_is_at_most_40_lines() {
    let dir = TempDir::new().expect("tempdir");
    let mut content = String::from(
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n<!-- todo-format: 1 -->\n# Big File\n\n## Tasks\n\n",
    );
    for i in 1..=5000 {
        let (status, extra) = match i % 5 {
            0 => ("done", ""),
            1 => (
                "claimed",
                "- owner: agent-1\n- claimed: 2026-11-15T18:00+01:00\n",
            ),
            2 => (
                "blocked",
                "- note (2026-11-15, agent-1): blocked — waiting\n",
            ),
            _ => ("open", ""),
        };
        content.push_str(&format!(
            "### T-{i:04} · Generated task {i}\n- status: {status}\n- priority: P1\n{extra}\n"
        ));
    }
    fs::write(dir.path().join("TODO.md"), content).expect("write 5000-task TODO.md");

    let out = todo_bin()
        .current_dir(dir.path())
        .args(["summary"])
        .output()
        .expect("run summary");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let line_count = stdout.lines().count();
    assert!(
        line_count <= 40,
        "summary produced {line_count} lines (max 40):\n{stdout}"
    );
    assert!(stdout.contains("5000 total"));
}
