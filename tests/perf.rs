//! Performance test on a generated 5,000-task `TODO.md` (T-065; NFR-3, TEST-9).
//!
//! Run in release mode with:
//! `cargo test --release --test perf -- --ignored`

use std::fmt::Write as _;
use std::fs;
use std::process::Command;
use std::time::{Duration, Instant};

use tempfile::TempDir;

fn generate_5000_task_file() -> String {
    let mut out = String::with_capacity(700 * 1024);
    out.push_str("<!-- TEST FIXTURE: invented data, not a real task list -->\n");
    out.push_str("## Now\n");
    for i in 1..=5000usize {
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
- priority: {prio} · type: task · area: core\n\
- accept: completes within NFR-3 budget"
        );
    }
    out
}

#[test]
#[ignore = "Performance benchmark (NFR-3, TEST-9); run with: cargo test --release --test perf -- --ignored"]
fn perf_5000_tasks_list_show_and_write_meet_nfr3_budgets() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let work = tmp.path().join("work");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&work).unwrap();

    let body = generate_5000_task_file();
    assert!(
        body.len() >= 500_000,
        "expected ~600 KB file, got {} bytes",
        body.len()
    );
    fs::write(work.join("TODO.md"), body.as_bytes()).unwrap();

    let bin = env!("CARGO_BIN_EXE_todo");

    let run = |args: &[&str]| -> Duration {
        let start = Instant::now();
        let output = Command::new(bin)
            .args(args)
            .current_dir(&work)
            .env("HOME", &home)
            .env("USERPROFILE", &home)
            .env("APPDATA", &home)
            .env_remove("TODO_FILE")
            .env_remove("TODO_AGENT")
            .output()
            .unwrap();
        let elapsed = start.elapsed();
        assert!(
            output.status.success(),
            "command {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        elapsed
    };

    // Warm up filesystem cache once.
    let _ = run(&["show", "T-0001"]);

    let list_time = run(&["list"]);
    let show_time = run(&["show", "T-2500"]);
    let write_time = run(&[
        "add",
        "--title",
        "Perf benchmark added task",
        "--priority",
        "P1",
        "--agent",
        "perf-bot",
    ]);

    eprintln!(
        "NFR-3 timings (5,000 tasks, {} bytes): list={:?}, show={:?}, add={:?}",
        body.len(),
        list_time,
        show_time,
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
        write_time < Duration::from_millis(200),
        "write (add) took {:?} (budget < 200 ms)",
        write_time
    );
}
