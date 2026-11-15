mod common;

use std::fs;

use serde_json::Value;
use tempfile::TempDir;

use common::todo_bin;

#[test]
fn agents_install_and_check_preserve_surrounding_bytes_and_report_missing_or_outdated() {
    let dir = TempDir::new().expect("tempdir");
    let agents_path = dir.path().join("AGENTS.md");

    // 1. Missing file -> agents check exits 1.
    let missing_check = todo_bin()
        .current_dir(dir.path())
        .args(["agents", "check"])
        .output()
        .expect("run agents check on missing file");
    assert_eq!(missing_check.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&missing_check.stdout).contains("missing"));

    // 2. Existing file with CRLF and no block -> agents check exits 1.
    let prefix = "# Repo Rules\r\n\r\nKeep this exact CRLF header.\r\n";
    fs::write(&agents_path, prefix).expect("write AGENTS.md");
    let no_block_check = todo_bin()
        .current_dir(dir.path())
        .args(["agents", "check"])
        .output()
        .expect("run agents check without block");
    assert_eq!(no_block_check.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&no_block_check.stdout).contains("missing agent rules block"));

    // 3. Preview without --yes makes no changes.
    let preview = todo_bin()
        .current_dir(dir.path())
        .args(["agents", "install"])
        .output()
        .expect("run agents install preview");
    assert!(preview.status.success());
    assert!(String::from_utf8_lossy(&preview.stdout).contains("pass --yes"));
    assert_eq!(fs::read(&agents_path).expect("read"), prefix.as_bytes());

    // 4. Install --yes adds block while preserving prefix bytes.
    let install = todo_bin()
        .current_dir(dir.path())
        .args(["agents", "install", "--yes"])
        .output()
        .expect("run agents install --yes");
    assert!(
        install.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&install.stderr)
    );
    let installed_bytes = fs::read(&agents_path).expect("read installed");
    assert!(
        installed_bytes.starts_with(prefix.as_bytes()),
        "prefix bytes were altered!"
    );

    // 5. Check now passes with exit 0.
    let ok_check = todo_bin()
        .current_dir(dir.path())
        .args(["agents", "check", "--json"])
        .output()
        .expect("run agents check ok");
    assert!(ok_check.status.success());
    let ok_json: Value = serde_json::from_slice(&ok_check.stdout).expect("parse json");
    assert_eq!(ok_json["data"]["status"], "current");

    // 6. Outdated block in CLAUDE.md with preserved prefix + suffix CRLF bytes.
    let claude_path = dir.path().join("CLAUDE.md");
    let claude_prefix = "# Claude Rules\r\n\r\n<!-- todo:begin -->\r\n";
    let claude_suffix = "<!-- todo:end -->\r\n\r\n## Footer\r\nUntouched CRLF footer.\r\n";
    let outdated = format!("{claude_prefix}Old rules.\r\n{claude_suffix}");
    fs::write(&claude_path, outdated.as_bytes()).expect("write CLAUDE.md");

    let outdated_check = todo_bin()
        .current_dir(dir.path())
        .args(["agents", "check", "--into", "CLAUDE.md"])
        .output()
        .expect("run agents check outdated");
    assert_eq!(outdated_check.status.code(), Some(1));
    let outdated_stdout = String::from_utf8_lossy(&outdated_check.stdout);
    assert!(
        outdated_stdout.contains("CLAUDE.md:3: outdated agent rules block"),
        "stdout: {outdated_stdout}"
    );

    // 7. Install --into CLAUDE.md --yes replaces only between markers, keeping prefix & suffix byte-for-byte.
    let update = todo_bin()
        .current_dir(dir.path())
        .args(["agents", "install", "--into", "CLAUDE.md", "--yes"])
        .output()
        .expect("run agents install --into CLAUDE.md --yes");
    assert!(update.status.success());
    let updated_bytes = fs::read(&claude_path).expect("read CLAUDE.md");
    assert!(
        updated_bytes.starts_with(claude_prefix.as_bytes()),
        "prefix bytes before <!-- todo:begin --> changed!"
    );
    assert!(
        updated_bytes.ends_with(claude_suffix.as_bytes()),
        "suffix bytes after <!-- todo:end --> changed!"
    );

    let recheck = todo_bin()
        .current_dir(dir.path())
        .args(["agents", "check", "--into", "CLAUDE.md"])
        .output()
        .expect("recheck CLAUDE.md");
    assert!(recheck.status.success());
}
