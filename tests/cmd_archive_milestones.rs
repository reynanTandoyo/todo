//! CLI integration tests for `archive` and milestone trees (T-103; R-ARCH-1, R-ARCH-2): a tree
//! moves whole or not at all, `--older-than` measures the top-level milestone, the preview and the
//! result say how many finished entries were kept back and why, and an archived tree is found again
//! by `show` and `list --archive`. Every fixture is invented.

mod common;

use std::fs;
use std::path::PathBuf;
use std::process::Output;

use serde_json::Value;
use tempfile::TempDir;

use common::{parse_json, read_todo, run_todo, setup_proj, stdout_text};

/// Fixed clock for note dates and ages (`TODO_TEST_NOW`, test-hooks feature).
const NOW: &str = "2026-11-18T10:00:00+01:00";

const FIXTURE: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->\n";

/// One invented entry of the task list.
struct Entry {
    id: &'static str,
    title: &'static str,
    status: &'static str,
    /// `milestone` for a milestone (it gets a goal); `task` for anything else.
    kind: &'static str,
    /// The parent's ID, or `""` for an entry outside any milestone.
    parent: &'static str,
    /// The date of its only note (`YYYY-MM-DD`); ages for `--older-than` come from it.
    date: &'static str,
    /// The section heading the entry sits under.
    section: &'static str,
}

fn entry(
    id: &'static str,
    title: &'static str,
    status: &'static str,
    kind: &'static str,
    parent: &'static str,
    date: &'static str,
    section: &'static str,
) -> Entry {
    Entry {
        id,
        title,
        status,
        kind,
        parent,
        date,
        section,
    }
}

/// Renders the entries in the order given, with one heading per run of entries in a section.
fn render(entries: &[Entry]) -> String {
    let mut text = String::from(FIXTURE);
    let mut current: Option<&str> = None;
    for e in entries {
        if current != Some(e.section) {
            text.push_str(&format!("\n## {}\n\n", e.section));
            current = Some(e.section);
        }
        text.push_str(&format!(
            "### {} · {}\n- status: {}\n- owner: —\n",
            e.id, e.title, e.status
        ));
        if e.kind == "milestone" {
            text.push_str("- priority: P1 · type: milestone\n");
            text.push_str("- goal: the test scene renders with lighting\n");
        } else {
            text.push_str("- priority: P1\n");
        }
        if !e.parent.is_empty() {
            text.push_str(&format!("- milestone: {}\n", e.parent));
        }
        text.push_str(&format!("- note ({}, planner): recorded\n\n", e.date));
    }
    text
}

/// The standard tree. T-001 (milestone) holds T-002 (milestone), which holds T-003, and T-004 (task,
/// under "Later"). T-005 is a finished entry outside any milestone and T-006 is open. `s` gives the
/// statuses of T-001 to T-004.
fn standard(s: [&'static str; 4]) -> Vec<Entry> {
    vec![
        entry(
            "T-001",
            "Develop the renderer",
            s[0],
            "milestone",
            "",
            "2026-10-01",
            "Now",
        ),
        entry(
            "T-002",
            "Lighting",
            s[1],
            "milestone",
            "T-001",
            "2026-10-02",
            "Now",
        ),
        entry(
            "T-003",
            "C shader",
            s[2],
            "task",
            "T-002",
            "2026-10-03",
            "Now",
        ),
        entry(
            "T-004",
            "GPU compiler",
            s[3],
            "task",
            "T-001",
            "2026-10-04",
            "Later",
        ),
        entry(
            "T-005",
            "Unrelated cleanup",
            "done",
            "task",
            "",
            "2026-10-05",
            "Later",
        ),
        entry(
            "T-006",
            "Open chore",
            "open",
            "task",
            "",
            "2026-10-06",
            "Later",
        ),
    ]
}

struct Proj {
    _tmp: TempDir,
    home: PathBuf,
    work: PathBuf,
}

fn proj(entries: &[Entry]) -> Proj {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo) = setup_proj(&tmp, &render(entries));
    Proj {
        _tmp: tmp,
        home,
        work,
    }
}

/// Runs `todo <args>` with the fixed clock.
fn run(p: &Proj, args: &[&str]) -> Output {
    run_todo(&p.home, &p.work, args, &[("TODO_TEST_NOW", NOW)]).unwrap()
}

/// Runs `todo --json <args>` with the fixed clock.
fn run_json(p: &Proj, args: &[&str]) -> Output {
    let mut all = vec!["--json"];
    all.extend_from_slice(args);
    run(p, &all)
}

/// Runs a real archive: `archive --yes --agent coordinator` plus `extra` options.
fn archive_yes(p: &Proj, extra: &[&str]) -> Output {
    let mut args = vec!["archive"];
    args.extend_from_slice(extra);
    args.extend_from_slice(&["--yes", "--agent", "coordinator"]);
    let out = run_json(p, &args);
    assert_eq!(
        out.status.code(),
        Some(0),
        "archive failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

/// The `id` of each entry in a JSON list, in order.
fn ids(list: &Value) -> Vec<&str> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|v| v["id"].as_str().unwrap())
        .collect()
}

/// Whether TODO.md holds the entry `id` as a heading.
fn in_todo(p: &Proj, id: &str) -> bool {
    let heading = format!("### {id} · ");
    read_todo(&p.work)
        .lines()
        .any(|l| l.starts_with(heading.as_str()))
}

/// The text of TODO-archive.md, or an empty string when it does not exist yet.
fn archive_text(p: &Proj) -> String {
    fs::read_to_string(p.work.join("TODO-archive.md")).unwrap_or_default()
}

/// Whether TODO-archive.md holds the entry `id` as a heading.
fn in_archive(p: &Proj, id: &str) -> bool {
    let heading = format!("### {id} · ");
    archive_text(p)
        .lines()
        .any(|l| l.starts_with(heading.as_str()))
}

#[test]
fn finished_tree_moves_whole_in_one_step() {
    let p = proj(&standard(["done", "done", "done", "done"]));

    let json = parse_json(&archive_yes(&p, &[]).stdout);
    assert_eq!(json["data"]["applied"], true);
    assert_eq!(json["data"]["count"], 5);
    assert_eq!(
        ids(&json["data"]["tasks"]),
        ["T-001", "T-002", "T-003", "T-004", "T-005"]
    );
    assert_eq!(json["data"]["kept_back"], serde_json::json!([]));

    // Every member left TODO.md in the same run and is in the archive, under its own section.
    for id in ["T-001", "T-002", "T-003", "T-004", "T-005"] {
        assert!(!in_todo(&p, id), "{id} still in TODO.md");
        assert!(in_archive(&p, id), "{id} missing from the archive");
    }
    assert!(in_todo(&p, "T-006"), "the open entry must stay");
    let archive = archive_text(&p);
    let later = archive.find("## Later").expect("Later section in archive");
    assert!(archive[later..].contains("### T-004 · GPU compiler"));
    assert!(archive[..later].contains("### T-003 · C shader"));
}

#[test]
fn dropped_tree_is_finished_too() {
    let p = proj(&standard(["dropped", "dropped", "done", "dropped"]));

    let json = parse_json(&archive_yes(&p, &[]).stdout);
    assert_eq!(json["data"]["count"], 5);
    assert!(!in_todo(&p, "T-001"));
    assert!(!in_todo(&p, "T-004"));
}

#[test]
fn open_top_level_milestone_keeps_its_finished_entries_and_says_why() {
    let p = proj(&standard(["open", "done", "done", "done"]));

    // The preview names what would move and what stays, with the reason, and changes nothing.
    let preview = run(&p, &["archive", "--agent", "coordinator"]);
    assert_eq!(preview.status.code(), Some(0));
    let text = stdout_text(&preview);
    assert!(
        text.contains("would archive 1 task(s) to TODO-archive.md:"),
        "{text}"
    );
    assert!(text.contains("T-005 [done] · Unrelated cleanup"), "{text}");
    assert!(
        text.contains("kept back 3 finished entries in TODO.md:"),
        "{text}"
    );
    assert!(
        text.contains("T-002 [done] · Lighting — milestone T-001 is still open"),
        "{text}"
    );
    assert!(text.contains("T-004 [done] · GPU compiler — milestone T-001 is still open"));
    assert!(text.contains("pass --yes"), "{text}");
    assert!(!p.work.join("TODO-archive.md").exists());

    let json = parse_json(&run_json(&p, &["archive", "--agent", "coordinator"]).stdout);
    assert_eq!(json["data"]["applied"], false);
    assert_eq!(json["data"]["count"], 1);
    let kept = json["data"]["kept_back"].as_array().unwrap();
    assert_eq!(ids(&json["data"]["kept_back"]), ["T-002", "T-003", "T-004"]);
    assert_eq!(kept[0]["status"], "done");
    assert_eq!(kept[0]["section"], "Now");
    assert_eq!(kept[0]["reason"], "milestone T-001 is still open");

    // The real run says the same in text, then moves only T-005.
    let applied = run(&p, &["archive", "--yes", "--agent", "coordinator"]);
    assert_eq!(applied.status.code(), Some(0));
    let text = stdout_text(&applied);
    assert!(
        text.contains("archived 1 task(s) to TODO-archive.md"),
        "{text}"
    );
    assert!(
        text.contains("kept back 3 finished entries in TODO.md:"),
        "{text}"
    );
    assert!(!in_archive(&p, "T-002"));
    assert!(in_archive(&p, "T-005"));

    // Every milestone still in TODO.md keeps all its entries: the children are all there.
    for id in ["T-001", "T-002", "T-003", "T-004"] {
        assert!(in_todo(&p, id), "{id} must stay in TODO.md");
    }
    let root = parse_json(&run_json(&p, &["show", "T-001"]).stdout);
    assert_eq!(root["data"]["archived"], false);
    assert_eq!(ids(&root["data"]["children"]), ["T-002", "T-004"]);
    let sub = parse_json(&run_json(&p, &["show", "T-002"]).stdout);
    assert_eq!(ids(&sub["data"]["children"]), ["T-003"]);
}

#[test]
fn done_top_level_milestone_with_open_entries_keeps_the_whole_tree() {
    // A milestone closed with --force keeps its open work in TODO.md.
    let p = proj(&standard(["done", "done", "open", "done"]));

    let json = parse_json(&archive_yes(&p, &[]).stdout);
    assert_eq!(json["data"]["count"], 1);
    assert_eq!(ids(&json["data"]["tasks"]), ["T-005"]);
    let kept = json["data"]["kept_back"].as_array().unwrap();
    assert_eq!(ids(&json["data"]["kept_back"]), ["T-001", "T-002", "T-004"]);
    assert_eq!(
        kept[0]["reason"],
        "milestone T-001 has 1 unfinished entry below it"
    );
    for id in ["T-001", "T-002", "T-003", "T-004"] {
        assert!(in_todo(&p, id), "{id} must stay in TODO.md");
    }
    assert!(!in_archive(&p, "T-003"));
}

#[test]
fn blocked_entry_keeps_its_tree_like_an_open_one() {
    let p = proj(&standard(["done", "done", "blocked", "done"]));

    let json = parse_json(&archive_yes(&p, &[]).stdout);
    assert_eq!(json["data"]["count"], 1);
    assert_eq!(ids(&json["data"]["kept_back"]), ["T-001", "T-002", "T-004"]);
    assert!(in_todo(&p, "T-003"));
}

#[test]
fn older_than_measures_the_top_level_milestone() {
    // The milestone is recent (3 days) and its children are old: nothing in the tree moves, and the
    // old standalone entry does. The tree is held by its age, so it is not reported as kept back.
    let recent_root = proj(&[
        entry(
            "T-001",
            "Develop the renderer",
            "done",
            "milestone",
            "",
            "2026-11-15",
            "Now",
        ),
        entry(
            "T-002",
            "Lighting",
            "done",
            "task",
            "T-001",
            "2026-10-01",
            "Now",
        ),
        entry(
            "T-005",
            "Unrelated cleanup",
            "done",
            "task",
            "",
            "2026-10-05",
            "Now",
        ),
    ]);
    let json = parse_json(&archive_yes(&recent_root, &["--older-than", "10"]).stdout);
    assert_eq!(ids(&json["data"]["tasks"]), ["T-005"]);
    assert_eq!(json["data"]["kept_back"], serde_json::json!([]));
    assert!(in_todo(&recent_root, "T-001"));
    assert!(in_todo(&recent_root, "T-002"));

    // The milestone is old (48 days) and a child is recent (3 days): the whole tree moves, and the
    // recent standalone entry stays.
    let old_root = proj(&[
        entry(
            "T-001",
            "Develop the renderer",
            "done",
            "milestone",
            "",
            "2026-10-01",
            "Now",
        ),
        entry(
            "T-002",
            "Lighting",
            "done",
            "task",
            "T-001",
            "2026-11-15",
            "Now",
        ),
        entry(
            "T-003",
            "Recent cleanup",
            "done",
            "task",
            "",
            "2026-11-15",
            "Now",
        ),
    ]);
    let json = parse_json(&archive_yes(&old_root, &["--older-than", "10"]).stdout);
    assert_eq!(ids(&json["data"]["tasks"]), ["T-001", "T-002"]);
    assert!(in_todo(&old_root, "T-003"));
}

#[test]
fn entry_whose_milestone_chain_is_broken_stays_and_says_so() {
    let p = proj(&[
        entry(
            "T-001",
            "Old milestone",
            "done",
            "milestone",
            "",
            "2026-10-01",
            "Now",
        ),
        entry(
            "T-002",
            "Orphan",
            "done",
            "task",
            "T-099",
            "2026-10-02",
            "Now",
        ),
    ]);

    let json = parse_json(&archive_yes(&p, &[]).stdout);
    assert_eq!(ids(&json["data"]["tasks"]), ["T-001"]);
    let kept = json["data"]["kept_back"].as_array().unwrap();
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0]["id"], "T-002");
    assert!(
        kept[0]["reason"]
            .as_str()
            .unwrap()
            .contains("does not reach a top-level milestone"),
        "{}",
        kept[0]["reason"]
    );
    assert!(in_todo(&p, "T-002"));
}

#[test]
fn task_that_entries_name_as_their_milestone_stays() {
    // T-001 is a task, not a milestone, yet T-002 names it. Moving T-001 would leave T-002 pointing
    // at an archived entry, so T-001 stays and T-002 (whose chain is broken) stays too.
    let p = proj(&[
        entry(
            "T-001",
            "Damaged parent",
            "done",
            "task",
            "",
            "2026-10-01",
            "Now",
        ),
        entry(
            "T-002",
            "Child",
            "done",
            "task",
            "T-001",
            "2026-10-02",
            "Now",
        ),
        entry(
            "T-003",
            "Unrelated cleanup",
            "done",
            "task",
            "",
            "2026-10-03",
            "Now",
        ),
    ]);

    let json = parse_json(&archive_yes(&p, &[]).stdout);
    assert_eq!(ids(&json["data"]["tasks"]), ["T-003"]);
    assert_eq!(ids(&json["data"]["kept_back"]), ["T-001", "T-002"]);
    assert!(in_todo(&p, "T-001"));
    assert!(in_todo(&p, "T-002"));
}

#[test]
fn entry_outside_the_tree_that_names_a_member_keeps_the_tree() {
    // T-003 is a task under T-001, so it belongs to the tree. T-004 names that task as its
    // milestone, which a milestone chain cannot reach: it stays, and the tree would leave it
    // pointing at an archived entry, so the whole tree stays too.
    let p = proj(&[
        entry(
            "T-001",
            "Develop the renderer",
            "done",
            "milestone",
            "",
            "2026-10-01",
            "Now",
        ),
        entry(
            "T-002",
            "Lighting",
            "done",
            "task",
            "T-001",
            "2026-10-02",
            "Now",
        ),
        entry(
            "T-003",
            "C shader",
            "done",
            "task",
            "T-001",
            "2026-10-03",
            "Now",
        ),
        entry(
            "T-004",
            "GPU compiler",
            "done",
            "task",
            "T-003",
            "2026-10-04",
            "Now",
        ),
    ]);

    let json = parse_json(&archive_yes(&p, &[]).stdout);
    assert_eq!(json["data"]["count"], 0);
    assert_eq!(
        ids(&json["data"]["kept_back"]),
        ["T-001", "T-002", "T-003", "T-004"]
    );
    let kept = json["data"]["kept_back"].as_array().unwrap();
    assert_eq!(
        kept[2]["reason"],
        "T-004 still names T-003 as its milestone"
    );
    assert!(in_todo(&p, "T-001"));
    assert!(in_todo(&p, "T-004"));
}

#[test]
fn archived_tree_is_found_by_show_and_list_archive() {
    let p = proj(&standard(["done", "done", "done", "done"]));
    archive_yes(&p, &[]);

    // show finds an archived entry with its milestone chain (top down, R-MILE-4) and its children.
    let leaf = parse_json(&run_json(&p, &["show", "T-003"]).stdout);
    assert_eq!(leaf["data"]["archived"], true);
    assert_eq!(leaf["data"]["milestones"][0]["id"], "T-001");
    assert_eq!(leaf["data"]["milestones"][0]["status"], "done");
    assert_eq!(leaf["data"]["milestones"][1]["id"], "T-002");
    assert_eq!(
        leaf["data"]["milestones"][1]["goal"],
        "the test scene renders with lighting"
    );

    let root = parse_json(&run_json(&p, &["show", "T-001"]).stdout);
    assert_eq!(root["data"]["archived"], true);
    assert_eq!(ids(&root["data"]["children"]), ["T-002", "T-004"]);

    let plain = run(&p, &["show", "T-004"]);
    assert_eq!(plain.status.code(), Some(0));
    assert!(stdout_text(&plain).contains("### T-004 · GPU compiler"));

    // list --archive finds every member of the tree.
    let listed = parse_json(&run_json(&p, &["list", "--archive"]).stdout);
    let found = ids(&listed["data"]["tasks"]);
    for id in ["T-001", "T-002", "T-003", "T-004", "T-005"] {
        assert!(found.contains(&id), "{id} not listed from the archive");
    }

    // With the whole tree archived, the file still passes check.
    let check = run(&p, &["check"]);
    assert_eq!(
        check.status.code(),
        Some(0),
        "check: {}",
        stdout_text(&check)
    );
}
