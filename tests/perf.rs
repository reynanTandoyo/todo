//! Performance tests on a generated 5,000-entry `TODO.md` (T-065, T-105; NFR-3, TEST-9).
//!
//! Two files are generated: the flat 5,000-task file (T-065) and a variant of it whose entries
//! form a milestone tree four levels deep (T-105). Both run in CI, and not in a normal local
//! `cargo test`. Run them in release mode with:
//! `cargo test --release --test perf -- --ignored`

mod common;

use common::{stderr_text, stdout_text};

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::{Duration, Instant};

use tempfile::TempDir;

/// Number of entries in each generated file.
const TOTAL_ENTRIES: usize = 5000;

/// Shape of the milestone tree in the variant (T-105). Depth 1 holds the top-level milestones,
/// depth 2 and depth 3 hold nested milestones, and the tasks sit at depth 4, below the depth-3
/// milestones. So a task at depth 4 has three milestones above it.
const TOP_MILESTONES: usize = 10;
const MIDDLE_PER_TOP: usize = 6;
const LOWER_PER_MIDDLE: usize = 5;

/// A depth-1 milestone with the most entries below it: `list --milestone` lists the whole branch.
const TOP_MILESTONE: &str = "T-0001";
/// A depth-3 milestone, the parent of the written task and of the timed `add`.
const DEEP_MILESTONE: &str = "T-0101";
/// An open task at depth 4 (its chain is T-0002, T-0017, T-0101), used by `show` and `note`.
const DEEP_TASK: &str = "T-2501";

const FIXTURE: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->\n";

fn generate_5000_task_file() -> String {
    let mut out = String::with_capacity(950 * 1024);
    out.push_str(FIXTURE);
    out.push_str("## Now\n");
    for i in 1..=TOTAL_ENTRIES {
        let prio = match i % 4 {
            0 => "P0",
            1 => "P1",
            2 => "P2",
            _ => "P3",
        };
        let status = if i % 10 == 0 { "done" } else { "open" };
        let owner = if status == "done" { "alice" } else { "—" };
        let _ = writeln!(
            out,
            "\n### T-{i:04} · Generated benchmark task {i}\n\
- status: {status}\n\
- owner: {owner}\n\
- priority: {prio} · type: task · area: core"
        );
        if i > 1 && i % 5 == 0 {
            let _ = writeln!(out, "- depends: T-{:04}", i - 1);
        }
        out.push_str("- accept: completes within NFR-3 budget\n");
        if i % 4 == 0 {
            out.push_str("- note (2026-01-15, alice): initial triage note for benchmark task\n");
        }
    }
    out
}

/// Writes a milestone heading and its fields. `parent` is the ID of the milestone above it.
fn push_milestone(out: &mut String, id: usize, parent: Option<usize>) {
    let _ = writeln!(
        out,
        "\n### T-{id:04} · Generated benchmark milestone {id}\n\
- status: open\n\
- owner: —\n\
- priority: P1 · type: milestone"
    );
    if let Some(parent) = parent {
        let _ = writeln!(out, "- milestone: T-{parent:04}");
    }
    let _ = writeln!(out, "- goal: generated goal for milestone {id}");
}

/// The variant of [`generate_5000_task_file`] (T-105): the same 5,000 entries, but the first
/// 370 are milestones in three nested levels and the other 4,630 are tasks at depth 4. IDs follow
/// the file order: top-level milestones, then depth 2, then depth 3, then the tasks.
fn generate_milestone_tree_file() -> String {
    let middle_count = TOP_MILESTONES * MIDDLE_PER_TOP;
    let lower_count = middle_count * LOWER_PER_MIDDLE;
    let milestone_count = TOP_MILESTONES + middle_count + lower_count;
    let task_count = TOTAL_ENTRIES - milestone_count;

    // ID of the k-th milestone at each depth (0-based), and of the t-th task.
    let middle_id = |k: usize| TOP_MILESTONES + 1 + k;
    let lower_id = |j: usize| TOP_MILESTONES + middle_count + 1 + j;
    let task_id = |t: usize| milestone_count + 1 + t;

    let mut out = String::with_capacity(950 * 1024);
    out.push_str(FIXTURE);
    out.push_str("<!-- todo-format: 2 -->\n");
    out.push_str("## Now\n");

    for top in 0..TOP_MILESTONES {
        push_milestone(&mut out, top + 1, None);
    }
    for k in 0..middle_count {
        push_milestone(&mut out, middle_id(k), Some(k / MIDDLE_PER_TOP + 1));
    }
    for j in 0..lower_count {
        push_milestone(&mut out, lower_id(j), Some(middle_id(j / LOWER_PER_MIDDLE)));
    }

    for t in 0..task_count {
        let id = task_id(t);
        let parent = lower_id(t % lower_count);
        let prio = match id % 4 {
            0 => "P0",
            1 => "P1",
            2 => "P2",
            _ => "P3",
        };
        let status = if id % 10 == 0 { "done" } else { "open" };
        let owner = if status == "done" { "alice" } else { "—" };
        let _ = writeln!(
            out,
            "\n### T-{id:04} · Generated benchmark task {id}\n\
- status: {status}\n\
- owner: {owner}\n\
- priority: {prio} · type: task · area: core\n\
- milestone: T-{parent:04}"
        );
        if id > milestone_count + 1 && id % 5 == 0 {
            let _ = writeln!(out, "- depends: T-{:04}", id - 1);
        }
        out.push_str("- accept: completes within NFR-3 budget\n");
        if id % 4 == 0 {
            out.push_str("- note (2026-01-15, alice): initial triage note for benchmark task\n");
        }
    }
    out
}

/// Runs `todo` with `args` in `work`, with `HOME` and `APPDATA` in `home` (TEST-8), and returns
/// its output and how long it took. The command must succeed.
fn run_todo(home: &Path, work: &Path, args: &[&str]) -> (Output, Duration) {
    let start = Instant::now();
    let output = common::todo_bin()
        .args(args)
        .current_dir(work)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("APPDATA", home)
        .output()
        .unwrap();
    let elapsed = start.elapsed();
    assert!(
        output.status.success(),
        "command {:?} failed: {}",
        args,
        stderr_text(&output)
    );
    (output, elapsed)
}

/// Median of five runs per command, so one slow outlier on a busy CI runner doesn't decide.
fn median(mut times: Vec<Duration>) -> Duration {
    times.sort();
    times[times.len() / 2]
}

/// Creates a temporary project with `TODO.md` = `body`; returns the temporary folder and the
/// `home` and `work` folders inside it.
fn project_with(body: &str) -> (TempDir, PathBuf, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let work = tmp.path().join("work");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&work).unwrap();
    fs::write(work.join("TODO.md"), body.as_bytes()).unwrap();
    (tmp, home, work)
}

#[test]
#[ignore = "Performance benchmark (NFR-3, TEST-9); run with: cargo test --release --test perf -- --ignored"]
fn perf_5000_tasks_list_show_and_write_meet_nfr3_budgets() {
    let body = generate_5000_task_file();
    assert!(
        body.len() >= 500_000,
        "expected ~600 KB file, got {} bytes",
        body.len()
    );
    let (_tmp, home, work) = project_with(&body);

    let run = |args: &[&str]| run_todo(&home, &work, args).1;

    // Warm up the file-system cache, plus one untimed write so no measured write pays the
    // one-off costs (creating the state folder, the first snapshot, antivirus scanning of new
    // files on CI runners). NFR-3 is about steady-state speed.
    let _ = run(&["show", "T-0001"]);
    let _ = run(&["note", "T-0001", "warm-up", "--agent", "perf-bot"]);

    let list_time = median((0..5).map(|_| run(&["list"])).collect());
    let show_time = median((0..5).map(|_| run(&["show", "T-2500"])).collect());
    let note_time = median(
        (0..5)
            .map(|_| {
                run(&[
                    "note",
                    "T-2500",
                    "perf benchmark note",
                    "--agent",
                    "perf-bot",
                ])
            })
            .collect(),
    );
    let write_time = median(
        (0..5)
            .map(|i| {
                let title = format!("Perf benchmark added task {i}");
                run(&[
                    "add",
                    "--title",
                    &title,
                    "--priority",
                    "P1",
                    "--agent",
                    "perf-bot",
                ])
            })
            .collect(),
    );

    eprintln!(
        "NFR-3 timings (5,000 tasks, {} bytes): list={:?}, show={:?}, note={:?}, add={:?}",
        body.len(),
        list_time,
        show_time,
        note_time,
        write_time
    );

    assert!(
        list_time < Duration::from_millis(100),
        "list took {:?} (budget < 100 ms)",
        list_time
    );
    assert!(
        show_time < Duration::from_millis(100),
        "show took {:?} (budget < 100 ms)",
        show_time
    );
    assert!(
        note_time < Duration::from_millis(200),
        "write (note) took {:?} (budget < 200 ms)",
        note_time
    );
    assert!(
        write_time < Duration::from_millis(200),
        "write (add) took {:?} (budget < 200 ms)",
        write_time
    );
}

#[test]
#[ignore = "Performance benchmark (NFR-3, TEST-9, T-105); run with: cargo test --release --test perf -- --ignored"]
fn perf_5000_entry_milestone_tree_meets_nfr3_budgets() {
    let body = generate_milestone_tree_file();
    assert!(
        body.len() >= 500_000,
        "expected ~600 KB file, got {} bytes",
        body.len()
    );
    assert_eq!(
        body.matches("\n### T-").count(),
        TOTAL_ENTRIES,
        "the variant must hold {TOTAL_ENTRIES} entries"
    );
    let (_tmp, home, work) = project_with(&body);

    // Untimed sanity check that the file really is four levels deep: `show` of a depth-4 task
    // prints three milestones above it, from the top down.
    let (shown, _) = run_todo(&home, &work, &["show", DEEP_TASK]);
    let chain = stdout_text(&shown)
        .lines()
        .skip_while(|line| !line.starts_with("Milestones above"))
        .skip(1)
        .take_while(|line| line.starts_with("- ") || line.starts_with("  goal:"))
        .filter(|line| line.starts_with("- "))
        .count();
    assert_eq!(
        chain, 3,
        "{DEEP_TASK} should have three milestones above it"
    );

    let run = |args: &[&str]| run_todo(&home, &work, args).1;

    // Warm-up as in the flat test. The first write also adds the tool stamp to the file.
    let _ = run(&["tree"]);
    let _ = run(&["note", DEEP_TASK, "warm-up", "--agent", "perf-bot"]);

    let tree_time = median((0..5).map(|_| run(&["tree"])).collect());
    let next_time = median((0..5).map(|_| run(&["next"])).collect());
    let list_time = median(
        (0..5)
            .map(|_| run(&["list", "--milestone", TOP_MILESTONE]))
            .collect(),
    );
    let show_time = median((0..5).map(|_| run(&["show", DEEP_TASK])).collect());
    let note_time = median(
        (0..5)
            .map(|_| {
                run(&[
                    "note",
                    DEEP_TASK,
                    "perf benchmark note",
                    "--agent",
                    "perf-bot",
                ])
            })
            .collect(),
    );
    let write_time = median(
        (0..5)
            .map(|i| {
                let title = format!("Perf benchmark added task under a milestone {i}");
                run(&[
                    "add",
                    "--title",
                    &title,
                    "--priority",
                    "P1",
                    "--milestone",
                    DEEP_MILESTONE,
                    "--agent",
                    "perf-bot",
                ])
            })
            .collect(),
    );

    eprintln!(
        "NFR-3 milestone-tree timings (5,000 entries, four levels, {} bytes): tree={:?}, next={:?}, \
         list --milestone={:?}, show={:?}, note={:?}, add --milestone={:?}",
        body.len(),
        tree_time,
        next_time,
        list_time,
        show_time,
        note_time,
        write_time
    );

    assert!(
        tree_time < Duration::from_millis(100),
        "tree took {:?} (budget < 100 ms)",
        tree_time
    );
    assert!(
        next_time < Duration::from_millis(100),
        "next took {:?} (budget < 100 ms)",
        next_time
    );
    assert!(
        list_time < Duration::from_millis(100),
        "list --milestone took {:?} (budget < 100 ms)",
        list_time
    );
    assert!(
        show_time < Duration::from_millis(100),
        "show took {:?} (budget < 100 ms)",
        show_time
    );
    assert!(
        note_time < Duration::from_millis(200),
        "write (note) took {:?} (budget < 200 ms)",
        note_time
    );
    assert!(
        write_time < Duration::from_millis(200),
        "write (add --milestone) took {:?} (budget < 200 ms)",
        write_time
    );
}
