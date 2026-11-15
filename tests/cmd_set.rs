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
