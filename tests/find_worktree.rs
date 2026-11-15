//! Tests for linked git worktree resolution (T-011; R-FIND-5, NFR-5, TEST-6).

mod common;

use std::path::{Path, PathBuf};

use tempfile::TempDir;

use common::git;
use todo::error::{ErrorCode, ExitCode};
use todo::find::{FindOptions, FoundBy, WorktreeMode, find_todo};

fn canonical_no_verbatim(path: &Path) -> std::io::Result<PathBuf> {
    let p = std::fs::canonicalize(path)?;
    #[cfg(windows)]
    {
        if let Some(s) = p.to_str() {
            if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
                return Ok(PathBuf::from(format!(r"\\{rest}")));
            }
            if let Some(rest) = s.strip_prefix(r"\\?\") {
                return Ok(PathBuf::from(rest));
            }
        }
    }
    Ok(p)
}

#[test]
fn real_linked_worktree_resolves_to_main_checkout_and_subfolder() {
    let tmp = TempDir::new().unwrap();
    let main_repo = tmp.path().join("main-repo");
    let wt_dir = tmp.path().join("wt-feature");
    std::fs::create_dir_all(&main_repo).unwrap();

    git(&main_repo, &["init"]);
    let main_todo = main_repo.join("TODO.md");
    std::fs::write(
        &main_todo,
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n# Main\n",
    )
    .unwrap();
    let sub_rel = Path::new("crates").join("engine");
    std::fs::create_dir_all(main_repo.join(&sub_rel)).unwrap();
    std::fs::write(main_repo.join(&sub_rel).join("lib.rs"), "// fixture\n").unwrap();
    git(&main_repo, &["add", "."]);
    git(&main_repo, &["commit", "-m", "initial"]);

    git(
        &main_repo,
        &["worktree", "add", "-b", "feature", wt_dir.to_str().unwrap()],
    );

    let expected_main_todo = canonical_no_verbatim(&main_todo).unwrap();

    // From worktree root: resolves to main checkout's TODO.md.
    let from_wt_root = find_todo(&FindOptions {
        start_dir: Some(&wt_dir),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        canonical_no_verbatim(&from_wt_root.path).unwrap(),
        expected_main_todo
    );
    assert_eq!(from_wt_root.found_by, FoundBy::WorktreeMain);
    assert!(from_wt_root.warnings.is_empty());
    assert!(
        from_wt_root
            .notice
            .as_deref()
            .is_some_and(|n| n.contains("TODO.md")),
        "expected stderr notice naming the file, got {:?}",
        from_wt_root.notice
    );

    // From a subfolder inside the worktree: maps relative path onto main checkout.
    let wt_sub = wt_dir.join(&sub_rel);
    let from_wt_sub = find_todo(&FindOptions {
        start_dir: Some(&wt_sub),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        canonical_no_verbatim(&from_wt_sub.path).unwrap(),
        expected_main_todo
    );
    assert_eq!(from_wt_sub.found_by, FoundBy::WorktreeMain);

    // If a subfolder only exists in the worktree (new dir on feature branch), still walks up to main root.
    let wt_new_sub = wt_dir.join("only-in-worktree").join("nested");
    std::fs::create_dir_all(&wt_new_sub).unwrap();
    let from_new_sub = find_todo(&FindOptions {
        start_dir: Some(&wt_new_sub),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        canonical_no_verbatim(&from_new_sub.path).unwrap(),
        expected_main_todo
    );
    assert_eq!(from_new_sub.found_by, FoundBy::WorktreeMain);

    // With worktree = "local", uses the worktree's own TODO.md without warning.
    let expected_wt_todo = canonical_no_verbatim(&wt_dir.join("TODO.md")).unwrap();
    let local_mode = find_todo(&FindOptions {
        start_dir: Some(&wt_sub),
        worktree: WorktreeMode::Local,
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        canonical_no_verbatim(&local_mode.path).unwrap(),
        expected_wt_todo
    );
    assert_eq!(local_mode.found_by, FoundBy::Walk);
    assert!(local_mode.warnings.is_empty());
    assert!(local_mode.notice.is_none());
}

#[test]
fn linked_worktree_stops_at_main_checkout_when_main_has_no_todo() {
    let tmp = TempDir::new().unwrap();
    // Put a decoy TODO.md above main_repo; search must never escape main_repo.
    std::fs::write(
        tmp.path().join("TODO.md"),
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n# Outer\n",
    )
    .unwrap();

    let main_repo = tmp.path().join("main-repo");
    let wt_dir = tmp.path().join("wt-no-todo");
    std::fs::create_dir_all(&main_repo).unwrap();
    git(&main_repo, &["init"]);
    std::fs::write(main_repo.join("README.md"), "# Test\n").unwrap();
    git(&main_repo, &["add", "."]);
    git(&main_repo, &["commit", "-m", "init"]);

    git(
        &main_repo,
        &["worktree", "add", "-b", "feature", wt_dir.to_str().unwrap()],
    );

    // Even if the worktree itself has an untracked/stale TODO.md, shared mode searches main checkout.
    std::fs::write(
        wt_dir.join("TODO.md"),
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n# Stale\n",
    )
    .unwrap();

    let err = find_todo(&FindOptions {
        start_dir: Some(&wt_dir),
        ..Default::default()
    })
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::NoFile);
    assert_eq!(err.exit_code, ExitCode::FileError);
}

#[test]
fn bare_repo_worktree_falls_back_to_local_file_with_warning() {
    let tmp = TempDir::new().unwrap();
    let seed = tmp.path().join("seed");
    let bare = tmp.path().join("bare.git");
    let wt = tmp.path().join("wt-from-bare");
    std::fs::create_dir_all(&seed).unwrap();

    git(&seed, &["init"]);
    std::fs::write(
        seed.join("TODO.md"),
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n# Seed\n",
    )
    .unwrap();
    git(&seed, &["add", "."]);
    git(&seed, &["commit", "-m", "seed"]);

    git(
        tmp.path(),
        &[
            "clone",
            "--bare",
            seed.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    git(&bare, &["worktree", "add", wt.to_str().unwrap()]);

    let found = find_todo(&FindOptions {
        start_dir: Some(&wt),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        canonical_no_verbatim(&found.path).unwrap(),
        canonical_no_verbatim(&wt.join("TODO.md")).unwrap()
    );
    assert_eq!(found.found_by, FoundBy::WorktreeFallback);
    assert!(
        found.warnings.iter().any(|w| w.contains("bare")),
        "expected bare-repo fallback warning, got {:?}",
        found.warnings
    );
}

#[test]
fn unusual_git_file_layout_falls_back_with_warning() {
    let tmp = TempDir::new().unwrap();
    let wt = tmp.path().join("weird-wt");
    let fake_gitdir = tmp.path().join("elsewhere");
    std::fs::create_dir_all(&wt).unwrap();
    std::fs::create_dir_all(&fake_gitdir).unwrap();
    // Missing `commondir` inside fake_gitdir.
    std::fs::write(
        wt.join(".git"),
        format!("gitdir: {}\n", fake_gitdir.display()),
    )
    .unwrap();
    let wt_todo = wt.join("TODO.md");
    std::fs::write(
        &wt_todo,
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n# Weird\n",
    )
    .unwrap();

    let found = find_todo(&FindOptions {
        start_dir: Some(&wt),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(found.path, wt_todo);
    assert_eq!(found.found_by, FoundBy::WorktreeFallback);
    assert!(!found.warnings.is_empty());
}
