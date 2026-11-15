mod common;

use std::fs;

use serde_json::Value;
use tempfile::TempDir;

use common::todo_bin;

#[test]
fn init_creates_valid_todo_that_passes_check_and_refuses_overwrite() {
    let dir = TempDir::new().expect("tempdir");

    let out = todo_bin()
        .current_dir(dir.path())
        .args(["init", "--title", "Sample Project"])
        .output()
        .expect("run init");
    assert!(
        out.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("<!-- todo:begin -->"));
    assert!(stdout.contains("<!-- todo:end -->"));

    let created = fs::read_to_string(dir.path().join("TODO.md")).expect("read TODO.md");
    assert!(created.starts_with("<!-- todo-format: 1 -->"));
    assert!(created.contains("# Sample Project"));
    assert!(created.contains("<!-- todo:begin -->"));
    assert!(created.contains("<!-- todo:end -->"));
    assert!(created.contains("## Tasks"));

    // Created file must pass `todo check`.
    let check = todo_bin()
        .current_dir(dir.path())
        .args(["check"])
        .output()
        .expect("run check");
    assert!(
        check.status.success(),
        "check failed on newly initialized TODO.md: {}",
        String::from_utf8_lossy(&check.stdout)
    );

    // Modify TODO.md and verify `todo init` refuses and never overwrites it.
    let sentinel = "<!-- custom content -->\n";
    fs::write(dir.path().join("TODO.md"), sentinel).expect("write sentinel");

    let second = todo_bin()
        .current_dir(dir.path())
        .args(["init", "--json"])
        .output()
        .expect("run second init");
    assert_eq!(second.status.code(), Some(4));
    let json: Value = serde_json::from_slice(&second.stdout).expect("parse init error json");
    assert_eq!(json["ok"], false);
    assert_eq!(json["error"]["code"], "UNSAFE_FILE");

    assert_eq!(
        fs::read_to_string(dir.path().join("TODO.md")).expect("read TODO.md after"),
        sentinel
    );
}
