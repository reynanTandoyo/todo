//! Tests for the `why` field across the write commands: `add --why`, `set --why`, `why` in
//! `add --from` and `apply`, hand-written `why` lines, and `show --json` / `schema` (T-091;
//! R-WRITE-17, TEST-4).

mod common;

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Output, Stdio};

use common::{
    clean_todo_command, parse_json, read_todo, run_todo, setup_fixture_proj, setup_proj,
    stderr_text, write_toml,
};
use serde_json::Value;
use tempfile::TempDir;

/// Test clock (`TODO_TEST_NOW`, test-hooks builds): notes written by the tool are dated 2026-11-18.
const NOW: &str = "2026-11-18T10:00:00+01:00";

/// One section with T-001: a priority line with `type` and `area`, `problem` and `accept`, no `why`.
const BASE: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
## Tasks\n\n\
### T-001 · Render the first frame\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: feature · area: renderer\n\
- problem: nothing is drawn yet\n\
- accept: a frame is written to disk\n";

const TRICKY: &str = r#"Keep "double", 'single', `backticks`, and $env:VAR $(sub) intact!"#;

/// Runs `todo` with the test clock set and `stdin_data` on standard input.
fn run_stdin(home: &Path, work: &Path, args: &[&str], stdin_data: &str) -> Output {
    let mut child = clean_todo_command(home, work)
        .args(args)
        .env("TODO_TEST_NOW", NOW)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn todo");
    child
        .stdin
        .take()
        .expect("todo stdin")
        .write_all(stdin_data.as_bytes())
        .expect("write todo stdin");
    child.wait_with_output().expect("wait for todo")
}

/// Runs `todo` with the test clock set.
fn run_at_now(home: &Path, work: &Path, args: &[&str]) -> Output {
    run_todo(home, work, args, &[("TODO_TEST_NOW", NOW)]).expect("run todo")
}

#[test]
fn add_why_is_written_before_depends_and_problem() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, BASE);

    let out = run_at_now(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "alice",
            "add",
            "--title",
            "Add the shading stage",
            "--priority",
            "P2",
            "--type",
            "feature",
            "--area",
            "renderer",
            "--why",
            "shading makes the frame match the reference images",
            "--depends",
            "T-001",
            "--problem",
            "the frame is flat",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));
    let json = parse_json(&out.stdout);
    assert_eq!(json["data"]["id"], "T-002");
    assert_eq!(
        json["data"]["why"],
        "shading makes the frame match the reference images"
    );

    // `why` comes after the combined priority line and before `depends` and `problem` (§10).
    let expected = format!(
        "{BASE}\n### T-002 · Add the shading stage\n\
- status: open\n\
- owner: —\n\
- priority: P2 · type: feature · area: renderer\n\
- why: shading makes the frame match the reference images\n\
- depends: T-001\n\
- problem: the frame is flat\n\
- note (2026-11-18, alice): added\n"
    );
    assert_eq!(read_todo(&work), expected);
}

#[test]
fn add_field_why_is_the_same_field_as_the_why_flag() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, BASE);

    let out = run_at_now(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "alice",
            "add",
            "--title",
            "Light pass",
            "--priority",
            "P2",
            "--depends",
            "T-001",
            "--field",
            "why=lights come after shading",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));
    assert_eq!(
        parse_json(&out.stdout)["data"]["why"],
        "lights come after shading"
    );

    let text = read_todo(&work);
    assert_eq!(
        text.matches("- why: ").count(),
        1,
        "why written once: {text}"
    );
    assert!(text.ends_with(
        "### T-002 · Light pass\n\
- status: open\n\
- owner: —\n\
- priority: P2\n\
- why: lights come after shading\n\
- depends: T-001\n\
- note (2026-11-18, alice): added\n"
    ));
}

#[test]
fn add_why_reads_stdin_and_an_at_file_in_a_folder_with_spaces_and_non_ascii() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, BASE);

    // `--why -` reads standard input; quotes, backticks and `$` arrive intact.
    let from_stdin = run_stdin(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "alice",
            "add",
            "--title",
            "From stdin",
            "--priority",
            "P2",
            "--why",
            "-",
        ],
        TRICKY,
    );
    assert_eq!(
        from_stdin.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&from_stdin)
    );
    assert_eq!(parse_json(&from_stdin.stdout)["data"]["why"], TRICKY);

    // `--why @path` reads a UTF-8 file (with a byte-order mark) from a folder with spaces and `é`.
    let sub = work.join("my notes café");
    fs::create_dir_all(&sub).unwrap();
    let msg_file = sub.join("why.txt");
    fs::write(&msg_file, format!("\u{feff}{TRICKY}\n")).unwrap();
    let at_arg = format!("@{}", msg_file.display());
    let from_file = run_at_now(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "alice",
            "add",
            "--title",
            "From file",
            "--priority",
            "P2",
            "--why",
            &at_arg,
        ],
    );
    assert_eq!(
        from_file.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&from_file)
    );
    assert_eq!(parse_json(&from_file.stdout)["data"]["why"], TRICKY);

    // Both tasks read back the same text from disk.
    let shown = run_at_now(&home, &work, &["--json", "show", "T-003"]);
    assert_eq!(parse_json(&shown.stdout)["data"]["why"], TRICKY);
}

#[test]
fn required_why_refuses_add_and_add_from_and_leaves_the_file_alone() {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo_file) = setup_proj(&tmp, BASE);
    write_toml(&work, "required_fields = [\"priority\", \"why\"]\n");
    let before = fs::read(&todo_file).unwrap();

    // `add` without a reason is refused: exit 1, MISSING_FIELD, file byte-identical.
    let refused = run_at_now(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "alice",
            "add",
            "--title",
            "No reason given",
            "--priority",
            "P2",
        ],
    );
    assert_eq!(refused.status.code(), Some(1));
    let err = parse_json(&refused.stdout);
    assert_eq!(err["error"]["code"], "MISSING_FIELD");
    assert!(
        err["error"]["message"].as_str().unwrap().contains("why"),
        "message: {err}"
    );
    assert_eq!(fs::read(&todo_file).unwrap(), before);

    // `add --from` without a reason is refused the same way, and nothing is written.
    let batch = run_stdin(
        &home,
        &work,
        &["--json", "--agent", "alice", "add", "--from", "-"],
        "{\"title\":\"Batch without reason\",\"priority\":\"P2\"}\n",
    );
    assert_eq!(batch.status.code(), Some(1));
    assert_eq!(parse_json(&batch.stdout)["error"]["code"], "MISSING_FIELD");
    assert_eq!(fs::read(&todo_file).unwrap(), before);

    // With a reason the same add succeeds.
    let ok = run_at_now(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "alice",
            "add",
            "--title",
            "With a reason",
            "--priority",
            "P2",
            "--why",
            "it is the next step",
        ],
    );
    assert_eq!(ok.status.code(), Some(0), "stderr: {}", stderr_text(&ok));
}

#[test]
fn set_why_inserts_after_area_replaces_in_place_and_unsets() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, BASE);

    // No `why` yet: the line goes after the combined priority line, before `problem`.
    let set = run_at_now(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "alice",
            "set",
            "T-001",
            "--why",
            "a frame must come out first",
        ],
    );
    assert_eq!(set.status.code(), Some(0), "stderr: {}", stderr_text(&set));
    assert_eq!(
        parse_json(&set.stdout)["data"]["why"],
        "a frame must come out first"
    );
    let with_why = BASE.replace(
        "- problem:",
        "- why: a frame must come out first\n- problem:",
    );
    assert_eq!(read_todo(&work), with_why);

    // A second `set` replaces the line where it stands.
    let again = run_at_now(
        &home,
        &work,
        &[
            "--agent",
            "alice",
            "set",
            "T-001",
            "--why",
            "the frame is the first goal",
        ],
    );
    assert_eq!(
        again.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&again)
    );
    assert_eq!(
        read_todo(&work),
        with_why.replace(
            "- why: a frame must come out first",
            "- why: the frame is the first goal"
        )
    );

    // `--unset why` removes only that line: the file is the original again.
    let unset = run_at_now(
        &home,
        &work,
        &["--agent", "alice", "set", "T-001", "--unset", "why"],
    );
    assert_eq!(
        unset.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&unset)
    );
    assert_eq!(read_todo(&work), BASE);
}

#[test]
fn set_why_is_refused_for_a_task_another_agent_holds_and_leaves_the_file_alone() {
    let tmp = TempDir::new().unwrap();
    let claimed = BASE
        .replace(
            "- owner: —\n",
            "- owner: bob\n- claimed: 2026-11-18T09:00+01:00\n",
        )
        .replace("- status: open\n", "- status: claimed\n");
    let (home, work, todo_file) = setup_proj(&tmp, &claimed);
    let before = fs::read(&todo_file).unwrap();

    let refused = run_at_now(
        &home,
        &work,
        &[
            "--json", "--agent", "alice", "set", "T-001", "--why", "not mine",
        ],
    );
    assert_eq!(refused.status.code(), Some(1));
    assert_eq!(parse_json(&refused.stdout)["error"]["code"], "NOT_OWNER");
    assert_eq!(fs::read(&todo_file).unwrap(), before);
}

#[test]
fn hand_written_why_line_is_read_as_the_why_field_and_keeps_working() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_fixture_proj(&tmp, "why_hand_written.md");
    let original = read_todo(&work);

    // `show --json` reports the hand-written line as `why`, not as an unknown field.
    let shown = run_at_now(&home, &work, &["--json", "show", "T-001"]);
    assert_eq!(
        shown.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&shown)
    );
    let json = parse_json(&shown.stdout);
    assert_eq!(
        json["data"]["why"],
        "the renderer must show a frame before shading is added"
    );
    assert!(json["data"]["fields"].get("why").is_none(), "{json}");

    // The file is still a clean task list.
    let check = run_at_now(&home, &work, &["check"]);
    assert_eq!(
        check.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&check)
    );

    // `set --why` changes the existing line where it is and touches nothing else.
    let set = run_at_now(
        &home,
        &work,
        &[
            "--agent",
            "alice",
            "set",
            "T-001",
            "--why",
            "the first frame is the goal",
        ],
    );
    assert_eq!(set.status.code(), Some(0), "stderr: {}", stderr_text(&set));
    assert_eq!(
        read_todo(&work),
        original.replace(
            "- why: the renderer must show a frame before shading is added",
            "- why: the first frame is the goal"
        )
    );
}

#[test]
fn show_json_has_why_and_schema_lists_it() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, BASE);

    // A task without `why` still has the key, with null.
    let shown = run_at_now(&home, &work, &["--json", "show", "T-001"]);
    assert_eq!(
        shown.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&shown)
    );
    let json = parse_json(&shown.stdout);
    assert_eq!(json["data"].get("why"), Some(&Value::Null), "{json}");

    // `schema` lists `why` among the task fields.
    let schema = run_at_now(&home, &work, &["--json", "schema"]);
    assert_eq!(
        schema.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&schema)
    );
    let fields = parse_json(&schema.stdout)["data"]["task"]["fields"]
        .as_array()
        .expect("task fields array")
        .clone();
    assert!(fields.iter().any(|f| f == "why"), "fields: {fields:?}");
}

#[test]
fn add_from_and_apply_write_why_in_the_field_order() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, BASE);

    // `add --from`: top-level `why`, and `why` inside `fields`.
    let batch = run_stdin(
        &home,
        &work,
        &["--json", "--agent", "alice", "add", "--from", "-"],
        "{\"title\":\"Ship the first frame\",\"priority\":\"P2\",\"why\":\"the reference scene must match\"}\n\
{\"title\":\"Light pass\",\"priority\":\"P2\",\"depends\":\"T-001\",\"fields\":{\"why\":\"lights come after shading\"}}\n",
    );
    assert_eq!(
        batch.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&batch)
    );
    assert_eq!(
        parse_json(&batch.stdout)["data"]["ids"],
        serde_json::json!(["T-002", "T-003"])
    );
    let after_batch = format!(
        "{BASE}\n\
### T-002 · Ship the first frame\n\
- status: open\n\
- owner: —\n\
- priority: P2\n\
- why: the reference scene must match\n\
- note (2026-11-18, alice): added\n\
\n\
### T-003 · Light pass\n\
- status: open\n\
- owner: —\n\
- priority: P2\n\
- why: lights come after shading\n\
- depends: T-001\n\
- note (2026-11-18, alice): added\n"
    );
    assert_eq!(read_todo(&work), after_batch);

    // `apply`: an `add` operation with `why`, and a `set` operation that adds `why` to T-001.
    let applied = run_stdin(
        &home,
        &work,
        &["--json", "--agent", "alice", "apply", "-"],
        "{\"op\":\"add\",\"ref\":\"buffer\",\"title\":\"Frame buffer\",\"priority\":\"P1\",\"why\":\"the buffer holds the frame\"}\n\
{\"op\":\"set\",\"task\":\"T-001\",\"why\":\"the first frame is the goal\"}\n",
    );
    assert_eq!(
        applied.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&applied)
    );

    let expected = after_batch.replace(
        "- problem: nothing is drawn yet",
        "- why: the first frame is the goal\n- problem: nothing is drawn yet",
    ) + "\n### T-004 · Frame buffer\n\
- status: open\n\
- owner: —\n\
- priority: P1\n\
- why: the buffer holds the frame\n\
- note (2026-11-18, alice): added\n";
    assert_eq!(read_todo(&work), expected);
}

#[test]
fn set_why_reads_stdin_and_an_at_file() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, BASE);

    // `set --why -` reads standard input.
    let from_stdin = run_stdin(
        &home,
        &work,
        &["--json", "--agent", "alice", "set", "T-001", "--why", "-"],
        TRICKY,
    );
    assert_eq!(
        from_stdin.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&from_stdin)
    );
    assert_eq!(parse_json(&from_stdin.stdout)["data"]["why"], TRICKY);

    // `set --why @path` reads a UTF-8 file with a byte-order mark from a folder with spaces and `é`.
    let sub = work.join("my notes café");
    fs::create_dir_all(&sub).unwrap();
    let msg_file = sub.join("why.txt");
    fs::write(&msg_file, "\u{feff}Replaced from a file\n").unwrap();
    let at_arg = format!("@{}", msg_file.display());
    let from_file = run_at_now(
        &home,
        &work,
        &[
            "--json", "--agent", "alice", "set", "T-001", "--why", &at_arg,
        ],
    );
    assert_eq!(
        from_file.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&from_file)
    );
    assert_eq!(
        parse_json(&from_file.stdout)["data"]["why"],
        "Replaced from a file"
    );
    assert_eq!(
        read_todo(&work).matches("- why: ").count(),
        1,
        "one why line after two sets"
    );
}
