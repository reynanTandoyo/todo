//! `todo check` on milestone trees and references (T-090; R-MILE-5, R-KIND-2): one invented fixture
//! per problem, each reported with its line and with no extra problem, a clean three-level tree
//! that exits 0, and an entry below an archived milestone. Every fixture is invented; every test
//! runs in its own temporary folder.

mod common;

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use common::{clean_todo_command, stdout_text};

/// The text of an invented fixture in `tests/fixtures/`.
fn fixture_text(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    fs::read_to_string(&path).unwrap_or_default()
}

/// The 1-based number of the first line of `text` that contains `needle`, or 0 when none does
/// (so a missing line makes the expected output fail visibly).
fn line_of(text: &str, needle: &str) -> usize {
    text.lines()
        .position(|line| line.contains(needle))
        .map_or(0, |index| index + 1)
}

/// Writes `files` into a new folder, runs `todo check` there and returns the exit code and the text
/// output. The environment is clean, so no `TODO_FILE` from the caller reaches the test.
fn check_files(files: &[(&str, &str)]) -> (Option<i32>, String) {
    let dir = TempDir::new().expect("create temp folder");
    for (name, text) in files {
        fs::write(dir.path().join(name), text).expect("write fixture file");
    }
    let mut cmd = clean_todo_command(dir.path(), dir.path());
    cmd.arg("check");
    let out = cmd.output().expect("run todo check");
    (out.status.code(), stdout_text(&out))
}

/// The problem lines of a check run: every output line that starts with the file label.
fn problem_lines(stdout: &str) -> Vec<&str> {
    stdout
        .lines()
        .filter(|line| line.starts_with("TODO.md:"))
        .collect()
}

#[test]
fn missing_and_non_milestone_parents_are_reported_with_their_line() {
    let text = fixture_text("milestone_check_bad_refs.md");
    let (code, stdout) = check_files(&[("TODO.md", text.as_str())]);

    assert_eq!(code, Some(1), "expected exit 1:\n{stdout}");
    let missing = format!(
        "TODO.md:{}: T-002: milestone T-099 does not exist",
        line_of(&text, "- milestone: T-099")
    );
    let not_milestone = format!(
        "TODO.md:{}: T-003: T-004 is not a milestone",
        line_of(&text, "- milestone: T-004")
    );
    assert!(stdout.contains(&missing), "expected `{missing}`:\n{stdout}");
    assert!(
        stdout.contains(&not_milestone),
        "expected `{not_milestone}`:\n{stdout}"
    );
    assert_eq!(
        problem_lines(&stdout).len(),
        2,
        "exactly two problems:\n{stdout}"
    );
}

#[test]
fn milestone_cycles_are_reported_once_with_their_line() {
    let text = fixture_text("milestone_check_cycle.md");
    let (code, stdout) = check_files(&[("TODO.md", text.as_str())]);

    assert_eq!(code, Some(1), "expected exit 1:\n{stdout}");
    let two_cycle = format!(
        "TODO.md:{}: milestone cycle: T-001 -> T-002 -> T-001",
        line_of(&text, "- milestone: T-001")
    );
    let self_cycle = format!(
        "TODO.md:{}: milestone cycle: T-003 -> T-003",
        line_of(&text, "- milestone: T-003")
    );
    assert!(
        stdout.contains(&two_cycle),
        "expected `{two_cycle}`:\n{stdout}"
    );
    assert!(
        stdout.contains(&self_cycle),
        "expected `{self_cycle}`:\n{stdout}"
    );
    assert_eq!(
        problem_lines(&stdout).len(),
        2,
        "exactly two problems:\n{stdout}"
    );
}

#[test]
fn closed_milestones_with_unfinished_entries_are_reported_with_their_line() {
    let text = fixture_text("milestone_check_unfinished.md");
    let (code, stdout) = check_files(&[("TODO.md", text.as_str())]);

    assert_eq!(code, Some(1), "expected exit 1:\n{stdout}");
    let done = format!(
        "TODO.md:{}: T-001: done milestone has unfinished entries below it: T-002 (open), T-003 (claimed)",
        line_of(&text, "### T-001 · Done milestone")
    );
    let dropped = format!(
        "TODO.md:{}: T-004: dropped milestone has unfinished entries below it: T-005 (blocked)",
        line_of(&text, "### T-004 · Dropped milestone")
    );
    assert!(stdout.contains(&done), "expected `{done}`:\n{stdout}");
    assert!(stdout.contains(&dropped), "expected `{dropped}`:\n{stdout}");
    assert_eq!(
        problem_lines(&stdout).len(),
        2,
        "exactly two problems:\n{stdout}"
    );
}

#[test]
fn milestone_without_goal_is_reported_with_its_line() {
    let text = fixture_text("milestone_check_no_goal.md");
    let (code, stdout) = check_files(&[("TODO.md", text.as_str())]);

    assert_eq!(code, Some(1), "expected exit 1:\n{stdout}");
    let no_goal = format!(
        "TODO.md:{}: T-001: milestone has no goal",
        line_of(&text, "### T-001 · Develop the renderer")
    );
    assert!(stdout.contains(&no_goal), "expected `{no_goal}`:\n{stdout}");
    assert_eq!(
        problem_lines(&stdout).len(),
        1,
        "exactly one problem:\n{stdout}"
    );
}

#[test]
fn about_naming_a_missing_id_is_reported_with_its_line() {
    let text = fixture_text("milestone_check_about.md");
    let (code, stdout) = check_files(&[("TODO.md", text.as_str())]);

    assert_eq!(code, Some(1), "expected exit 1:\n{stdout}");
    let question = format!(
        "TODO.md:{}: T-001: about T-099 does not exist",
        line_of(&text, "- about: T-099")
    );
    // A non-canonical ID (`t-0099`) is still read as T-099 and reported in canonical form.
    let idea = format!(
        "TODO.md:{}: T-004: about T-099 does not exist",
        line_of(&text, "- about: t-0099")
    );
    assert!(
        stdout.contains(&question),
        "expected `{question}`:\n{stdout}"
    );
    assert!(stdout.contains(&idea), "expected `{idea}`:\n{stdout}");
    assert_eq!(
        problem_lines(&stdout).len(),
        2,
        "exactly two problems:\n{stdout}"
    );
}

#[test]
fn clean_three_level_tree_exits_zero() {
    let text = fixture_text("milestone_check_clean.md");
    let (code, stdout) = check_files(&[("TODO.md", text.as_str())]);

    assert_eq!(code, Some(0), "expected exit 0:\n{stdout}");
    assert!(stdout.contains("ok (0 problems)"), "{stdout}");
}

#[test]
fn references_into_the_archive_count_as_existing_and_closed_parents_are_reported() {
    let todo = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 1 -->
# Live work under archived milestones

## Tasks

### T-002 · Open task under an archived milestone
- status: open
- owner: —
- priority: P1
- milestone: T-001

### T-003 · Finished task under an archived milestone
- status: done
- owner: —
- priority: P1
- milestone: T-001

### T-004 · Question about an archived task
- status: open
- owner: —
- priority: P2 · type: question
- about: T-005
";
    let archive = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 1 -->
# Archive

## Tasks

### T-001 · Archived milestone
- status: done
- owner: —
- priority: P1 · type: milestone
- goal: the first piece of work is finished

### T-005 · Archived task
- status: done
- owner: —
- priority: P2
- milestone: T-001
";
    let (code, stdout) = check_files(&[("TODO.md", todo), ("TODO-archive.md", archive)]);

    assert_eq!(code, Some(1), "expected exit 1:\n{stdout}");
    let closed = format!(
        "TODO.md:{}: T-002: milestone T-001 is archived as done, but this entry is open",
        line_of(todo, "### T-002 · Open task")
    );
    assert!(stdout.contains(&closed), "expected `{closed}`:\n{stdout}");
    assert_eq!(
        problem_lines(&stdout).len(),
        1,
        "archived references must not be reported as missing:\n{stdout}"
    );
}
