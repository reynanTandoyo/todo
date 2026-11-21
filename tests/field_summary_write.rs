//! Tests for writing the `summary` field (T-209; R-WRITE-19, TEST-4): `add --summary`, `add --from`
//! and `apply` (add and set), `set --summary`, `set --unset summary` and `set --field summary=`, the
//! children of `split`, `-` and `@path` input, line breaks that become spaces, and a refusal that
//! leaves the file byte-identical.

mod common;

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Output, Stdio};

use common::{
    clean_todo_command, parse_json, read_todo, run_todo, setup_proj, stderr_text, write_toml,
};
use tempfile::TempDir;

/// Test clock (`TODO_TEST_NOW`, test-hooks builds): notes written by the tool are dated 2026-11-18.
const NOW: &str = "2026-11-18T10:00:00+01:00";

/// One section with T-001: a priority line with `type` and `area`, `why`, `problem` and `accept`,
/// and no `summary`.
const BASE: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
## Tasks\n\n\
### T-001 · Render the first frame\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: feature · area: renderer\n\
- why: the first frame shows that the pipeline runs\n\
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

/// The file after a write. These fixtures have no format line, so the first write migrates them
/// (format.md §18): the format line goes first, then the running tool stamp (R-VER-1), as
/// `stamped` in tests/cmd_why.rs does.
fn stamped(text: &str) -> String {
    format!(
        "<!-- todo-format: 3 -->\n<!-- todo-tool: {} -->\n{text}",
        env!("CARGO_PKG_VERSION")
    )
}

/// `BASE` with `summary` written on its own line, directly after `why`.
fn base_with_summary(summary: &str) -> String {
    BASE.replace("- problem:", &format!("- summary: {summary}\n- problem:"))
}

/// `text` with the date of every note replaced by `DATE`. The tool dates notes with the test clock
/// (`TODO_TEST_NOW`) only in a build with the `test-hooks` feature, and the verify command of T-209
/// runs without it, so the tests that compare whole files compare the note dates as a placeholder.
fn with_note_dates_masked(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        match line
            .strip_prefix("- note (")
            .and_then(|rest| rest.split_once(", "))
        {
            Some((_, after)) => {
                out.push_str("- note (DATE, ");
                out.push_str(after);
            }
            None => out.push_str(line),
        }
    }
    out
}

#[test]
fn add_summary_is_written_after_why_and_before_depends() {
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
            "Shading",
            "--priority",
            "P2",
            "--difficulty",
            "3",
            "--type",
            "feature",
            "--area",
            "renderer",
            "--why",
            "shading makes the frame match the reference images",
            "--summary",
            "The shading pass, so the frame looks right.",
            "--depends",
            "T-001",
            "--problem",
            "the frame is flat",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));
    assert_eq!(
        parse_json(&out.stdout)["data"]["summary"],
        "The shading pass, so the frame looks right."
    );

    let expected = format!(
        "{BASE}\n### T-002 · Shading\n\
- status: open\n\
- owner: —\n\
- priority: P2 · difficulty: 3 · type: feature · area: renderer\n\
- why: shading makes the frame match the reference images\n\
- summary: The shading pass, so the frame looks right.\n\
- depends: T-001\n\
- problem: the frame is flat\n\
- note (2026-11-18, alice): added\n"
    );
    assert_eq!(
        with_note_dates_masked(&read_todo(&work)),
        with_note_dates_masked(&stamped(&expected))
    );
}

#[test]
fn add_summary_reads_stdin_and_an_at_file_and_line_breaks_become_spaces() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, BASE);

    // `--summary -` reads standard input; the line break becomes a space.
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
            "--difficulty",
            "3",
            "--summary",
            "-",
        ],
        "First line\nsecond line\n",
    );
    assert_eq!(
        from_stdin.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&from_stdin)
    );
    assert_eq!(
        parse_json(&from_stdin.stdout)["data"]["summary"],
        "First line second line"
    );

    // `--summary @path` reads a UTF-8 file (with a byte-order mark) from a folder with spaces and `é`.
    let sub = work.join("my notes café");
    fs::create_dir_all(&sub).unwrap();
    let msg_file = sub.join("summary.txt");
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
            "--difficulty",
            "3",
            "--summary",
            &at_arg,
        ],
    );
    assert_eq!(
        from_file.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&from_file)
    );
    assert_eq!(parse_json(&from_file.stdout)["data"]["summary"], TRICKY);

    // Both tasks read back the same text from disk.
    let shown = run_at_now(&home, &work, &["--json", "show", "T-003"]);
    assert_eq!(parse_json(&shown.stdout)["data"]["summary"], TRICKY);
}

#[test]
fn add_summary_is_allowed_on_a_milestone_a_question_and_an_idea() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, BASE);

    let milestone = run_at_now(
        &home,
        &work,
        &[
            "--agent",
            "alice",
            "add",
            "--title",
            "Frame pipeline",
            "--type",
            "milestone",
            "--goal",
            "the renderer draws a whole frame",
            "--summary",
            "The renderer draws a whole frame, one picture at a time.",
        ],
    );
    assert_eq!(
        milestone.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&milestone)
    );
    let question = run_at_now(
        &home,
        &work,
        &[
            "--agent",
            "alice",
            "add",
            "--title",
            "Which shader",
            "--type",
            "question",
            "--summary",
            "Whether the first shader needs shadows.",
        ],
    );
    assert_eq!(
        question.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&question)
    );
    let idea = run_at_now(
        &home,
        &work,
        &[
            "--agent",
            "alice",
            "add",
            "--title",
            "Hot reload",
            "--type",
            "idea",
            "--summary",
            "Reload the shaders while the scene runs.",
        ],
    );
    assert_eq!(
        idea.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&idea)
    );

    let expected = format!(
        "{BASE}\n\
### T-002 · Frame pipeline\n\
- status: open\n\
- owner: —\n\
- type: milestone\n\
- goal: the renderer draws a whole frame\n\
- summary: The renderer draws a whole frame, one picture at a time.\n\
- note (2026-11-18, alice): added\n\
\n\
### T-003 · Which shader\n\
- status: open\n\
- owner: —\n\
- type: question\n\
- summary: Whether the first shader needs shadows.\n\
- note (2026-11-18, alice): added\n\
\n\
### T-004 · Hot reload\n\
- status: open\n\
- owner: —\n\
- type: idea\n\
- summary: Reload the shaders while the scene runs.\n\
- note (2026-11-18, alice): added\n"
    );
    assert_eq!(
        with_note_dates_masked(&read_todo(&work)),
        with_note_dates_masked(&stamped(&expected))
    );
}

#[test]
fn add_from_writes_summary_at_top_level_and_inside_fields() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, BASE);

    let batch = run_stdin(
        &home,
        &work,
        &["--json", "--agent", "alice", "add", "--from", "-"],
        "{\"title\":\"Ship the first frame\",\"priority\":\"P2\",\"difficulty\":\"3\",\"why\":\"the reference scene must match\",\"summary\":\"The first finished frame.\"}\n\
{\"title\":\"Light pass\",\"priority\":\"P2\",\"difficulty\":\"3\",\"depends\":\"T-001\",\"fields\":{\"summary\":\"Lights are added to the frame.\"}}\n",
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
    let expected = format!(
        "{BASE}\n\
### T-002 · Ship the first frame\n\
- status: open\n\
- owner: —\n\
- priority: P2 · difficulty: 3\n\
- why: the reference scene must match\n\
- summary: The first finished frame.\n\
- note (2026-11-18, alice): added\n\
\n\
### T-003 · Light pass\n\
- status: open\n\
- owner: —\n\
- priority: P2 · difficulty: 3\n\
- summary: Lights are added to the frame.\n\
- depends: T-001\n\
- note (2026-11-18, alice): added\n"
    );
    assert_eq!(
        with_note_dates_masked(&read_todo(&work)),
        with_note_dates_masked(&stamped(&expected))
    );
}

#[test]
fn a_refused_add_from_batch_with_a_summary_leaves_the_file_alone() {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo_file) = setup_proj(&tmp, BASE);
    let before = fs::read(&todo_file).unwrap();

    // The second line repeats the title of the first: the whole batch is refused, nothing is written.
    let batch = run_stdin(
        &home,
        &work,
        &["--json", "--agent", "alice", "add", "--from", "-"],
        "{\"title\":\"Good one\",\"priority\":\"P2\",\"difficulty\":\"3\",\"summary\":\"Fine.\"}\n\
{\"title\":\"good one\",\"priority\":\"P2\",\"difficulty\":\"3\",\"summary\":\"Duplicate.\"}\n",
    );
    assert_eq!(batch.status.code(), Some(1));
    assert_eq!(
        parse_json(&batch.stdout)["error"]["code"],
        "DUPLICATE_TITLE"
    );
    assert_eq!(fs::read(&todo_file).unwrap(), before);
}

#[test]
fn required_summary_refuses_add_without_it_and_leaves_the_file_alone() {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo_file) = setup_proj(&tmp, BASE);
    write_toml(&work, "required_fields = [\"priority\", \"summary\"]\n");
    let before = fs::read(&todo_file).unwrap();

    // `add` without a summary is refused: exit 1, MISSING_FIELD, file byte-identical.
    let refused = run_at_now(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "alice",
            "add",
            "--title",
            "No summary given",
            "--priority",
            "P2",
            "--difficulty",
            "3",
        ],
    );
    assert_eq!(refused.status.code(), Some(1));
    let err = parse_json(&refused.stdout);
    assert_eq!(err["error"]["code"], "MISSING_FIELD");
    assert!(
        err["error"]["message"]
            .as_str()
            .unwrap()
            .contains("summary"),
        "message: {err}"
    );
    assert_eq!(fs::read(&todo_file).unwrap(), before);

    // The same add with `--summary` is written: the flag counts as the field being present.
    let ok = run_at_now(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "alice",
            "add",
            "--title",
            "With a summary",
            "--priority",
            "P2",
            "--difficulty",
            "3",
            "--summary",
            "It has one now.",
        ],
    );
    assert_eq!(ok.status.code(), Some(0), "stderr: {}", stderr_text(&ok));
}

#[test]
fn apply_add_and_set_write_summary_in_the_field_order() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, BASE);

    let applied = run_stdin(
        &home,
        &work,
        &["--json", "--agent", "alice", "apply", "-"],
        "{\"op\":\"add\",\"ref\":\"buffer\",\"title\":\"Frame buffer\",\"priority\":\"P1\",\"difficulty\":\"3\",\"why\":\"the buffer holds the frame\",\"summary\":\"The memory that holds one frame.\"}\n\
{\"op\":\"set\",\"task\":\"T-001\",\"summary\":\"The first picture the renderer draws.\"}\n",
    );
    assert_eq!(
        applied.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&applied)
    );

    let expected = stamped(
        &(base_with_summary("The first picture the renderer draws.")
            + "\n### T-002 · Frame buffer\n\
- status: open\n\
- owner: —\n\
- priority: P1 · difficulty: 3\n\
- why: the buffer holds the frame\n\
- summary: The memory that holds one frame.\n\
- note (2026-11-18, alice): added\n"),
    );
    assert_eq!(
        with_note_dates_masked(&read_todo(&work)),
        with_note_dates_masked(&expected)
    );
}

#[test]
fn apply_add_reads_summary_from_fields_and_turns_line_breaks_into_spaces() {
    // `summary` given inside `fields` must be written as the field, not dropped as a known key.
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, BASE);

    let applied = run_stdin(
        &home,
        &work,
        &["--json", "--agent", "alice", "apply", "-"],
        "{\"op\":\"add\",\"title\":\"Reason in fields\",\"priority\":\"P2\",\"difficulty\":\"3\",\"fields\":{\"summary\":\"carried in fields\"}}\n\
{\"op\":\"add\",\"title\":\"Two lines\",\"priority\":\"P2\",\"difficulty\":\"3\",\"summary\":\"First line\\nsecond line\"}\n",
    );
    assert_eq!(
        applied.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&applied)
    );

    let first = run_at_now(&home, &work, &["--json", "show", "T-002"]);
    assert_eq!(
        parse_json(&first.stdout)["data"]["summary"],
        "carried in fields"
    );
    let second = run_at_now(&home, &work, &["--json", "show", "T-003"]);
    assert_eq!(
        parse_json(&second.stdout)["data"]["summary"],
        "First line second line"
    );
    let text = read_todo(&work);
    assert_eq!(
        text.matches("- summary: carried in fields\n").count(),
        1,
        "{text}"
    );
    assert!(
        text.contains("- summary: First line second line\n"),
        "{text}"
    );
}

#[test]
fn a_refused_apply_with_a_summary_leaves_the_file_alone() {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo_file) = setup_proj(&tmp, BASE);
    let before = fs::read(&todo_file).unwrap();

    // The second operation names a task that does not exist: nothing of the batch is written.
    let applied = run_stdin(
        &home,
        &work,
        &["--json", "--agent", "alice", "apply", "-"],
        "{\"op\":\"add\",\"title\":\"Never written\",\"priority\":\"P2\",\"difficulty\":\"3\",\"summary\":\"Not written.\"}\n\
{\"op\":\"set\",\"task\":\"T-999\",\"summary\":\"No such task.\"}\n",
    );
    assert!(
        !applied.status.success(),
        "stderr: {}",
        stderr_text(&applied)
    );
    assert_eq!(fs::read(&todo_file).unwrap(), before);
}

#[test]
fn set_summary_inserts_after_why_replaces_in_place_and_unsets() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, BASE);

    // No `summary` yet: the line goes directly after `why`, before `problem`.
    let set = run_at_now(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "alice",
            "set",
            "T-001",
            "--summary",
            "The first picture the renderer draws.",
        ],
    );
    assert_eq!(set.status.code(), Some(0), "stderr: {}", stderr_text(&set));
    assert_eq!(
        parse_json(&set.stdout)["data"]["summary"],
        "The first picture the renderer draws."
    );
    assert_eq!(
        read_todo(&work),
        stamped(&base_with_summary("The first picture the renderer draws."))
    );

    // A second `set` replaces the line where it stands: still one summary line.
    let again = run_at_now(
        &home,
        &work,
        &[
            "--agent",
            "alice",
            "set",
            "T-001",
            "--summary",
            "The first frame, drawn.",
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
        stamped(&base_with_summary("The first frame, drawn."))
    );

    // `--unset summary` removes only that line: the file is the original again.
    let unset = run_at_now(
        &home,
        &work,
        &["--agent", "alice", "set", "T-001", "--unset", "summary"],
    );
    assert_eq!(
        unset.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&unset)
    );
    assert_eq!(read_todo(&work), stamped(BASE));
}

#[test]
fn set_field_summary_is_the_same_field_as_the_summary_flag() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, BASE);

    let set = run_at_now(
        &home,
        &work,
        &[
            "--agent",
            "alice",
            "set",
            "T-001",
            "--field",
            "summary=Written through the field flag.",
        ],
    );
    assert_eq!(set.status.code(), Some(0), "stderr: {}", stderr_text(&set));
    let text = read_todo(&work);
    assert_eq!(text.matches("- summary: ").count(), 1, "{text}");
    assert_eq!(
        text,
        stamped(&base_with_summary("Written through the field flag."))
    );
}

#[test]
fn set_summary_reads_stdin_and_an_at_file_and_line_breaks_become_spaces() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, BASE);

    // `set --summary -` reads standard input; the line break becomes a space.
    let from_stdin = run_stdin(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "alice",
            "set",
            "T-001",
            "--summary",
            "-",
        ],
        "First line\nsecond line\n",
    );
    assert_eq!(
        from_stdin.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&from_stdin)
    );
    assert_eq!(
        parse_json(&from_stdin.stdout)["data"]["summary"],
        "First line second line"
    );

    // `set --summary @path` reads a UTF-8 file with a byte-order mark from a folder with spaces and `é`.
    let sub = work.join("my notes café");
    fs::create_dir_all(&sub).unwrap();
    let msg_file = sub.join("summary.txt");
    fs::write(&msg_file, "\u{feff}Replaced from a file\n").unwrap();
    let at_arg = format!("@{}", msg_file.display());
    let from_file = run_at_now(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "alice",
            "set",
            "T-001",
            "--summary",
            &at_arg,
        ],
    );
    assert_eq!(
        from_file.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&from_file)
    );
    assert_eq!(
        parse_json(&from_file.stdout)["data"]["summary"],
        "Replaced from a file"
    );
    assert_eq!(
        read_todo(&work).matches("- summary: ").count(),
        1,
        "one summary line after two sets"
    );
}

#[test]
fn set_summary_is_refused_for_a_task_another_agent_holds_and_leaves_the_file_alone() {
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
            "--json",
            "--agent",
            "alice",
            "set",
            "T-001",
            "--summary",
            "not mine",
        ],
    );
    assert_eq!(refused.status.code(), Some(1));
    assert_eq!(parse_json(&refused.stdout)["error"]["code"], "NOT_OWNER");
    assert_eq!(fs::read(&todo_file).unwrap(), before);
}

#[test]
fn split_children_take_the_summary_from_their_json_lines() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, BASE);

    let split = run_stdin(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "alice",
            "split",
            "T-001",
            "--from",
            "-",
            "--goal",
            "the renderer draws the scene",
        ],
        "{\"ref\":\"clear\",\"title\":\"Clear the buffer\",\"priority\":\"P2\",\"difficulty\":\"2\",\"why\":\"the frame starts empty\",\"summary\":\"Clear the picture before each frame.\"}\n\
{\"title\":\"Draw the triangle\",\"priority\":\"P2\",\"difficulty\":\"2\",\"depends\":\"@clear\",\"summary\":\"Draw one triangle on the picture.\"}\n",
    );
    assert_eq!(
        split.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&split)
    );

    let text = read_todo(&work);
    assert!(
        text.contains(
            "- milestone: T-001\n- why: the frame starts empty\n- summary: Clear the picture before each frame.\n"
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "- milestone: T-001\n- summary: Draw one triangle on the picture.\n- depends: T-002\n"
        ),
        "{text}"
    );

    let shown = run_at_now(&home, &work, &["--json", "show", "T-003"]);
    assert_eq!(
        parse_json(&shown.stdout)["data"]["summary"],
        "Draw one triangle on the picture."
    );
}

#[test]
fn a_refused_split_with_a_summary_child_leaves_the_file_alone() {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo_file) = setup_proj(&tmp, BASE);
    let before = fs::read(&todo_file).unwrap();

    // A child that names a section is refused before anything is written (R-SPLIT-1).
    let split = run_stdin(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "alice",
            "split",
            "T-001",
            "--from",
            "-",
            "--goal",
            "the renderer draws the scene",
        ],
        "{\"title\":\"Stray child\",\"section\":\"Tasks\",\"priority\":\"P2\",\"difficulty\":\"2\",\"summary\":\"Refused.\"}\n",
    );
    assert_eq!(
        split.status.code(),
        Some(1),
        "stderr: {}",
        stderr_text(&split)
    );
    assert_eq!(fs::read(&todo_file).unwrap(), before);
}
