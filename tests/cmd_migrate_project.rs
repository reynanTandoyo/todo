//! `todo migrate` for the installed project files (`R-MIG-2`): what is listed, what `--yes`
//! updates and what it leaves alone, the summary line, and a second run that changes nothing.
//!
//! Every project is built with the real install commands (`init`, `agents install`, `skill
//! install`, `hooks install`) in a temporary folder, then made out of date by hand.

mod common;

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use common::{parse_json, run_in, stderr_text, stdout_text};

const BEGIN: &str = "<!-- todo:begin -->";
const END: &str = "<!-- todo:end -->";

/// Runs `args` in `dir`, which must succeed, and returns its standard output.
fn ok(dir: &Path, args: &[&str]) -> String {
    let out = run_in(dir, args);
    assert!(
        out.status.success(),
        "{args:?} failed: {}",
        stderr_text(&out)
    );
    stdout_text(&out)
}

/// A project with a `TODO.md`, `AGENTS.md`, `CLAUDE.md`, the Claude Code skill and the hooks, all
/// written by the install commands.
fn install_all(dir: &Path) {
    ok(dir, &["init"]);
    ok(dir, &["agents", "install", "--yes"]);
    ok(dir, &["agents", "install", "--into", "CLAUDE.md", "--yes"]);
    ok(dir, &["skill", "install", "--yes"]);
    ok(dir, &["hooks", "install", "--yes"]);
}

/// `text` with the lines between the rules markers replaced by `body` (LF lines).
fn replace_block(text: &str, body: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed == BEGIN {
            out.push_str(line);
            out.push_str(body);
            out.push('\n');
            inside = true;
        } else if trimmed == END {
            inside = false;
            out.push_str(line);
        } else if !inside {
            out.push_str(line);
        }
    }
    out
}

fn rewrite(dir: &Path, rel: &str, change: impl Fn(&str) -> String) {
    let path = dir.join(rel);
    let text = fs::read_to_string(&path).expect("read file to change");
    fs::write(&path, change(&text)).expect("write changed file");
}

/// Makes the rules blocks of `TODO.md`, `AGENTS.md` and `CLAUDE.md`, the skill, and the
/// `SessionStart` hook out of date.
fn make_outdated(dir: &Path) {
    rewrite(dir, "TODO.md", |t| replace_block(t, "Old rules."));
    rewrite(dir, "AGENTS.md", |t| replace_block(t, "Old rules."));
    rewrite(dir, "CLAUDE.md", |t| replace_block(t, "Older rules."));
    fs::write(dir.join(".claude/skills/todo/SKILL.md"), "old skill\n").expect("write skill");
    rewrite(dir, ".claude/settings.json", |t| {
        t.replace(
            "\"command\": \"todo summary\"",
            "\"command\": \"todo summ\"",
        )
    });
}

/// The bytes of every file that the install commands and `migrate` may touch.
fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
    [
        "TODO.md",
        "AGENTS.md",
        "CLAUDE.md",
        ".claude/skills/todo/SKILL.md",
        ".claude/settings.json",
        ".claude/settings.json.bak",
    ]
    .iter()
    .filter_map(|rel| {
        fs::read(dir.join(rel))
            .ok()
            .map(|b| ((*rel).to_string(), b))
    })
    .collect()
}

#[test]
fn migrate_lists_the_out_of_date_project_files_and_changes_nothing() {
    let dir = TempDir::new().expect("tempdir");
    install_all(dir.path());
    make_outdated(dir.path());
    let before = snapshot(dir.path());

    let out = run_in(dir.path(), &["migrate", "--agent", "t1"]);
    assert!(out.status.success(), "{}", stderr_text(&out));
    let stdout = stdout_text(&out);
    for line in [
        "TODO.md: out of date (agent rules block)",
        "AGENTS.md: out of date (agent rules block)",
        "CLAUDE.md: out of date (agent rules block)",
        ".claude/skills/todo/SKILL.md: out of date (Claude Code skill)",
        ".claude/settings.json: out of date (Claude Code hooks (SessionStart))",
        "run todo migrate --yes to update them",
    ] {
        assert!(
            stdout.lines().any(|l| l == line),
            "missing {line:?} in:\n{stdout}"
        );
    }
    assert_eq!(snapshot(dir.path()), before, "a listing must not write");
}

#[test]
fn migrate_yes_updates_exactly_the_listed_files_and_a_second_run_changes_nothing() {
    let dir = TempDir::new().expect("tempdir");
    install_all(dir.path());
    make_outdated(dir.path());

    let stdout = ok(dir.path(), &["migrate", "--yes", "--agent", "t1"]);
    for line in [
        "updated TODO.md: agent rules block",
        "updated AGENTS.md: agent rules block",
        "updated CLAUDE.md: agent rules block",
        "updated .claude/skills/todo/SKILL.md: Claude Code skill",
        "updated .claude/settings.json: Claude Code hooks (SessionStart)",
    ] {
        assert!(
            stdout.lines().any(|l| l == line),
            "missing {line:?} in:\n{stdout}"
        );
    }

    // The skill is exactly what `skill install` writes.
    let fresh = TempDir::new().expect("tempdir");
    ok(fresh.path(), &["init"]);
    ok(fresh.path(), &["skill", "install", "--yes"]);
    assert_eq!(
        fs::read(dir.path().join(".claude/skills/todo/SKILL.md")).expect("skill"),
        fs::read(fresh.path().join(".claude/skills/todo/SKILL.md")).expect("fresh skill"),
    );

    // Afterwards the checks are clean.
    ok(dir.path(), &["check"]);
    ok(dir.path(), &["agents", "check"]);
    ok(dir.path(), &["agents", "check", "--into", "CLAUDE.md"]);

    // A second run changes nothing and lists nothing.
    let after_first = snapshot(dir.path());
    let second = ok(dir.path(), &["migrate", "--yes", "--agent", "t1"]);
    assert!(!second.contains("updated "), "{second}");
    assert!(!second.contains("out of date"), "{second}");
    assert_eq!(snapshot(dir.path()), after_first);

    // And the summary no longer asks for it.
    let summary = ok(dir.path(), &["summary"]);
    assert!(!summary.contains("todo migrate --yes"), "{summary}");
}

#[test]
fn migrate_yes_never_installs_a_file_that_is_not_there() {
    let dir = TempDir::new().expect("tempdir");
    ok(dir.path(), &["init"]);
    rewrite(dir.path(), "TODO.md", |t| replace_block(t, "Old rules."));
    // A settings file with only the user's own hook: nothing of todo's is in it.
    fs::create_dir_all(dir.path().join(".claude")).expect("mkdir");
    let user_settings = "{\n  \"hooks\": {\n    \"Stop\": [\n      {\n        \"hooks\": [{\"type\": \"command\", \"command\": \"echo done\"}]\n      }\n    ]\n  }\n}\n";
    fs::write(dir.path().join(".claude/settings.json"), user_settings).expect("write settings");
    let before = snapshot(dir.path());

    let stdout = ok(dir.path(), &["migrate", "--yes", "--agent", "t1"]);
    assert!(
        stdout.contains("updated TODO.md: agent rules block"),
        "{stdout}"
    );
    assert!(!dir.path().join("AGENTS.md").exists());
    assert!(!dir.path().join("CLAUDE.md").exists());
    assert!(!dir.path().join(".claude/skills").exists());
    assert!(!dir.path().join(".claude/settings.json.bak").exists());
    assert_eq!(
        fs::read_to_string(dir.path().join(".claude/settings.json")).expect("settings"),
        user_settings,
        "a settings file without todo's hooks is not touched"
    );
    assert_eq!(
        snapshot(dir.path()).len(),
        before.len(),
        "no file is created"
    );
}

#[test]
fn migrate_rewrites_only_the_managed_hook_entries_and_adds_no_missing_event() {
    let dir = TempDir::new().expect("tempdir");
    ok(dir.path(), &["init"]);
    fs::create_dir_all(dir.path().join(".claude")).expect("mkdir");
    // SessionStart and PreToolUse are installed but out of date; SessionEnd is not installed.
    // The user's own hook and the key order outside todo's entries must stay as they are.
    let before = concat!(
        "{\n",
        "  \"permissions\": {\n",
        "    \"allow\": [\"Bash(git status)\"]\n",
        "  },\n",
        "  \"hooks\": {\n",
        "    \"PreToolUse\": [\n",
        "      {\n",
        "        \"_managed_by\": \"todo\",\n",
        "        \"hooks\": [\n",
        "          {\n",
        "            \"command\": \"todo guard\",\n",
        "            \"type\": \"command\"\n",
        "          }\n",
        "        ],\n",
        "        \"matcher\": \"Edit|Write\"\n",
        "      },\n",
        "      {\n",
        "        \"matcher\": \"Bash\",\n",
        "        \"hooks\": [{\"type\": \"command\", \"command\": \"echo \\\"}]\\\"\"}]\n",
        "      }\n",
        "    ],\n",
        "    \"SessionStart\": [\n",
        "      {\n",
        "        \"_managed_by\": \"todo\",\n",
        "        \"hooks\": [\n",
        "          {\n",
        "            \"command\": \"todo summ\",\n",
        "            \"type\": \"command\"\n",
        "          }\n",
        "        ]\n",
        "      }\n",
        "    ]\n",
        "  }\n",
        "}\n"
    );
    fs::write(dir.path().join(".claude/settings.json"), before).expect("write settings");

    let stdout = ok(dir.path(), &["migrate", "--yes", "--agent", "t1"]);
    assert!(
        stdout.contains(
            "updated .claude/settings.json: Claude Code hooks (SessionStart, PreToolUse)"
        ),
        "{stdout}"
    );

    let after = fs::read_to_string(dir.path().join(".claude/settings.json")).expect("settings");
    let expected = before
        .replace(
            "\"command\": \"todo summ\"",
            "\"command\": \"todo summary\"",
        )
        .replace(
            "\"matcher\": \"Edit|Write\"",
            "\"matcher\": \"Edit|Write|MultiEdit\"",
        );
    assert_eq!(after, expected, "only the managed entries may change");
    assert!(
        !after.contains("SessionEnd"),
        "a missing event is not installed"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join(".claude/settings.json.bak")).expect("backup"),
        before,
        "the file is backed up before it changes"
    );
}

#[test]
fn the_rules_block_is_replaced_and_the_bytes_around_it_are_kept() {
    let dir = TempDir::new().expect("tempdir");
    ok(dir.path(), &["init"]);
    let head = "# Agents\r\n\r\nIntro that mentions `TODO.md`.\r\n\r\n";
    let block = format!("{BEGIN}\r\nOld rules.\r\n{END}");
    let tail = "\r\nTrailer line\r\n";
    fs::write(dir.path().join("AGENTS.md"), format!("{head}{block}{tail}"))
        .expect("write AGENTS.md");

    ok(dir.path(), &["migrate", "--yes", "--agent", "t1"]);

    let after = fs::read_to_string(dir.path().join("AGENTS.md")).expect("AGENTS.md");
    assert!(after.starts_with(head), "{after:?}");
    assert!(
        after.ends_with(&format!("{END}\r\nTrailer line\r\n")),
        "{after:?}"
    );
    assert!(!after.contains("Old rules."), "{after:?}");
    assert!(
        !after.replace("\r\n", "").contains('\n'),
        "line endings stay CRLF: {after:?}"
    );
    ok(dir.path(), &["agents", "check"]);
}

#[test]
fn summary_names_migrate_on_its_second_line_while_something_is_out_of_date() {
    let dir = TempDir::new().expect("tempdir");
    install_all(dir.path());
    fs::write(
        dir.path().join(".claude/skills/todo/SKILL.md"),
        "old skill\n",
    )
    .expect("skill");

    let summary = ok(dir.path(), &["summary"]);
    let lines: Vec<&str> = summary.lines().collect();
    assert!(lines.len() <= 40, "{summary}");
    assert_eq!(
        lines.get(1).copied(),
        Some("project files out of date (.claude/skills/todo/SKILL.md): run todo migrate --yes"),
        "{summary}"
    );
}

#[test]
fn dry_run_shows_the_diffs_that_yes_would_write_and_writes_nothing() {
    let dir = TempDir::new().expect("tempdir");
    install_all(dir.path());
    make_outdated(dir.path());
    let before = snapshot(dir.path());

    let stdout = ok(
        dir.path(),
        &["migrate", "--dry-run", "--yes", "--agent", "t1"],
    );
    for header in [
        "+++ b/TODO.md",
        "+++ b/AGENTS.md",
        "+++ b/CLAUDE.md",
        "+++ b/.claude/skills/todo/SKILL.md",
        "+++ b/.claude/settings.json",
    ] {
        assert!(stdout.contains(header), "missing {header} in:\n{stdout}");
    }
    assert_eq!(snapshot(dir.path()), before, "a dry run writes nothing");

    // Without --yes the dry run lists the files and shows no diff of them.
    let listed = ok(dir.path(), &["migrate", "--dry-run", "--agent", "t1"]);
    assert!(
        listed.contains("AGENTS.md: out of date (agent rules block)"),
        "{listed}"
    );
    assert!(!listed.contains("+++ b/AGENTS.md"), "{listed}");
    assert_eq!(snapshot(dir.path()), before);
}

#[test]
fn an_invalid_settings_file_is_a_warning_and_the_other_files_are_still_updated() {
    let dir = TempDir::new().expect("tempdir");
    install_all(dir.path());
    make_outdated(dir.path());
    fs::write(dir.path().join(".claude/settings.json"), "{ not json").expect("settings");

    let out = run_in(dir.path(), &["migrate", "--yes", "--agent", "t1"]);
    assert!(out.status.success(), "{}", stderr_text(&out));
    assert!(
        stderr_text(&out).contains("not valid JSON"),
        "{}",
        stderr_text(&out)
    );
    assert_eq!(
        fs::read_to_string(dir.path().join(".claude/settings.json")).expect("settings"),
        "{ not json",
        "the broken file is left alone"
    );
    ok(dir.path(), &["agents", "check"]);
}

#[test]
fn json_output_lists_each_project_file_with_whether_it_was_updated() {
    let dir = TempDir::new().expect("tempdir");
    install_all(dir.path());
    make_outdated(dir.path());

    let out = run_in(dir.path(), &["--json", "migrate", "--yes", "--agent", "t1"]);
    assert!(out.status.success(), "{}", stderr_text(&out));
    let json = parse_json(&out.stdout);
    let project = json["data"]["project"].as_array().expect("project array");
    let rows: Vec<(String, String, bool)> = project
        .iter()
        .map(|row| {
            (
                row["file"].as_str().unwrap_or_default().to_string(),
                row["kind"].as_str().unwrap_or_default().to_string(),
                row["updated"].as_bool().unwrap_or(false),
            )
        })
        .collect();
    assert_eq!(
        rows,
        vec![
            ("TODO.md".to_string(), "rules_block".to_string(), true),
            ("AGENTS.md".to_string(), "rules_block".to_string(), true),
            ("CLAUDE.md".to_string(), "rules_block".to_string(), true),
            (
                ".claude/skills/todo/SKILL.md".to_string(),
                "skill".to_string(),
                true
            ),
            (
                ".claude/settings.json".to_string(),
                "hooks".to_string(),
                true
            ),
        ]
    );
    assert_eq!(project[4]["events"][0], "SessionStart");
}
