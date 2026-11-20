//! CLI integration tests for `todo split` (T-156; R-SPLIT-1, R-WRITE-13, R-MILE-1, TEST-4): the
//! task becomes a milestone with its children in one write, the goal, what the task loses, the
//! children's dependencies, the refusals that leave the file byte-identical, `--dry-run`, and the IDs.
//! Every fixture is invented.

mod common;

use std::fs;
use std::path::PathBuf;
use std::process::Output;

use tempfile::TempDir;

use common::{parse_json, read_todo, run_todo, run_with_stdin, setup_proj, stdout_text};

/// Fixed clock for note dates (`TODO_TEST_NOW`, test-hooks feature).
const NOW: &str = "2026-11-18T10:00:00+01:00";

const MARKER: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->";

/// A TODO.md in the current format with the running tool stamp, and `body` under `## Now`.
fn doc(body: &str) -> String {
    format!(
        "{MARKER}\n<!-- todo-format: 3 -->\n<!-- todo-tool: {} -->\n\n## Now\n\n{body}",
        env!("CARGO_PKG_VERSION")
    )
}

/// T-001, the task to split, with every field a task can carry; T-002 waits for it.
const TO_SPLIT: &str = r#"### T-001 · Render the scene
- status: open
- owner: —
- priority: P1 · difficulty: 4 · type: feature · area: cli
- why: the scene shows on screen
- files: src/render/*.rs
- read: docs/render.md
- steps:
  1. draw the floor
  2. draw the cube
- verify: cargo test render
- final: keep the renderer whole
- attempts: 1
- verified: 2026-11-15
- accept: the test scene renders with lighting
- note (2026-11-15, planner): added

### T-002 · Lighting
- status: open
- owner: —
- priority: P1 · difficulty: 3
- depends: T-001
- note (2026-11-15, planner): added
"#;

/// Two children: the second depends on the first by `@ref`.
const CHILDREN: &str = r#"{"title":"Draw the floor","priority":"P1","difficulty":"2","ref":"floor"}
{"title":"Light the cube","priority":"P2","difficulty":"3","depends":"@floor","why":"the cube needs light"}
"#;

struct Proj {
    _tmp: TempDir,
    home: PathBuf,
    work: PathBuf,
}

fn proj(body: &str) -> Proj {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo) = setup_proj(&tmp, &doc(body));
    Proj {
        _tmp: tmp,
        home,
        work,
    }
}

/// Writes `content` as `name` in the project folder and returns its file name.
fn write_file(p: &Proj, name: &str, content: &str) -> String {
    fs::write(p.work.join(name), content).unwrap();
    name.to_string()
}

/// Runs `todo <args>` with the fixed clock.
fn run(p: &Proj, args: &[&str]) -> Output {
    run_todo(&p.home, &p.work, args, &[("TODO_TEST_NOW", NOW)]).unwrap()
}

/// Runs `todo --json <args>` with the fixed clock.
fn run_json(p: &Proj, args: &[&str]) -> Output {
    let mut all = vec!["--json"];
    all.extend_from_slice(args);
    run(p, &all)
}

/// The stable error code of a refused `--json` command.
fn error_code(out: &Output) -> String {
    parse_json(&out.stdout)["error"]["code"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Asserts that `args` succeeds in text mode and returns the output.
fn assert_ok(p: &Proj, args: &[&str]) -> Output {
    let out = run(p, args);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

/// Asserts that `args` is refused with exit 1 and `code`, and that `TODO.md` is byte-identical.
fn assert_refused(p: &Proj, args: &[&str], code: &str) -> Output {
    let before = fs::read(p.work.join("TODO.md")).unwrap();
    let out = run_json(p, args);
    assert_eq!(out.status.code(), Some(1), "exit code for {args:?}");
    assert_eq!(error_code(&out), code, "error code for {args:?}");
    let after = fs::read(p.work.join("TODO.md")).unwrap();
    assert_eq!(before, after, "TODO.md changed by refused {args:?}");
    out
}

#[test]
fn split_turns_the_task_into_a_milestone_with_its_children_in_one_write() {
    let p = proj(TO_SPLIT);
    let children = write_file(&p, "children.jsonl", CHILDREN);

    let out = assert_ok(
        &p,
        &[
            "split",
            "T-001",
            "--from",
            children.as_str(),
            "--agent",
            "planner",
        ],
    );
    assert_eq!(
        stdout_text(&out),
        "T-001: split ok\nsplit into T-003, T-004\n"
    );

    // The task keeps its other lines and its note; the children follow its block, in order, and
    // the file is otherwise as it was. Its goal is its accept line.
    let expected = doc(r#"### T-001 · Render the scene
- status: open
- owner: —
- priority: P1 · type: milestone · area: cli
- goal: the test scene renders with lighting
- why: the scene shows on screen
- accept: the test scene renders with lighting
- note (2026-11-15, planner): added
- note (2026-11-18, planner): split into T-003, T-004

### T-003 · Draw the floor
- status: open
- owner: —
- priority: P1 · difficulty: 2
- milestone: T-001
- note (2026-11-18, planner): added

### T-004 · Light the cube
- status: open
- owner: —
- priority: P2 · difficulty: 3
- milestone: T-001
- why: the cube needs light
- depends: T-003
- note (2026-11-18, planner): added

### T-002 · Lighting
- status: open
- owner: —
- priority: P1 · difficulty: 3
- depends: T-001
- note (2026-11-15, planner): added
"#);
    assert_eq!(read_todo(&p.work), expected);
}

#[test]
fn split_writes_one_snapshot_and_the_json_names_the_task_and_its_children() {
    let p = proj(TO_SPLIT);
    let children = write_file(&p, "children.jsonl", CHILDREN);

    let out = assert_ok(
        &p,
        &[
            "--json",
            "split",
            "T-001",
            "--from",
            children.as_str(),
            "--agent",
            "planner",
        ],
    );
    let json = parse_json(&out.stdout);
    assert_eq!(json["schema"], "todo.split.v1");
    assert_eq!(json["data"]["task"]["id"], "T-001");
    assert_eq!(json["data"]["task"]["type"], "milestone");
    assert_eq!(
        json["data"]["task"]["goal"],
        "the test scene renders with lighting"
    );
    assert_eq!(json["data"]["ids"][0], "T-003");
    assert_eq!(json["data"]["ids"][1], "T-004");
    assert_eq!(json["data"]["count"], 2);
    assert_eq!(json["data"]["added"][1]["id"], "T-004");
    assert_eq!(json["data"]["added"][1]["milestone"], "T-001");

    // One write, one snapshot: the history names the command once.
    let history = assert_ok(&p, &["--json", "history"]);
    let snapshots = parse_json(&history.stdout)["data"]["snapshots"].clone();
    let splits = snapshots
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["command"] == "split")
        .count();
    assert_eq!(splits, 1, "{snapshots}");
}

#[test]
fn split_takes_the_goal_from_the_flag_and_reads_the_children_from_standard_input() {
    let p = proj(TO_SPLIT);
    let out = run_with_stdin(
        &p.work,
        &[
            "split",
            "T-001",
            "--from",
            "-",
            "--goal",
            "the renderer draws the scene",
            "--agent",
            "planner",
        ],
        CHILDREN,
    );
    assert_eq!(out.status.code(), Some(0), "{}", stdout_text(&out));
    let text = read_todo(&p.work);
    assert!(
        text.contains("- goal: the renderer draws the scene\n- why: the scene shows on screen\n"),
        "{text}"
    );
    assert!(!text.contains("- goal: the test scene"), "{text}");
    assert!(text.contains("split into T-003, T-004"), "{text}");
}

#[test]
fn split_needs_a_goal_from_the_flag_or_an_accept_line() {
    let p = proj(
        TO_SPLIT
            .replace("- accept: the test scene renders with lighting\n", "")
            .as_str(),
    );
    let children = write_file(&p, "children.jsonl", CHILDREN);
    let out = assert_refused(
        &p,
        &[
            "split",
            "T-001",
            "--from",
            children.as_str(),
            "--agent",
            "planner",
        ],
        "MISSING_FIELD",
    );
    assert!(
        parse_json(&out.stdout)["error"]["field"] == "goal",
        "{}",
        stdout_text(&out)
    );
}

#[test]
fn split_drops_the_claim_of_the_agent_that_splits_it() {
    let body = TO_SPLIT.replacen(
        "- status: open\n- owner: —\n",
        "- status: claimed\n- owner: planner\n- claimed: 2026-11-15T09:00+01:00\n- session: s-1\n",
        1,
    );
    let p = proj(&body);
    let children = write_file(&p, "children.jsonl", CHILDREN);

    assert_ok(
        &p,
        &[
            "split",
            "T-001",
            "--from",
            children.as_str(),
            "--agent",
            "planner",
        ],
    );
    let text = read_todo(&p.work);
    let block = text.split("### T-003").next().unwrap();
    assert!(
        block.contains("### T-001 · Render the scene\n- status: open\n- owner: —\n"),
        "{block}"
    );
    assert!(!block.contains("claimed:"), "{block}");
    assert!(!block.contains("session:"), "{block}");
}

#[test]
fn split_opens_a_task_that_was_given_back_as_blocked() {
    let body = TO_SPLIT.replacen(
        "- status: open\n- owner: —\n",
        "- status: blocked\n- owner: —\n",
        1,
    ).replace(
        "- note (2026-11-15, planner): added\n\n",
        "- note (2026-11-15, planner): added\n- note (2026-11-16, worker): stuck — the lighting pass will not compile\n\n",
    );
    let p = proj(&body);
    let children = write_file(&p, "children.jsonl", CHILDREN);

    assert_ok(
        &p,
        &[
            "split",
            "T-001",
            "--from",
            children.as_str(),
            "--agent",
            "planner",
        ],
    );
    let text = read_todo(&p.work);
    let block = text.split("### T-003").next().unwrap();
    assert!(block.contains("- status: open\n"), "{block}");
    assert!(!block.contains("- status: blocked"), "{block}");
    assert!(
        block.contains("- note (2026-11-16, worker): stuck — the lighting pass will not compile\n"),
        "{block}"
    );
}

#[test]
fn split_keeps_a_task_under_its_own_milestone_and_its_children_under_the_task() {
    let body = format!(
        "### T-000 · Renderer
- status: open
- owner: —
- priority: P1 · type: milestone
- goal: the renderer draws scenes
- note (2026-11-15, planner): added

{}",
        TO_SPLIT.replace(
            "- priority: P1 · difficulty: 4 · type: feature · area: cli
",
            "- priority: P1 · difficulty: 4 · type: feature · area: cli
- milestone: T-000
",
        )
    );
    let p = proj(&body);
    let children = write_file(&p, "children.jsonl", CHILDREN);

    assert_ok(
        &p,
        &[
            "split",
            "T-001",
            "--from",
            children.as_str(),
            "--agent",
            "planner",
        ],
    );
    let text = read_todo(&p.work);
    assert!(
        text.contains("- priority: P1 · type: milestone · area: cli\n- milestone: T-000\n"),
        "{text}"
    );
    assert!(text.contains("### T-003 · Draw the floor\n- status: open\n- owner: —\n- priority: P1 · difficulty: 2\n- milestone: T-001\n"), "{text}");
}

#[test]
fn tasks_that_depended_on_the_split_task_wait_for_the_whole_milestone() {
    let p = proj(TO_SPLIT);
    let children = write_file(&p, "children.jsonl", CHILDREN);
    assert_ok(
        &p,
        &[
            "split",
            "T-001",
            "--from",
            children.as_str(),
            "--agent",
            "planner",
        ],
    );

    // T-002 names the milestone in `depends`: it waits until the milestone is done, not for T-001 alone.
    let out = assert_ok(&p, &["--json", "show", "T-002"]);
    assert_eq!(parse_json(&out.stdout)["data"]["ready"], false);

    // Finish the children, then the milestone: only then is T-002 ready.
    assert_ok(&p, &["done", "T-003", "--agent", "planner"]);
    assert_ok(&p, &["done", "T-004", "--agent", "planner"]);
    let done = assert_ok(&p, &["done", "T-001", "--agent", "planner"]);
    assert!(
        stdout_text(&done).contains("now ready: T-002 Lighting"),
        "{}",
        stdout_text(&done)
    );
}

#[test]
fn split_children_take_the_next_ids_after_the_highest_one() {
    let p = proj(TO_SPLIT);
    let children = write_file(&p, "children.jsonl", CHILDREN);
    assert_ok(
        &p,
        &[
            "split",
            "T-001",
            "--from",
            children.as_str(),
            "--agent",
            "planner",
        ],
    );
    let out = assert_ok(
        &p,
        &[
            "--json",
            "add",
            "--title",
            "Probe task",
            "--priority",
            "P3",
            "--difficulty",
            "1",
            "--agent",
            "planner",
        ],
    );
    assert_eq!(parse_json(&out.stdout)["data"]["id"], "T-005");
}

#[test]
fn split_dry_run_prints_the_diff_and_writes_nothing() {
    let p = proj(TO_SPLIT);
    let children = write_file(&p, "children.jsonl", CHILDREN);
    let before = fs::read(p.work.join("TODO.md")).unwrap();

    let out = assert_ok(
        &p,
        &[
            "split",
            "T-001",
            "--from",
            children.as_str(),
            "--agent",
            "planner",
            "--dry-run",
        ],
    );
    let text = stdout_text(&out);
    assert!(text.contains("--- a/TODO.md"), "{text}");
    assert!(text.contains("+### T-003 · Draw the floor"), "{text}");
    assert!(text.contains("+- milestone: T-001"), "{text}");
    assert!(text.contains("split into T-003, T-004"), "{text}");
    assert_eq!(before, fs::read(p.work.join("TODO.md")).unwrap());

    // The dry run took no IDs either.
    assert_eq!(next_add_id_text(&p), "T-003");
}

/// The ID a probe `add` would get, read from its text output.
fn next_add_id_text(p: &Proj) -> String {
    let out = assert_ok(
        p,
        &[
            "add",
            "--title",
            "Probe task",
            "--priority",
            "P3",
            "--difficulty",
            "1",
            "--agent",
            "planner",
        ],
    );
    stdout_text(&out).split(':').next().unwrap().to_string()
}

#[test]
fn split_refuses_a_done_dropped_milestone_question_or_idea_and_changes_nothing() {
    let body = "### T-001 · Finished\n- status: done\n- owner: planner\n- priority: P1 · difficulty: 2\n- note (2026-11-15, planner): added\n\n\
### T-002 · Abandoned\n- status: dropped\n- owner: —\n- priority: P1 · difficulty: 2\n- note (2026-11-15, planner): added\n\n\
### T-003 · Renderer\n- status: open\n- owner: —\n- priority: P1 · type: milestone\n- goal: the renderer draws scenes\n- note (2026-11-15, planner): added\n\n\
### T-004 · Should shadows be soft?\n- status: open\n- owner: —\n- type: question\n- note (2026-11-15, planner): added\n\n\
### T-005 · Cloud shaders\n- status: open\n- owner: —\n- type: idea\n- note (2026-11-15, planner): added\n";
    let p = proj(body);
    let children = write_file(&p, "children.jsonl", CHILDREN);
    let args = |id: &str| {
        vec![
            "split".to_string(),
            id.to_string(),
            "--from".to_string(),
            children.clone(),
            "--goal".to_string(),
            "g".to_string(),
            "--agent".to_string(),
            "planner".to_string(),
        ]
    };
    for (id, code) in [
        ("T-001", "BAD_TRANSITION"),
        ("T-002", "BAD_TRANSITION"),
        ("T-003", "BAD_VALUE"),
        ("T-004", "BAD_VALUE"),
        ("T-005", "BAD_VALUE"),
    ] {
        let owned = args(id);
        let refs: Vec<&str> = owned.iter().map(String::as_str).collect();
        assert_refused(&p, &refs, code);
    }
}

#[test]
fn split_refuses_a_task_claimed_by_another_agent_unless_forced() {
    let body = TO_SPLIT.replacen(
        "- status: open\n- owner: —\n",
        "- status: claimed\n- owner: alice\n- claimed: 2026-11-15T09:00+01:00\n",
        1,
    );
    let p = proj(&body);
    let children = write_file(&p, "children.jsonl", CHILDREN);

    assert_refused(
        &p,
        &[
            "split",
            "T-001",
            "--from",
            children.as_str(),
            "--agent",
            "bob",
        ],
        "NOT_OWNER",
    );

    assert_ok(
        &p,
        &[
            "split",
            "T-001",
            "--from",
            children.as_str(),
            "--agent",
            "bob",
            "--force",
        ],
    );
    let text = read_todo(&p.work);
    let block = text.split("### T-003").next().unwrap();
    assert!(
        block.contains("- note (2026-11-18, bob): forced split (owner was alice)\n- note (2026-11-18, bob): split into T-003, T-004\n"),
        "{block}"
    );
    assert!(block.contains("- status: open\n- owner: —\n"), "{block}");
    assert!(!block.contains("claimed:"), "{block}");
}

#[test]
fn split_refuses_a_child_that_names_a_milestone_a_section_or_no_level_and_changes_nothing() {
    let p = proj(TO_SPLIT);
    // (children, error code, the start of the message)
    let cases = [
        (
            r#"{"title":"Draw the floor","priority":"P1","difficulty":"2"}
{"title":"Light the cube","priority":"P2","difficulty":"3","milestone":"T-002"}"#,
            "BAD_VALUE",
            "--from line 2: a child names no milestone",
        ),
        (
            r#"{"title":"Draw the floor","priority":"P1","difficulty":"2","section":"Now"}"#,
            "BAD_VALUE",
            "--from line 1: a child takes the section",
        ),
        (
            r#"{"title":"Should it shine?","type":"question"}"#,
            "BAD_VALUE",
            "--from line 1: a question is not part of the milestone tree",
        ),
        (
            r#"{"title":"No level","priority":"P1"}"#,
            "MISSING_FIELD",
            "missing required field `difficulty`",
        ),
    ];
    for (index, (lines, code, message)) in cases.iter().enumerate() {
        let children = write_file(&p, &format!("bad-{index}.jsonl"), lines);
        let out = assert_refused(
            &p,
            &[
                "split",
                "T-001",
                "--from",
                children.as_str(),
                "--goal",
                "g",
                "--agent",
                "planner",
            ],
            code,
        );
        let text = parse_json(&out.stdout)["error"]["message"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(text.starts_with(message), "{text}");
    }
}

#[test]
fn split_is_all_or_nothing_and_leaves_the_ids_alone() {
    let p = proj(TO_SPLIT);
    // The second child names a dependency that does not exist: nothing is written, and the next
    // task still gets T-003.
    let bad_ref = write_file(
        &p,
        "dangling.jsonl",
        "{\"title\":\"Draw the floor\",\"priority\":\"P1\",\"difficulty\":\"2\",\"ref\":\"floor\"}\n\
         {\"title\":\"Light the cube\",\"priority\":\"P2\",\"difficulty\":\"3\",\"depends\":\"@nope\"}\n",
    );
    assert_refused(
        &p,
        &[
            "split",
            "T-001",
            "--from",
            bad_ref.as_str(),
            "--goal",
            "g",
            "--agent",
            "planner",
        ],
        "BAD_VALUE",
    );

    // A child whose title another active task has is refused the same way.
    let clash = write_file(
        &p,
        "clash.jsonl",
        "{\"title\":\"Lighting\",\"priority\":\"P1\",\"difficulty\":\"2\"}\n",
    );
    assert_refused(
        &p,
        &[
            "split",
            "T-001",
            "--from",
            clash.as_str(),
            "--goal",
            "g",
            "--agent",
            "planner",
        ],
        "DUPLICATE_TITLE",
    );

    assert_eq!(next_add_id_text(&p), "T-003");
}

#[test]
fn split_by_any_agent_when_the_task_has_no_owner() {
    let p = proj(TO_SPLIT);
    let children = write_file(&p, "children.jsonl", CHILDREN);
    assert_ok(
        &p,
        &[
            "split",
            "T-001",
            "--from",
            children.as_str(),
            "--agent",
            "carol",
        ],
    );
    assert!(read_todo(&p.work).contains("- note (2026-11-18, carol): split into T-003, T-004\n"));
}
