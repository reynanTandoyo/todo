//! Integration tests for `todo search` (T-046; R-READ-6).

mod common;

use common::parse_json;

use std::fs;

use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn search_finds_text_in_titles_fields_and_notes_and_archive() {
    let dir = TempDir::new().unwrap();
    let home = dir.path().join("home");
    let work = dir.path().join("work");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&work).unwrap();

    fs::write(
        work.join("TODO.md"),
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
         <!-- todo-format: 1 -->\n\n\
         ## Tasks\n\n\
         ### T-001 · Alpha parser fix\n\
         - status: open\n\
         - priority: P1\n\
         - files: src/lexer.rs\n\
         - note (2026-11-15, alice): checked tokenizer\n\n\
         ### T-002 · Beta task\n\
         - status: blocked\n\
         - priority: P2\n\
         - problem: fails on UTF-8 BOM\n\
         - note (2026-11-15, bob): blocked — waiting on parser team\n",
    )
    .unwrap();

    fs::write(
        work.join("TODO-archive.md"),
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
         <!-- todo-format: 1 -->\n\n\
         ## Tasks\n\n\
         ### T-099 · Archived parser work\n\
         - status: done\n\
         - priority: P1\n\
         - owner: alice\n\
         - note (2026-11-14, alice): done — shipped\n",
    )
    .unwrap();

    // Search in TODO.md: matches title of T-001 and note of T-002.
    let out = cargo_bin_cmd!("todo")
        .current_dir(&work)
        .env("HOME", &home)
        .env("APPDATA", &home)
        .args(["search", "PARSER"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("T-001"), "{stdout}");
    assert!(stdout.contains("T-002"), "{stdout}");
    assert!(!stdout.contains("T-099"), "{stdout}");
    assert!(stdout.contains("2 matches in 2 tasks"), "{stdout}");

    // Search in field value.
    let field_out = cargo_bin_cmd!("todo")
        .current_dir(&work)
        .env("HOME", &home)
        .env("APPDATA", &home)
        .args(["--json", "search", "UTF-8 BOM"])
        .output()
        .unwrap();
    assert!(field_out.status.success());
    let json: Value = parse_json(&field_out.stdout);
    assert_eq!(json["schema"], "todo.search.v1");
    assert_eq!(json["data"]["count"], 1);
    assert_eq!(json["data"]["matches"][0]["id"], "T-002");
    assert_eq!(json["data"]["matches"][0]["field"], "problem");

    // Search in archive.
    let arch_out = cargo_bin_cmd!("todo")
        .current_dir(&work)
        .env("HOME", &home)
        .env("APPDATA", &home)
        .args(["--json", "search", "parser", "--archive"])
        .output()
        .unwrap();
    assert!(arch_out.status.success());
    let arch_json: Value = parse_json(&arch_out.stdout);
    assert_eq!(arch_json["data"]["archive"], true);
    assert_eq!(arch_json["data"]["count"], 1);
    assert_eq!(arch_json["data"]["matches"][0]["id"], "T-099");
}
