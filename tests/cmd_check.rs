mod common;

use std::fs;
use std::path::Path;

use serde_json::Value;
use tempfile::TempDir;

use common::todo_bin;

fn run_check(dir: &Path) -> (i32, String, Value) {
    let text_out = todo_bin()
        .current_dir(dir)
        .args(["check"])
        .output()
        .expect("run check");
    let json_out = todo_bin()
        .current_dir(dir)
        .args(["check", "--json"])
        .output()
        .expect("run check --json");
    let code = text_out.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&text_out.stdout).into_owned();
    let json: Value = serde_json::from_slice(&json_out.stdout).expect("parse check json");
    assert_eq!(json_out.status.code().unwrap_or(-1), code);
    (code, stdout, json)
}

#[test]
fn clean_fixture_exits_zero() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("TODO.md"),
        "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 1 -->
# Clean Tasks

## Tasks

### T-001 · Clean open task
- status: open
- owner: —
- priority: P1
",
    )
    .expect("write TODO.md");

    let (code, stdout, json) = run_check(dir.path());
    assert_eq!(code, 0, "expected exit 0, got stdout: {stdout}");
    assert_eq!(json["ok"], true);
    assert_eq!(json["data"]["clean"], true);
    assert_eq!(json["data"]["count"], 0);
}

#[test]
fn check_reports_every_r_check_1_and_r_dep_3_problem_with_line() {
    // 1. Invalid UTF-8
    {
        let dir = TempDir::new().expect("tempdir");
        let mut bytes = b"<!-- todo-format: 1 -->\n# Title\n".to_vec();
        bytes.extend_from_slice(&[0xff, 0xfe, b'\n']);
        fs::write(dir.path().join("TODO.md"), bytes).expect("write invalid utf8");
        let (code, stdout, _) = run_check(dir.path());
        assert_eq!(code, 1);
        assert!(stdout.contains("TODO.md:3:"), "stdout: {stdout}");
    }

    // 2. Git conflict marker + unclosed code fence
    {
        let dir = TempDir::new().expect("tempdir");
        fs::write(
            dir.path().join("TODO.md"),
            "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 1 -->
# Conflict
<<<<<<< HEAD
## Tasks
=======
>>>>>>> branch
```rust
fn unclosed() {}
",
        )
        .expect("write conflict");
        let (code, stdout, _) = run_check(dir.path());
        assert_eq!(code, 1);
        assert!(stdout.contains("TODO.md:4:"), "conflict line: {stdout}");
        assert!(stdout.contains("TODO.md:8:"), "fence line: {stdout}");
    }

    // 3. Unclosed HTML comment & unknown format version
    {
        let dir = TempDir::new().expect("tempdir");
        fs::write(
            dir.path().join("TODO.md"),
            "\
<!-- todo-format: 99 -->
<!-- TEST FIXTURE: invented data, not a real task list -->
# Version
<!-- unclosed comment
",
        )
        .expect("write bad version");
        let (code, stdout, _) = run_check(dir.path());
        assert_eq!(code, 1);
        assert!(stdout.contains("TODO.md:1:"), "version line: {stdout}");
        assert!(stdout.contains("TODO.md:4:"), "comment line: {stdout}");
    }

    // 4. All task-level, dependency, config, temp-file, and outdated-rules problems
    {
        let dir = TempDir::new().expect("tempdir");
        fs::write(
            dir.path().join("todo.toml"),
            "\
priorities = [\"P0\", \"P1\"]
types = [\"feature\", \"bug\"]
areas = [\"core\"]
claim_ttl_hours = 1
",
        )
        .expect("write todo.toml");
        fs::write(
            dir.path().join("TODO-archive.md"),
            "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 1 -->
# Archive

## Tasks

### T-099 · Archived duplicate
- status: done
- owner: —
",
        )
        .expect("write archive");
        fs::write(dir.path().join("TODO.md.1234.deadbeef.tmp"), "leftover")
            .expect("write temp file");

        fs::write(
            dir.path().join("TODO.md"),
            "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 1 -->
# Main

<!-- todo:begin -->
Outdated rules text.
<!-- todo:end -->

## Tasks

### T-001 · Duplicate and cycle A
- status: open
- owner: alice
- priority: P9
- type: chore
- area: ui
- depends: T-002, T-999, T-003

### T-001 · Duplicate ID
- status: bogus
- priority: P0
- priority: P1

### T-002 · Cycle B
- status: claimed
- depends: T-001

### T-003 · Dropped dep target
- status: dropped
- owner: —

### T-004 · Stale claim
- status: claimed
- owner: bob
- claimed: 2020-01-01T00:00+00:00

### T-005 · Blocked without reason
- status: blocked
- owner: —

### T-006 bad heading without separator

### T-099 · Collides with archive
- status: open
- owner: —
",
        )
        .expect("write TODO.md");

        let (code, stdout, json) = run_check(dir.path());
        assert_eq!(code, 1);
        assert_eq!(json["ok"], false);

        for needle in [
            "TODO.md:5: outdated agent rules block",
            "TODO.md:11:", // open with owner / bad priority/type/area / missing dep / dropped dep / cycle
            "T-001: open task still has an owner",
            "T-001: priority \"P9\" is not in configured priorities",
            "T-001: type \"chore\" is not in configured types",
            "T-001: area \"ui\" is not in configured areas",
            "T-001: dependency T-999 does not exist",
            "T-001: depends on dropped task T-003",
            "dependency cycle: T-001 -> T-002 -> T-001",
            "TODO.md:19:", // duplicate ID
            "TODO.md:20:", // unknown status
            "TODO.md:22:", // repeated field
            "T-002: claimed task has no owner",
            "T-002: claimed task has no claimed timestamp",
            "T-004: stale claim by bob",
            "T-005: blocked task has no reason note",
            "TODO.md:41:", // probable mistake heading
            "TODO.md:43:", // archive duplicate
            "TODO.md.1234.deadbeef.tmp:1: leftover temporary file",
        ] {
            assert!(
                stdout.contains(needle),
                "expected `{needle}` in check output:\n{stdout}"
            );
        }
    }
}

#[test]
fn check_fix_repairs_only_mechanical_issues_and_reports_the_rest() {
    let dir = TempDir::new().expect("tempdir");
    let tmp_file = dir.path().join("TODO.md.999.abcd.tmp");
    fs::write(&tmp_file, "leftover").expect("write tmp");
    fs::write(
        dir.path().join("TODO.md"),
        "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 1 -->

<!-- todo:begin -->
Stale rules.
<!-- todo:end -->

## Tasks

### t-1: Needs canonical heading and status
- priority: P1

### T-002 · Still blocked without reason
- status: blocked
- depends: T-999
",
    )
    .expect("write TODO.md");

    let out = todo_bin()
        .current_dir(dir.path())
        .args(["check", "--fix", "--agent", "alice"])
        .output()
        .expect("run check --fix");
    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8(out.stdout).expect("utf8");
    assert!(
        stdout.contains("T-002: blocked task has no reason note"),
        "{stdout}"
    );
    assert!(
        stdout.contains("T-002: dependency T-999 does not exist"),
        "{stdout}"
    );
    assert!(!stdout.contains("outdated agent rules block"), "{stdout}");
    assert!(!stdout.contains("leftover temporary file"), "{stdout}");

    assert!(!tmp_file.exists(), "tmp file should be deleted");
    let updated = fs::read_to_string(dir.path().join("TODO.md")).expect("read TODO.md");
    assert!(updated.contains("### T-001 \u{00b7} Needs canonical heading and status\n- status: open\n- priority: P1\n"), "{updated}");
    assert!(!updated.contains("Stale rules."), "{updated}");
    assert!(
        updated.contains(
            "### T-002 \u{00b7} Still blocked without reason\n- status: blocked\n- depends: T-999\n"
        ),
        "{updated}"
    );
}
