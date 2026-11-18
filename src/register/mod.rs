//! The per-user project register (T-093; R-PROJ-1, R-PROJ-2).
//!
//! The register is one JSON file per user, read by `todo overview` and changed by `todo project`.
//! todo never searches the PC for task lists: a project is known only when someone registers it.
//! Changes go through [`locked_update`], which takes the register's own operating-system lock and
//! replaces the file in one step, so parallel `project add`s lose nothing.

pub mod overview;

use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::atomic::{atomic_replace, atomic_write_bytes, read_file_with_snapshot};
use crate::error::{ErrorCode, Result, TodoError};
use crate::find::{fs_path, strip_verbatim_prefix};
use crate::lock::{LockConfig, lock_file_waiting};
use crate::version::RUNNING_VERSION;

/// The format of the register file, the `format` number in its JSON (R-VER-2).
pub const REGISTER_FORMAT: u32 = 1;

/// One registered project (R-PROJ-1): its name, the absolute path of its `TODO.md` and when it was added.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    /// The name, unique in the register whatever the letter case.
    pub name: String,
    /// The absolute path of the project's `TODO.md`, as it was resolved when the project was added.
    pub path: String,
    /// The local time the project was added, with its UTC offset (R-IO-6).
    pub added: String,
}

/// The register file: its format and the projects, in the order they were added.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Register {
    /// The format of the file; [`REGISTER_FORMAT`].
    pub format: u32,
    /// The registered projects, in the order they were added.
    pub projects: Vec<Project>,
}

impl Register {
    /// A register with no projects, in the current format.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            format: REGISTER_FORMAT,
            projects: Vec::new(),
        }
    }

    /// Adds `project`. The same file, or the same name, as a registered project is refused with
    /// `DUPLICATE_PROJECT` (exit 1), and the register is not changed.
    pub fn add(&mut self, project: Project) -> Result<()> {
        if let Some(existing) = self
            .projects
            .iter()
            .find(|p| same_path(&p.path, &project.path))
        {
            return Err(duplicate(format!(
                "{} is already registered as {}",
                project.path, existing.name
            )));
        }
        if let Some(existing) = self
            .projects
            .iter()
            .find(|p| same_name(&p.name, &project.name))
        {
            return Err(duplicate(format!(
                "a project named {} is already registered: {}",
                existing.name, existing.path
            )));
        }
        self.projects.push(project);
        Ok(())
    }

    /// Removes the project named `selector`; when no name matches, the project whose `TODO.md` is
    /// `path` (the canonical path the selector names, if it names one). `NOT_FOUND` (exit 1) when
    /// neither matches.
    pub fn remove(&mut self, selector: &str, path: Option<&str>) -> Result<Project> {
        let index = self
            .projects
            .iter()
            .position(|p| same_name(&p.name, selector))
            .or_else(|| {
                path.and_then(|wanted| {
                    self.projects
                        .iter()
                        .position(|p| same_path(&p.path, wanted))
                })
            });
        match index {
            Some(i) => Ok(self.projects.remove(i)),
            None => Err(not_found(selector)),
        }
    }
}

fn duplicate(message: String) -> TodoError {
    TodoError::new(ErrorCode::DuplicateProject, message)
}

/// The error for a `remove` that names no registered project.
#[must_use]
pub fn not_found(selector: &str) -> TodoError {
    TodoError::new(
        ErrorCode::NotFound,
        format!("no project named or at {selector} in the project register"),
    )
}

/// Whether two project names are the same: ignoring surrounding spaces and letter case.
#[must_use]
pub fn same_name(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

/// Whether two stored paths name the same file (R-LOCK-7). Windows ignores letter case in paths,
/// so there the comparison ignores it too; elsewhere the paths must match exactly.
#[must_use]
pub fn same_path(a: &str, b: &str) -> bool {
    if cfg!(windows) {
        a.to_lowercase() == b.to_lowercase()
    } else {
        a == b
    }
}

/// Checks a project name (R-EDIT-5): it is trimmed, it is not empty and it has no control
/// characters, so it stays on one line. Returns the trimmed name.
pub fn validate_name(raw: &str) -> Result<String> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(usage_error("a project name cannot be empty"));
    }
    if name.chars().any(char::is_control) {
        return Err(usage_error(format!(
            "the project name {name:?} has a control character"
        )));
    }
    Ok(name.to_owned())
}

fn usage_error(message: impl Into<String>) -> TodoError {
    TodoError::new(ErrorCode::Usage, message)
}

fn bad_value(message: impl Into<String>) -> TodoError {
    TodoError::new(ErrorCode::BadValue, message)
}

/// The name a project gets when `--name` is not given: the name of the folder that holds `todo_path`.
pub fn default_name(todo_path: &Path) -> Result<String> {
    let folder_name = todo_path
        .parent()
        .and_then(Path::file_name)
        .and_then(OsStr::to_str);
    match folder_name {
        Some(name) => validate_name(name),
        None => Err(usage_error(format!(
            "the folder of {} has no name to give the project; pass --name",
            todo_path.display()
        ))),
    }
}

/// The text of a path for the register, which stores paths as text. A path that is not valid
/// Unicode cannot be stored, and is refused with `BAD_VALUE`.
pub fn path_text(path: &Path) -> Result<String> {
    path.to_str().map(str::to_owned).ok_or_else(|| {
        bad_value(format!(
            "the path {} is not valid Unicode, so it cannot be registered",
            path.display()
        ))
    })
}

/// The register's path (R-PROJ-1): `TODO_REGISTER` when it is set, else the file in the user's
/// data folder. Returned as an absolute path.
pub fn register_path() -> Result<PathBuf> {
    if let Some(named) = env_path("TODO_REGISTER") {
        let absolute = std::path::absolute(&named).map_err(|e| {
            TodoError::new(
                ErrorCode::NoFile,
                format!(
                    "could not resolve the register path {}: {e}",
                    named.display()
                ),
            )
        })?;
        return Ok(strip_verbatim_prefix(&absolute));
    }
    default_register_path(
        cfg!(windows),
        env_path("LOCALAPPDATA"),
        env_path("XDG_DATA_HOME"),
        env_path("HOME"),
    )
    .ok_or_else(|| {
        TodoError::new(
            ErrorCode::NoFile,
            "no folder for the project register: set TODO_REGISTER, or LOCALAPPDATA on Windows, or HOME on Linux",
        )
    })
}

/// The register file in the user's data folder: `%LOCALAPPDATA%\todo\projects.json` on Windows,
/// `$XDG_DATA_HOME/todo/projects.json` or `~/.local/share/todo/projects.json` elsewhere (R-PROJ-1).
/// `None` when the folder cannot be known.
#[must_use]
pub fn default_register_path(
    windows: bool,
    local_app_data: Option<PathBuf>,
    xdg_data_home: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Option<PathBuf> {
    let data_folder = if windows {
        local_app_data?
    } else {
        xdg_data_home.or_else(|| home.map(|h| h.join(".local").join("share")))?
    };
    Some(data_folder.join("todo").join("projects.json"))
}

fn env_path(key: &str) -> Option<PathBuf> {
    std::env::var_os(key)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// The lock file of a register: `<register file>.lock` next to it (R-PROJ-1, R-LOCK-2).
#[must_use]
pub fn lock_path_of(register: &Path) -> PathBuf {
    let mut name = register.file_name().unwrap_or_default().to_os_string();
    name.push(".lock");
    register.with_file_name(name)
}

/// Reads the register without a lock: read commands never take one, and the file is replaced in
/// one step (R-LOCK-5). `None` when there is no register yet.
pub fn read_register(path: &Path) -> Result<Option<Register>> {
    match fs::read(fs_path(path).as_ref()) {
        Ok(bytes) => parse_register(&bytes, path).map(Some),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(TodoError::new(
            ErrorCode::WriteFailed,
            format!("failed reading {}: {e}", path.display()),
        )
        .with_file(path.display().to_string())),
    }
}

/// Parses the bytes of a register. A file in a format newer than this todo is refused with
/// `TOOL_TOO_OLD` (R-VER-2); a file that is not a register is refused with `UNSAFE_FILE`.
fn parse_register(bytes: &[u8], path: &Path) -> Result<Register> {
    #[derive(Deserialize)]
    struct FormatLine {
        format: Option<u32>,
    }

    let label = path.display().to_string();
    let header: FormatLine =
        serde_json::from_slice(bytes).map_err(|e| not_a_register(&label, &e.to_string()))?;
    match header.format {
        Some(REGISTER_FORMAT) => {}
        Some(version) if version > REGISTER_FORMAT => {
            return Err(TodoError::new(
                ErrorCode::ToolTooOld,
                format!(
                    "the project register {label} has format {version}, newer than this todo {RUNNING_VERSION} supports. Update the todo executable first (`todo help update`)."
                ),
            )
            .with_file(label));
        }
        _ => return Err(not_a_register(&label, "it has no known format number")),
    }
    serde_json::from_slice(bytes).map_err(|e| not_a_register(&label, &e.to_string()))
}

fn not_a_register(label: &str, reason: &str) -> TodoError {
    TodoError::new(
        ErrorCode::UnsafeFile,
        format!("{label} is not a project register: {reason}"),
    )
    .with_file(label.to_owned())
}

/// Changes the register under its own lock (R-PROJ-1).
///
/// The lock is waited for as a write waits for `write.lock` (R-LOCK-3). The register is read under
/// the lock, `change` runs on it, and the new file replaces the old one in one step, checked
/// against what was read (R-IO-1, R-IO-2). A missing register starts empty. When `change` fails,
/// nothing is written. A register that `change` leaves as it was is still written, so `change`
/// returns `Ok` only when it has changed the register.
pub fn locked_update<T>(path: &Path, change: impl FnOnce(&mut Register) -> Result<T>) -> Result<T> {
    if let Some(folder) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(fs_path(folder).as_ref()).map_err(|e| {
            TodoError::new(
                ErrorCode::WriteFailed,
                format!("failed creating {}: {e}", folder.display()),
            )
            .with_file(folder.display().to_string())
        })?;
    }

    let _lock = RegisterLock::acquire(path)?;
    let existing = match fs::metadata(fs_path(path).as_ref()) {
        Ok(_) => Some(read_file_with_snapshot(path)?),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => {
            return Err(TodoError::new(
                ErrorCode::WriteFailed,
                format!("failed reading {}: {e}", path.display()),
            ));
        }
    };
    let mut register = match &existing {
        Some(read) => parse_register(&read.bytes, path)?,
        None => Register::empty(),
    };

    let value = change(&mut register)?;

    let mut bytes = serde_json::to_vec_pretty(&register).map_err(|e| {
        TodoError::new(
            ErrorCode::WriteFailed,
            format!("failed serializing {}: {e}", path.display()),
        )
    })?;
    bytes.push(b'\n');
    match existing {
        Some(read) => atomic_replace(path, &read.snapshot, &bytes)?,
        None => atomic_write_bytes(path, &bytes)?,
    }
    Ok(value)
}

/// The operating-system lock on a register's lock file, released when dropped.
struct RegisterLock {
    file: File,
}

impl RegisterLock {
    fn acquire(register: &Path) -> Result<Self> {
        let lock_path = lock_path_of(register);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(fs_path(&lock_path).as_ref())
            .map_err(|e| {
                TodoError::new(
                    ErrorCode::WriteFailed,
                    format!("failed opening lock file {}: {e}", lock_path.display()),
                )
            })?;
        let wait = LockConfig::default().effective_wait();
        if lock_file_waiting(&file, &lock_path, wait)? {
            Ok(Self { file })
        } else {
            Err(TodoError::new(
                ErrorCode::Busy,
                format!(
                    "the project register {} is busy: another todo is changing it; try again",
                    register.display()
                ),
            )
            .with_file(register.display().to_string()))
        }
    }
}

impl Drop for RegisterLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(name: &str, path: &str) -> Project {
        Project {
            name: name.to_owned(),
            path: path.to_owned(),
            added: "2026-11-19T10:00+01:00".to_owned(),
        }
    }

    #[test]
    fn default_path_is_in_the_user_data_folder() {
        let local = PathBuf::from(r"C:\Users\Example\AppData\Local");
        assert_eq!(
            default_register_path(true, Some(local.clone()), None, None),
            Some(local.join("todo").join("projects.json"))
        );
        assert_eq!(default_register_path(true, None, None, None), None);

        let home = PathBuf::from("/home/example");
        assert_eq!(
            default_register_path(false, None, None, Some(home.clone())),
            Some(home.join(".local/share/todo/projects.json"))
        );
        let xdg = PathBuf::from("/home/example/data");
        assert_eq!(
            default_register_path(false, Some(local), Some(xdg.clone()), Some(home)),
            Some(xdg.join("todo").join("projects.json"))
        );
        assert_eq!(default_register_path(false, None, None, None), None);
    }

    #[test]
    fn lock_file_sits_next_to_the_register() {
        assert_eq!(
            lock_path_of(Path::new("/data/todo/projects.json")),
            PathBuf::from("/data/todo/projects.json.lock")
        );
    }

    #[test]
    fn names_are_trimmed_and_must_be_one_line() {
        assert_eq!(validate_name("  renderer ").unwrap(), "renderer");
        assert_eq!(validate_name("   ").unwrap_err().code, ErrorCode::Usage);
        assert_eq!(
            validate_name("two\nlines").unwrap_err().code,
            ErrorCode::Usage
        );
        assert!(same_name("Renderer", "renderer "));
        assert!(!same_name("renderer", "render"));
    }

    #[test]
    fn paths_compare_ignoring_case_only_on_windows() {
        assert_eq!(
            same_path(r"C:\Projekte\TODO.md", r"c:\projekte\todo.md"),
            cfg!(windows)
        );
        assert!(same_path("/a/TODO.md", "/a/TODO.md"));
        assert_eq!(same_path("/a/TODO.md", "/A/TODO.md"), cfg!(windows));
    }

    #[test]
    fn a_second_file_or_name_is_a_duplicate_and_changes_nothing() {
        let mut register = Register::empty();
        register
            .add(project("renderer", "/work/renderer/TODO.md"))
            .unwrap();

        let same_file = register
            .add(project("other", "/work/renderer/TODO.md"))
            .unwrap_err();
        assert_eq!(same_file.code, ErrorCode::DuplicateProject);
        assert_eq!(same_file.exit_code, crate::error::ExitCode::Refused);

        let same_name = register
            .add(project("Renderer", "/work/other/TODO.md"))
            .unwrap_err();
        assert_eq!(same_name.code, ErrorCode::DuplicateProject);

        assert_eq!(register.projects.len(), 1);
        register
            .add(project("audio", "/work/audio/TODO.md"))
            .unwrap();
        assert_eq!(register.projects.len(), 2);
    }

    #[test]
    fn remove_takes_a_name_first_and_then_a_path() {
        let mut register = Register::empty();
        register
            .add(project("renderer", "/work/renderer/TODO.md"))
            .unwrap();
        register
            .add(project("audio", "/work/audio/TODO.md"))
            .unwrap();

        let by_path = register
            .remove("/work/audio", Some("/work/audio/TODO.md"))
            .unwrap();
        assert_eq!(by_path.name, "audio");
        let by_name = register.remove("RENDERER", None).unwrap();
        assert_eq!(by_name.name, "renderer");
        assert!(register.projects.is_empty());

        let missing = register.remove("audio", None).unwrap_err();
        assert_eq!(missing.code, ErrorCode::NotFound);
        assert_eq!(missing.exit_code, crate::error::ExitCode::Refused);
    }

    #[test]
    fn a_register_in_a_newer_format_is_refused_and_a_broken_one_is_unsafe() {
        let newer = br#"{"format": 2, "projects": []}"#;
        let err = parse_register(newer, Path::new("projects.json")).unwrap_err();
        assert_eq!(err.code, ErrorCode::ToolTooOld);
        assert_eq!(err.exit_code, crate::error::ExitCode::FileError);

        for broken in [
            &b"not json"[..],
            &b"[]"[..],
            &br#"{"projects": []}"#[..],
            &br#"{"format": 1, "projects": 3}"#[..],
        ] {
            let err = parse_register(broken, Path::new("projects.json")).unwrap_err();
            assert_eq!(err.code, ErrorCode::UnsafeFile, "{broken:?}");
        }
    }

    #[test]
    fn the_register_round_trips_through_its_json() {
        let mut register = Register::empty();
        register
            .add(project("renderer", r"C:\Projekte\Grafik Ärger\TODO.md"))
            .unwrap();
        let bytes = serde_json::to_vec_pretty(&register).unwrap();
        let back = parse_register(&bytes, Path::new("projects.json")).unwrap();
        assert_eq!(back, register);
    }
}
