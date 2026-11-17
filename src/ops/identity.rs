//! Who finished a task: the `by` field of `done` (`R-WRITE-16`) and the `.mailmap` check.
//!
//! Everything here is best effort. When `git` is missing, the folder is not in a repository or no
//! name is set, the answer is `None` and the caller simply leaves `by` out (`NFR-5`). An email
//! address is only ever handed to `git check-mailmap`; it is never returned.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Runs `git <args>` in `dir`; returns trimmed standard output when git ran and succeeded.
fn git_output(dir: Option<&Path>, args: &[&str]) -> Option<String> {
    let mut cmd = Command::new("git");
    if let Some(d) = dir {
        cmd.current_dir(d);
    }
    let output = cmd.args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

/// The author name of `commit` as git shows it with `.mailmap` applied (`%aN`).
#[must_use]
pub fn commit_author_name(dir: Option<&Path>, commit: &str) -> Option<String> {
    let commit = commit.trim();
    if commit.is_empty() || commit.starts_with('-') {
        return None;
    }
    git_output(dir, &["log", "-1", "--format=%aN", commit, "--"])
}

/// The configured git identity of the repository at `dir` (`user.name`), mapped through
/// `.mailmap` with `user.email`. `None` outside a repository or without `user.name`.
#[must_use]
pub fn configured_author_name(dir: Option<&Path>) -> Option<String> {
    git_output(dir, &["rev-parse", "--git-dir"])?;
    let name = git_output(dir, &["config", "user.name"])?;
    let Some(email) = git_output(dir, &["config", "user.email"]) else {
        return Some(name);
    };
    let contact = format!("{name} <{email}>");
    match git_output(dir, &["check-mailmap", &contact]) {
        Some(mapped) => Some(name_part(&mapped)),
        None => Some(name),
    }
}

/// The name before the `<email>` of a `Name <email>` string.
fn name_part(s: &str) -> String {
    match s.rfind('<') {
        Some(i) => s[..i].trim().to_string(),
        None => s.trim().to_string(),
    }
}

/// The `by` value for a task being marked done: the author of `commit` when one is given,
/// otherwise the configured git identity. Never fails (`R-WRITE-16`).
#[must_use]
pub fn done_by(dir: Option<&Path>, commit: Option<&str>) -> Option<String> {
    match commit {
        Some(c) => commit_author_name(dir, c),
        None => configured_author_name(dir),
    }
}

/// The nearest folder at or above `start` that holds a `.git` entry (the repository root).
#[must_use]
pub fn repo_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|p| p.join(".git").exists())
        .map(Path::to_path_buf)
}

/// The proper names (left side) listed in `.mailmap` text, lines as in git's own format:
/// `Proper Name <proper@email> [Commit Name] <commit@email>`. `#` starts a comment.
#[must_use]
pub fn mailmap_proper_names(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let name = line.split('<').next()?.trim();
            if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mailmap_names() {
        let text = "# comment\n\nAda L <ada@x.org> <a@old>\nAda L <ada@x.org> Ada Lovelace <al@home>\n<only@email> <o@x>\nBob <bob@x.org>\n";
        assert_eq!(mailmap_proper_names(text), vec!["Ada L", "Ada L", "Bob"]);
    }

    #[test]
    fn name_part_drops_email() {
        assert_eq!(name_part("Ada L <ada@x.org>"), "Ada L");
        assert_eq!(name_part("Ada L"), "Ada L");
    }
}
