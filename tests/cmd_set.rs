//! CLI integration tests for `todo set` (T-027; R-WRITE-8, R-CLAIM-3, TEST-4).

mod common;

use std::fs;

use tempfile::TempDir;

use common::{parse_json, read_text, setup_proj, stderr_text, stdout_text};

#[test]
fn set_edits_all_options_combined_priority_field_and_unset() {
    let tmp = TempDir::new().unwrap();
    let initial = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
## Now\n\n\
### T-001 · Initial title\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: feature · area: cli\n\
- fix: old fix\n\
- custom: old custom\n\
\n\
### T-002 · Claimed by alice\n\
- status: claimed\n\
- owner: alice\n\
- claimed: 2026-11-15T10:00+01:00\n\
- priority: P2\n";
    let (home, work, todo_file) = setup_proj(&tmp, initial);
    let now = "2026-11-15T14:00:00+01:00";

    // Update combined priority line in place (--priority + --area, keeping --type), plus all other options.
    let res1 = common::run_todo(
        &home,
        &work,
        &[
            "set",
            "T-001",
            "--agent",
            "bob",
            "--title",
            "Updated title",
            "--priority",
            "P0",
            "--area",
            "ops",
            "--depends",
            "T-002",
            "--files",
            "src/cli/set.rs",
            "--problem",
            "missing set",
            "--accept",
            "all options work",
            "--field",
            "reqs=R-WRITE-8",
            "--field",
            "custom=new custom",
            "--unset",
            "fix",
        ],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert!(res1.status.success(), "stderr: {}", stderr_text(&res1));
    assert_eq!(stdout_text(&res1).trim(), "T-001: set ok");

    let after1 = read_text(&todo_file);
    assert!(
        after1.contains(
            "### T-001 · Updated title\n\
- status: open\n\
- owner: —\n\
- priority: P0 · type: feature · area: ops\n\
- depends: T-002\n\
- files: src/cli/set.rs\n\
- problem: missing set\n\
- accept: all options work\n\
- custom: new custom\n\
- reqs: R-WRITE-8\n"
        ),
        "unexpected file:\n{after1}"
    );
    assert!(!after1.contains("- fix: old fix"));

    // Update --type and --unset custom and --unset area on T-001.
    let res2 = common::run_todo(
        &home,
        &work,
        &[
            "--json", "set", "T-001", "--agent", "bob", "--type", "chore", "--unset", "area",
            "--unset", "custom",
        ],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert!(res2.status.success());
    let j2: serde_json::Value = parse_json(&res2.stdout);
    assert_eq!(j2["schema"], "todo.set.v1");
    assert_eq!(j2["data"]["type"], "chore");
    assert!(j2["data"]["area"].is_null());

    let after2 = read_text(&todo_file);
    assert!(after2.contains("- priority: P0 · type: chore\n"));
    assert!(!after2.contains("- custom:"));

    // Ownership check on T-002 (claimed by alice): bob without --force -> NOT_OWNER; with --force -> forced note.
    let before_refusal = fs::read(&todo_file).unwrap();
    let no_force = common::run_todo(
        &home,
        &work,
        &[
            "--json",
            "set",
            "T-002",
            "--priority",
            "P0",
            "--agent",
            "bob",
        ],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert_eq!(no_force.status.code(), Some(1));
    let j_nf: serde_json::Value = parse_json(&no_force.stdout);
    assert_eq!(j_nf["error"]["code"], "NOT_OWNER");
    assert_eq!(fs::read(&todo_file).unwrap(), before_refusal);

    let forced = common::run_todo(
        &home,
        &work,
        &[
            "set",
            "T-002",
            "--priority",
            "P0",
            "--agent",
            "bob",
            "--force",
        ],
        &[("TODO_TEST_NOW", now)],
    )
    .unwrap();
    assert!(forced.status.success());
    let after_forced = read_text(&todo_file);
    assert!(after_forced.contains("- priority: P0\n"));
    assert!(after_forced.contains("- note (2026-11-15, bob): forced set (owner was alice)\n"));
}

#[test]
fn set_refuses_status_owner_claimed_and_invalid_args() {
    let tmp = TempDir::new().unwrap();
    let initial = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
## Now\n\n\
### T-001 · Task one\n\
- status: open\n\
- owner: —\n\
- priority: P1\n";
    let (home, work, todo_file) = setup_proj(&tmp, initial);
    let before = fs::read(&todo_file).unwrap();

    for key in ["status", "owner", "claimed", "STATUS", "Owner"] {
        let f = common::run_todo(
            &home,
            &work,
            &[
                "--json",
                "set",
                "T-001",
                "--agent",
                "bob",
                "--field",
                &format!("{key}=x"),
            ],
            &[],
        )
        .unwrap();
        assert_eq!(f.status.code(), Some(1), "key={key}");
        let j: serde_json::Value = parse_json(&f.stdout);
        assert_eq!(j["error"]["code"], "BAD_VALUE");

        let u = common::run_todo(
            &home,
            &work,
            &["--json", "set", "T-001", "--agent", "bob", "--unset", key],
            &[],
        )
        .unwrap();
        assert_eq!(u.status.code(), Some(1), "unset key={key}");
        let ju: serde_json::Value = parse_json(&u.stdout);
        assert_eq!(ju["error"]["code"], "BAD_VALUE");
    }

    // No fields specified -> exit 2 (Usage).
    let no_opts = common::run_todo(&home, &work, &["set", "T-001", "--agent", "bob"], &[]).unwrap();
    assert_eq!(no_opts.status.code(), Some(2));

    assert_eq!(fs::read(&todo_file).unwrap(), before);
}

// T-179: a priority is checked as `add` checks it, and a refused value writes nothing.

/// The `set` fixture: one task with a configured priority, in the current format.
const PRIORITY_FIXTURE: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
<!-- todo-format: 3 -->\n\n## Now\n\n\
### T-001 · Lighting\n\
- status: open\n\
- owner: —\n\
- priority: P1 · difficulty: 2\n\
- note (2026-11-15, planner): added\n";

#[test]
fn set_refuses_a_priority_that_is_not_configured_and_writes_nothing() {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo_file) = setup_proj(&tmp, PRIORITY_FIXTURE);
    let before = fs::read(&todo_file).unwrap();

    for args in [
        vec![
            "--json",
            "set",
            "T-001",
            "--priority",
            "P9",
            "--agent",
            "bob",
        ],
        vec![
            "--json",
            "set",
            "T-001",
            "--field",
            "priority=P9",
            "--agent",
            "bob",
        ],
        vec![
            "--json",
            "set",
            "T-001",
            "--priority",
            "P1 · x",
            "--agent",
            "bob",
        ],
    ] {
        let out = common::run_todo(&home, &work, &args, &[]).unwrap();
        assert_eq!(
            out.status.code(),
            Some(1),
            "{args:?}: {}",
            stdout_text(&out)
        );
        let json = parse_json(&out.stdout);
        assert_eq!(json["error"]["code"], "BAD_VALUE", "{args:?}");
        assert_eq!(
            fs::read(&todo_file).unwrap(),
            before,
            "{args:?} changed TODO.md"
        );
    }
}

#[test]
fn set_refuses_an_unconfigured_priority_with_the_message_add_gives() {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo_file) = setup_proj(&tmp, PRIORITY_FIXTURE);
    let before = fs::read(&todo_file).unwrap();

    let set = common::run_todo(
        &home,
        &work,
        &[
            "--json",
            "set",
            "T-001",
            "--priority",
            "P9",
            "--agent",
            "bob",
        ],
        &[],
    )
    .unwrap();
    let add = common::run_todo(
        &home,
        &work,
        &[
            "--json",
            "add",
            "--title",
            "Other",
            "--priority",
            "P9",
            "--difficulty",
            "2",
            "--agent",
            "bob",
        ],
        &[],
    )
    .unwrap();
    let set_error = parse_json(&set.stdout)["error"].clone();
    let add_error = parse_json(&add.stdout)["error"].clone();
    assert_eq!(set_error["code"], add_error["code"]);
    assert_eq!(set_error["message"], add_error["message"]);
    assert_eq!(fs::read(&todo_file).unwrap(), before);
}

#[test]
fn set_writes_a_configured_priority_in_its_configured_spelling() {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo_file) = setup_proj(&tmp, PRIORITY_FIXTURE);

    let out = common::run_todo(
        &home,
        &work,
        &["set", "T-001", "--priority", "p3", "--agent", "bob"],
        &[],
    )
    .unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    let text = read_text(&todo_file);
    assert!(text.contains("- priority: P3 · difficulty: 2\n"), "{text}");
}

#[test]
fn set_checks_the_priority_against_the_configured_priorities() {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo_file) = setup_proj(&tmp, PRIORITY_FIXTURE);
    common::write_toml(&work, "priorities = [\"URGENT\", \"NORMAL\", \"LOW\"]\n");
    let before = fs::read(&todo_file).unwrap();

    // P1 is a priority of the default list, not of this one: refused, with this list named.
    let refused = common::run_todo(
        &home,
        &work,
        &[
            "--json",
            "set",
            "T-001",
            "--priority",
            "P1",
            "--agent",
            "bob",
        ],
        &[],
    )
    .unwrap();
    assert_eq!(refused.status.code(), Some(1));
    let json = parse_json(&refused.stdout);
    assert_eq!(json["error"]["code"], "BAD_VALUE");
    assert_eq!(
        json["error"]["message"],
        "invalid priority 'P1'; expected one of: URGENT, NORMAL, LOW"
    );
    assert_eq!(fs::read(&todo_file).unwrap(), before);

    let ok = common::run_todo(
        &home,
        &work,
        &["set", "T-001", "--priority", "normal", "--agent", "bob"],
        &[],
    )
    .unwrap();
    assert_eq!(ok.status.code(), Some(0), "{}", stderr_text(&ok));
    assert!(read_text(&todo_file).contains("- priority: NORMAL · difficulty: 2\n"));
}
