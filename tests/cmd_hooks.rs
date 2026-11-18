//! Integration tests for `todo hooks` and `todo guard` (T-053; R-AGENT-4).

mod common;

use common::{parse_json, read_text};

use std::fs;
use std::io::Write;
use std::process::Stdio;

use serde_json::Value;
use tempfile::TempDir;

/// A minimal `TODO.md`, so that the commands find the project folder.
const TODO_MD: &str = "<!-- todo-format: 2 -->\n\n## Tasks\n";

#[test]
fn hooks_install_and_uninstall_preserve_other_entries_and_formatting() {
    let dir = TempDir::new().unwrap();
    // The settings are in the folder of TODO.md, so there is a TODO.md (R-AGENT-4).
    fs::write(dir.path().join("TODO.md"), TODO_MD).unwrap();
    let claude_dir = dir.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    let settings_path = claude_dir.join("settings.json");

    // Pre-existing settings with 4-space indentation and custom hook.
    let initial = "{\n    \"custom\": true,\n    \"hooks\": {\n        \"PreToolUse\": [\n            {\n                \"matcher\": \"Bash\",\n                \"hooks\": [\n                    {\n                        \"type\": \"command\",\n                        \"command\": \"echo custom\"\n                    }\n                ]\n            }\n        ]\n    }\n}\n";
    fs::write(&settings_path, initial).unwrap();

    // Without --yes: nothing changes.
    let preview = common::assert_todo_bin()
        .current_dir(dir.path())
        .args(["hooks", "install"])
        .output()
        .unwrap();
    assert!(preview.status.success());
    assert_eq!(read_text(&settings_path), initial);

    // With --yes: installs hooks, creates backup, preserves 4-space indentation & custom hook.
    let install = common::assert_todo_bin()
        .current_dir(dir.path())
        .args(["hooks", "install", "--yes"])
        .output()
        .unwrap();
    assert!(install.status.success());
    let backup = claude_dir.join("settings.json.bak");
    assert_eq!(read_text(&backup), initial);

    let after = read_text(&settings_path);
    assert!(after.contains("    \"custom\": true"), "{after}");
    assert!(after.contains("\"command\": \"todo summary\""), "{after}");
    assert!(after.contains("\"command\": \"todo guard\""), "{after}");
    assert!(after.contains("\"command\": \"echo custom\""), "{after}");

    // Uninstall without --yes: nothing changes.
    let un_prev = common::assert_todo_bin()
        .current_dir(dir.path())
        .args(["hooks", "uninstall"])
        .output()
        .unwrap();
    assert!(un_prev.status.success());
    assert_eq!(read_text(&settings_path), after);

    // Uninstall with --yes: removes only todo entries, keeps custom hook.
    let uninstall = common::assert_todo_bin()
        .current_dir(dir.path())
        .args(["hooks", "uninstall", "--yes"])
        .output()
        .unwrap();
    assert!(uninstall.status.success());
    let final_text = read_text(&settings_path);
    assert!(!final_text.contains("todo summary"), "{final_text}");
    assert!(!final_text.contains("todo guard"), "{final_text}");
    assert!(
        final_text.contains("\"command\": \"echo custom\""),
        "{final_text}"
    );
}

#[test]
fn hooks_install_keeps_key_order_and_every_other_byte_of_the_settings() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("TODO.md"), TODO_MD).unwrap();
    let claude_dir = dir.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    let settings_path = claude_dir.join("settings.json");

    // Keys out of alphabetical order, an escape and inline arrays: a parse-and-write would sort
    // the keys and reformat the values.
    let initial = "{\n  \"zeta\": {\"caf\\u00e9\": [1, 2]},\n  \"permissions\": {\"allow\": [\"Bash(git status)\"]},\n  \"env\": {\"B\": \"2\"},\n  \"alpha\": true\n}\n";
    fs::write(&settings_path, initial).unwrap();

    let install = common::assert_todo_bin()
        .current_dir(dir.path())
        .args(["hooks", "install", "--yes"])
        .output()
        .unwrap();
    assert!(install.status.success());
    assert_eq!(read_text(&claude_dir.join("settings.json.bak")), initial);

    // The settings keep their text; the hooks object is added after the last setting.
    let head = initial.strip_suffix("\n}\n").unwrap();
    let after = read_text(&settings_path);
    assert!(after.starts_with(head), "{after}");
    assert!(after.ends_with("\n}\n"), "{after}");
    assert!(after.contains("\"command\": \"todo guard\""), "{after}");
    let value: Value = serde_json::from_str(&after).unwrap();
    assert_eq!(value["zeta"]["café"], serde_json::json!([1, 2]));

    // A second install changes nothing.
    let again = common::assert_todo_bin()
        .current_dir(dir.path())
        .args(["hooks", "install", "--yes"])
        .output()
        .unwrap();
    assert!(again.status.success());
    assert_eq!(read_text(&settings_path), after);
}

#[test]
fn hooks_uninstall_keeps_every_other_byte_of_the_settings() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("TODO.md"), TODO_MD).unwrap();
    let claude_dir = dir.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    let settings_path = claude_dir.join("settings.json");

    // The same settings as the install test: keys out of order, an escape, inline arrays.
    let initial = "{\n  \"zeta\": {\"caf\\u00e9\": [1, 2]},\n  \"permissions\": {\"allow\": [\"Bash(git status)\"]},\n  \"env\": {\"B\": \"2\"},\n  \"alpha\": true\n}\n";
    fs::write(&settings_path, initial).unwrap();

    let install = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args(["hooks", "install", "--yes"])
        .output()
        .unwrap();
    assert!(install.status.success());
    let installed = read_text(&settings_path);
    assert_ne!(installed, initial);

    // Uninstall removes exactly what install added: the original bytes come back.
    let uninstall = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args(["hooks", "uninstall", "--yes"])
        .output()
        .unwrap();
    assert!(uninstall.status.success());
    assert_eq!(read_text(&settings_path), initial);
    assert_eq!(read_text(&claude_dir.join("settings.json.bak")), installed);

    // Nothing is left to remove: a second run changes nothing.
    let again = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args(["hooks", "uninstall", "--yes"])
        .output()
        .unwrap();
    assert!(again.status.success());
    assert_eq!(read_text(&settings_path), initial);
}

#[test]
fn hooks_uninstall_does_not_create_a_settings_file() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("TODO.md"), TODO_MD).unwrap();

    let uninstall = cargo_bin_cmd!("todo")
        .current_dir(dir.path())
        .args(["hooks", "uninstall", "--yes"])
        .output()
        .unwrap();
    assert!(uninstall.status.success());
    assert!(!dir.path().join(".claude").join("settings.json").exists());
}

#[test]
fn guard_blocks_todo_and_archive_unless_leased_and_allows_other_files() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("TODO.md"),
        "<!-- todo-format: 1 -->\n\n## Tasks\n",
    )
    .unwrap();

    // 1. Edit of TODO.md via stdin JSON -> blocked (exit 2) with command suggestion.
    let mut child = common::todo_bin()
        .current_dir(dir.path())
        .arg("guard")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(br#"{"tool_name":"Edit","tool_input":{"file_path":"TODO.md"}}"#)
        .unwrap();
    let blocked = child.wait_with_output().unwrap();
    assert_eq!(blocked.status.code(), Some(2));
    let stderr = String::from_utf8(blocked.stderr).unwrap();
    assert!(stderr.contains("todo "), "{stderr}");

    // 2. Edit of src/main.rs -> allowed (exit 0).
    let mut child2 = common::todo_bin()
        .current_dir(dir.path())
        .arg("guard")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child2
        .stdin
        .as_mut()
        .unwrap()
        .write_all(br#"{"tool_name":"Write","tool_input":{"file_path":"src/main.rs"}}"#)
        .unwrap();
    let other = child2.wait_with_output().unwrap();
    assert_eq!(other.status.code(), Some(0));

    // 3. With a live lease -> TODO.md edit is allowed (exit 0).
    let lock_out = common::assert_todo_bin()
        .current_dir(dir.path())
        .args(["lock", "--agent", "alice"])
        .output()
        .unwrap();
    assert!(lock_out.status.success());

    let leased = common::assert_todo_bin()
        .current_dir(dir.path())
        .args(["--json", "guard", "--path", "TODO.md"])
        .output()
        .unwrap();
    assert_eq!(leased.status.code(), Some(0));
    let json: Value = parse_json(&leased.stdout);
    assert_eq!(json["data"]["allowed"], true);
}
