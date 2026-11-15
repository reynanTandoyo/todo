//! Finding TODO.md, including linked git worktrees (T-010, T-011; R-FIND-1..7).

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use crate::error::{ErrorCode, Result, TodoError};

/// File name searched by default when walking directories.
pub const TODO_FILENAME: &str = "TODO.md";

/// How linked git worktrees should be resolved (`worktree` setting in `todo.toml`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorktreeMode {
    /// Linked worktrees search in the main checkout instead (default, R-FIND-5).
    #[default]
    Shared,
    /// Each worktree uses its own local `TODO.md` (`worktree = "local"`).
    Local,
}

/// Which configuration source provided an explicit `--file` path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplicitSource {
    /// Provided via `--file <path>` (R-FIND-1).
    Flag,
    /// Provided via the `TODO_FILE` environment variable (R-FIND-2).
    Env,
}

/// How the active `TODO.md` file was located (for `todo where`, R-FIND-7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoundBy {
    /// Specified via `--file <path>` (R-FIND-1).
    Flag,
    /// Specified via `TODO_FILE` environment variable (R-FIND-2).
    Env,
    /// Found by walking up directories from the starting directory (R-FIND-3).
    Walk,
    /// Mapped from a linked git worktree to the main checkout (R-FIND-5).
    WorktreeMain,
    /// Linked git worktree where main checkout could not be found; fell back to local file (R-FIND-5).
    WorktreeFallback,
}

impl FoundBy {
    /// Returns a short, stable label describing how the file was found.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Flag => "flag",
            Self::Env => "env",
            Self::Walk => "walk",
            Self::WorktreeMain => "worktree-main",
            Self::WorktreeFallback => "worktree-fallback",
        }
    }
}

/// Options controlling how [`find_todo`] locates `TODO.md`.
#[derive(Debug, Clone, Default)]
pub struct FindOptions<'a> {
    /// Explicit path from `--file` or `TODO_FILE`, skipping directory search.
    pub explicit_file: Option<&'a Path>,
    /// Source of `explicit_file` (`Flag` or `Env`). Defaults to `Flag` when omitted.
    pub explicit_source: Option<ExplicitSource>,
    /// Starting directory for the upward search. Defaults to `std::env::current_dir()`.
    pub start_dir: Option<&'a Path>,
    /// How linked git worktrees should be handled.
    pub worktree: WorktreeMode,
}

/// Result of locating `TODO.md`, together with git repository context and notices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundTodo {
    /// Path to the resolved `TODO.md` file.
    pub path: PathBuf,
    /// How the file was found.
    pub found_by: FoundBy,
    /// Root of the git working tree containing the resolved file, if inside a git repository.
    pub repo_root: Option<PathBuf>,
    /// Path to the git directory for this checkout (`.git` dir or worktree gitdir), if any.
    pub git_dir: Option<PathBuf>,
    /// Path to the git common directory (shared across worktrees), if inside a git repository.
    pub git_common_dir: Option<PathBuf>,
    /// Informational notice to print to standard error in text mode (e.g. R-FIND-5).
    pub notice: Option<String>,
    /// Warnings generated during discovery (e.g. R-FIND-5 fallback).
    pub warnings: Vec<String>,
}

/// Converts a path to a form safe for Windows file-system APIs even past 260 characters (NFR-7).
#[cfg(windows)]
pub(crate) fn fs_path(path: &Path) -> Cow<'_, Path> {
    let s = path.as_os_str();
    if let Some(str_val) = s.to_str()
        && (str_val.starts_with(r"\\?\") || str_val.starts_with(r"\\.\"))
    {
        return Cow::Borrowed(path);
    }
    let abs = match std::path::absolute(path) {
        Ok(p) => p,
        Err(_) => return Cow::Borrowed(path),
    };
    let abs_str = abs.to_string_lossy();
    let utf16_len = abs_str.encode_utf16().count();
    if utf16_len < 240 {
        return Cow::Borrowed(path);
    }
    if let Some(rest) = abs_str.strip_prefix(r"\\") {
        Cow::Owned(PathBuf::from(format!(r"\\?\UNC\{rest}")))
    } else {
        Cow::Owned(PathBuf::from(format!(r"\\?\{abs_str}")))
    }
}

/// On non-Windows targets, paths are passed through unchanged.
#[cfg(not(windows))]
pub(crate) fn fs_path(path: &Path) -> Cow<'_, Path> {
    Cow::Borrowed(path)
}

/// Strips the Windows `\\?\` verbatim prefix for clean user-facing paths and uniform comparisons.
#[must_use]
pub(crate) fn strip_verbatim_prefix(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(s) = path.to_str() {
            if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
                return PathBuf::from(format!(r"\\{rest}"));
            }
            if let Some(rest) = s.strip_prefix(r"\\?\") {
                return PathBuf::from(rest);
            }
        }
    }
    path.to_path_buf()
}

fn path_is_file(path: &Path) -> bool {
    std::fs::metadata(fs_path(path).as_ref()).is_ok_and(|m| m.is_file())
}

fn path_is_dir(path: &Path) -> bool {
    std::fs::metadata(fs_path(path).as_ref()).is_ok_and(|m| m.is_dir())
}

fn path_exists(path: &Path) -> bool {
    std::fs::symlink_metadata(fs_path(path).as_ref()).is_ok()
}

fn absolute_clean(path: &Path, base: &Path) -> Result<PathBuf> {
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    };
    let abs = std::path::absolute(&joined).map_err(|e| {
        TodoError::new(
            ErrorCode::NoFile,
            format!("could not resolve path {}: {e}", joined.display()),
        )
    })?;
    Ok(strip_verbatim_prefix(&abs))
}

fn no_file_error(searched_from: &Path) -> TodoError {
    TodoError::new(
        ErrorCode::NoFile,
        format!(
            "no TODO.md found searching from {} — run `todo init` to create one",
            searched_from.display()
        ),
    )
}

/// Finds `TODO.md` according to R-FIND-1 through R-FIND-6.
pub fn find_todo(opts: &FindOptions<'_>) -> Result<FoundTodo> {
    let cwd = match opts.start_dir {
        Some(dir) => {
            let abs = std::path::absolute(dir).map_err(|e| {
                TodoError::new(
                    ErrorCode::NoFile,
                    format!("invalid starting directory {}: {e}", dir.display()),
                )
            })?;
            strip_verbatim_prefix(&abs)
        }
        None => {
            let cur = std::env::current_dir().map_err(|e| {
                TodoError::new(
                    ErrorCode::NoFile,
                    format!("could not determine current directory: {e}"),
                )
            })?;
            strip_verbatim_prefix(&cur)
        }
    };

    if let Some(explicit) = opts.explicit_file {
        let resolved = absolute_clean(explicit, &cwd)?;
        if !path_is_file(&resolved) {
            return Err(TodoError::new(
                ErrorCode::NoFile,
                format!(
                    "TODO file not found: {} — run `todo init` to create one",
                    resolved.display()
                ),
            )
            .with_file(resolved.display().to_string()));
        }
        let found_by = match opts.explicit_source.unwrap_or(ExplicitSource::Flag) {
            ExplicitSource::Flag => FoundBy::Flag,
            ExplicitSource::Env => FoundBy::Env,
        };
        let git_ctx = resolved
            .parent()
            .and_then(find_git_root_upward)
            .map(|root| inspect_git_root(&root));

        let (repo_root, git_dir, git_common_dir) = match git_ctx {
            Some(GitContext::Normal { root, dot_git }) => {
                (Some(root), Some(dot_git.clone()), Some(dot_git))
            }
            Some(GitContext::UnknownDotGit { root }) => (Some(root), None, None),
            None => (None, None, None),
        };

        return Ok(FoundTodo {
            path: resolved,
            found_by,
            repo_root,
            git_dir,
            git_common_dir,
            notice: None,
            warnings: Vec::new(),
        });
    }

    // Walk up from `cwd` to find `TODO.md`, stopping at the first `.git` repository root (R-FIND-3, R-FIND-4).
    let mut current = cwd.as_path();
    loop {
        let candidate = current.join(TODO_FILENAME);
        let has_git = path_exists(&current.join(".git"));
        if path_is_file(&candidate) {
            let git_ctx = if has_git {
                Some(inspect_git_root(current))
            } else {
                find_git_root_upward(current).map(|root| inspect_git_root(&root))
            };
            let (repo_root, git_dir, git_common_dir) = match git_ctx {
                Some(GitContext::Normal { root, dot_git }) => {
                    (Some(root), Some(dot_git.clone()), Some(dot_git))
                }
                Some(GitContext::UnknownDotGit { root }) => (Some(root), None, None),
                None => (None, None, None),
            };
            return Ok(FoundTodo {
                path: candidate,
                found_by: FoundBy::Walk,
                repo_root,
                git_dir,
                git_common_dir,
                notice: None,
                warnings: Vec::new(),
            });
        }
        if has_git {
            return Err(no_file_error(&cwd));
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => return Err(no_file_error(&cwd)),
        }
    }
}

enum GitContext {
    Normal { root: PathBuf, dot_git: PathBuf },
    UnknownDotGit { root: PathBuf },
}

fn find_git_root_upward(start: &Path) -> Option<PathBuf> {
    let mut cur = start;
    loop {
        if path_exists(&cur.join(".git")) {
            return Some(cur.to_path_buf());
        }
        cur = cur.parent()?;
    }
}

fn inspect_git_root(root: &Path) -> GitContext {
    let dot_git = root.join(".git");
    if path_is_dir(&dot_git) {
        GitContext::Normal {
            root: root.to_path_buf(),
            dot_git,
        }
    } else {
        GitContext::UnknownDotGit {
            root: root.to_path_buf(),
        }
    }
}
