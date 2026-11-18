//! Integration tests for `todo merge-driver` (T-076; R-GIT-2, R-GIT-3).

mod common;

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::Output;

use common::{FIXTURE_MARKER, git, todo_bin};
use tempfile::TempDir;

fn task(id: u32, title: &str, extra: &[&str], notes: &[&str]) -> String {
    let mut s = format!("### T-{id:03} · {title}\n- status: open\n- owner: —\n");
    for e in extra {
        s.push_str(&format!("- {e}\n"));
    }
    for n in notes {
        s.push_str(&format!("- note (2026-11-15, {n}\n"));
    }
    s
}

/// A file with a Now section holding `tasks` (each separated by a blank line).
fn file(tasks: &[String]) -> String {
    let mut s = format!("{FIXTURE_MARKER}\n# Demo\n\n## Now\n\n");
    s.push_str(&tasks.join("\n"));
    s.push_str("\n## Later\n\nprose\n");
    s
}

struct Run {
    out: Output,
    result: String,
}

fn run_driver(dir: &Path, base: &str, ours: &str, theirs: &str) -> Run {
    let (b, o, t) = (dir.join("b.md"), dir.join("o.md"), dir.join("t.md"));
    fs::write(&b, base).unwrap();
    fs::write(&o, ours).unwrap();
    fs::write(&t, theirs).unwrap();
    let out = todo_bin()
        .current_dir(dir)
        .args(["merge-driver", "run"])
        .args([&b, &o, &t])
        .arg("TODO.md")
        .output()
        .unwrap();
    let result = String::from_utf8(fs::read(&o).unwrap()).unwrap();
    Run { out, result }
}

fn code(r: &Run) -> i32 {
    r.out.status.code().unwrap()
}

#[test]
fn tasks_changed_on_different_sides_merge_cleanly() {
    let d = TempDir::new().unwrap();
    let t1 = task(1, "One", &["priority: P2"], &["a): added"]);
    let t2 = task(2, "Two", &[], &["a): added"]);
    let base = file(&[t1, t2]);
    let ours = base.replacen(
        "- status: open\n- owner: —\n",
        "- status: claimed\n- owner: x\n",
        1,
    );
    let theirs = base.replacen(
        "### T-002 · Two\n- status: open\n- owner: —\n",
        "### T-002 · Two\n- status: open\n- owner: —\n- area: cli\n",
        1,
    );
    let r = run_driver(d.path(), &base, &ours, &theirs);
    assert_eq!(code(&r), 0, "{}", String::from_utf8_lossy(&r.out.stderr));
    assert!(
        r.result
            .contains("- status: claimed\n- owner: x\n- priority: P2")
    );
    assert!(
        r.result
            .contains("### T-002 · Two\n- status: open\n- owner: —\n- area: cli\n")
    );
    assert!(!r.result.contains("<<<<<<<"));
}

#[test]
fn different_fields_of_one_task_merge_and_notes_keep_ours_first() {
    let d = TempDir::new().unwrap();
    let base = file(&[task(1, "One", &["priority: P2"], &["a): added"])]);
    let ours = base.replace("- status: open", "- status: blocked").replace(
        "(2026-11-15, a): added\n",
        "(2026-11-15, a): added\n- note (2026-11-16, ours): o1\n",
    );
    let theirs = base.replace("priority: P2", "priority: P1").replace(
        "(2026-11-15, a): added\n",
        "(2026-11-15, a): added\n- note (2026-11-16, theirs): t1\n",
    );
    let r = run_driver(d.path(), &base, &ours, &theirs);
    assert_eq!(code(&r), 0, "{}", r.result);
    let want = "### T-001 · One\n- status: blocked\n- owner: —\n- priority: P1\n- note (2026-11-15, a): added\n- note (2026-11-16, ours): o1\n- note (2026-11-16, theirs): t1\n";
    assert!(r.result.contains(want), "{}", r.result);
}

#[test]
fn same_field_changed_differently_conflicts_around_the_block() {
    let d = TempDir::new().unwrap();
    let base = file(&[task(1, "One", &[], &[]), task(2, "Two", &[], &[])]);
    let ours = base.replacen("- owner: —", "- owner: alice", 1);
    let theirs = base.replacen("- owner: —", "- owner: bob", 1);
    let r = run_driver(d.path(), &base, &ours, &theirs);
    assert_eq!(code(&r), 1);
    assert!(
        r.result.contains(
            "<<<<<<< ours\n### T-001 · One\n- status: open\n- owner: alice\n=======\n### T-001 · One\n- status: open\n- owner: bob\n>>>>>>> theirs\n"
        ),
        "{}",
        r.result
    );
    assert!(r.result.contains("### T-002 · Two\n"));
    assert!(String::from_utf8_lossy(&r.out.stderr).contains("conflict"));
}

#[test]
fn same_new_id_on_both_sides_conflicts_unless_identical() {
    let d = TempDir::new().unwrap();
    let base = file(&[task(1, "One", &[], &[])]);
    let with = |title: &str| {
        base.replace(
            "\n## Later",
            &format!("\n{}\n## Later", task(2, title, &[], &["x): added"])),
        )
    };
    let r = run_driver(d.path(), &base, &with("Mine"), &with("Yours"));
    assert_eq!(code(&r), 1);
    assert!(
        r.result.contains("<<<<<<< ours\n### T-002 · Mine\n"),
        "{}",
        r.result
    );
    assert!(r.result.contains("=======\n### T-002 · Yours\n"));

    let r = run_driver(d.path(), &base, &with("Same"), &with("Same"));
    assert_eq!(code(&r), 0);
    assert_eq!(r.result.matches("### T-002").count(), 1);
}

#[test]
fn different_new_tasks_at_the_same_place_are_both_kept() {
    let d = TempDir::new().unwrap();
    let base = file(&[task(1, "One", &[], &[])]);
    let add = |id: u32, t: &str| {
        base.replace(
            "\n## Later",
            &format!("\n{}\n## Later", task(id, t, &[], &[])),
        )
    };
    let r = run_driver(d.path(), &base, &add(2, "Mine"), &add(3, "Yours"));
    assert_eq!(code(&r), 0, "{}", r.result);
    let a = r.result.find("### T-002").unwrap();
    let b = r.result.find("### T-003").unwrap();
    assert!(a < b, "ours first");
    assert!(r.result.contains("## Later\n\nprose\n"));
    // Plain git conflicts on the same input.
    let (bb, oo, tt) = (
        d.path().join("pb"),
        d.path().join("po"),
        d.path().join("pt"),
    );
    fs::write(&bb, &base).unwrap();
    fs::write(&oo, add(2, "Mine")).unwrap();
    fs::write(&tt, add(3, "Yours")).unwrap();
    let st = common::git_command()
        .arg("merge-file")
        .args([&oo, &bb, &tt])
        .status()
        .unwrap();
    assert_eq!(st.code(), Some(1));
}

#[test]
fn prose_conflict_falls_back_to_git_markers() {
    let d = TempDir::new().unwrap();
    let base = file(&[task(1, "One", &[], &[])]);
    let ours = base.replace("prose", "ours prose");
    let theirs = base.replace("prose", "their prose");
    let r = run_driver(d.path(), &base, &ours, &theirs);
    assert_eq!(code(&r), 1);
    assert!(
        r.result
            .contains("<<<<<<< ours\nours prose\n=======\ntheir prose\n>>>>>>> theirs\n"),
        "{}",
        r.result
    );
}

#[test]
fn different_format_versions_fall_back_to_git() {
    let d = TempDir::new().unwrap();
    let base = format!(
        "<!-- todo-format: 1 -->\n{}",
        file(&[task(1, "One", &[], &[])])
    );
    let ours = base.replace("- owner: —", "- owner: a");
    let theirs = base
        .replace("todo-format: 1", "todo-format: 2")
        .replace("## Later", "## Later two");
    let r = run_driver(d.path(), &base, &ours, &theirs);
    // Plain git merges these (different regions); no task-level logic was involved.
    assert_eq!(code(&r), 0, "{}", r.result);
    assert!(r.result.contains("todo-format: 2") && r.result.contains("owner: a"));
}

#[test]
fn unparsable_input_falls_back_to_git() {
    let d = TempDir::new().unwrap();
    let (b, o, t) = (d.path().join("b"), d.path().join("o"), d.path().join("t"));
    fs::write(&b, "line one\nline two\nline three\n").unwrap();
    fs::write(&o, "LINE one\nline two\nline three\n").unwrap();
    fs::write(&t, b"line one\nline two\nline THREE \xFF\n").unwrap();
    let out = todo_bin()
        .args(["merge-driver", "run"])
        .args([&b, &o, &t])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let merged = fs::read(&o).unwrap();
    assert!(merged.starts_with(b"LINE one\n") && merged.ends_with(b"THREE \xFF\n"));
}

#[test]
fn every_output_line_comes_from_an_input_and_crlf_is_kept() {
    let d = TempDir::new().unwrap();
    let crlf = |s: String| s.replace('\n', "\r\n");
    let base = crlf(file(&[
        task(1, "One", &[], &["a): added"]),
        task(2, "Two", &[], &[]),
    ]));
    let ours = base
        .replacen("- status: open", "- status: claimed", 1)
        .replace(
            "a): added\r\n",
            "a): added\r\n- note (2026-11-16, ours): o1\r\n",
        );
    // Theirs: an LF note and a new LF task mixed into the CRLF file.
    let theirs = base
        .replacen("- owner: —\r\n", "- owner: —\r\n- area: cli\r\n", 2)
        .replace(
            "a): added\r\n",
            "a): added\r\n- note (2026-11-16, theirs): t1\n",
        )
        .replace(
            "\r\n## Later",
            &format!("\r\n{}\r\n## Later", task(3, "Three", &[], &[])),
        );
    let r = run_driver(d.path(), &base, &ours, &theirs);
    assert_eq!(code(&r), 0, "{}", r.result);
    let inputs: HashSet<&str> = [&base, &ours, &theirs]
        .iter()
        .flat_map(|s| s.split_inclusive('\n'))
        .collect();
    for line in r.result.split_inclusive('\n') {
        assert!(inputs.contains(line), "line not from an input: {line:?}");
    }
    assert!(r.result.contains("- note (2026-11-16, theirs): t1\n"));
    assert!(r.result.contains("### T-003 · Three\n"));
    assert!(r.result.contains("- note (2026-11-16, ours): o1\r\n"));
}

fn init_repo(dir: &Path) {
    git(dir, &["init", "-q", "-b", "main"]);
    git(dir, &["config", "user.name", "x"]);
    git(dir, &["config", "user.email", "x@example.invalid"]);
}

#[test]
fn install_and_uninstall_are_idempotent_and_keep_other_lines() {
    let d = TempDir::new().unwrap();
    init_repo(d.path());
    let attrs = d.path().join(".gitattributes");
    fs::write(&attrs, "*.png binary\r\n*.md text eol=lf").unwrap();

    for _ in 0..2 {
        let out = common::run_in(d.path(), &["merge-driver", "install"]);
        assert!(out.status.success(), "{}", common::stderr_text(&out));
    }
    assert_eq!(
        fs::read_to_string(&attrs).unwrap(),
        "*.png binary\r\n*.md text eol=lf\r\nTODO.md merge=todo\r\n"
    );
    assert_eq!(
        git(d.path(), &["config", "--local", "merge.todo.driver"]),
        "todo merge-driver run %O %A %B %P"
    );
    assert!(git(d.path(), &["config", "--local", "merge.todo.name"]).contains("todo"));

    for _ in 0..2 {
        let out = common::run_in(d.path(), &["merge-driver", "uninstall"]);
        assert!(out.status.success(), "{}", common::stderr_text(&out));
    }
    assert_eq!(
        fs::read_to_string(&attrs).unwrap(),
        "*.png binary\r\n*.md text eol=lf\r\n"
    );
    let cfg = common::git_command()
        .current_dir(d.path())
        .args(["config", "--local", "--get", "merge.todo.driver"])
        .output()
        .unwrap();
    assert!(!cfg.status.success());

    // With nothing else in .gitattributes, uninstall removes the file again.
    fs::remove_file(&attrs).unwrap();
    assert!(
        common::run_in(d.path(), &["merge-driver", "install"])
            .status
            .success()
    );
    assert_eq!(fs::read_to_string(&attrs).unwrap(), "TODO.md merge=todo\n");
    assert!(
        common::run_in(d.path(), &["merge-driver", "uninstall"])
            .status
            .success()
    );
    assert!(!attrs.exists());
}

#[test]
fn install_outside_a_repository_is_refused() {
    let d = TempDir::new().unwrap();
    let inner = d.path().join("inner");
    fs::create_dir(&inner).unwrap();
    // Stop git searching upward, in case the temp folder sits inside a repository.
    let out = todo_bin()
        .current_dir(&inner)
        .env("GIT_CEILING_DIRECTORIES", d.path())
        .args(["merge-driver", "install"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
}

fn commit_all(dir: &Path, msg: &str) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", msg]);
}

#[test]
fn real_git_merge_is_clean_with_the_driver_where_plain_git_conflicts() {
    let d = TempDir::new().unwrap();
    let dir = d.path();
    init_repo(dir);
    let base = file(&[task(1, "One", &[], &["a): added"])]);
    fs::write(dir.join("TODO.md"), &base).unwrap();
    commit_all(dir, "base");

    // Both branches add a note to T-001 and a new task at the end of the section.
    let side = |note: &str, id: u32, title: &str| {
        base.replace(
            "a): added\n",
            &format!("a): added\n- note (2026-11-16, {note}): n\n"),
        )
        .replace(
            "\n## Later",
            &format!("\n{}\n## Later", task(id, title, &[], &[])),
        )
    };
    git(dir, &["checkout", "-q", "-b", "other"]);
    fs::write(dir.join("TODO.md"), side("theirs", 3, "Theirs")).unwrap();
    commit_all(dir, "other");
    git(dir, &["checkout", "-q", "main"]);
    fs::write(dir.join("TODO.md"), side("ours", 2, "Ours")).unwrap();
    commit_all(dir, "main");

    // Plain git: conflict.
    let plain = common::git_command()
        .current_dir(dir)
        .args(["merge", "--no-commit", "--no-ff", "other"])
        .output()
        .unwrap();
    assert!(
        !plain.status.success(),
        "plain git was expected to conflict"
    );
    git(dir, &["merge", "--abort"]);

    // With the driver, pointing at the test binary.
    assert!(
        common::run_in(dir, &["merge-driver", "install"])
            .status
            .success()
    );
    let bin = assert_cmd::cargo::cargo_bin("todo");
    let driver = format!(
        "\"{}\" merge-driver run %O %A %B %P",
        bin.display().to_string().replace('\\', "/")
    );
    git(dir, &["config", "merge.todo.driver", &driver]);
    commit_all(dir, "attributes");

    let merged = common::git_command()
        .current_dir(dir)
        .args(["merge", "-q", "-m", "merge", "other"])
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .output()
        .unwrap();
    assert!(
        merged.status.success(),
        "{}{}",
        String::from_utf8_lossy(&merged.stdout),
        String::from_utf8_lossy(&merged.stderr)
    );
    let text = fs::read_to_string(dir.join("TODO.md")).unwrap();
    assert!(!text.contains("<<<<<<<"), "{text}");
    for want in [
        "(2026-11-16, ours): n",
        "(2026-11-16, theirs): n",
        "### T-002 · Ours",
        "### T-003 · Theirs",
    ] {
        assert!(text.contains(want), "missing {want}: {text}");
    }
}
