//! Tests for the per-file state folder and state JSON helpers (T-012; R-HIST-1, R-LOCK-7).

mod common;

use std::path::Path;

use tempfile::TempDir;

use common::{git, parse_json, run_todo, setup_proj, stderr_text};
use todo::atomic::read_file_with_snapshot;
use todo::find::{ExplicitSource, FindOptions, find_todo};
use todo::state::{IdsState, LastWrite, Lease, LockHolder, StateDir};

#[test]
fn main_checkout_worktree_and_different_path_spellings_share_state_dir() {
    let tmp = TempDir::new().unwrap();
    let main_repo = tmp.path().join("main-repo");
    let wt_dir = tmp.path().join("wt-1");
    let sub = main_repo.join("nested").join("dir");
    std::fs::create_dir_all(&sub).unwrap();

    git(&main_repo, &["init"]);
    let main_todo = main_repo.join("TODO.md");
    std::fs::write(
        &main_todo,
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n# Main\n",
    )
    .unwrap();
    git(&main_repo, &["add", "."]);
    git(&main_repo, &["commit", "-m", "init"]);
    git(
        &main_repo,
        &["worktree", "add", "-b", "wt", wt_dir.to_str().unwrap()],
    );

    let found_main = find_todo(&FindOptions {
        start_dir: Some(&main_repo),
        ..Default::default()
    })
    .unwrap();
    let found_wt = find_todo(&FindOptions {
        start_dir: Some(&wt_dir),
        ..Default::default()
    })
    .unwrap();
    // Relative spelling from a nested subfolder: `..\..\TODO.md`.
    let rel_spelling = Path::new("..").join("..").join("TODO.md");
    let found_rel = find_todo(&FindOptions {
        explicit_file: Some(&rel_spelling),
        explicit_source: Some(ExplicitSource::Flag),
        start_dir: Some(&sub),
        ..Default::default()
    })
    .unwrap();

    let state_main = StateDir::from_found(&found_main).unwrap();
    let state_wt = StateDir::from_found(&found_wt).unwrap();
    let state_rel = StateDir::from_found(&found_rel).unwrap();

    assert!(state_main.is_in_git());
    assert_eq!(state_main.path(), state_wt.path());
    assert_eq!(state_main.path(), state_rel.path());

    // State directory is created on demand, not during `from_found`.
    assert!(!state_main.path().exists());

    // Different letter casing on Windows must also resolve to the same state dir.
    #[cfg(windows)]
    {
        let upper_or_lower = main_todo.to_string_lossy().to_lowercase();
        let found_case = find_todo(&FindOptions {
            explicit_file: Some(Path::new(&upper_or_lower)),
            explicit_source: Some(ExplicitSource::Flag),
            start_dir: Some(&main_repo),
            ..Default::default()
        })
        .unwrap();
        let state_case = StateDir::from_found(&found_case).unwrap();
        assert_eq!(state_main.path(), state_case.path());
    }
}

#[test]
fn outside_git_creates_dot_todo_with_gitignore_on_demand() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("standalone");
    std::fs::create_dir_all(&dir).unwrap();
    let todo_file = dir.join("TODO.md");
    std::fs::write(
        &todo_file,
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n# Standalone\n",
    )
    .unwrap();

    let found = find_todo(&FindOptions {
        start_dir: Some(&dir),
        ..Default::default()
    })
    .unwrap();
    let state = StateDir::from_found(&found).unwrap();
    assert!(!state.is_in_git());
    // Compare canonically: temp folders on CI runners can use 8.3 short names
    // (C:\Users\RUNNER~1\...), while the state folder is resolved to the long path.
    assert_eq!(
        state.path().file_name(),
        Some(std::ffi::OsStr::new(".todo"))
    );
    assert_eq!(
        std::fs::canonicalize(state.path().parent().unwrap()).unwrap(),
        std::fs::canonicalize(&dir).unwrap()
    );
    assert!(!state.path().exists());

    state.ensure_created().unwrap();
    assert!(state.path().is_dir());
    let gitignore = std::fs::read_to_string(state.path().join(".gitignore")).unwrap();
    assert_eq!(gitignore.trim(), "*");
}

#[test]
fn state_json_files_round_trip_atomically() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("proj");
    std::fs::create_dir_all(&dir).unwrap();
    let todo_file = dir.join("TODO.md");
    std::fs::write(
        &todo_file,
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n# State\n",
    )
    .unwrap();

    let found = find_todo(&FindOptions {
        start_dir: Some(&dir),
        ..Default::default()
    })
    .unwrap();
    let state = StateDir::from_found(&found).unwrap();

    assert!(state.read_holder().unwrap().is_none());
    assert!(state.read_lease().unwrap().is_none());
    assert!(state.read_ids().unwrap().is_none());
    assert!(state.read_last_write().unwrap().is_none());

    let holder = LockHolder {
        agent: "agent-a".to_owned(),
        command: "claim".to_owned(),
        pid: 1234,
        started_at: "2026-11-15T14:00:00+01:00".to_owned(),
    };
    state.write_holder(&holder).unwrap();
    assert_eq!(state.read_holder().unwrap().as_ref(), Some(&holder));
    state.remove_holder().unwrap();
    assert!(state.read_holder().unwrap().is_none());

    let lease = Lease {
        agent: "human-1".to_owned(),
        started_at: "2026-11-15T14:00:00+01:00".to_owned(),
        expires_at: "2026-11-15T14:10:00+01:00".to_owned(),
    };
    state.write_lease(&lease).unwrap();
    assert_eq!(state.read_lease().unwrap().as_ref(), Some(&lease));
    state.remove_lease().unwrap();
    assert!(state.read_lease().unwrap().is_none());

    let ids = IdsState { highest_id: 42 };
    state.write_ids(&ids).unwrap();
    assert_eq!(state.read_ids().unwrap(), Some(ids));

    let snap = read_file_with_snapshot(&todo_file).unwrap().snapshot;
    let lw = LastWrite::from_snapshot(&snap);
    state.write_last_write(&lw).unwrap();
    assert_eq!(state.read_last_write().unwrap().as_ref(), Some(&lw));

    // Verify no temporary files remain in the state directory.
    let tmp_count = std::fs::read_dir(state.path())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
        .count();
    assert_eq!(tmp_count, 0);
}

#[test]
fn status_reports_the_write_lock_free_when_the_state_folder_has_no_lock_file() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo_file) = setup_proj(
        &tmp,
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n# Status without lock file\n",
    );
    // The state folder exists, but no command has locked yet, so it holds no write.lock.
    let state_dir = work.join(".todo");
    std::fs::create_dir_all(&state_dir).unwrap();
    let lock_file = state_dir.join("write.lock");
    assert!(!lock_file.exists());

    let out = run_todo(&home, &work, &["--json", "status"], &[]).unwrap();
    assert!(out.status.success(), "status failed: {}", stderr_text(&out));
    let parsed = parse_json(&out.stdout);
    assert_eq!(parsed["schema"], "todo.status.v1");
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["data"]["write_lock"]["held"], false);
    assert!(parsed["data"]["lease"].is_null());

    // Status reads the lock without taking it, so it creates no write.lock (R-LOCK-5).
    assert!(!lock_file.exists(), "status created write.lock");
}
