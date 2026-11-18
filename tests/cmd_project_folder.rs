//! The project folder of the agent commands (`R-AGENT-3`): `agents check`, `agents install`,
//! `skill install` and `hooks install` use the folder that holds `TODO.md`, whatever the current
//! folder is (T-127).

mod common;

use std::fs;

use tempfile::TempDir;

use common::{run_in, stderr_text, stdout_text};

#[test]
fn agent_commands_run_from_a_subfolder_write_to_the_folder_of_todo_md() {
    let tmp = TempDir::new().expect("tempdir");
    let project = tmp.path().join("project");
    let sub = project.join("src").join("deep");
    fs::create_dir_all(&sub).expect("subfolder");
    let init = run_in(&project, &["init"]);
    assert!(init.status.success(), "init failed: {}", stderr_text(&init));

    for args in [
        &["agents", "install", "--yes"][..],
        &["agents", "install", "--into", "CLAUDE.md", "--yes"][..],
        &["skill", "install", "--yes"][..],
        &["hooks", "install", "--yes"][..],
    ] {
        let out = run_in(&sub, args);
        assert!(
            out.status.success(),
            "{args:?} failed: {}",
            stderr_text(&out)
        );
    }

    assert!(project.join("AGENTS.md").is_file());
    assert!(project.join("CLAUDE.md").is_file());
    assert!(project.join(".claude/skills/todo/SKILL.md").is_file());
    assert!(project.join(".claude/settings.json").is_file());
    assert!(!sub.join("AGENTS.md").exists());
    assert!(!sub.join("CLAUDE.md").exists());
    assert!(!sub.join(".claude").exists());

    // The check reads the project's files from the subfolder as well.
    let current = run_in(&sub, &["agents", "check"]);
    assert!(current.status.success(), "{}", stdout_text(&current));

    fs::write(
        project.join("AGENTS.md"),
        "<!-- todo:begin -->\nold rules\n<!-- todo:end -->\n",
    )
    .expect("outdated AGENTS.md");
    let outdated = run_in(&sub, &["agents", "check"]);
    assert_eq!(outdated.status.code(), Some(1));
    let stdout = stdout_text(&outdated);
    assert!(
        stdout.contains("AGENTS.md:1: outdated agent rules block"),
        "{stdout}"
    );
}

#[test]
fn agent_commands_without_a_todo_md_stop_and_write_nothing_in_the_current_folder() {
    let tmp = TempDir::new().expect("tempdir");
    let folder = tmp.path().join("bare");
    fs::create_dir_all(&folder).expect("folder");

    // The explicit TODO.md does not exist, so there is no project folder to write to.
    for args in [
        &["agents", "install", "--yes"][..],
        &["skill", "install", "--yes"][..],
        &["hooks", "install", "--yes"][..],
    ] {
        let mut full = vec!["--file", "TODO.md"];
        full.extend_from_slice(args);
        let out = run_in(&folder, &full);
        assert!(!out.status.success(), "{args:?} should be refused");
    }

    assert!(!folder.join("AGENTS.md").exists());
    assert!(!folder.join(".claude").exists());
}
