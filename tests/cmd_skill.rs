//! Integration tests for `todo skill install` (T-054; R-AGENT-5).

use std::fs;

use assert_cmd::cargo::cargo_bin_cmd;
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

    // Without --yes: does not create file.
    let preview = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args(["skill", "install"])
        .output()
        .unwrap();
    assert!(preview.status.success());
    assert!(!skill_path.exists());

    // With --yes: creates SKILL.md containing the same RULES_TEXT as `help agents`.
    let install = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args(["--json", "skill", "install", "--yes"])
        .output()
        .unwrap();
    assert!(install.status.success());
    let json: Value = serde_json::from_slice(&install.stdout).unwrap();
    assert_eq!(json["schema"], "todo.skill.v1");
    assert_eq!(json["data"]["applied"], true);
    assert_eq!(json["data"]["created"], true);

    let content = fs::read_to_string(&skill_path).unwrap();
    assert!(content.starts_with("---\nname: todo\n"), "{content}");
    assert!(
        content.contains(todo::agents::RULES_TEXT.trim()),
        "skill must contain built-in RULES_TEXT"
    );
}
