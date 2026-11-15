//! Finding TODO.md, including linked git worktrees (T-010, T-011; R-FIND-1..7).

use std::borrow::Cow;
use std::ffi::OsStr;
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

/// Canonicalizes a path using long-path support and strips any Windows verbatim prefix.
pub(crate) fn canonicalize_clean(path: &Path) -> std::io::Result<PathBuf> {
    let canon = std::fs::canonicalize(fs_path(path).as_ref())?;
    Ok(strip_verbatim_prefix(&canon))
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

fn read_to_string_safe(path: &Path) -> std::io::Result<String> {
    std::fs::read_to_string(fs_path(path).as_ref())
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

fn resolve_relative_path(base_dir: &Path, raw: &str) -> PathBuf {
    let p = Path::new(raw);
    let joined = if p.is_absolute() {
        p.to_path_buf()
    } else {
        base_dir.join(p)
    };
    match std::path::absolute(&joined) {
        Ok(abs) => strip_verbatim_prefix(&abs),
        Err(_) => joined,
    }
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

        let (repo_root, git_dir, git_common_dir) = git_context_paths(git_ctx.as_ref());

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

    // Check if `cwd` is inside a git repository first so linked worktrees map to the main checkout
    // before checking any local `TODO.md` in the worktree (R-FIND-5).
    if let Some(git_root) = find_git_root_upward(&cwd) {
        let git_ctx = inspect_git_root(&git_root);
        match git_ctx {
            GitContext::Normal {
                ref root,
                ref dot_git,
            } => {
                if let Some(found_path) = walk_up_bounded(&cwd, root) {
                    return Ok(FoundTodo {
                        path: found_path,
                        found_by: FoundBy::Walk,
                        repo_root: Some(root.clone()),
                        git_dir: Some(dot_git.clone()),
                        git_common_dir: Some(dot_git.clone()),
                        notice: None,
                        warnings: Vec::new(),
                    });
                }
                return Err(no_file_error(&cwd));
            }
            GitContext::LinkedWorktree {
                ref wt_root,
                ref main_checkout,
                ref worktree_git_dir,
                ref common_git_dir,
            } => {
                if opts.worktree == WorktreeMode::Local {
                    if let Some(found_path) = walk_up_bounded(&cwd, wt_root) {
                        return Ok(FoundTodo {
                            path: found_path,
                            found_by: FoundBy::Walk,
                            repo_root: Some(wt_root.clone()),
                            git_dir: Some(worktree_git_dir.clone()),
                            git_common_dir: Some(common_git_dir.clone()),
                            notice: None,
                            warnings: Vec::new(),
                        });
                    }
                    return Err(no_file_error(&cwd));
                }

                let rel = relative_subpath(&cwd, wt_root).unwrap_or_default();
                let mapped_start = if rel.as_os_str().is_empty() {
                    main_checkout.clone()
                } else {
                    main_checkout.join(&rel)
                };
                if let Some(found_path) = walk_up_bounded(&mapped_start, main_checkout) {
                    let notice = format!("linked worktree: using {}", found_path.display());
                    return Ok(FoundTodo {
                        path: found_path,
                        found_by: FoundBy::WorktreeMain,
                        repo_root: Some(main_checkout.clone()),
                        git_dir: Some(worktree_git_dir.clone()),
                        git_common_dir: Some(common_git_dir.clone()),
                        notice: Some(notice),
                        warnings: Vec::new(),
                    });
                }
                return Err(no_file_error(&mapped_start));
            }
            GitContext::LinkedFallback {
                ref wt_root,
                ref worktree_git_dir,
                ref common_git_dir,
                ref reason,
            } => {
                if let Some(found_path) = walk_up_bounded(&cwd, wt_root) {
                    let warnings = if opts.worktree == WorktreeMode::Local {
                        Vec::new()
                    } else {
                        vec![format!(
                            "could not locate main git checkout ({reason}); using worktree file {}",
                            found_path.display()
                        )]
                    };
                    let found_by = if opts.worktree == WorktreeMode::Local {
                        FoundBy::Walk
                    } else {
                        FoundBy::WorktreeFallback
                    };
                    return Ok(FoundTodo {
                        path: found_path,
                        found_by,
                        repo_root: Some(wt_root.clone()),
                        git_dir: worktree_git_dir.clone(),
                        git_common_dir: common_git_dir.clone(),
                        notice: None,
                        warnings,
                    });
                }
                return Err(no_file_error(&cwd));
            }
        }
    }

    // Outside any git repository: walk up to the drive/filesystem root (R-FIND-3, R-FIND-4).
    let mut current = cwd.as_path();
    loop {
        let candidate = current.join(TODO_FILENAME);
        if path_is_file(&candidate) {
            return Ok(FoundTodo {
                path: candidate,
                found_by: FoundBy::Walk,
                repo_root: None,
                git_dir: None,
                git_common_dir: None,
                notice: None,
                warnings: Vec::new(),
            });
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => return Err(no_file_error(&cwd)),
        }
    }
}

fn walk_up_bounded(start: &Path, stop_at: &Path) -> Option<PathBuf> {
    let canon_stop = canonicalize_clean(stop_at).ok();
    let mut current = start;
    loop {
        let candidate = current.join(TODO_FILENAME);
        if path_is_file(&candidate) {
            return Some(candidate);
        }
        if current == stop_at {
            return None;
        }
        if let Some(ref target) = canon_stop
            && let Ok(canon_cur) = canonicalize_clean(current)
            && &canon_cur == target
        {
            return None;
        }
        current = current.parent()?;
    }
}

fn relative_subpath(child: &Path, root: &Path) -> Option<PathBuf> {
    if let Ok(rel) = child.strip_prefix(root) {
        return Some(rel.to_path_buf());
    }
    if let (Ok(canon_child), Ok(canon_root)) = (canonicalize_clean(child), canonicalize_clean(root))
        && let Ok(rel) = canon_child.strip_prefix(&canon_root)
    {
        return Some(rel.to_path_buf());
    }
    None
}

enum GitContext {
    Normal {
        root: PathBuf,
        dot_git: PathBuf,
    },
    LinkedWorktree {
        wt_root: PathBuf,
        main_checkout: PathBuf,
        worktree_git_dir: PathBuf,
        common_git_dir: PathBuf,
    },
    LinkedFallback {
        wt_root: PathBuf,
        worktree_git_dir: Option<PathBuf>,
        common_git_dir: Option<PathBuf>,
        reason: String,
    },
}

fn git_context_paths(
    ctx: Option<&GitContext>,
) -> (Option<PathBuf>, Option<PathBuf>, Option<PathBuf>) {
    match ctx {
        Some(GitContext::Normal { root, dot_git }) => (
            Some(root.clone()),
            Some(dot_git.clone()),
            Some(dot_git.clone()),
        ),
        Some(GitContext::LinkedWorktree {
            wt_root,
            worktree_git_dir,
            common_git_dir,
            ..
        }) => (
            Some(wt_root.clone()),
            Some(worktree_git_dir.clone()),
            Some(common_git_dir.clone()),
        ),
        Some(GitContext::LinkedFallback {
            wt_root,
            worktree_git_dir,
            common_git_dir,
            ..
        }) => (
            Some(wt_root.clone()),
            worktree_git_dir.clone(),
            common_git_dir.clone(),
        ),
        None => (None, None, None),
    }
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
        return GitContext::Normal {
            root: root.to_path_buf(),
            dot_git,
        };
    }

    let Ok(dot_git_text) = read_to_string_safe(&dot_git) else {
        return GitContext::LinkedFallback {
            wt_root: root.to_path_buf(),
            worktree_git_dir: None,
            common_git_dir: None,
            reason: "unreadable .git file".to_owned(),
        };
    };

    let Some(gitdir_raw) = dot_git_text
        .lines()
        .find_map(|line| line.trim().strip_prefix("gitdir:").map(str::trim))
        .filter(|s| !s.is_empty())
    else {
        return GitContext::LinkedFallback {
            wt_root: root.to_path_buf(),
            worktree_git_dir: None,
            common_git_dir: None,
            reason: "missing gitdir in .git file".to_owned(),
        };
    };

    let worktree_git_dir = resolve_relative_path(root, gitdir_raw);
    if !path_is_dir(&worktree_git_dir) {
        return GitContext::LinkedFallback {
            wt_root: root.to_path_buf(),
            worktree_git_dir: Some(worktree_git_dir),
            common_git_dir: None,
            reason: "gitdir target does not exist".to_owned(),
        };
    }

    let commondir_file = worktree_git_dir.join("commondir");
    let Ok(commondir_text) = read_to_string_safe(&commondir_file) else {
        return GitContext::LinkedFallback {
            wt_root: root.to_path_buf(),
            worktree_git_dir: Some(worktree_git_dir),
            common_git_dir: None,
            reason: "missing commondir file".to_owned(),
        };
    };

    let Some(commondir_raw) = commondir_text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
    else {
        return GitContext::LinkedFallback {
            wt_root: root.to_path_buf(),
            worktree_git_dir: Some(worktree_git_dir),
            common_git_dir: None,
            reason: "empty commondir file".to_owned(),
        };
    };

    let common_git_dir = resolve_relative_path(&worktree_git_dir, commondir_raw);
    if !path_is_dir(&common_git_dir) {
        return GitContext::LinkedFallback {
            wt_root: root.to_path_buf(),
            worktree_git_dir: Some(worktree_git_dir),
            common_git_dir: Some(common_git_dir),
            reason: "commondir target does not exist".to_owned(),
        };
    }

    let core = read_core_config(&common_git_dir.join("config"));
    if core.bare {
        return GitContext::LinkedFallback {
            wt_root: root.to_path_buf(),
            worktree_git_dir: Some(worktree_git_dir),
            common_git_dir: Some(common_git_dir),
            reason: "bare repository".to_owned(),
        };
    }

    if let Some(worktree_cfg) = core.worktree {
        let main_checkout = resolve_relative_path(&common_git_dir, &worktree_cfg);
        if path_is_dir(&main_checkout) {
            return GitContext::LinkedWorktree {
                wt_root: root.to_path_buf(),
                main_checkout,
                worktree_git_dir,
                common_git_dir,
            };
        }
        return GitContext::LinkedFallback {
            wt_root: root.to_path_buf(),
            worktree_git_dir: Some(worktree_git_dir),
            common_git_dir: Some(common_git_dir),
            reason: "configured core.worktree does not exist".to_owned(),
        };
    }

    if common_git_dir.file_name() == Some(OsStr::new(".git"))
        && let Some(main_checkout) = common_git_dir.parent()
        && path_is_dir(main_checkout)
    {
        return GitContext::LinkedWorktree {
            wt_root: root.to_path_buf(),
            main_checkout: main_checkout.to_path_buf(),
            worktree_git_dir,
            common_git_dir,
        };
    }

    GitContext::LinkedFallback {
        wt_root: root.to_path_buf(),
        worktree_git_dir: Some(worktree_git_dir),
        common_git_dir: Some(common_git_dir),
        reason: "unusual repository layout".to_owned(),
    }
}

#[derive(Default)]
struct CoreGitConfig {
    bare: bool,
    worktree: Option<String>,
}

fn read_core_config(config_path: &Path) -> CoreGitConfig {
    let mut cfg = CoreGitConfig::default();
    let Ok(text) = read_to_string_safe(config_path) else {
        return cfg;
    };
    let mut in_core = false;
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_core = line.eq_ignore_ascii_case("[core]");
            continue;
        }
        if !in_core {
            continue;
        }
        if let Some((key, val)) = line.split_once('=') {
            let key = key.trim();
            let val = val
                .split(['#', ';'])
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches('"');
            if key.eq_ignore_ascii_case("bare") {
                cfg.bare = val.eq_ignore_ascii_case("true");
            } else if key.eq_ignore_ascii_case("worktree") && !val.is_empty() {
                cfg.worktree = Some(val.to_owned());
            }
        }
    }
    cfg
}
