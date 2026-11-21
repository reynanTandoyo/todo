//! Tests for `todo.toml` configuration loading and wiring (T-007; R-CFG-1, R-CFG-2).

mod common;

use std::fs;

use serde_json::Value;
use tempfile::TempDir;

use common::{git, parse_json, read_todo, stderr_text, write_todo, write_toml};
use todo::find::WorktreeMode;
use todo::ops::OpsConfig;

#[test]
fn all_fifteen_keys_parse_and_have_expected_defaults() {
    let def = OpsConfig::default();
    assert_eq!(def.id_config.prefix, "T");
    assert_eq!(def.id_config.width, 3);
    assert_eq!(def.priorities, vec!["P0", "P1", "P2", "P3"]);
    assert_eq!(def.required_fields, vec!["priority", "difficulty"]);
    assert!(def.types.is_empty());
    assert!(def.areas.is_empty());
    assert_eq!(def.default_section, None);
    assert_eq!(def.question_section, "Questions");
    assert_eq!(def.archive_file, "TODO-archive.md");
    assert_eq!(def.claim_ttl_hours, 24);
    assert_eq!(def.lock.lock_wait_seconds, 30);
    assert_eq!(def.lock.lease_minutes, 10);
    assert_eq!(def.lock.lease_max_minutes, 60);
    assert_eq!(def.snapshots_keep, 200);
    assert_eq!(def.worktree, WorktreeMode::Shared);

    let full = r#"
id_prefix = "REQ"
id_width = 4
priorities = ["URGENT", "NORMAL", "LOW"]
required_fields = ["priority", "type"]
types = ["feature", "bug"]
areas = ["cli", "docs"]
default_section = "Backlog"
question_section = "Open Questions"
archive_file = "archive-2026.md"
claim_ttl_hours = 12
lock_wait_seconds = 5
lease_minutes = 15
lease_max_minutes = 45
snapshots_keep = 50
worktree = "local"
"#;
    let cfg = OpsConfig::from_toml_str(full).unwrap();
    assert_eq!(cfg.id_config.prefix, "REQ");
    assert_eq!(cfg.id_config.width, 4);
    assert_eq!(cfg.priorities, vec!["URGENT", "NORMAL", "LOW"]);
    assert_eq!(cfg.required_fields, vec!["priority", "type"]);
    assert_eq!(cfg.types, vec!["feature", "bug"]);
    assert_eq!(cfg.areas, vec!["cli", "docs"]);
    assert_eq!(cfg.default_section.as_deref(), Some("Backlog"));
    assert_eq!(cfg.question_section, "Open Questions");
    assert_eq!(cfg.archive_file, "archive-2026.md");
    assert_eq!(cfg.claim_ttl_hours, 12);
    assert_eq!(cfg.lock.lock_wait_seconds, 5);
    assert_eq!(cfg.lock.lease_minutes, 15);
    assert_eq!(cfg.lock.lease_max_minutes, 45);
    assert_eq!(cfg.snapshots_keep, 50);
    assert_eq!(cfg.worktree, WorktreeMode::Local);
}

#[test]
fn unknown_key_and_bad_values_report_todo_toml_line_and_message() {
    let dir = TempDir::new().unwrap();
    write_todo(dir.path(), "<!-- todo:format=1 -->\n\n## Tasks\n");

    // Unknown key on line 3.
    write_toml(
        dir.path(),
        "# comment\nid_prefix = \"T\"\nunknown_setting = 123\n",
    );
    let out = common::assert_todo_bin()
        .current_dir(dir.path())
        .args(["list"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let stderr = stderr_text(&out);
    assert!(
        stderr.contains("todo.toml:3:"),
        "expected todo.toml:3: in stderr, got: {stderr}"
    );
    assert!(stderr.contains("unknown_setting"), "stderr: {stderr}");

    // Bad value on line 2.
    write_toml(dir.path(), "id_prefix = \"T\"\nworktree = \"bogus\"\n");
    let out = common::assert_todo_bin()
        .current_dir(dir.path())
        .args(["--json", "list"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let json: Value = parse_json(&out.stdout);
    assert_eq!(json["ok"], false);
    assert_eq!(json["error"]["code"], "BAD_VALUE");
    assert_eq!(json["error"]["file"], "todo.toml");
    assert_eq!(json["error"]["line"], 2);

    // Syntax error on line 2.
    write_toml(dir.path(), "id_prefix = \"T\"\nid_width = [unclosed\n");
    let err = OpsConfig::load_next_to(&dir.path().join("TODO.md")).unwrap_err();
    assert!(err.to_string().starts_with("todo.toml:2:"), "got: {}", err);
}

#[test]
fn cli_wires_prefix_width_priorities_required_types_areas_default_section_archive_ttl_and_lease() {
    let dir = TempDir::new().unwrap();
    write_todo(
        dir.path(),
        "<!-- todo:format=1 -->\n\n## Inbox\n\n## Other\n",
    );
    fs::write(
        dir.path().join("custom-archive.md"),
        "<!-- todo:format=1 -->\n\n## Archive\n\n### REQ-0005 · Archived item\n- status: done\n- owner: alice\n- priority: HIGH\n",
    )
    .unwrap();
    write_toml(
        dir.path(),
        r#"
id_prefix = "REQ"
id_width = 4
priorities = ["HIGH", "LOW"]
required_fields = ["priority", "type", "area"]
types = ["bug", "feat"]
areas = ["cli", "core"]
default_section = "Inbox"
archive_file = "custom-archive.md"
claim_ttl_hours = 1
lease_minutes = 5
lease_max_minutes = 15
"#,
    );

    // Missing required field `type` is refused.
    let missing = common::assert_todo_bin()
        .current_dir(dir.path())
        .args([
            "--agent",
            "alice",
            "add",
            "--title",
            "Missing type",
            "--priority",
            "HIGH",
            "--difficulty",
            "3",
            "--area",
            "cli",
        ])
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(1));

    // Invalid `type` is refused.
    let bad_type = common::assert_todo_bin()
        .current_dir(dir.path())
        .args([
            "--agent",
            "alice",
            "add",
            "--title",
            "Bad type",
            "--priority",
            "HIGH",
            "--difficulty",
            "3",
            "--type",
            "chore",
            "--area",
            "cli",
        ])
        .output()
        .unwrap();
    assert_eq!(bad_type.status.code(), Some(1));

    // Invalid `area` is refused.
    let bad_area = common::assert_todo_bin()
        .current_dir(dir.path())
        .args([
            "--agent",
            "alice",
            "add",
            "--title",
            "Bad area",
            "--priority",
            "HIGH",
            "--difficulty",
            "3",
            "--type",
            "bug",
            "--area",
            "ui",
        ])
        .output()
        .unwrap();
    assert_eq!(bad_area.status.code(), Some(1));

    // Valid add without `--section` uses `default_section = "Inbox"`, `REQ-0006` (after archived REQ-0005).
    let added = common::assert_todo_bin()
        .current_dir(dir.path())
        .args([
            "--json",
            "--agent",
            "alice",
            "add",
            "--title",
            "Valid task",
            "--priority",
            "HIGH",
            "--difficulty",
            "3",
            "--type",
            "bug",
            "--area",
            "cli",
        ])
        .output()
        .unwrap();
    assert_eq!(
        added.status.code(),
        Some(0),
        "stderr: {}",
        stderr_text(&added)
    );
    let added_json: Value = parse_json(&added.stdout);
    assert_eq!(added_json["data"]["id"], "REQ-0006");
    assert_eq!(added_json["data"]["section"], "Inbox");

    // `list --archive` reads `custom-archive.md`.
    let arch = common::assert_todo_bin()
        .current_dir(dir.path())
        .args(["--json", "list", "--archive"])
        .output()
        .unwrap();
    assert_eq!(arch.status.code(), Some(0));
    let arch_json: Value = parse_json(&arch.stdout);
    assert_eq!(arch_json["data"]["tasks"][0]["id"], "REQ-0005");

    // `lock` defaults to `lease_minutes = 5` and rejects `--minutes 20` (> `lease_max_minutes = 15`).
    let too_long = common::assert_todo_bin()
        .current_dir(dir.path())
        .args(["--agent", "alice", "lock", "--minutes", "20"])
        .output()
        .unwrap();
    assert_eq!(too_long.status.code(), Some(1));

    let locked = common::assert_todo_bin()
        .current_dir(dir.path())
        .args(["--json", "--agent", "alice", "lock"])
        .output()
        .unwrap();
    assert_eq!(locked.status.code(), Some(0));
    let locked_json: Value = parse_json(&locked.stdout);
    assert_eq!(locked_json["data"]["minutes"], 5);

    let _ = common::assert_todo_bin()
        .current_dir(dir.path())
        .args(["--agent", "alice", "unlock"])
        .output()
        .unwrap();

    // `claim_ttl_hours = 1`: a claim from 2 hours ago is stale.
    let todo_text = read_todo(dir.path());
    let stale_claim = todo_text
        .replace("- status: open", "- status: claimed")
        .replace(
            "- owner: —",
            "- owner: bob\n- claimed: 2020-01-01T00:00+00:00",
        );
    write_todo(dir.path(), &stale_claim);
    let stale_list = common::assert_todo_bin()
        .current_dir(dir.path())
        .args(["--json", "list", "--stale"])
        .output()
        .unwrap();
    assert_eq!(stale_list.status.code(), Some(0));
    let stale_json: Value = parse_json(&stale_list.stdout);
    assert_eq!(stale_json["data"]["count"], 1);
}

#[test]
fn worktree_local_in_todo_toml_uses_worktrees_own_todo() {
    let tmp = TempDir::new().unwrap();
    let main_dir = tmp.path().join("main");
    let wt_dir = tmp.path().join("wt");
    fs::create_dir_all(&main_dir).unwrap();

    git(&main_dir, &["init", "-q"]);
    git(&main_dir, &["config", "user.email", "test@example.com"]);
    git(&main_dir, &["config", "user.name", "Test"]);
    write_todo(
        &main_dir,
        "<!-- todo:format=1 -->\n\n## Tasks\n\n### T-001 · Main task\n- status: open\n- owner: —\n- priority: P1\n",
    );
    git(&main_dir, &["add", "TODO.md"]);
    git(&main_dir, &["commit", "-m", "init", "-q"]);
    git(
        &main_dir,
        &[
            "worktree",
            "add",
            "-b",
            "wt-branch",
            wt_dir.to_str().unwrap(),
        ],
    );

    // Put a local TODO.md + todo.toml (worktree = "local") in the linked worktree.
    write_todo(
        &wt_dir,
        "<!-- todo:format=1 -->\n\n## Tasks\n\n### T-099 · Local worktree task\n- status: open\n- owner: —\n- priority: P0\n",
    );
    write_toml(&wt_dir, "worktree = \"local\"\n");

    let out = common::assert_todo_bin()
        .current_dir(&wt_dir)
        .args(["--json", "list"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let json: Value = parse_json(&out.stdout);
    assert_eq!(json["data"]["tasks"][0]["id"], "T-099");
}

#[test]
fn verify_settings_have_their_defaults_and_bad_values_are_refused_with_the_line() {
    // T-158 (R-VERIFY-1, R-VERIFY-2): the defaults of the verify settings.
    let def = OpsConfig::default();
    assert!(def.verify_enabled);
    assert_eq!(def.verify_when, todo::ops::VerifyWhen::Done);
    assert_eq!(def.verify_timeout_seconds, 300);
    assert_eq!(def.verify_attempts, 3);
    assert!(def.verify_shell.is_empty());

    let set = OpsConfig::from_toml_str(
        "verify = false\nverify_when = \"wave\"\nverify_timeout_seconds = 5\nverify_attempts = 2\nverify_shell = [\"sh\", \"-c\"]\n",
    )
    .unwrap();
    assert!(!set.verify_enabled);
    assert_eq!(set.verify_when, todo::ops::VerifyWhen::Wave);
    assert_eq!(set.verify_timeout_seconds, 5);
    assert_eq!(set.verify_attempts, 2);
    assert_eq!(set.verify_shell, vec!["sh", "-c"]);

    // Each bad value is refused, and the error names its line in todo.toml.
    for (text, line) in [
        ("verify = 1\n", 1),
        ("verify_when = \"later\"\n", 1),
        ("\nverify_timeout_seconds = 0\n", 2),
        ("verify_attempts = 0\n", 1),
        ("verify_shell = \"sh\"\n", 1),
    ] {
        let err = OpsConfig::from_toml_str(text).unwrap_err();
        assert_eq!(err.code, todo::error::ErrorCode::BadValue, "{text:?}");
        assert_eq!(err.details.line, Some(line), "{text:?}: {}", err.message);
    }
}

#[test]
fn detail_settings_have_their_defaults_and_bad_values_are_refused_with_the_line() {
    // T-153 (R-DETAIL-2, R-CFG-2): the target level and the file limit of the detail rule.
    let def = OpsConfig::default();
    assert_eq!(def.target_rank(), 0, "the lowest level by default");
    assert_eq!(def.level1_max_files, 3);
    assert!(def.todo_folder.is_none(), "not loaded from a file");

    // Without its own target, a project's target is the first of its difficulties.
    let own = OpsConfig::from_toml_str("difficulties = [\"easy\", \"hard\"]\n").unwrap();
    assert_eq!(own.target_rank(), 0);
    assert!(own.target_set.is_none());

    // A target is kept in the spelling of `difficulties`, whatever the case written.
    let raised = OpsConfig::from_toml_str(
        "difficulties = [\"easy\", \"hard\"]\ntarget_difficulty = \"HARD\"\nlevel1_max_files = 5\n",
    )
    .unwrap();
    assert_eq!(raised.target_rank(), 1);
    assert_eq!(raised.target_set.as_deref(), Some("hard"));
    assert_eq!(raised.level1_max_files, 5);

    // Each bad value is refused, and the error names its line in todo.toml.
    for (text, line) in [
        ("target_difficulty = \"6\"\n", 1),
        (
            "difficulties = [\"easy\"]\n\ntarget_difficulty = \"medium\"\n",
            3,
        ),
        ("target_difficulty = 2\n", 1),
        ("level1_max_files = 0\n", 1),
        ("\nlevel1_max_files = \"3\"\n", 2),
    ] {
        let err = OpsConfig::from_toml_str(text).unwrap_err();
        assert_eq!(err.code, todo::error::ErrorCode::BadValue, "{text:?}");
        assert_eq!(err.details.line, Some(line), "{text:?}: {}", err.message);
    }
}

#[test]
fn load_next_to_names_the_folder_of_todo_md_for_the_read_paths() {
    // T-153 (R-DETAIL-2): `read` paths are relative to the folder that holds TODO.md.
    let dir = TempDir::new().unwrap();
    let cfg = OpsConfig::load_next_to(&dir.path().join("TODO.md")).unwrap();
    assert_eq!(cfg.todo_folder.as_deref(), Some(dir.path()));
}

#[test]
fn description_is_one_line_and_a_blank_or_missing_value_is_none() {
    // T-201 (R-CFG-2, R-REPORT-1): one sentence about the project, kept on one line.
    assert_eq!(OpsConfig::default().description, None);

    // Without the key, the description is None.
    let missing = OpsConfig::from_toml_str("id_prefix = \"T\"\n").unwrap();
    assert_eq!(missing.description, None);

    // A set value is kept as written, with non-ASCII text, and surrounding spaces are trimmed.
    let set = OpsConfig::from_toml_str("description = \"  Ein Werkzeug für Aufgabenlisten.  \"\n")
        .unwrap();
    assert_eq!(
        set.description.as_deref(),
        Some("Ein Werkzeug für Aufgabenlisten.")
    );

    // The value is also read from todo.toml next to TODO.md.
    let dir = TempDir::new().unwrap();
    write_toml(
        dir.path(),
        "description = \"Shared by agents and people.\"\n",
    );
    let from_file = OpsConfig::load_next_to(&dir.path().join("TODO.md")).unwrap();
    assert_eq!(
        from_file.description.as_deref(),
        Some("Shared by agents and people.")
    );

    // An empty or blank value is None.
    for text in ["description = \"\"\n", "description = \"   \"\n"] {
        let blank = OpsConfig::from_toml_str(text).unwrap();
        assert_eq!(blank.description, None, "{text:?}");
    }

    // Each line break becomes a single space, whether written as an escape or as a real line
    // break. A CRLF is one line break, so it gives one space too (format.md §10).
    let escaped = OpsConfig::from_toml_str(r#"description = "first line\nsecond\rthird""#).unwrap();
    assert_eq!(
        escaped.description.as_deref(),
        Some("first line second third")
    );
    let multi =
        OpsConfig::from_toml_str("description = \"\"\"first line\nsecond line\"\"\"\n").unwrap();
    assert_eq!(multi.description.as_deref(), Some("first line second line"));
    let crlf = OpsConfig::from_toml_str(r#"description = "first line\r\nsecond line""#).unwrap();
    assert_eq!(crlf.description.as_deref(), Some("first line second line"));
    let multi_crlf =
        OpsConfig::from_toml_str("description = \"\"\"first line\r\nsecond line\"\"\"\n").unwrap();
    assert_eq!(
        multi_crlf.description.as_deref(),
        Some("first line second line")
    );

    // A value that is not a string is the usual todo.toml error, with its line.
    let err = OpsConfig::from_toml_str("id_prefix = \"T\"\ndescription = 42\n").unwrap_err();
    assert_eq!(err.code, todo::error::ErrorCode::BadValue);
    assert_eq!(err.details.line, Some(2));
    assert!(
        err.message.contains("description must be a string"),
        "got: {}",
        err.message
    );
}
