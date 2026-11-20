//! CLI integration tests for milestones (T-087; R-MILE-1, R-MILE-2, TEST-4): the `goal` and
//! `milestone` fields, placement under a parent, the refusals that leave the file byte-identical,
//! claim/next/done/drop along the tree, `drop --tree`, `apply`, `add --from` and `show --json`.
//! Every fixture is invented.

mod common;

use std::fs;
use std::path::PathBuf;
use std::process::Output;

use tempfile::TempDir;

use common::{parse_json, read_todo, run_todo, setup_proj, stdout_text};

/// Fixed clock for note dates (`TODO_TEST_NOW`, test-hooks feature).
const NOW: &str = "2026-11-18T10:00:00+01:00";

const FIXTURE: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->\n";

/// A three-level tree: the milestone T-001 holds the milestone T-002 and the task T-004; T-002
/// holds the task T-003.
fn tree() -> String {
    format!(
        "{FIXTURE}## Now\n\n\
### T-001 · Develop the renderer\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: milestone\n\
- goal: the test scene renders with lighting at 60 frames per second\n\
- note (2026-11-15, planner): added\n\n\
### T-002 · Lighting\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: milestone\n\
- milestone: T-001\n\
- goal: point and directional lights match the reference images\n\
- note (2026-11-15, planner): added\n\n\
### T-003 · C shader\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: feature · area: cli\n\
- milestone: T-002\n\
- note (2026-11-15, planner): added\n\n\
### T-004 · GPU compiler\n\
- status: open\n\
- owner: —\n\
- priority: P2\n\
- milestone: T-001\n\
- note (2026-11-15, planner): added\n"
    )
}

struct Proj {
    _tmp: TempDir,
    home: PathBuf,
    work: PathBuf,
}

fn proj(body: &str) -> Proj {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo) = setup_proj(&tmp, body);
    Proj {
        _tmp: tmp,
        home,
        work,
    }
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

/// The file after a successful write. These fixtures have no format line, so the first write migrates
/// them (format.md §18): the format line goes first, then the running tool stamp (R-VER-1).
fn stamped(text: &str) -> String {
    format!(
        "<!-- todo-format: 3 -->\n<!-- todo-tool: {} -->\n{text}",
        env!("CARGO_PKG_VERSION")
    )
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

#[test]
fn add_milestone_needs_a_goal_and_a_parent_must_be_a_milestone() {
    let body = format!(
        "{FIXTURE}## Now\n\n\
### T-001 · Develop the renderer\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: milestone\n\
- goal: the test scene renders\n\
- note (2026-11-15, planner): added\n\n\
### T-002 · C shader\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: feature · area: cli\n\
- note (2026-11-15, planner): added\n"
    );
    let p = proj(&body);

    assert_refused(
        &p,
        &[
            "add",
            "--title",
            "Lighting",
            "--type",
            "milestone",
            "--priority",
            "P1",
            "--agent",
            "planner",
        ],
        "MISSING_FIELD",
    );
    assert_refused(
        &p,
        &[
            "add",
            "--title",
            "GPU compiler",
            "--priority",
            "P1",
            "--difficulty",
            "3",
            "--milestone",
            "T-002",
            "--agent",
            "planner",
        ],
        "BAD_VALUE",
    );
    assert_refused(
        &p,
        &[
            "add",
            "--title",
            "GPU compiler",
            "--priority",
            "P1",
            "--difficulty",
            "3",
            "--milestone",
            "T-099",
            "--agent",
            "planner",
        ],
        "NOT_FOUND",
    );
    assert_refused(
        &p,
        &[
            "add",
            "--title",
            "Is it a question?",
            "--type",
            "question",
            "--priority",
            "P1",
            "--difficulty",
            "3",
            "--milestone",
            "T-001",
            "--agent",
            "planner",
        ],
        "BAD_VALUE",
    );

    // A milestone under T-001 with its goal, then a task under the new milestone.
    assert_ok(
        &p,
        &[
            "add",
            "--title",
            "Lighting",
            "--type",
            "milestone",
            "--priority",
            "P1",
            "--goal",
            "point and directional lights match the reference images",
            "--milestone",
            "T-001",
            "--agent",
            "planner",
        ],
    );
    let out = assert_ok(
        &p,
        &[
            "add",
            "--title",
            "Shader compiler",
            "--priority",
            "P1",
            "--difficulty",
            "3",
            "--type",
            "feature",
            "--area",
            "cli",
            "--milestone",
            "T-003",
            "--agent",
            "planner",
        ],
    );
    assert_eq!(stdout_text(&out), "T-004: add ok\n");

    let expected = format!(
        "{body}\n\
### T-003 · Lighting\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: milestone\n\
- milestone: T-001\n\
- goal: point and directional lights match the reference images\n\
- note (2026-11-18, planner): added\n\n\
### T-004 · Shader compiler\n\
- status: open\n\
- owner: —\n\
- priority: P1 · difficulty: 3 · type: feature · area: cli\n\
- milestone: T-003\n\
- note (2026-11-18, planner): added\n"
    );
    assert_eq!(read_todo(&p.work), stamped(&expected));
}

#[test]
fn add_json_lists_the_milestone_chain_top_down() {
    let p = proj(&tree());
    let out = assert_ok(
        &p,
        &[
            "--json",
            "add",
            "--title",
            "Shader compiler",
            "--priority",
            "P2",
            "--difficulty",
            "3",
            "--type",
            "feature",
            "--milestone",
            "T-002",
            "--agent",
            "planner",
        ],
    );
    let json = parse_json(&out.stdout);
    assert_eq!(json["data"]["id"], "T-005");
    assert_eq!(json["data"]["milestone"], "T-002");
    assert_eq!(json["data"]["milestones"][0]["id"], "T-001");
    assert_eq!(json["data"]["milestones"][0]["status"], "open");
    assert_eq!(json["data"]["milestones"][1]["id"], "T-002");
    assert_eq!(
        json["data"]["milestones"][1]["goal"],
        "point and directional lights match the reference images"
    );
    assert!(json["data"]["goal"].is_null());
}

#[test]
fn set_moves_the_subtree_and_unsets_to_top_level() {
    let p = proj(&tree());

    // T-003 moves from T-002 to T-001; T-004 becomes top-level; T-002's goal is replaced.
    assert_ok(
        &p,
        &["set", "T-004", "--unset", "milestone", "--agent", "planner"],
    );
    assert_ok(
        &p,
        &[
            "set",
            "T-002",
            "--goal",
            "lights match the reference images",
            "--agent",
            "planner",
        ],
    );
    let out = assert_ok(
        &p,
        &["set", "T-003", "--milestone", "T-001", "--agent", "planner"],
    );
    assert_eq!(stdout_text(&out), "T-003: set ok\n");

    let expected = format!(
        "{FIXTURE}## Now\n\n\
### T-001 · Develop the renderer\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: milestone\n\
- goal: the test scene renders with lighting at 60 frames per second\n\
- note (2026-11-15, planner): added\n\n\
### T-002 · Lighting\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: milestone\n\
- milestone: T-001\n\
- goal: lights match the reference images\n\
- note (2026-11-15, planner): added\n\n\
### T-003 · C shader\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: feature · area: cli\n\
- milestone: T-001\n\
- note (2026-11-15, planner): added\n\n\
### T-004 · GPU compiler\n\
- status: open\n\
- owner: —\n\
- priority: P2\n\
- note (2026-11-15, planner): added\n"
    );
    assert_eq!(read_todo(&p.work), stamped(&expected));
}

#[test]
fn set_moves_an_entry_with_everything_below_it() {
    let p = proj(&tree());
    assert_ok(
        &p,
        &[
            "add",
            "--title",
            "Renderer tools",
            "--type",
            "milestone",
            "--priority",
            "P2",
            "--goal",
            "tools for the renderer team",
            "--agent",
            "planner",
        ],
    );
    assert_ok(
        &p,
        &["set", "T-002", "--milestone", "T-005", "--agent", "planner"],
    );

    // T-003 is still below T-002, and the chain now runs through T-005.
    let json = parse_json(&run(&p, &["--json", "show", "T-003"]).stdout);
    assert_eq!(json["data"]["milestone"], "T-002");
    assert_eq!(json["data"]["milestones"][0]["id"], "T-005");
    assert_eq!(json["data"]["milestones"][1]["id"], "T-002");

    let top = parse_json(&run(&p, &["--json", "show", "T-001"]).stdout);
    assert_eq!(
        top["data"]["children"],
        serde_json::json!([{"id": "T-004", "title": "GPU compiler", "kind": "task", "status": "open"}])
    );
}

#[test]
fn done_and_drop_of_a_milestone_count_claimed_and_blocked_entries_below() {
    let p = proj(&tree());
    assert_ok(&p, &["claim", "T-003", "--agent", "bob"]);
    let refused = assert_refused(
        &p,
        &["done", "T-002", "--agent", "planner"],
        "BAD_TRANSITION",
    );
    assert_eq!(
        parse_json(&refused.stdout)["error"]["unfinished"],
        serde_json::json!(["T-003"])
    );

    assert_ok(
        &p,
        &[
            "block",
            "T-003",
            "--note",
            "waiting for the scene",
            "--agent",
            "bob",
        ],
    );
    assert_refused(
        &p,
        &["done", "T-002", "--agent", "planner"],
        "BAD_TRANSITION",
    );
    assert_refused(
        &p,
        &["drop", "T-002", "--note", "scope cut", "--agent", "planner"],
        "BAD_TRANSITION",
    );
}

#[test]
fn set_refusals_leave_the_file_byte_identical() {
    let p = proj(&tree());
    // T-002 holds T-003, so it cannot stop being a milestone.
    assert_refused(
        &p,
        &["set", "T-002", "--type", "feature", "--agent", "planner"],
        "BAD_VALUE",
    );
    // T-001 would become its own ancestor through T-002.
    assert_refused(
        &p,
        &["set", "T-001", "--milestone", "T-002", "--agent", "planner"],
        "BAD_VALUE",
    );
    assert_refused(
        &p,
        &["set", "T-001", "--milestone", "T-001", "--agent", "planner"],
        "BAD_VALUE",
    );
    // A task is not a milestone.
    assert_refused(
        &p,
        &["set", "T-003", "--milestone", "T-004", "--agent", "planner"],
        "BAD_VALUE",
    );
    assert_refused(
        &p,
        &["set", "T-003", "--milestone", "T-099", "--agent", "planner"],
        "NOT_FOUND",
    );
    assert_refused(
        &p,
        &["set", "T-003", "--milestone", "", "--agent", "planner"],
        "BAD_VALUE",
    );
    // A question cannot hold a milestone.
    assert_refused(
        &p,
        &["set", "T-004", "--type", "question", "--agent", "planner"],
        "BAD_VALUE",
    );
}

#[test]
fn milestone_in_the_combined_priority_line_is_edited_in_place() {
    let body = format!(
        "{FIXTURE}## Now\n\n\
### T-001 · Develop the renderer\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: milestone\n\
- goal: the test scene renders\n\
- note (2026-11-15, planner): added\n\n\
### T-002 · C shader\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: feature · milestone: T-001 · area: cli\n\
- note (2026-11-15, planner): added\n"
    );
    let p = proj(&body);

    let json = parse_json(&run_json(&p, &["show", "T-002"]).stdout);
    assert_eq!(json["data"]["milestone"], "T-001");
    assert_eq!(json["data"]["milestones"][0]["id"], "T-001");

    assert_ok(
        &p,
        &["set", "T-002", "--milestone", "T-001", "--agent", "planner"],
    );
    assert_ok(
        &p,
        &["set", "T-002", "--unset", "milestone", "--agent", "planner"],
    );
    let expected = body.replace(
        "- priority: P1 · type: feature · milestone: T-001 · area: cli\n",
        "- priority: P1 · type: feature · area: cli\n",
    );
    assert_eq!(read_todo(&p.work), stamped(&expected));
}

#[test]
fn next_and_list_never_hand_out_milestones_and_claim_refuses_them() {
    let p = proj(
        &tree()
            .replace(
                "- priority: P1 · type: feature · area: cli\n",
                "- priority: P1 · difficulty: 2 · type: feature · area: cli\n",
            )
            .replace("- priority: P2\n", "- priority: P2 · difficulty: 2\n"),
    );

    let next = parse_json(&run(&p, &["--json", "next"]).stdout);
    assert_eq!(next["data"]["task"]["id"], "T-003");

    let ready = parse_json(&run(&p, &["--json", "list", "--ready"]).stdout);
    let ids: Vec<&str> = ready["data"]["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["T-003", "T-004"]);

    assert_refused(
        &p,
        &["claim", "T-001", "--agent", "planner"],
        "BAD_TRANSITION",
    );
    assert_refused(
        &p,
        &["claim", "T-002", "--agent", "planner"],
        "BAD_TRANSITION",
    );
}

#[test]
fn done_on_a_milestone_waits_for_the_entries_below_unless_forced() {
    let p = proj(&tree());

    let refused = assert_refused(
        &p,
        &["done", "T-002", "--agent", "planner"],
        "BAD_TRANSITION",
    );
    let unfinished = parse_json(&refused.stdout)["error"]["unfinished"].clone();
    assert_eq!(unfinished, serde_json::json!(["T-003"]));

    let out = assert_ok(
        &p,
        &[
            "done",
            "T-002",
            "--force",
            "--note",
            "lights checked",
            "--agent",
            "planner",
        ],
    );
    assert_eq!(stdout_text(&out), "T-002: done ok\n");

    let expected = format!(
        "{FIXTURE}## Now\n\n\
### T-001 · Develop the renderer\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: milestone\n\
- goal: the test scene renders with lighting at 60 frames per second\n\
- note (2026-11-15, planner): added\n\n\
### T-002 · Lighting\n\
- status: done\n\
- owner: planner\n\
- priority: P1 · type: milestone\n\
- milestone: T-001\n\
- goal: point and directional lights match the reference images\n\
- note (2026-11-15, planner): added\n\
- note (2026-11-18, planner): forced done (1 entries below unfinished)\n\
- note (2026-11-18, planner): done — lights checked\n\n\
### T-003 · C shader\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: feature · area: cli\n\
- milestone: T-002\n\
- note (2026-11-15, planner): added\n\n\
### T-004 · GPU compiler\n\
- status: open\n\
- owner: —\n\
- priority: P2\n\
- milestone: T-001\n\
- note (2026-11-15, planner): added\n"
    );
    assert_eq!(read_todo(&p.work), stamped(&expected));
}

#[test]
fn drop_of_a_milestone_names_drop_tree_and_drop_tree_drops_the_subtree() {
    let p = proj(&tree());

    assert_refused(
        &p,
        &["drop", "T-001", "--note", "scope cut", "--agent", "planner"],
        "BAD_TRANSITION",
    );
    assert_refused(
        &p,
        &[
            "drop",
            "T-003",
            "--tree",
            "--note",
            "scope cut",
            "--agent",
            "planner",
        ],
        "BAD_VALUE",
    );

    let out = assert_ok(
        &p,
        &[
            "drop",
            "T-001",
            "--tree",
            "--note",
            "scope cut",
            "--agent",
            "planner",
        ],
    );
    assert_eq!(
        stdout_text(&out),
        "T-001: drop ok\ndropped below T-001: T-002, T-003, T-004\n"
    );

    let expected = format!(
        "{FIXTURE}## Now\n\n\
### T-001 · Develop the renderer\n\
- status: dropped\n\
- owner: —\n\
- priority: P1 · type: milestone\n\
- goal: the test scene renders with lighting at 60 frames per second\n\
- note (2026-11-15, planner): added\n\
- note (2026-11-18, planner): dropped — scope cut\n\n\
### T-002 · Lighting\n\
- status: dropped\n\
- owner: —\n\
- priority: P1 · type: milestone\n\
- milestone: T-001\n\
- goal: point and directional lights match the reference images\n\
- note (2026-11-15, planner): added\n\
- note (2026-11-18, planner): dropped with T-001 — scope cut\n\n\
### T-003 · C shader\n\
- status: dropped\n\
- owner: —\n\
- priority: P1 · type: feature · area: cli\n\
- milestone: T-002\n\
- note (2026-11-15, planner): added\n\
- note (2026-11-18, planner): dropped with T-001 — scope cut\n\n\
### T-004 · GPU compiler\n\
- status: dropped\n\
- owner: —\n\
- priority: P2\n\
- milestone: T-001\n\
- note (2026-11-15, planner): added\n\
- note (2026-11-18, planner): dropped with T-001 — scope cut\n"
    );
    assert_eq!(read_todo(&p.work), stamped(&expected));
}

#[test]
fn drop_tree_refuses_another_agents_claim_unless_forced() {
    let p = proj(&tree());
    assert_ok(&p, &["claim", "T-003", "--agent", "bob"]);
    let before = read_todo(&p.work);

    assert_refused(
        &p,
        &[
            "drop",
            "T-001",
            "--tree",
            "--note",
            "scope cut",
            "--agent",
            "planner",
        ],
        "NOT_OWNER",
    );
    assert_eq!(read_todo(&p.work), before);

    assert_ok(
        &p,
        &[
            "drop",
            "T-001",
            "--tree",
            "--force",
            "--note",
            "scope cut",
            "--agent",
            "planner",
        ],
    );
    let after = read_todo(&p.work);
    assert!(after.contains("### T-003 · C shader\n- status: dropped\n- owner: bob\n"));
    assert!(after.contains("- note (2026-11-18, planner): forced drop (owner was bob)\n"));
    assert!(after.contains("- note (2026-11-18, planner): dropped with T-001 — scope cut\n"));
}

#[test]
fn show_json_returns_goal_milestone_chain_and_children() {
    let p = proj(&tree());

    let below = parse_json(&run(&p, &["--json", "show", "T-003"]).stdout);
    assert_eq!(below["data"]["milestone"], "T-002");
    assert!(below["data"]["goal"].is_null());
    assert_eq!(below["data"]["milestones"][0]["id"], "T-001");
    assert_eq!(
        below["data"]["milestones"][0]["goal"],
        "the test scene renders with lighting at 60 frames per second"
    );
    assert_eq!(below["data"]["milestones"][1]["id"], "T-002");
    assert_eq!(below["data"]["children"], serde_json::json!([]));

    let top = parse_json(&run(&p, &["--json", "show", "T-001"]).stdout);
    assert_eq!(
        top["data"]["goal"],
        "the test scene renders with lighting at 60 frames per second"
    );
    assert_eq!(top["data"]["milestones"], serde_json::json!([]));
    assert_eq!(
        top["data"]["children"],
        serde_json::json!([
            {"id": "T-002", "title": "Lighting", "kind": "milestone", "status": "open"},
            {"id": "T-004", "title": "GPU compiler", "kind": "task", "status": "open"}
        ])
    );
}

#[test]
fn add_from_accepts_milestone_references_and_refuses_bad_ones_whole() {
    let p = proj(&tree());
    let batch = p.work.join("batch.jsonl");

    fs::write(
        &batch,
        concat!(
            "{\"ref\":\"m\",\"title\":\"Shading\",\"type\":\"milestone\",\"priority\":\"P1\",",
            "\"goal\":\"surfaces shade correctly\",\"milestone\":\"T-001\"}\n",
            "{\"title\":\"Shading pass\",\"priority\":\"P2\",\"difficulty\":\"3\",\"milestone\":\"@m\"}\n",
            "{\"title\":\"Lighting pass\",\"priority\":\"P2\",\"difficulty\":\"3\",\"fields\":{\"milestone\":\"T-002\"}}\n",
        ),
    )
    .unwrap();
    let out = assert_ok(&p, &["add", "--from", "batch.jsonl", "--agent", "planner"]);
    assert_eq!(
        stdout_text(&out),
        "T-005: add ok\nT-006: add ok\nT-007: add ok\n"
    );

    let body = read_todo(&p.work);
    assert!(body.ends_with(concat!(
        "\n### T-005 · Shading\n",
        "- status: open\n",
        "- owner: —\n",
        "- priority: P1 · type: milestone\n",
        "- milestone: T-001\n",
        "- goal: surfaces shade correctly\n",
        "- note (2026-11-18, planner): added\n",
        "\n### T-006 · Shading pass\n",
        "- status: open\n",
        "- owner: —\n",
        "- priority: P2 · difficulty: 3\n",
        "- milestone: T-005\n",
        "- note (2026-11-18, planner): added\n",
        "\n### T-007 · Lighting pass\n",
        "- status: open\n",
        "- owner: —\n",
        "- priority: P2 · difficulty: 3\n",
        "- milestone: T-002\n",
        "- note (2026-11-18, planner): added\n",
    )));

    // Each batch below is refused as a whole, so nothing is written.
    fs::write(
        &batch,
        "{\"title\":\"Lost\",\"priority\":\"P2\",\"difficulty\":\"3\",\"milestone\":\"@nope\"}\n",
    )
    .unwrap();
    let out = run(
        &p,
        &[
            "--json",
            "add",
            "--from",
            "batch.jsonl",
            "--agent",
            "planner",
        ],
    );
    assert_eq!(error_code(&out), "BAD_VALUE");
    let before = read_todo(&p.work);

    for line in [
        "{\"title\":\"Lost\",\"priority\":\"P2\",\"difficulty\":\"3\",\"milestone\":\"T-004\"}\n",
        "{\"title\":\"Lost\",\"type\":\"milestone\",\"priority\":\"P2\"}\n",
        "{\"title\":\"Lost\",\"type\":\"question\",\"priority\":\"P2\",\"milestone\":\"T-001\"}\n",
        "{\"ref\":\"a\",\"title\":\"A\",\"type\":\"milestone\",\"goal\":\"g\",\"priority\":\"P1\",\"milestone\":\"@b\"}\n{\"ref\":\"b\",\"title\":\"B\",\"type\":\"milestone\",\"goal\":\"g\",\"priority\":\"P1\",\"milestone\":\"@a\"}\n",
    ] {
        fs::write(&batch, line).unwrap();
        let out = run(
            &p,
            &[
                "--json",
                "add",
                "--from",
                "batch.jsonl",
                "--agent",
                "planner",
            ],
        );
        assert_eq!(out.status.code(), Some(1), "exit code for {line:?}");
        assert_eq!(read_todo(&p.work), before, "TODO.md changed by {line:?}");
    }
}

#[test]
fn apply_takes_milestone_references_and_refuses_a_cycle_whole() {
    let p = proj(&tree());
    let ops = p.work.join("ops.jsonl");

    fs::write(
        &ops,
        concat!(
            "{\"op\":\"add\",\"ref\":\"m\",\"title\":\"Shading\",\"type\":\"milestone\",\"priority\":\"P1\",",
            "\"goal\":\"surfaces shade correctly\",\"milestone\":\"T-001\"}\n",
            "{\"op\":\"add\",\"ref\":\"t\",\"title\":\"Shading pass\",\"priority\":\"P2\",\"milestone\":\"@m\"}\n",
            "{\"op\":\"set\",\"task\":\"T-004\",\"milestone\":\"@m\"}\n",
        ),
    )
    .unwrap();
    let out = assert_ok(&p, &["apply", "ops.jsonl", "--agent", "planner"]);
    assert_eq!(
        stdout_text(&out),
        "T-005: add ok\nT-006: add ok\nT-004: set ok\n3 operations applied\n"
    );

    let expected = tree().replace(
        "- priority: P2\n- milestone: T-001\n",
        "- priority: P2\n- milestone: T-005\n",
    ) + concat!(
        "\n### T-005 · Shading\n",
        "- status: open\n",
        "- owner: —\n",
        "- priority: P1 · type: milestone\n",
        "- milestone: T-001\n",
        "- goal: surfaces shade correctly\n",
        "- note (2026-11-18, planner): added\n",
        "\n### T-006 · Shading pass\n",
        "- status: open\n",
        "- owner: —\n",
        "- priority: P2\n",
        "- milestone: T-005\n",
        "- note (2026-11-18, planner): added\n",
    );
    assert_eq!(read_todo(&p.work), stamped(&expected));

    // T-001 cannot move under its own descendant: the whole batch is refused.
    fs::write(
        &ops,
        concat!(
            "{\"op\":\"add\",\"title\":\"Another\",\"priority\":\"P2\",\"milestone\":\"T-002\"}\n",
            "{\"op\":\"set\",\"task\":\"T-001\",\"milestone\":\"T-005\"}\n",
        ),
    )
    .unwrap();
    let before = read_todo(&p.work);
    let out = run(&p, &["--json", "apply", "ops.jsonl", "--agent", "planner"]);
    assert_eq!(error_code(&out), "BAD_VALUE");
    assert_eq!(read_todo(&p.work), before);
}
