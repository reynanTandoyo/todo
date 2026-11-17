//! Tests for `todo scan` and `todo scan --add` (I-11).

mod common;

use std::fs;
use std::path::Path;
use std::process::Output;

use common::{
    clean_todo_command, git, parse_json, read_todo, stderr_text, stdout_text, write_todo,
};
use tempfile::TempDir;

const TODO_HEAD: &str = "<!-- todo-format: 1 -->\n\n## Now\n\n### T-001 · Existing thing\n- status: open\n- owner: —\n- priority: P2\n";

/// Creates `TODO.md` with `body` and returns `(home, work)`: the test folder and a `home` folder.
fn project(tmp: &TempDir, body: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let home = tmp.path().join("home");
    let work = tmp.path().join("work");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&work).expect("create work");
    write_todo(&work, body);
    (home, work)
}

fn write_file(dir: &Path, rel: &str, content: &str) {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create folder");
    }
    fs::write(path, content).expect("write file");
}

/// Runs `todo` in `work` with a clean environment (no `TODO_AGENT`) and the given arguments.
fn todo(home: &Path, work: &Path, args: &[&str]) -> Output {
    clean_todo_command(home, work)
        .args(args)
        .output()
        .expect("run todo")
}

#[test]
fn scan_without_git_lists_new_markers_and_skips_noise() {
    let tmp = TempDir::new().unwrap();
    let (home, work) = project(&tmp, TODO_HEAD);
    write_file(
        &work,
        "src/a.rs",
        "// TODO: fix parser\nlet x = 1; // MYTODO: no\n/* FIXME: slow path */\n<!-- todo: lower no -->\n// TODO:   \n",
    );
    write_file(&work, "src/sub/b.py", "x\n# TODO: in sub -->\n");
    write_file(&work, "target/t.rs", "TODO: target skip\n");
    write_file(&work, "node_modules/x/n.js", "TODO: node skip\n");
    write_file(&work, ".hidden/h.txt", "TODO: hidden skip\n");
    write_file(&work, "notes.txt", "FIXME: root file\n");
    fs::write(work.join("src/bin.dat"), b"\xff\xfeTODO: not utf8\n").unwrap();
    let mut huge = String::from("// TODO: huge file\n");
    huge.push_str(&"x".repeat(1_000_001));
    write_file(&work, "big.rs", &huge);
    write_file(&work, "docs/archive.md", "TODO: keep me\n");

    let out = todo(&home, &work, &["scan"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));
    let text = stdout_text(&out);
    assert_eq!(
        text,
        "docs/archive.md:1: TODO: keep me\n\
         notes.txt:1: FIXME: root file\n\
         src/a.rs:1: TODO: fix parser\n\
         src/a.rs:3: FIXME: slow path\n\
         src/sub/b.py:2: TODO: in sub\n"
    );
}

#[test]
fn scan_skips_comments_that_name_a_task_or_repeat_a_title() {
    let tmp = TempDir::new().unwrap();
    let (home, work) = project(&tmp, TODO_HEAD);
    write_file(
        &work,
        "src/a.rs",
        "# TODO: T-001 mentions a task\n// TODO: EXISTING thing\n// TODO: new one\n",
    );

    let out = todo(&home, &work, &["scan"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));
    assert_eq!(stdout_text(&out), "src/a.rs:3: TODO: new one\n");
}

#[test]
fn scan_never_reports_todo_md_or_the_archive() {
    let tmp = TempDir::new().unwrap();
    let (home, work) = project(
        &tmp,
        "<!-- todo-format: 1 -->\n\n## Now\n\n### T-001 · Own, TODO: check this\n- status: open\n- owner: —\n- priority: P2\n",
    );
    write_file(&work, "TODO-archive.md", "TODO: archived text\n");

    let out = todo(&home, &work, &["scan"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));
    assert_eq!(stdout_text(&out), "no new TODO: or FIXME: comments found\n");
}

#[test]
fn scan_json_uses_the_versioned_envelope() {
    let tmp = TempDir::new().unwrap();
    let (home, work) = project(&tmp, TODO_HEAD);
    write_file(&work, "src/a.rs", "// TODO: one\n");

    let out = todo(&home, &work, &["--json", "scan"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));
    let json = parse_json(&out.stdout);
    assert_eq!(json["schema"], "todo.scan.v1");
    assert_eq!(json["ok"], true);
    assert_eq!(json["data"]["count"], 1);
    assert_eq!(json["data"]["found"][0]["path"], "src/a.rs");
    assert_eq!(json["data"]["found"][0]["line"], 1);
    assert_eq!(json["data"]["found"][0]["text"], "TODO: one");
    assert!(json["data"].get("added").is_none());
}

#[test]
fn scan_add_writes_all_tasks_in_one_locked_write() {
    let tmp = TempDir::new().unwrap();
    let (home, work) = project(&tmp, TODO_HEAD);
    write_file(&work, "src/a.rs", "// TODO: first\n// FIXME: second\n");

    let out = todo(
        &home,
        &work,
        &[
            "--agent",
            "scanner",
            "scan",
            "--add",
            "--section",
            "Now",
            "--priority",
            "P1",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));
    assert_eq!(
        stdout_text(&out),
        "T-002: add ok  src/a.rs:1\nT-003: add ok  src/a.rs:2\n"
    );

    let todo_md = read_todo(&work);
    assert!(todo_md.contains("### T-002 · first\n"), "{todo_md}");
    assert!(
        todo_md.contains("- priority: P1\n- files: src/a.rs:1\n"),
        "{todo_md}"
    );
    assert!(todo_md.contains("### T-003 · second\n"), "{todo_md}");
    assert!(todo_md.contains("- files: src/a.rs:2\n"), "{todo_md}");

    // The added comments are known now, so a second scan finds nothing.
    let again = todo(&home, &work, &["scan"]);
    assert_eq!(
        stdout_text(&again),
        "no new TODO: or FIXME: comments found\n"
    );
}

#[test]
fn scan_add_json_reports_the_task_ids() {
    let tmp = TempDir::new().unwrap();
    let (home, work) = project(&tmp, TODO_HEAD);
    write_file(&work, "src/a.rs", "// TODO: only one\n");

    let out = todo(
        &home,
        &work,
        &[
            "--json",
            "--agent",
            "scanner",
            "scan",
            "--add",
            "--priority",
            "P0",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));
    let json = parse_json(&out.stdout);
    assert_eq!(json["schema"], "todo.scan.v1");
    assert_eq!(json["data"]["count"], 1);
    assert_eq!(json["data"]["added"][0], "T-002");
}

#[test]
fn scan_add_dry_run_writes_nothing() {
    let tmp = TempDir::new().unwrap();
    let (home, work) = project(&tmp, TODO_HEAD);
    write_file(&work, "src/a.rs", "// TODO: would add\n");
    let before = read_todo(&work);

    let out = todo(
        &home,
        &work,
        &[
            "--dry-run",
            "--agent",
            "scanner",
            "scan",
            "--add",
            "--priority",
            "P1",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));
    assert_eq!(read_todo(&work), before);
    assert!(!work.join(".todo").join("ids.json").exists());
}

#[test]
fn scan_add_needs_an_agent_name() {
    let tmp = TempDir::new().unwrap();
    let (home, work) = project(&tmp, TODO_HEAD);
    write_file(&work, "src/a.rs", "// TODO: needs agent\n");

    let out = todo(
        &home,
        &work,
        &["--json", "scan", "--add", "--priority", "P1"],
    );
    assert_eq!(out.status.code(), Some(2));
    let json = parse_json(&out.stdout);
    assert_eq!(json["error"]["code"], "NO_AGENT");
    assert_eq!(read_todo(&work), TODO_HEAD);
}

#[test]
fn scan_add_cuts_titles_to_120_characters() {
    let tmp = TempDir::new().unwrap();
    let (home, work) = project(&tmp, TODO_HEAD);
    let long = format!("TODO: {}", "ä".repeat(200));
    write_file(&work, "src/a.rs", &format!("{long}\n"));

    let out = todo(
        &home,
        &work,
        &["--agent", "scanner", "scan", "--add", "--priority", "P1"],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));
    let todo_md = read_todo(&work);
    let heading = todo_md
        .lines()
        .find(|l| l.starts_with("### T-002 · "))
        .expect("new heading");
    let title = heading.trim_start_matches("### T-002 · ");
    assert_eq!(title.chars().count(), 120);
    assert!(
        title.starts_with("ää"),
        "the title is the text after the marker"
    );
    assert!(todo_md.contains("- files: src/a.rs:1\n"));
}

#[test]
fn scan_add_skips_a_second_comment_with_the_same_title() {
    let tmp = TempDir::new().unwrap();
    let (home, work) = project(&tmp, TODO_HEAD);
    write_file(&work, "a.rs", "// TODO: same\n");
    write_file(&work, "b.rs", "// todo: same\n// TODO: same\n");

    let out = todo(
        &home,
        &work,
        &["--agent", "scanner", "scan", "--add", "--priority", "P1"],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));
    let text = stdout_text(&out);
    assert!(text.contains("T-002: add ok  a.rs:1"), "{text}");
    assert!(!text.contains("T-003"), "{text}");
    assert!(
        stderr_text(&out).contains("b.rs:2"),
        "{}",
        stderr_text(&out)
    );
}

#[test]
fn scan_section_and_priority_need_add() {
    let tmp = TempDir::new().unwrap();
    let (home, work) = project(&tmp, TODO_HEAD);

    let out = todo(&home, &work, &["scan", "--priority", "P1"]);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn scan_uses_git_ls_files_when_git_is_available() {
    let tmp = TempDir::new().unwrap();
    let (home, work) = project(&tmp, TODO_HEAD);
    git(&work, &["init", "-q"]);
    write_file(&work, ".gitignore", "ignored.rs\n");
    write_file(&work, "tracked.rs", "// TODO: tracked one\n");
    write_file(&work, "ignored.rs", "// TODO: ignored one\n");
    write_file(&work, "untracked.rs", "// TODO: untracked one\n");
    write_file(&work, "sp ace/ñ.rs", "// FIXME: unicode path\n");
    git(&work, &["add", ".gitignore", "tracked.rs", "sp ace"]);

    let out = todo(&home, &work, &["scan"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));
    assert_eq!(
        stdout_text(&out),
        "sp ace/ñ.rs:1: FIXME: unicode path\n\
         tracked.rs:1: TODO: tracked one\n\
         untracked.rs:1: TODO: untracked one\n"
    );
}

#[test]
fn scan_in_a_git_repo_covers_only_the_folder_holding_todo_md() {
    let tmp = TempDir::new().unwrap();
    let (home, work) = project(&tmp, TODO_HEAD);
    git(&work, &["init", "-q"]);
    write_file(&work, "top.rs", "// TODO: outside the task folder\n");
    let sub = work.join("sub");
    fs::create_dir_all(&sub).expect("create sub");
    write_todo(&sub, TODO_HEAD);
    write_file(&sub, "inner.rs", "// TODO: inside the task folder\n");

    let out = todo(&home, &sub, &["scan"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));
    assert_eq!(
        stdout_text(&out),
        "inner.rs:1: TODO: inside the task folder\n"
    );
}
