//! Integration tests for the `difficulty` field (T-108; R-WRITE-18, R-READ-1, R-READ-3, R-CFG-2).
//!
//! The helpers panic on setup failures, which is acceptable in test code (see `tests/common`).
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

mod common;

use std::io::Write;
use std::path::Path;
use std::process::{Output, Stdio};

use common::{
    clean_todo_command, parse_json, read_text, read_todo, run_todo, setup_proj, stderr_text,
    stdout_text, write_toml,
};
use tempfile::TempDir;

const MARKER: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->";

/// A TODO.md with one `## Tasks` section that holds `body`.
fn doc(body: &str) -> String {
    format!("{MARKER}\n<!-- todo-format: 1 -->\n\n## Tasks\n\n{body}")
}

/// The same file after a write: migrated to the current format (R-MIG-1), with the running tool stamp
/// directly after the format line (R-VER-1).
fn written(body: &str) -> String {
    format!(
        "{MARKER}\n<!-- todo-format: 3 -->\n<!-- todo-tool: {} -->\n\n## Tasks\n\n{body}",
        env!("CARGO_PKG_VERSION")
    )
}

/// Runs `todo` with `args` and succeeds, returning its output.
fn ok(home: &Path, work: &Path, args: &[&str]) -> Output {
    let out = run_todo(home, work, args, &[]).unwrap();
    assert!(
        out.status.success(),
        "{args:?} failed:\n{}{}",
        stdout_text(&out),
        stderr_text(&out)
    );
    out
}

/// Runs `todo` with `args` and expects a refusal with exit code 1.
fn refused(home: &Path, work: &Path, args: &[&str]) -> Output {
    let out = run_todo(home, work, args, &[]).unwrap();
    assert_eq!(
        out.status.code(),
        Some(1),
        "{args:?} should be refused:\n{}{}",
        stdout_text(&out),
        stderr_text(&out)
    );
    out
}

/// Runs `todo` with `args`, feeding `input` on standard input.
fn run_stdin(home: &Path, work: &Path, args: &[&str], input: &str) -> Output {
    let mut child = clean_todo_command(home, work)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn todo");
    child
        .stdin
        .as_mut()
        .expect("todo stdin")
        .write_all(input.as_bytes())
        .expect("write todo stdin");
    child.wait_with_output().expect("wait for todo")
}

/// The `- status:` value of task `id` in `text`.
fn status_of(text: &str, id: &str) -> String {
    let heading = format!("### {id} ·");
    let block = text.split(heading.as_str()).nth(1).expect("task heading");
    block
        .lines()
        .find_map(|line| line.strip_prefix("- status: "))
        .expect("status line")
        .to_string()
}

/// The whitespace-separated columns of the `list` line that starts with `id`.
fn list_columns(stdout: &str, id: &str) -> Vec<String> {
    let line = stdout
        .lines()
        .find(|l| l.starts_with(id))
        .unwrap_or_else(|| panic!("no list line for {id}:\n{stdout}"));
    line.split_whitespace().map(str::to_string).collect()
}

#[test]
fn add_writes_difficulty_on_the_combined_line_right_after_priority() {
    let tmp = TempDir::new().unwrap();
    let (home, work, file) = setup_proj(&tmp, &doc(""));

    ok(
        &home,
        &work,
        &[
            "--agent",
            "t-agent",
            "add",
            "--title",
            "Parse the config",
            "--priority",
            "P1",
            "--difficulty",
            "4",
            "--type",
            "feature",
            "--area",
            "cli",
        ],
    );
    let text = read_text(&file);
    assert!(
        text.contains(
            "### T-001 · Parse the config\n- status: open\n- owner: —\n\
             - priority: P1 · difficulty: 4 · type: feature · area: cli\n"
        ),
        "{text}"
    );
}

#[test]
fn add_takes_difficulty_from_field_case_insensitively_and_refuses_unknown_values() {
    let tmp = TempDir::new().unwrap();
    let (home, work, file) = setup_proj(&tmp, &doc(""));

    ok(
        &home,
        &work,
        &[
            "--agent",
            "t-agent",
            "add",
            "--title",
            "Field spelling",
            "--priority",
            "P2",
            "--field",
            "difficulty=2",
        ],
    );
    assert!(
        read_todo(&work).contains("- priority: P2 · difficulty: 2\n"),
        "value is written in its configured spelling"
    );

    let before = read_text(&file);
    let out = refused(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "t-agent",
            "add",
            "--title",
            "Too hard to name",
            "--priority",
            "P1",
            "--difficulty",
            "extreme",
        ],
    );
    assert_eq!(parse_json(&out.stdout)["error"]["code"], "BAD_VALUE");
    assert_eq!(
        read_text(&file),
        before,
        "a refused add leaves TODO.md alone"
    );
}

#[test]
fn set_edits_difficulty_in_place_on_the_combined_line_and_unsets_it() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Parse the config\n- status: open\n- owner: —\n\
                - priority: P1 · type: bug · area: ui\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));

    ok(
        &home,
        &work,
        &["--agent", "t-agent", "set", "T-001", "--difficulty", "2"],
    );
    assert_eq!(
        read_text(&file),
        written(
            "### T-001 · Parse the config\n- status: open\n- owner: —\n\
             - priority: P1 · difficulty: 2 · type: bug · area: ui\n"
        )
    );

    ok(
        &home,
        &work,
        &["--agent", "t-agent", "set", "T-001", "--difficulty", "4"],
    );
    assert!(read_todo(&work).contains("- priority: P1 · difficulty: 4 · type: bug · area: ui\n"));

    ok(
        &home,
        &work,
        &[
            "--agent",
            "t-agent",
            "set",
            "T-001",
            "--unset",
            "difficulty",
        ],
    );
    assert_eq!(
        read_text(&file),
        written(body),
        "unsetting restores the original bytes"
    );
}

#[test]
fn new_difficulty_and_type_follow_the_field_order_of_format_md() {
    let tmp = TempDir::new().unwrap();
    let body =
        "### T-001 · Parse the config\n- status: open\n- owner: —\n- priority: P2 · area: docs\n";
    let (home, work, _) = setup_proj(&tmp, &doc(body));

    ok(
        &home,
        &work,
        &[
            "--agent",
            "t-agent",
            "set",
            "T-001",
            "--type",
            "feature",
            "--difficulty",
            "3",
        ],
    );
    assert!(
        read_todo(&work).contains("- priority: P2 · difficulty: 3 · type: feature · area: docs\n"),
        "{}",
        read_todo(&work)
    );
}

#[test]
fn difficulty_on_its_own_line_is_read_and_edited_in_place() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Parse the config\n- status: open\n- owner: —\n\
                - priority: P1\n- difficulty: 4\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));

    let show = ok(&home, &work, &["--json", "show", "T-001"]);
    assert_eq!(parse_json(&show.stdout)["data"]["difficulty"], "4");

    ok(
        &home,
        &work,
        &["--agent", "t-agent", "set", "T-001", "--difficulty", "2"],
    );
    assert_eq!(
        read_text(&file),
        written(
            "### T-001 · Parse the config\n- status: open\n- owner: —\n\
             - priority: P1\n- difficulty: 2\n"
        )
    );

    ok(
        &home,
        &work,
        &[
            "--agent",
            "t-agent",
            "set",
            "T-001",
            "--unset",
            "difficulty",
        ],
    );
    assert_eq!(
        read_text(&file),
        written("### T-001 · Parse the config\n- status: open\n- owner: —\n- priority: P1\n")
    );
}

#[test]
fn repeated_difficulty_blocks_writes_to_that_task() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Parse the config\n- status: open\n- owner: —\n\
                - priority: P1 · difficulty: 2\n- difficulty: 4\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));
    let before = read_text(&file);

    let out = run_todo(
        &home,
        &work,
        &["--agent", "t-agent", "set", "T-001", "--priority", "P0"],
        &[],
    )
    .unwrap();
    assert!(!out.status.success(), "a repeated field blocks the write");
    assert_eq!(read_text(&file), before);
}

#[test]
fn add_from_and_apply_write_difficulty_and_refuse_unknown_values() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Existing task\n- status: open\n- owner: —\n- priority: P1\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));

    let out = run_stdin(
        &home,
        &work,
        &["--agent", "t-agent", "add", "--from", "-"],
        "{\"title\":\"From file\",\"priority\":\"P1\",\"difficulty\":\"3\",\"section\":\"Tasks\"}\n",
    );
    assert!(out.status.success(), "{}", stderr_text(&out));
    assert!(
        read_todo(&work).contains(
            "### T-002 · From file\n- status: open\n- owner: —\n- priority: P1 · difficulty: 3\n"
        ),
        "{}",
        read_todo(&work)
    );

    let before = read_text(&file);
    let out = run_stdin(
        &home,
        &work,
        &["--json", "--agent", "t-agent", "add", "--from", "-"],
        "{\"title\":\"Bad one\",\"priority\":\"P1\",\"difficulty\":\"huge\",\"section\":\"Tasks\"}\n",
    );
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(parse_json(&out.stdout)["error"]["code"], "BAD_VALUE");
    assert_eq!(read_text(&file), before);

    let out = run_stdin(
        &home,
        &work,
        &["--agent", "t-agent", "apply", "-"],
        "{\"op\":\"set\",\"task\":\"T-001\",\"difficulty\":\"4\"}\n\
         {\"op\":\"add\",\"title\":\"Via apply\",\"priority\":\"P2\",\"difficulty\":\"2\",\"section\":\"Tasks\"}\n",
    );
    assert!(out.status.success(), "{}", stderr_text(&out));
    let text = read_todo(&work);
    assert!(text.contains("- priority: P1 · difficulty: 4\n"), "{text}");
    assert!(text.contains("### T-003 · Via apply\n"), "{text}");
    assert!(text.contains("- priority: P2 · difficulty: 2\n"), "{text}");

    let before = read_text(&file);
    let out = run_stdin(
        &home,
        &work,
        &["--agent", "t-agent", "apply", "-"],
        "{\"op\":\"set\",\"task\":\"T-001\",\"priority\":\"P0\",\"difficulty\":\"huge\"}\n",
    );
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(read_text(&file), before, "a refused apply changes nothing");
}

#[test]
fn todo_toml_sets_the_allowed_difficulties_and_their_order() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, &doc(""));
    write_toml(&work, "difficulties = [\"trivial\", \"tough\"]\n");

    let out = refused(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "t-agent",
            "add",
            "--title",
            "Old word",
            "--priority",
            "P1",
            "--difficulty",
            "2",
        ],
    );
    assert_eq!(parse_json(&out.stdout)["error"]["code"], "BAD_VALUE");

    ok(
        &home,
        &work,
        &[
            "--agent",
            "t-agent",
            "add",
            "--title",
            "Tough one",
            "--priority",
            "P1",
            "--difficulty",
            "TOUGH",
        ],
    );
    ok(
        &home,
        &work,
        &[
            "--agent",
            "t-agent",
            "add",
            "--title",
            "Trivial one",
            "--priority",
            "P2",
            "--difficulty",
            "trivial",
            "--why",
            "the reason for the work",
            "--accept",
            "the result is visible",
            "--steps",
            "open the file",
            "--verify",
            "cargo test",
            "--files",
            "src/lib.rs",
        ],
    );
    // `trivial` is the easiest value, which is the target level: its task is handed out only with
    // its detail complete (R-DETAIL-2), so this one has all of it. `tough` is not within `trivial`.
    let next = ok(
        &home,
        &work,
        &["--json", "next", "--max-difficulty", "trivial"],
    );
    assert_eq!(
        parse_json(&next.stdout)["data"]["task"]["title"],
        "Trivial one"
    );

    let out = refused(&home, &work, &["list", "--difficulty", "hard"]);
    let text = format!("{}{}", stdout_text(&out), stderr_text(&out));
    assert!(text.contains("invalid difficulty 'hard'"), "{text}");
}

#[test]
fn todo_toml_refuses_an_empty_difficulties_list_and_can_require_difficulty() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_proj(&tmp, &doc(""));

    write_toml(&work, "difficulties = []\n");
    let out = refused(&home, &work, &["list"]);
    let text = format!("{}{}", stdout_text(&out), stderr_text(&out));
    assert!(text.contains("difficulties must not be empty"), "{text}");

    write_toml(&work, "required_fields = [\"priority\", \"difficulty\"]\n");
    let out = refused(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "t-agent",
            "add",
            "--title",
            "Needs a level",
            "--priority",
            "P1",
        ],
    );
    assert_eq!(parse_json(&out.stdout)["error"]["code"], "MISSING_FIELD");
}

#[test]
fn list_filters_by_difficulty_and_shows_the_column_only_when_used() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Easy one\n- status: open\n- priority: P1 · difficulty: 2\n\n\
                ### T-002 · Hard one\n- status: open\n- priority: P1 · difficulty: 4\n\n\
                ### T-003 · No level\n- status: open\n- priority: P2\n\n\
                ### T-004 · Hand written\n- status: open\n- priority: P2\n- difficulty: extreme\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));

    let all = stdout_text(&ok(&home, &work, &["list"]));
    assert_eq!(
        list_columns(&all, "T-001")[..4],
        ["T-001", "P1", "2", "open"],
        "{all}"
    );
    assert_eq!(
        list_columns(&all, "T-003")[..4],
        ["T-003", "P2", "-", "open"],
        "{all}"
    );
    assert_eq!(
        list_columns(&all, "T-004")[..4],
        ["T-004", "P2", "extreme", "open"],
        "{all}"
    );

    let hard = stdout_text(&ok(&home, &work, &["list", "--difficulty", "4"]));
    assert!(hard.contains("T-002") && !hard.contains("T-001"), "{hard}");
    assert!(hard.contains("1 task"), "{hard}");

    let two = stdout_text(&ok(&home, &work, &["list", "--difficulty", "2,4"]));
    assert!(two.contains("2 tasks"), "{two}");

    let json = parse_json(&ok(&home, &work, &["--json", "list", "--difficulty", "4"]).stdout);
    assert_eq!(json["data"]["count"], 1);
    assert_eq!(json["data"]["tasks"][0]["id"], "T-002");
    assert_eq!(json["data"]["tasks"][0]["difficulty"], "4");

    let all_json = parse_json(&ok(&home, &work, &["--json", "list"]).stdout);
    assert!(
        all_json["data"]["tasks"][2]["difficulty"].is_null(),
        "a task without difficulty shows null"
    );

    let where_json =
        parse_json(&ok(&home, &work, &["--json", "list", "--where", "difficulty=4"]).stdout);
    assert_eq!(where_json["data"]["count"], 1);

    let out = refused(&home, &work, &["--json", "list", "--difficulty", "extreme"]);
    assert_eq!(parse_json(&out.stdout)["error"]["code"], "BAD_VALUE");

    // Without any difficulty on the listed tasks the column is left out.
    std::fs::write(
        file,
        doc("### T-003 · No level\n- status: open\n- priority: P2\n"),
    )
    .unwrap();
    let plain = stdout_text(&ok(&home, &work, &["list"]));
    assert_eq!(
        list_columns(&plain, "T-003")[..3],
        ["T-003", "P2", "open"],
        "{plain}"
    );
}

#[test]
fn next_hands_out_only_matching_difficulties_and_keeps_the_priority_order() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Hard and urgent\n- status: open\n- priority: P0 · difficulty: 4\n\n\
                ### T-002 · Urgent without a level\n- status: open\n- priority: P1\n\n\
                ### T-003 · Easy P1\n- status: open\n- priority: P1 · difficulty: 2\n\n\
                ### T-004 · Medium P2\n- status: open\n- priority: P2 · difficulty: 3\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));

    let picked = |args: &[&str]| -> serde_json::Value {
        let mut full = vec!["--json", "next"];
        full.extend_from_slice(args);
        parse_json(&ok(&home, &work, &full).stdout)["data"]["task"]["id"].clone()
    };

    // Difficulty does not change the order: the P0 task comes first whatever its difficulty.
    assert_eq!(picked(&[]), "T-001");
    assert_eq!(picked(&["--difficulty", "3"]), "T-004");
    assert_eq!(picked(&["--difficulty", "4,3"]), "T-001");
    // T-002 is earlier in the file and P1, but has no difficulty, so a difficulty option skips it.
    assert_eq!(picked(&["--max-difficulty", "2"]), "T-003");
    assert_eq!(picked(&["--max-difficulty", "3"]), "T-003");
    assert_eq!(picked(&["--max-difficulty", "4"]), "T-001");
    assert_eq!(
        picked(&["--difficulty", "2", "--max-difficulty", "3"]),
        "T-003"
    );

    for args in [
        &["--difficulty", "extreme"][..],
        &["--max-difficulty", "extreme"][..],
    ] {
        let mut full = vec!["--json", "next"];
        full.extend_from_slice(args);
        let out = refused(&home, &work, &full);
        assert_eq!(parse_json(&out.stdout)["error"]["code"], "BAD_VALUE");
    }

    // A task without difficulty is never handed out, with or without a difficulty option (R-LEVEL-1).
    let only_plain = "### T-009 · Plain task\n- status: open\n- priority: P1\n";
    std::fs::write(&file, doc(only_plain)).unwrap();
    assert!(picked(&[]).is_null());
    assert!(picked(&["--max-difficulty", "4"]).is_null());
    assert!(picked(&["--difficulty", "2"]).is_null());
}

#[test]
fn next_claim_takes_only_tasks_within_the_difficulty_and_keeps_own_claims_to_match() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Hard and urgent\n- status: open\n- priority: P0 · difficulty: 4\n\n\
                ### T-002 · Easy P1\n- status: open\n- priority: P1 · difficulty: 2\n\n\
                ### T-003 · Medium P2\n- status: open\n- priority: P2 · difficulty: 3\n";
    let (home, work, _) = setup_proj(&tmp, &doc(body));

    let claimed = parse_json(
        &ok(
            &home,
            &work,
            &[
                "--json",
                "--agent",
                "alice",
                "next",
                "--claim",
                "--max-difficulty",
                "3",
            ],
        )
        .stdout,
    );
    assert_eq!(claimed["data"]["task"]["id"], "T-002");
    let text = read_todo(&work);
    assert_eq!(status_of(&text, "T-002"), "claimed");
    assert_eq!(status_of(&text, "T-001"), "open");

    // Alice's own claim (easy) is within `easy`, so `next` returns it again.
    let own = parse_json(
        &ok(
            &home,
            &work,
            &[
                "--json",
                "--agent",
                "alice",
                "next",
                "--max-difficulty",
                "2",
            ],
        )
        .stdout,
    );
    assert_eq!(own["data"]["task"]["id"], "T-002");
    assert_eq!(own["data"]["already_claimed"], true);

    // Her claim is easy, so it is not the answer for `--difficulty hard`: the hard task is.
    let other = parse_json(
        &ok(
            &home,
            &work,
            &["--json", "--agent", "alice", "next", "--difficulty", "4"],
        )
        .stdout,
    );
    assert_eq!(other["data"]["task"]["id"], "T-001");
    assert_eq!(other["data"]["already_claimed"], false);
}

#[test]
fn difficulty_is_kept_when_the_priority_line_is_created_or_removed() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Parse the config\n- status: open\n- owner: —\n";
    let (home, work, file) = setup_proj(&tmp, &doc(body));

    // No priority line yet: priority and difficulty share one new line, in order.
    ok(
        &home,
        &work,
        &[
            "--agent",
            "t-agent",
            "set",
            "T-001",
            "--difficulty",
            "4",
            "--priority",
            "P1",
        ],
    );
    assert_eq!(
        read_text(&file),
        written(
            "### T-001 · Parse the config\n- status: open\n- owner: —\n- priority: P1 · difficulty: 4\n"
        )
    );

    // Removing the priority moves the difficulty to its own line.
    ok(
        &home,
        &work,
        &["--agent", "t-agent", "set", "T-001", "--unset", "priority"],
    );
    assert_eq!(
        read_text(&file),
        written("### T-001 · Parse the config\n- status: open\n- owner: —\n- difficulty: 4\n")
    );
}

#[test]
fn show_json_and_schema_list_difficulty() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Parse the config\n- status: open\n- owner: —\n- priority: P1 · difficulty: 4\n";
    let (home, work, _) = setup_proj(&tmp, &doc(body));

    let show = parse_json(&ok(&home, &work, &["--json", "show", "T-001"]).stdout);
    assert_eq!(show["data"]["difficulty"], "4");

    let schema = parse_json(&ok(&home, &work, &["--json", "schema"]).stdout);
    let fields: Vec<&str> = schema["data"]["task"]["fields"]
        .as_array()
        .expect("task fields")
        .iter()
        .filter_map(|f| f.as_str())
        .collect();
    assert!(fields.contains(&"difficulty"), "{fields:?}");
}

#[test]
fn check_warns_about_a_difficulty_that_is_not_configured_as_it_does_for_type_and_area() {
    let tmp = TempDir::new().unwrap();
    let body = "### T-001 · Combined level\n- status: open\n- owner: —\n- priority: P1 · difficulty: extreme\n\n\
                ### T-002 · Own line level\n- status: open\n- owner: —\n- priority: P2\n- difficulty: Huge\n\n\
                ### T-003 · Configured spelling\n- status: open\n- owner: —\n- priority: P2 · difficulty: 4\n";
    let (home, work, _) = setup_proj(&tmp, &doc(body));

    let out = run_todo(&home, &work, &["check"], &[]).unwrap();
    let text = format!("{}{}", stdout_text(&out), stderr_text(&out));
    assert_eq!(out.status.code(), Some(1), "{text}");
    assert!(
        text.contains("T-001: difficulty \"extreme\" is not in configured difficulties"),
        "{text}"
    );
    assert!(
        text.contains("T-002: difficulty \"Huge\" is not in configured difficulties"),
        "{text}"
    );
    assert!(
        !text.contains("T-003:"),
        "a configured difficulty in another case is not reported: {text}"
    );
}
