//! Tests for finding TODO.md (T-010, T-011; R-FIND-1..6, NFR-7, TEST-6).

use std::path::{Path, PathBuf};

use tempfile::TempDir;
use todo::error::{ErrorCode, ExitCode};
use todo::find::{ExplicitSource, FindOptions, FoundBy, WorktreeMode, find_todo};

fn write_todo(dir: &Path) -> std::io::Result<PathBuf> {
    let path = dir.join("TODO.md");
    std::fs::write(
        &path,
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n# Tasks\n",
    )?;
    Ok(path)
}

#[cfg(windows)]
fn long_safe_create_dir_all(path: &Path) -> std::io::Result<()> {
    let abs = std::path::absolute(path)?;
    let s = abs.to_string_lossy();
    if s.encode_utf16().count() >= 240 && !s.starts_with(r"\\?\") {
        let verbatim = PathBuf::from(format!(r"\\?\{s}"));
        std::fs::create_dir_all(verbatim)
    } else {
        std::fs::create_dir_all(path)
    }
}

#[cfg(not(windows))]
fn long_safe_create_dir_all(path: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path)
}

#[cfg(windows)]
fn long_safe_write(path: &Path, content: &str) -> std::io::Result<()> {
    let abs = std::path::absolute(path)?;
    let s = abs.to_string_lossy();
    if s.encode_utf16().count() >= 240 && !s.starts_with(r"\\?\") {
        let verbatim = PathBuf::from(format!(r"\\?\{s}"));
        std::fs::write(verbatim, content)
    } else {
        std::fs::write(path, content)
    }
}

#[cfg(not(windows))]
fn long_safe_write(path: &Path, content: &str) -> std::io::Result<()> {
    std::fs::write(path, content)
}

#[test]
fn finds_todo_at_root_and_from_subfolder() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("repo");
    let sub = root.join("crates").join("core").join("src");
    std::fs::create_dir_all(root.join(".git")).unwrap();
    std::fs::create_dir_all(&sub).unwrap();
    let expected = write_todo(&root).unwrap();

    let from_root = find_todo(&FindOptions {
        start_dir: Some(&root),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(from_root.path, expected);
    assert_eq!(from_root.found_by, FoundBy::Walk);
    assert_eq!(from_root.repo_root.as_deref(), Some(root.as_path()));

    let from_sub = find_todo(&FindOptions {
        start_dir: Some(&sub),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(from_sub.path, expected);
    assert_eq!(from_sub.found_by, FoundBy::Walk);
}

#[test]
fn stops_at_git_repository_root_and_does_not_pick_parent_todo() {
    let tmp = TempDir::new().unwrap();
    let outer = tmp.path();
    write_todo(outer).unwrap();

    let inner_repo = outer.join("child-repo");
    let inner_sub = inner_repo.join("nested").join("dir");
    std::fs::create_dir_all(inner_repo.join(".git")).unwrap();
    std::fs::create_dir_all(&inner_sub).unwrap();

    let err = find_todo(&FindOptions {
        start_dir: Some(&inner_sub),
        ..Default::default()
    })
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::NoFile);
    assert_eq!(err.exit_code, ExitCode::FileError);
    assert!(
        err.message.contains("todo init"),
        "expected suggestion for `todo init`, got: {}",
        err.message
    );
}

#[test]
fn outside_git_walks_up_to_parent_todo() {
    let tmp = TempDir::new().unwrap();
    let parent = tmp.path().join("workspace");
    let child = parent.join("a").join("b");
    std::fs::create_dir_all(&child).unwrap();
    let expected = write_todo(&parent).unwrap();

    let found = find_todo(&FindOptions {
        start_dir: Some(&child),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(found.path, expected);
    assert_eq!(found.found_by, FoundBy::Walk);
    assert!(found.repo_root.is_none());
}

#[test]
fn explicit_file_flag_and_env_skip_search() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(repo.join(".git")).unwrap();
    write_todo(&repo).unwrap();

    let custom_dir = tmp.path().join("other");
    std::fs::create_dir_all(&custom_dir).unwrap();
    let custom_file = custom_dir.join("custom-tasks.md");
    std::fs::write(
        &custom_file,
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n# Custom\n",
    )
    .unwrap();

    let by_flag = find_todo(&FindOptions {
        explicit_file: Some(&custom_file),
        explicit_source: Some(ExplicitSource::Flag),
        start_dir: Some(&repo),
        worktree: WorktreeMode::Shared,
    })
    .unwrap();
    assert_eq!(by_flag.path, custom_file);
    assert_eq!(by_flag.found_by, FoundBy::Flag);

    let by_env = find_todo(&FindOptions {
        explicit_file: Some(&custom_file),
        explicit_source: Some(ExplicitSource::Env),
        start_dir: Some(&repo),
        worktree: WorktreeMode::Shared,
    })
    .unwrap();
    assert_eq!(by_env.path, custom_file);
    assert_eq!(by_env.found_by, FoundBy::Env);

    let missing = custom_dir.join("does-not-exist.md");
    let err = find_todo(&FindOptions {
        explicit_file: Some(&missing),
        explicit_source: Some(ExplicitSource::Flag),
        start_dir: Some(&repo),
        worktree: WorktreeMode::Shared,
    })
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::NoFile);
    assert_eq!(err.exit_code, ExitCode::FileError);
}

#[test]
fn paths_with_spaces_non_ascii_and_over_260_chars_work() {
    let tmp = TempDir::new().unwrap();
    let special = tmp.path().join("project with spaces").join("日本語 — äöü");
    std::fs::create_dir_all(special.join(".git")).unwrap();
    let expected_special = write_todo(&special).unwrap();

    let found_special = find_todo(&FindOptions {
        start_dir: Some(&special),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(found_special.path, expected_special);

    // Build a path well over 260 characters.
    let mut deep = tmp.path().join("long-root");
    long_safe_create_dir_all(&deep.join(".git")).unwrap();
    for i in 0..12 {
        deep = deep.join(format!("segment_{i:02}_with_padding_chars"));
    }
    let sub = deep.join("leaf_dir");
    long_safe_create_dir_all(&sub).unwrap();
    assert!(
        sub.to_string_lossy().len() > 280,
        "expected path > 280 chars, got {}",
        sub.to_string_lossy().len()
    );

    let deep_todo = deep.join("TODO.md");
    long_safe_write(
        &deep_todo,
        "<!-- TEST FIXTURE: invented data, not a real task list -->\n# Deep\n",
    )
    .unwrap();

    let found_deep = find_todo(&FindOptions {
        start_dir: Some(&sub),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(found_deep.path, std::path::absolute(&deep_todo).unwrap());
}
