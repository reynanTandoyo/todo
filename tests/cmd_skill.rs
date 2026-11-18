//! Integration tests for `todo skill install` (T-054; R-AGENT-5).

mod common;

use common::{parse_json, read_text, run_in};

use serde_json::Value;
use tempfile::TempDir;

#[test]
fn skill_install_writes_claude_skill_from_rules_text() {
    let dir = TempDir::new().unwrap();
    let skill_path = dir
        .path()
        .join(".claude")
        .join("skills")
        .join("todo")
        .join("SKILL.md");

    // The skill is in the folder of TODO.md, so there is a TODO.md (R-AGENT-5).
    let init = run_in(dir.path(), &["init"]);
    assert!(init.status.success());

    // Without --yes: does not create file.
    let preview = common::assert_todo_bin()
        .current_dir(dir.path())
        .args(["skill", "install"])
        .output()
        .unwrap();
    assert!(preview.status.success());
    assert!(!skill_path.exists());

    // With --yes: creates SKILL.md containing the same RULES_TEXT as `help agents`.
    let install = common::assert_todo_bin()
        .current_dir(dir.path())
        .args(["--json", "skill", "install", "--yes"])
        .output()
        .unwrap();
    assert!(install.status.success());
    let json: Value = parse_json(&install.stdout);
    assert_eq!(json["schema"], "todo.skill.v1");
    assert_eq!(json["data"]["applied"], true);
    assert_eq!(json["data"]["created"], true);

    let content = read_text(&skill_path);
    assert!(content.starts_with("---\nname: todo\n"), "{content}");
    assert!(
        content.contains(todo::agents::RULES_TEXT.trim()),
        "skill must contain built-in RULES_TEXT"
    );
}
