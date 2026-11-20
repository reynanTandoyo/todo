//! What `done --commit` asks git (R-SCOPE-2, NFR-5): whether the folder of `TODO.md` is inside a
//! repository, and which files a commit changed. Git runs before the write lock is taken; the answer
//! is checked against the task's `files` under the lock (`ops::scope::check_done_commit`).
//!
//! git is used only when it is on `PATH`. Without it, or outside a repository, the answer is
//! [`CommitScope::NoGit`] and nothing is checked.

use std::path::Path;
use std::process::Command;

use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::scope::{CommitScope, same_path};

/// Runs `git <args>` in `dir`; its standard output when git ran and succeeded, otherwise `None`.
fn git_stdout(dir: Option<&Path>, args: &[&str]) -> Option<Vec<u8>> {
    let mut cmd = Command::new("git");
    if let Some(d) = dir {
        cmd.current_dir(d);
    }
    let output = cmd.args(args).output().ok()?;
    output.status.success().then_some(output.stdout)
}

/// Whether git can tell about the folder `dir`: git runs there and the folder is inside a repository.
#[must_use]
pub fn in_git_repository(dir: Option<&Path>) -> bool {
    git_stdout(dir, &["rev-parse", "--git-dir"]).is_some()
}

/// What `done` needs from git before the lock (R-SCOPE-2) for the `commit` it was given: any name git
/// knows, such as a hash or `HEAD`. Without git the answer is [`CommitScope::NoGit`]; without a
/// commit, [`CommitScope::NoCommit`]. Otherwise it lists the files the commit changed, except the
/// files that are always allowed (`always_allowed`, such as `TODO.md` and the archive file).
pub fn commit_scope(
    dir: Option<&Path>,
    commit: Option<&str>,
    always_allowed: &[&str],
) -> Result<CommitScope> {
    if !in_git_repository(dir) {
        return Ok(CommitScope::NoGit);
    }
    let Some(commit) = commit else {
        return Ok(CommitScope::NoCommit);
    };
    let changed = changed_files(dir, commit)?
        .into_iter()
        .filter(|path| !always_allowed.iter().any(|name| same_path(path, name)))
        .collect();
    Ok(CommitScope::Changed(changed))
}

/// [`commit_scope`] for the TODO.md at `todo_file`, as `done` and the `done` operations of `apply`
/// ask it (R-SCOPE-2): `TODO.md` itself and the archive file, `archive_file`, are always allowed.
pub fn scope_of_commit(
    todo_file: &Path,
    archive_file: &str,
    commit: Option<&str>,
) -> Result<CommitScope> {
    let folder = todo_file.parent().filter(|p| !p.as_os_str().is_empty());
    let todo_name = todo_file
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    commit_scope(folder, commit, &[todo_name.as_str(), archive_file])
}

/// The files that `commit` changed compared with its first parent (for a root commit, the files it
/// added). Both paths of a rename count. The paths are relative to the folder `dir`, with `/`
/// separators; a path outside that folder starts with `../`.
pub fn changed_files(dir: Option<&Path>, commit: &str) -> Result<Vec<String>> {
    let unreadable = || {
        TodoError::new(
            ErrorCode::BadValue,
            format!("could not list the files of git commit '{commit}'"),
        )
    };
    // The same refusal as `done` gives for a commit git does not know (`R-WRITE-4`).
    let unknown = || {
        TodoError::new(
            ErrorCode::BadValue,
            format!("unknown git commit '{commit}'"),
        )
    };
    if commit.starts_with('-') {
        return Err(unknown());
    }
    let prefix = git_stdout(dir, &["rev-parse", "--show-prefix"]).ok_or_else(unreadable)?;
    let prefix = String::from_utf8_lossy(&prefix)
        .trim_end_matches(['\r', '\n'])
        .to_string();
    // `commit` then its parents; the first parent is the second word.
    let parents =
        git_stdout(dir, &["rev-list", "--parents", "-n", "1", commit, "--"]).ok_or_else(unknown)?;
    let first_parent = String::from_utf8_lossy(&parents)
        .split_whitespace()
        .nth(1)
        .map(str::to_string);
    let names = match first_parent.as_deref() {
        Some(parent) => git_stdout(
            dir,
            &[
                "diff-tree",
                "-r",
                "-z",
                "--name-only",
                "--no-renames",
                "--no-commit-id",
                parent,
                commit,
            ],
        ),
        None => git_stdout(
            dir,
            &[
                "diff-tree",
                "-r",
                "-z",
                "--name-only",
                "--no-renames",
                "--no-commit-id",
                "--root",
                commit,
            ],
        ),
    }
    .ok_or_else(unreadable)?;
    Ok(String::from_utf8_lossy(&names)
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(|path| relative_to_folder(&prefix, path))
        .collect())
}

/// `path`, a path from the top of the repository with `/` separators, as a path relative to the
/// folder whose position in the repository is `prefix` (`proj/`, or empty at the top itself).
fn relative_to_folder(prefix: &str, path: &str) -> String {
    let folder: Vec<&str> = prefix.split('/').filter(|part| !part.is_empty()).collect();
    let parts: Vec<&str> = path.split('/').collect();
    let common = folder
        .iter()
        .zip(&parts)
        .take_while(|(a, b)| a == b)
        .count();
    let mut out: Vec<&str> = vec![".."; folder.len() - common];
    out.extend_from_slice(&parts[common..]);
    out.join("/")
}

#[cfg(test)]
mod tests {
    use super::relative_to_folder;

    #[test]
    fn repository_paths_become_paths_of_the_folder_of_todo_md() {
        assert_eq!(relative_to_folder("proj/", "proj/src/a.rs"), "src/a.rs");
        assert_eq!(relative_to_folder("proj/", "other/x.txt"), "../other/x.txt");
        assert_eq!(relative_to_folder("", "other/x.txt"), "other/x.txt");
        assert_eq!(relative_to_folder("a/b/c/", "a/b/x"), "../x");
        assert_eq!(relative_to_folder("a/b/", "a/b/c/d"), "c/d");
    }
}
