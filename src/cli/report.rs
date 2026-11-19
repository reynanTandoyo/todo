//! `todo report --html` command: one read-only HTML page for people, for the current project or for
//! every registered project with `--all` (T-092, T-094; R-REPORT-1, R-REPORT-2, R-LOCK-5).

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::Args as ClapArgs;
use serde::Serialize;

use crate::atomic::atomic_write_bytes;
use crate::cli::{CommandKind, CommandMeta, Context, load_read};
use crate::error::{ErrorCode, Result, TodoError};
use crate::find::fs_path;
use crate::lock::current_time;
use crate::output::Output;
use crate::register::{read_register, register_path};
use crate::report::{build_all_page, build_page, render_all_page, render_page};
use crate::state::StateDir;

/// The file the page is written to by default, in the state folder (`docs/requirements.md` §7).
const DEFAULT_FILE: &str = "report.html";

/// The file `--all` writes by default, next to the project register (R-REPORT-2).
const DEFAULT_ALL_FILE: &str = "report-all.html";

/// Metadata for the `report` command.
pub const META: CommandMeta = CommandMeta {
    name: "report",
    summary: "Write one HTML page for people: the work, questions, ideas, done and dropped",
    kind: CommandKind::Read,
    details: "Writes one self-contained HTML page for people; `--html` is required, as the page is the only report so far. The page shows the work as a tree of milestones, each with its goal and progress, and the tasks indented below it, with top-level tasks last; the unfinished tasks per area; open questions, each linked to the milestone or task it is about; ideas; and the done and dropped entries, the dropped ones with the reason. Each task opens to show its fields and notes. The page has inline CSS and no script, loads nothing from the network, and has no buttons or forms. It takes no lock, never changes TODO.md, and is written only when asked. Done and dropped entries in TODO-archive.md are listed too, marked archived. The default file is report.html in the state folder, so git never sees it; `--out <path>` writes elsewhere. `--all` writes one page for every project in the project register instead of the current one: a card per project (tasks per status, top-level milestones with progress, blocked tasks, open questions), then each project's lists. It reads the register and each registered TODO.md and archive, never searches folders, works from any folder and writes nothing in the projects; a project that is gone, cannot be parsed or needs a newer todo shows its problem instead of its lists. Its default file is report-all.html next to the register. `--open` opens the page with the system's default program for HTML files. Text output is the path; `--json` returns the path, the size in bytes and whether the page was opened.",
    examples: &[
        "todo report --html",
        "todo report --html --open",
        "todo report --html --out \"$env:USERPROFILE\\Desktop\\tasks.html\"",
        "todo report --html --all",
        "todo report --html --all --out \"$env:USERPROFILE\\Desktop\\Alle Projekte.html\"",
        "todo report --html --json",
    ],
};

/// Arguments for `todo report`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Write the HTML page. Required: the page is the only report so far.
    #[arg(long)]
    pub html: bool,

    /// Write one page for every registered project (`todo project list`) instead of the current one.
    #[arg(long)]
    pub all: bool,

    /// Write the page to this file instead of `report.html` in the state folder (with `--all`,
    /// `report-all.html` next to the project register).
    #[arg(long, value_name = "PATH")]
    pub out: Option<PathBuf>,

    /// Open the page afterwards with the system's default program for HTML files.
    #[arg(long)]
    pub open: bool,
}

#[derive(Serialize)]
struct ReportJson {
    path: String,
    bytes: usize,
    opened: bool,
}

/// Runs `todo report --html`, or `todo report --html --all`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    run_with_opener(args, ctx, out, &open_with_default_program)
}

/// The body of [`run`], with the function that opens the page passed in, so that tests can check
/// the `--open` handling without starting a program.
fn run_with_opener(
    args: Args,
    ctx: &Context,
    out: &mut Output,
    opener: &dyn Fn(&Path) -> io::Result<()>,
) -> Result<()> {
    if !args.html {
        return Err(TodoError::new(
            ErrorCode::Usage,
            "report needs --html: the HTML page is the only report so far",
        ));
    }
    if args.all {
        let register = register_path()?;
        return run_all_with_opener(&register, args, out, opener);
    }

    let (cfg, loaded) = load_read(ctx, out, false, true)?;
    let page = build_page(
        &loaded.doc,
        loaded.archive.as_ref(),
        &cfg,
        current_time(),
        &loaded.file_label,
    );
    let html = render_page(&page);

    let path = match args.out {
        Some(path) => path,
        None => {
            let state = StateDir::from_found(&loaded.found)?;
            state.ensure_created()?;
            state.path().join(DEFAULT_FILE)
        }
    };
    write_and_finish(&path, &html, args.open, out, opener)
}

/// `todo report --html --all`: one page for the projects in the register at `register` (R-REPORT-2).
/// It reads the register and each project's files, and writes only the page. Without `--out` the
/// page goes to `report-all.html` next to the register.
fn run_all_with_opener(
    register: &Path,
    args: Args,
    out: &mut Output,
    opener: &dyn Fn(&Path) -> io::Result<()>,
) -> Result<()> {
    let projects = read_register(register)?
        .map(|reg| reg.projects)
        .unwrap_or_default();
    let page = build_all_page(&projects, current_time());
    for warning in page.projects.iter().flat_map(|project| &project.warnings) {
        out.warn(warning.clone());
    }
    let html = render_all_page(&page);

    let path = match args.out {
        Some(path) => path,
        None => default_all_path(register)?,
    };
    write_and_finish(&path, &html, args.open, out, opener)
}

/// The file `--all` writes by default: `report-all.html` next to the register. Its folder is made
/// when it is missing, as the folder of the register is made when a project is first added.
fn default_all_path(register: &Path) -> Result<PathBuf> {
    let path = register.with_file_name(DEFAULT_ALL_FILE);
    if let Some(folder) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(fs_path(folder).as_ref()).map_err(|e| {
            TodoError::new(
                ErrorCode::WriteFailed,
                format!("failed creating {}: {e}", folder.display()),
            )
            .with_file(folder.display().to_string())
        })?;
    }
    Ok(path)
}

/// Writes `html` to `path`, opens it when `open` is set, and reports the path (or, with `--json`,
/// the path, the size and whether it was opened).
fn write_and_finish(
    path: &Path,
    html: &str,
    open: bool,
    out: &mut Output,
    opener: &dyn Fn(&Path) -> io::Result<()>,
) -> Result<()> {
    atomic_write_bytes(path, html.as_bytes())?;

    let opened = if open {
        match opener(path) {
            Ok(()) => true,
            Err(err) => {
                out.warn(format!("could not open {}: {err}", path.display()));
                false
            }
        }
    } else {
        false
    };

    if out.is_json() {
        out.emit_json_ok(
            META.name,
            &ReportJson {
                path: path.display().to_string(),
                bytes: html.len(),
                opened,
            },
        );
    } else {
        out.write_line(&path.display().to_string());
    }
    Ok(())
}

/// Starts the system's default program for `path` and does not wait for it.
fn open_with_default_program(path: &Path) -> io::Result<()> {
    let (program, args) = open_command(path);
    Command::new(program).args(args).spawn().map(drop)
}

/// The program and arguments that open `path` with the system's default program for HTML files.
/// On Windows, `rundll32` opens the file without a shell, so a path with spaces or non-ASCII
/// characters is passed as one argument.
fn open_command(path: &Path) -> (OsString, Vec<OsString>) {
    let mut args = Vec::new();
    let program = if cfg!(windows) {
        args.push(OsString::from("url.dll,FileProtocolHandler"));
        "rundll32.exe"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    args.push(path.as_os_str().to_os_string());
    (OsString::from(program), args)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::io::Write;
    use std::rc::Rc;

    use tempfile::TempDir;

    use super::*;
    use crate::find::ExplicitSource;

    /// A writer that keeps what is written, so a test can read the output back.
    #[derive(Clone, Default)]
    struct Captured(Rc<RefCell<Vec<u8>>>);

    impl Write for Captured {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    const BODY: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
<!-- todo-format: 2 -->\n\
# Lamp room\n\n\
## Now\n\n\
### T-001 · Trim the wick\n\
- status: open\n\
- owner: —\n\
- priority: P1\n";

    fn ctx_for(todo: &Path) -> Context {
        Context {
            file: Some(todo.to_path_buf()),
            file_source: Some(ExplicitSource::Flag),
            json: false,
            agent: None,
            dry_run: false,
            yes: false,
            force: false,
            no_color: true,
            session: None,
            session_arg: None,
        }
    }

    fn args(html: bool, out: Option<PathBuf>, open: bool) -> Args {
        Args {
            html,
            all: false,
            out,
            open,
        }
    }

    fn all_args(out: Option<PathBuf>, open: bool) -> Args {
        Args {
            html: true,
            all: true,
            out,
            open,
        }
    }

    /// A project folder with `TODO.md` holding [`BODY`]; returns the folder and the file.
    fn project(tmp: &TempDir) -> (PathBuf, PathBuf) {
        let dir = tmp.path().join("lamp room");
        std::fs::create_dir_all(&dir).unwrap();
        let todo = dir.join("TODO.md");
        std::fs::write(&todo, BODY).unwrap();
        (dir, todo)
    }

    #[test]
    fn open_command_passes_the_path_as_one_argument() {
        let path = Path::new(r"C:\Users\Zoë\Tâche finale\report.html");
        let (program, args) = open_command(path);
        assert_eq!(args.last().map(OsString::as_os_str), Some(path.as_os_str()));
        if cfg!(windows) {
            assert_eq!(program, OsString::from("rundll32.exe"));
            assert_eq!(args.len(), 2);
            assert_eq!(args[0], OsString::from("url.dll,FileProtocolHandler"));
        } else {
            assert_eq!(args.len(), 1);
        }
    }

    #[test]
    fn report_without_html_is_a_usage_error() {
        let tmp = TempDir::new().unwrap();
        let (_dir, todo) = project(&tmp);
        let mut out = Output::with_writers(
            false,
            Box::new(Captured::default()),
            Box::new(Captured::default()),
        );
        let err = run_with_opener(
            args(false, None, false),
            &ctx_for(&todo),
            &mut out,
            &|_| Ok(()),
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::Usage);
    }

    #[test]
    fn open_hands_the_written_page_to_the_opener() {
        let tmp = TempDir::new().unwrap();
        let (dir, todo) = project(&tmp);
        let opened: RefCell<Vec<PathBuf>> = RefCell::new(Vec::new());
        let stdout = Captured::default();
        let mut out = Output::with_writers(
            false,
            Box::new(stdout.clone()),
            Box::new(Captured::default()),
        );
        run_with_opener(
            args(true, None, true),
            &ctx_for(&todo),
            &mut out,
            &|path| {
                opened.borrow_mut().push(path.to_path_buf());
                Ok(())
            },
        )
        .unwrap();
        let opened = opened.into_inner();
        assert_eq!(opened.len(), 1);
        assert!(opened[0].ends_with(Path::new(".todo").join(DEFAULT_FILE)));
        assert!(opened[0].is_file());
        assert!(dir.join(".todo").join(DEFAULT_FILE).is_file());
        let printed = String::from_utf8(stdout.0.borrow().clone()).unwrap();
        assert_eq!(printed, format!("{}\n", opened[0].display()));
    }

    #[test]
    fn a_failed_open_is_a_warning_and_the_page_is_still_written() {
        let tmp = TempDir::new().unwrap();
        let (dir, todo) = project(&tmp);
        let mut out = Output::with_writers(
            true,
            Box::new(Captured::default()),
            Box::new(Captured::default()),
        );
        run_with_opener(
            args(true, None, true),
            &ctx_for(&todo),
            &mut out,
            &|_| Err(io::Error::other("no program")),
        )
        .unwrap();
        assert!(dir.join(".todo").join(DEFAULT_FILE).is_file());
        assert!(
            out.warnings()
                .iter()
                .any(|w| w.starts_with("could not open") && w.contains("no program")),
            "{:?}",
            out.warnings()
        );
    }

    #[test]
    fn all_writes_next_to_the_register_and_hands_the_page_to_the_opener() {
        let tmp = TempDir::new().unwrap();
        // The folder of the register does not exist yet: the default page makes it.
        let register = tmp.path().join("per user").join("projects.json");
        let opened: RefCell<Vec<PathBuf>> = RefCell::new(Vec::new());
        let stdout = Captured::default();
        let mut out = Output::with_writers(
            false,
            Box::new(stdout.clone()),
            Box::new(Captured::default()),
        );
        run_all_with_opener(&register, all_args(None, true), &mut out, &|path| {
            opened.borrow_mut().push(path.to_path_buf());
            Ok(())
        })
        .unwrap();

        let expected = tmp.path().join("per user").join(DEFAULT_ALL_FILE);
        assert_eq!(opened.into_inner(), vec![expected.clone()]);
        let html = fs::read_to_string(&expected).unwrap();
        assert!(html.contains("No project is registered yet."), "{html}");
        let printed = String::from_utf8(stdout.0.borrow().clone()).unwrap();
        assert_eq!(printed, format!("{}\n", expected.display()));
        assert!(!register.exists(), "the report must not create the register");
    }

    #[test]
    fn all_with_out_writes_only_to_the_given_file_and_makes_no_folder() {
        let tmp = TempDir::new().unwrap();
        let register = tmp.path().join("per user").join("projects.json");
        let target = tmp.path().join("no such folder").join("all.html");
        let mut out = Output::with_writers(
            true,
            Box::new(Captured::default()),
            Box::new(Captured::default()),
        );
        let err = run_all_with_opener(
            &register,
            all_args(Some(target.clone()), false),
            &mut out,
            &|_| Ok(()),
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::WriteFailed);
        assert!(!target.exists());
        assert!(!tmp.path().join("per user").exists());

        let target = tmp.path().join("all projects.html");
        run_all_with_opener(
            &register,
            all_args(Some(target.clone()), false),
            &mut out,
            &|_| Ok(()),
        )
        .unwrap();
        assert!(target.is_file());
        assert!(!tmp.path().join("per user").join(DEFAULT_ALL_FILE).exists());
    }
}
