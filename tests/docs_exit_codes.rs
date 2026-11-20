//! Checks the table of error codes in `docs/json.md` (`R-OUT-2`, `R-OUT-3`) against the exit codes
//! that the program really gives. The table and `todo schema` must give the same exit code for every
//! code, and commands that make each code must exit with the code the table gives (REL-5, T-141).

// The helpers panic when the document or a command is wrong, which is how a test fails, as in the
// other test files (AGENTS.md "Test rules").
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

use common::{parse_json, stderr_text, todo_bin};

/// `docs/json.md`, read from this repository.
fn json_doc() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("docs")
        .join("json.md");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The exit code of each error code in the table of `docs/json.md`. Every exit cell must be a single
/// number, so a row such as `2 (1 for check)` is refused.
fn error_table_exits(doc: &str) -> BTreeMap<String, i64> {
    let mut table = BTreeMap::new();
    let mut in_section = false;
    for line in doc.lines() {
        if line.starts_with("## ") {
            in_section = line.starts_with("## Error Codes");
            continue;
        }
        if !in_section || !line.starts_with("| `") {
            continue;
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        let code = cells[1].trim_matches('`').to_owned();
        let exit = cells[2].parse::<i64>().unwrap_or_else(|_| {
            panic!(
                "the exit of {code} in docs/json.md is not a single number: {:?}",
                cells[2]
            )
        });
        assert!(
            table.insert(code.clone(), exit).is_none(),
            "{code} is listed twice in docs/json.md"
        );
    }
    assert!(!table.is_empty(), "no error-code table in docs/json.md");
    table
}

/// The exit code of each error code as `todo schema --json` reports it.
fn schema_error_exits() -> BTreeMap<String, i64> {
    let out = todo_bin()
        .args(["--json", "schema"])
        .output()
        .expect("run todo schema");
    assert!(out.status.success(), "stderr: {}", stderr_text(&out));
    let real = parse_json(&out.stdout);
    real["data"]["error_codes"]
        .as_array()
        .expect("error_codes")
        .iter()
        .map(|entry| {
            (
                entry["code"].as_str().expect("code").to_owned(),
                entry["exit_code"].as_i64().expect("exit_code"),
            )
        })
        .collect()
}

/// Set-up that a case needs besides its files.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Setup {
    Plain,
    /// The TODO.md is read-only (`READ_ONLY`).
    ReadOnly,
    /// Another process holds the write lock (`BUSY`).
    HeldWriteLock,
    /// The folder of the project register is a file, so the register cannot be written
    /// (`WRITE_FAILED`).
    RegisterFolderIsAFile,
}

/// A command that makes one error. `before` lists commands that must succeed first.
struct ErrorCase {
    code: &'static str,
    /// The TODO.md of the case, or `None` when there is no file.
    todo: Option<&'static str>,
    /// `todo.toml` next to the TODO.md, when the case needs one.
    config: Option<&'static str>,
    setup: Setup,
    /// Commands that must succeed first, each run as `--json --file <TODO.md>` plus its arguments.
    before: &'static [&'static [&'static str]],
    /// The command that fails, run as `--json --file <TODO.md>` plus its arguments.
    args: &'static [&'static str],
}

const BASE_BODY: &str = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 2 -->
# Fixture

## Now

### T-001 · First task
- status: open
- owner: —
- priority: P1
";

/// One open task with a `verify` line that always fails, on any shell (`exit 1`).
const VERIFY_BODY: &str = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 2 -->
# Fixture

## Now

### T-001 · First task
- status: open
- owner: —
- priority: P1 · difficulty: 2
- verify: exit 1
";

const AMBIGUOUS_BODY: &str = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 2 -->
# Fixture

## Now

### T-001 · First task
- status: open
- owner: —
- priority: P1

### T-002 · First job
- status: open
- owner: —
- priority: P2
";

/// Two sections, `Now` and `Later`, with no `default_section` in `todo.toml`.
const TWO_SECTIONS_BODY: &str = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 2 -->
# Fixture

## Now

## Later
";

/// Two sections that the name `Next` matches, so `--section Next` is ambiguous (`AMBIGUOUS_SECTION`).
const TWO_NEXT_BODY: &str = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 2 -->
# Fixture

## Next steps

## Next release
";

const CONFLICT_BODY: &str = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 2 -->
# Broken

## Now

<<<<<<< HEAD
### T-001 · Conflicted
- status: open
- owner: —
- priority: P1
=======
### T-001 · Conflicted
- status: done
- owner: —
- priority: P1
>>>>>>> branch
";

const NEWER_BODY: &str = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 3 -->
<!-- todo-tool: 0.9.0 -->
# Newer

## Now
";

/// `todo.toml` that does not wait for the write lock or a lease.
const NO_WAIT: &str = "lock_wait_seconds = 0\n";

/// Two open tasks that name the same file, so one claim blocks the other (`FILES_BUSY`, R-SCOPE-1).
const FILES_BODY: &str = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 2 -->
# Fixture

## Now

### T-001 · First task
- status: open
- owner: —
- priority: P1
- files: src/a.c

### T-002 · Second task
- status: open
- owner: —
- priority: P1
- files: src/a.c
";

/// `add` of a second task in `Now`, as `agent` (the arguments after `--json --file`).
const ADD_SECOND_TASK: &[&str] = &[
    "--agent",
    "alice",
    "add",
    "--title",
    "Second task",
    "--priority",
    "P2",
    "--section",
    "Now",
];

/// One case per error code that a command makes with one process. `CHANGED_DURING_WRITE` needs a
/// second process, and is checked where that is made (see the test).
const ERROR_CASES: &[ErrorCase] = &[
    ErrorCase {
        code: "NO_FILE",
        todo: None,
        config: None,
        setup: Setup::Plain,
        before: &[],
        args: &["list"],
    },
    ErrorCase {
        code: "NO_AGENT",
        todo: Some(BASE_BODY),
        config: None,
        setup: Setup::Plain,
        before: &[],
        args: &[
            "add",
            "--title",
            "Second task",
            "--priority",
            "P2",
            "--section",
            "Now",
        ],
    },
    ErrorCase {
        code: "USAGE",
        todo: Some(BASE_BODY),
        config: None,
        setup: Setup::Plain,
        before: &[],
        args: &["list", "--no-such-option"],
    },
    ErrorCase {
        code: "NOT_FOUND",
        todo: Some(BASE_BODY),
        config: None,
        setup: Setup::Plain,
        before: &[],
        args: &["show", "T-999"],
    },
    ErrorCase {
        code: "AMBIGUOUS",
        todo: Some(AMBIGUOUS_BODY),
        config: None,
        setup: Setup::Plain,
        before: &[],
        args: &["show", "First"],
    },
    ErrorCase {
        code: "ALREADY_CLAIMED",
        todo: Some(BASE_BODY),
        config: None,
        setup: Setup::Plain,
        before: &[&["--agent", "alice", "claim", "T-001"]],
        args: &["--agent", "bob", "claim", "T-001"],
    },
    ErrorCase {
        code: "NOT_OWNER",
        todo: Some(BASE_BODY),
        config: None,
        setup: Setup::Plain,
        before: &[&["--agent", "alice", "claim", "T-001"]],
        args: &["--agent", "bob", "release", "T-001"],
    },
    ErrorCase {
        code: "BAD_TRANSITION",
        todo: Some(BASE_BODY),
        config: None,
        setup: Setup::Plain,
        before: &[],
        // R-WRITE-9: `unblock` is refused for an open task.
        args: &["--agent", "alice", "unblock", "T-001"],
    },
    ErrorCase {
        code: "DUPLICATE_TITLE",
        todo: Some(BASE_BODY),
        config: None,
        setup: Setup::Plain,
        before: &[],
        args: &[
            "--agent",
            "alice",
            "add",
            "--title",
            "First task",
            "--priority",
            "P2",
            "--section",
            "Now",
        ],
    },
    ErrorCase {
        code: "UNKNOWN_SECTION",
        todo: Some(BASE_BODY),
        config: None,
        setup: Setup::Plain,
        before: &[],
        args: &[
            "--agent",
            "alice",
            "add",
            "--title",
            "Second task",
            "--priority",
            "P2",
            "--section",
            "Nowhere",
        ],
    },
    // Without --section, in a file with several sections and no default_section, add refuses with
    // UNKNOWN_SECTION too (R-WRITE-1).
    ErrorCase {
        code: "UNKNOWN_SECTION",
        todo: Some(TWO_SECTIONS_BODY),
        config: None,
        setup: Setup::Plain,
        before: &[],
        args: &[
            "--agent",
            "alice",
            "add",
            "--title",
            "Second task",
            "--priority",
            "P2",
        ],
    },
    ErrorCase {
        code: "AMBIGUOUS_SECTION",
        todo: Some(TWO_NEXT_BODY),
        config: None,
        setup: Setup::Plain,
        before: &[],
        args: &[
            "--agent",
            "alice",
            "add",
            "--title",
            "Second task",
            "--priority",
            "P2",
            "--section",
            "Next",
        ],
    },
    ErrorCase {
        code: "MISSING_FIELD",
        todo: Some(BASE_BODY),
        config: None,
        setup: Setup::Plain,
        before: &[],
        args: &[
            "--agent",
            "alice",
            "add",
            "--title",
            "Second task",
            "--section",
            "Now",
        ],
    },
    ErrorCase {
        code: "BAD_VALUE",
        todo: Some(BASE_BODY),
        config: None,
        setup: Setup::Plain,
        before: &[],
        args: &[
            "--agent",
            "alice",
            "add",
            "--title",
            "Second task",
            "--priority",
            "P2",
            "--section",
            "Now",
            "--difficulty",
            "impossible",
        ],
    },
    ErrorCase {
        code: "BUSY",
        todo: Some(BASE_BODY),
        config: Some(NO_WAIT),
        setup: Setup::HeldWriteLock,
        before: &[],
        args: ADD_SECOND_TASK,
    },
    ErrorCase {
        code: "LEASE_HELD",
        todo: Some(BASE_BODY),
        config: Some(NO_WAIT),
        setup: Setup::Plain,
        before: &[&["--agent", "alice", "lock", "--minutes", "5"]],
        args: &[
            "--agent",
            "bob",
            "add",
            "--title",
            "Second task",
            "--priority",
            "P2",
            "--section",
            "Now",
        ],
    },
    ErrorCase {
        code: "UNSAFE_FILE",
        todo: Some(CONFLICT_BODY),
        config: None,
        setup: Setup::Plain,
        before: &[],
        args: ADD_SECOND_TASK,
    },
    ErrorCase {
        code: "READ_ONLY",
        todo: Some(BASE_BODY),
        config: None,
        setup: Setup::ReadOnly,
        before: &[],
        args: ADD_SECOND_TASK,
    },
    ErrorCase {
        code: "WRITE_FAILED",
        todo: Some(BASE_BODY),
        config: None,
        setup: Setup::RegisterFolderIsAFile,
        before: &[],
        args: &["project", "add"],
    },
    ErrorCase {
        code: "DUPLICATE_PROJECT",
        todo: Some(BASE_BODY),
        config: None,
        setup: Setup::Plain,
        before: &[&["project", "add"]],
        args: &["project", "add"],
    },
    ErrorCase {
        code: "VERIFY_FAILED",
        todo: Some(VERIFY_BODY),
        config: None,
        setup: Setup::Plain,
        before: &[],
        args: &["verify", "T-001", "--agent", "alice"],
    },
    ErrorCase {
        code: "VERIFY_NOT_RUN",
        todo: Some(VERIFY_BODY),
        config: Some("verify = false\n"),
        setup: Setup::Plain,
        before: &[],
        args: &["verify", "T-001", "--agent", "alice"],
    },
    ErrorCase {
        code: "NO_GIT_HISTORY",
        todo: Some(BASE_BODY),
        config: None,
        setup: Setup::Plain,
        before: &[],
        args: &["align"],
    },
    ErrorCase {
        code: "TOOL_TOO_OLD",
        todo: Some(NEWER_BODY),
        config: None,
        setup: Setup::Plain,
        before: &[],
        args: &["list"],
    },
    ErrorCase {
        code: "FILES_BUSY",
        todo: Some(FILES_BODY),
        config: None,
        setup: Setup::Plain,
        before: &[&["--agent", "alice", "claim", "T-001"]],
        args: &["--agent", "bob", "claim", "T-002"],
    },
];

/// `todo --json --file <todo>` plus `args`, run in `work`, with the register at `register`.
fn case_command(todo: &Path, work: &Path, register: &Path, args: &[&str]) -> Command {
    let mut cmd = todo_bin();
    cmd.current_dir(work)
        .env("TODO_REGISTER", register)
        .arg("--json")
        .arg("--file")
        .arg(todo)
        .args(args);
    cmd
}

/// Takes the write lock of `work/TODO.md` in this process, as another todo would hold it. Outside git
/// the state folder is `.todo` next to the file (R-HIST-1).
fn hold_write_lock(work: &Path) -> File {
    let state = work.join(".todo");
    fs::create_dir_all(&state).expect("create state folder");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(state.join("write.lock"))
        .expect("open write.lock");
    file.try_lock().expect("take the write lock");
    file
}

fn set_read_only(path: &Path, read_only: bool) {
    let mut permissions = fs::metadata(path).expect("metadata").permissions();
    permissions.set_readonly(read_only);
    fs::set_permissions(path, permissions).expect("set permissions");
}

#[test]
fn error_table_in_json_md_matches_todo_schema() {
    assert_eq!(
        error_table_exits(&json_doc()),
        schema_error_exits(),
        "docs/json.md and `todo schema` disagree on an exit code"
    );
}

#[test]
fn error_codes_exit_with_the_code_the_table_gives() {
    let table = error_table_exits(&json_doc());
    let mut triggered = BTreeSet::new();
    for case in ERROR_CASES {
        let dir = TempDir::new().expect("tempdir");
        let work = dir.path().join("work");
        fs::create_dir_all(&work).expect("create work folder");
        let todo = work.join("TODO.md");
        if let Some(body) = case.todo {
            fs::write(&todo, body).expect("write TODO.md");
        }
        if let Some(config) = case.config {
            fs::write(work.join("todo.toml"), config).expect("write todo.toml");
        }
        let register: PathBuf = if case.setup == Setup::RegisterFolderIsAFile {
            let blocker = work.join("blocker");
            fs::write(&blocker, "a file, not a folder").expect("write blocker");
            blocker.join("projects.json")
        } else {
            dir.path().join("register").join("projects.json")
        };

        let _lock = (case.setup == Setup::HeldWriteLock).then(|| hold_write_lock(&work));
        for step in case.before {
            let out = case_command(&todo, &work, &register, step)
                .output()
                .expect("run setup command");
            assert!(
                out.status.success(),
                "{}: setup {step:?} failed: {}",
                case.code,
                stderr_text(&out)
            );
        }
        if case.setup == Setup::ReadOnly {
            set_read_only(&todo, true);
        }
        let out = case_command(&todo, &work, &register, case.args)
            .output()
            .expect("run todo");
        if case.setup == Setup::ReadOnly {
            set_read_only(&todo, false);
        }

        let expected = *table
            .get(case.code)
            .unwrap_or_else(|| panic!("{} is not in the table of docs/json.md", case.code));
        assert_eq!(
            out.status.code().map(i64::from),
            Some(expected),
            "{} with {:?}: docs/json.md gives exit {expected}; stdout: {}; stderr: {}",
            case.code,
            case.args,
            String::from_utf8_lossy(&out.stdout),
            stderr_text(&out)
        );
        let json = parse_json(&out.stdout);
        assert_eq!(json["error"]["code"], case.code, "{json}");
        triggered.insert(case.code);
    }

    // CHANGED_DURING_WRITE needs another process to change TODO.md while a write waits for its rename.
    // tests/lock_lease.rs makes that happen with the test hooks and checks for exit 3.
    let elsewhere = ["CHANGED_DURING_WRITE"];
    for code in table.keys() {
        assert!(
            triggered.contains(code.as_str()) || elsewhere.contains(&code.as_str()),
            "no case in this test makes {code}, so its exit code is not checked"
        );
    }
}
