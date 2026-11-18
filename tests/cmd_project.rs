//! `todo project add`, `remove` and `list` (T-093; R-PROJ-1, TEST-5, TEST-6, TEST-8).
//!
//! Every test points the register at a temporary folder, through `TODO_REGISTER` or through the
//! user data folders of a temporary home, so the real register of the person who runs the tests is
//! never read or written. The project folders and their `TODO.md` files are invented.

mod common;

use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Child, Output, Stdio};

use serde_json::Value;
use tempfile::TempDir;

use common::{FIXTURE_MARKER, clean_todo_command, parse_json, stderr_text, stdout_text};

/// The invented `TODO.md` that every project folder of these tests holds.
fn project_body() -> String {
    format!(
        "{FIXTURE_MARKER}\n<!-- todo-format: 2 -->\n<!-- todo-tool: 0.6.0 -->\n# Fixture\n\n## Tasks\n\n### T-001 · First task\n- status: open\n- owner: —\n- priority: P1\n"
    )
}

/// Creates `<base>/<name>` with a `TODO.md` in it and returns the folder.
fn project_folder(base: &Path, name: &str) -> PathBuf {
    let dir = base.join(name);
    fs::create_dir_all(&dir).expect("create project folder");
    fs::write(dir.join("TODO.md"), project_body()).expect("write TODO.md");
    dir
}

/// The register file of a test: `<tmp>/register/projects.json`. Its folder does not exist yet.
fn register_in(tmp: &TempDir) -> PathBuf {
    tmp.path().join("register").join("projects.json")
}

/// A `todo` command in `dir` with `TODO_REGISTER` set to `register`.
fn todo_command(home: &Path, dir: &Path, register: &Path) -> std::process::Command {
    let mut cmd = clean_todo_command(home, dir);
    cmd.env("TODO_REGISTER", register);
    cmd
}

/// Runs `todo args` in `dir` against `register`.
fn run_project(home: &Path, dir: &Path, register: &Path, args: &[&str]) -> Output {
    todo_command(home, dir, register)
        .args(args)
        .output()
        .expect("run todo")
}

/// Parses the register file on disk.
fn read_register_json(register: &Path) -> Value {
    let text = fs::read_to_string(register).expect("read register");
    serde_json::from_str(&text).expect("register is JSON")
}

/// The project names in the register file, in order.
fn names_in(register: &Path) -> Vec<String> {
    read_register_json(register)["projects"]
        .as_array()
        .expect("projects array")
        .iter()
        .map(|p| p["name"].as_str().expect("name").to_owned())
        .collect()
}

/// Asserts that `output` failed with `code` and exit status `exit`, and returns its JSON error.
fn assert_json_error(output: &Output, code: &str, exit: i32) -> Value {
    assert_eq!(
        output.status.code(),
        Some(exit),
        "stdout: {}\nstderr: {}",
        stdout_text(output),
        stderr_text(output)
    );
    let json = parse_json(&output.stdout);
    assert_eq!(json["ok"], false, "{json}");
    assert_eq!(json["error"]["code"], code, "{json}");
    json
}

#[test]
fn add_registers_the_folder_of_the_current_folder_by_default() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let register = register_in(&tmp);
    let work = project_folder(tmp.path(), "Grafik Engine Ärger");

    let out = run_project(&home, &work, &register, &["project", "add"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    let text = stdout_text(&out);
    assert!(
        text.starts_with("registered Grafik Engine Ärger: "),
        "unexpected output: {text}"
    );
    assert!(text.trim_end().ends_with("TODO.md"), "{text}");

    let json = read_register_json(&register);
    assert_eq!(json["format"], 1);
    let project = &json["projects"][0];
    assert_eq!(project["name"], "Grafik Engine Ärger");
    let path = project["path"].as_str().expect("path");
    assert!(Path::new(path).is_absolute(), "{path}");
    assert!(path.ends_with("TODO.md"), "{path}");
    let added = project["added"].as_str().expect("added");
    assert_eq!(added.len(), 22, "local time with offset: {added}");
    assert_eq!(added.as_bytes()[10], b'T', "{added}");
}

#[test]
fn add_takes_a_folder_and_a_name_and_json_shows_the_project() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let register = register_in(&tmp);
    let renderer = project_folder(tmp.path(), "renderer");
    let elsewhere = project_folder(tmp.path(), "elsewhere");

    let arg = renderer.display().to_string();
    let out = run_project(
        &home,
        &elsewhere,
        &register,
        &["--json", "project", "add", &arg, "--name", "renderer"],
    );
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    let json = parse_json(&out.stdout);
    assert_eq!(json["schema"], "todo.project.v1");
    assert_eq!(json["ok"], true);
    assert_eq!(json["data"]["project"]["name"], "renderer");
    assert_eq!(
        json["data"]["register"],
        register.display().to_string(),
        "the register that was written"
    );
    assert_eq!(names_in(&register), vec!["renderer".to_owned()]);
}

#[test]
fn add_searches_up_from_a_subfolder_and_stops_at_the_repository_root() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let register = register_in(&tmp);

    // A repository with its own TODO.md, added from a subfolder (R-FIND-3).
    let repo = tmp.path().join("repo");
    fs::create_dir_all(repo.join(".git")).unwrap();
    fs::write(repo.join("TODO.md"), project_body()).unwrap();
    let sub = repo.join("sub");
    fs::create_dir_all(&sub).unwrap();
    let out = run_project(&home, &sub, &register, &["project", "add"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    assert_eq!(names_in(&register), vec!["repo".to_owned()]);

    // A repository without a TODO.md under a folder that has one: the search stops at the
    // repository root, so the parent's file is not registered (R-FIND-4).
    fs::write(tmp.path().join("TODO.md"), project_body()).unwrap();
    let bare = tmp.path().join("bare");
    fs::create_dir_all(bare.join(".git")).unwrap();
    let bare_sub = bare.join("sub");
    fs::create_dir_all(&bare_sub).unwrap();
    let out = run_project(&home, &bare_sub, &register, &["--json", "project", "add"]);
    assert_json_error(&out, "NO_FILE", 4);
    assert_eq!(names_in(&register), vec!["repo".to_owned()]);
}

/// Asserts that `output` failed with exit status `exit`, without looking at its JSON.
fn assert_json_error_free(output: &Output, exit: i32) {
    assert_eq!(
        output.status.code(),
        Some(exit),
        "stdout: {}\nstderr: {}",
        stdout_text(output),
        stderr_text(output)
    );
}

#[test]
fn add_without_a_todo_md_is_refused_and_creates_no_register() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let register = register_in(&tmp);
    let empty = tmp.path().join("empty-repo");
    fs::create_dir_all(empty.join(".git")).unwrap();

    let out = run_project(&home, &empty, &register, &["--json", "project", "add"]);
    assert_json_error(&out, "NO_FILE", 4);
    assert!(!register.exists(), "no register must be written");
    assert!(
        !register.parent().unwrap().exists(),
        "no folder must be created either"
    );
}

#[test]
fn add_refuses_a_folder_that_is_not_there() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let register = register_in(&tmp);
    let missing = tmp.path().join("missing");

    let arg = missing.display().to_string();
    let out = run_project(
        &home,
        tmp.path(),
        &register,
        &["--json", "project", "add", &arg],
    );
    assert_json_error(&out, "NO_FILE", 4);
    assert!(!register.exists());
}

#[test]
fn the_same_file_twice_is_refused_and_the_register_is_unchanged() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let register = register_in(&tmp);
    let renderer = project_folder(tmp.path(), "renderer");

    let first = run_project(&home, &renderer, &register, &["project", "add"]);
    assert_json_error_free(&first, 0);
    let before = fs::read(&register).unwrap();

    // The same folder written another way: with `/./` in it, and with the name given.
    let spelled = format!("{}/./", renderer.display());
    let out = run_project(
        &home,
        tmp.path(),
        &register,
        &["--json", "project", "add", &spelled, "--name", "other"],
    );
    assert_json_error(&out, "DUPLICATE_PROJECT", 1);
    assert_eq!(
        fs::read(&register).unwrap(),
        before,
        "register must not change"
    );
}

#[test]
fn the_same_name_for_another_file_is_refused_whatever_the_letter_case() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let register = register_in(&tmp);
    let renderer = project_folder(tmp.path(), "renderer");
    let audio = project_folder(tmp.path(), "audio");

    let first = run_project(
        &home,
        &renderer,
        &register,
        &["project", "add", "--name", "Renderer"],
    );
    assert_json_error_free(&first, 0);
    let before = fs::read(&register).unwrap();

    let out = run_project(
        &home,
        &audio,
        &register,
        &["--json", "project", "add", "--name", "renderer"],
    );
    assert_json_error(&out, "DUPLICATE_PROJECT", 1);
    assert_eq!(fs::read(&register).unwrap(), before);
}

#[test]
fn a_name_must_be_one_line_and_not_empty() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let register = register_in(&tmp);
    let renderer = project_folder(tmp.path(), "renderer");

    for name in ["   ", "two\nlines"] {
        let out = run_project(
            &home,
            &renderer,
            &register,
            &["--json", "project", "add", "--name", name],
        );
        assert_json_error(&out, "USAGE", 2);
    }
    assert!(!register.exists());
}

#[test]
fn remove_takes_a_name_or_a_path_and_refuses_an_unknown_one() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let register = register_in(&tmp);
    let renderer = project_folder(tmp.path(), "renderer");
    let audio = project_folder(tmp.path(), "audio");
    let video = project_folder(tmp.path(), "video");
    for folder in [&renderer, &audio, &video] {
        let out = run_project(&home, folder, &register, &["project", "add"]);
        assert_json_error_free(&out, 0);
    }
    assert_eq!(names_in(&register), ["renderer", "audio", "video"]);

    // By name, ignoring case.
    let out = run_project(
        &home,
        tmp.path(),
        &register,
        &["project", "remove", "RENDERER"],
    );
    assert_json_error_free(&out, 0);
    assert!(stdout_text(&out).starts_with("removed renderer: "));

    // By the path of the folder: its TODO.md is the project.
    let arg = audio.display().to_string();
    let out = run_project(&home, tmp.path(), &register, &["project", "remove", &arg]);
    assert_json_error_free(&out, 0);
    assert_eq!(names_in(&register), ["video"]);

    // Unknown names and paths: NOT_FOUND, exit 1, nothing changed.
    let before = fs::read(&register).unwrap();
    let out = run_project(
        &home,
        tmp.path(),
        &register,
        &["--json", "project", "remove", "nope"],
    );
    assert_json_error(&out, "NOT_FOUND", 1);
    assert_eq!(fs::read(&register).unwrap(), before);
}

#[test]
fn remove_without_a_register_is_not_found_and_creates_nothing() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let register = register_in(&tmp);

    let out = run_project(
        &home,
        tmp.path(),
        &register,
        &["--json", "project", "remove", "renderer"],
    );
    assert_json_error(&out, "NOT_FOUND", 1);
    assert!(!register.parent().unwrap().exists());
}

#[test]
fn list_shows_each_project_and_an_empty_register_says_so() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let register = register_in(&tmp);

    let out = run_project(&home, tmp.path(), &register, &["project", "list"]);
    assert_json_error_free(&out, 0);
    assert!(stdout_text(&out).starts_with("No projects are registered in "));
    let json = parse_json(
        &run_project(&home, tmp.path(), &register, &["--json", "project", "list"]).stdout,
    );
    assert_eq!(json["schema"], "todo.project.v1");
    assert_eq!(json["data"]["projects"], serde_json::json!([]));
    assert!(!register.exists(), "listing must not create the register");

    let renderer = project_folder(tmp.path(), "renderer");
    let audio = project_folder(tmp.path(), "audio");
    for folder in [&renderer, &audio] {
        let out = run_project(&home, folder, &register, &["project", "add"]);
        assert_json_error_free(&out, 0);
    }

    let out = run_project(&home, tmp.path(), &register, &["project", "list"]);
    assert_json_error_free(&out, 0);
    let lines: Vec<String> = stdout_text(&out).lines().map(str::to_owned).collect();
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(lines[0].starts_with("renderer | "), "{lines:?}");
    assert!(lines[0].contains("TODO.md | added "), "{lines:?}");
    assert!(lines[1].starts_with("audio | "), "{lines:?}");

    let json = parse_json(
        &run_project(&home, tmp.path(), &register, &["--json", "project", "list"]).stdout,
    );
    let projects = json["data"]["projects"].as_array().expect("projects");
    assert_eq!(projects.len(), 2);
    for project in projects {
        assert!(project["name"].is_string());
        assert!(project["path"].is_string());
        assert!(project["added"].is_string());
    }
}

#[test]
fn parallel_adds_lose_nothing() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let register = register_in(&tmp);

    let folders: Vec<PathBuf> = (0..20)
        .map(|i| project_folder(tmp.path(), &format!("project {i:02} Ärger")))
        .collect();
    let children: Vec<Child> = folders
        .iter()
        .enumerate()
        .map(|(i, folder)| {
            todo_command(&home, folder, &register)
                .args(["project", "add", "--name", &format!("p{i:02}")])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn todo")
        })
        .collect();
    for child in children {
        let out = child.wait_with_output().expect("wait for todo");
        assert_json_error_free(&out, 0);
    }

    let mut names = names_in(&register);
    assert_eq!(names.len(), 20, "{names:?}");
    names.sort();
    names.dedup();
    assert_eq!(names.len(), 20, "every add must be kept: {names:?}");
}

#[test]
fn the_register_defaults_to_the_user_data_folder() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let renderer = project_folder(tmp.path(), "renderer");

    // `clean_todo_command` points LOCALAPPDATA and XDG_DATA_HOME into `home`, and TODO_REGISTER is
    // not set.
    let expected = if cfg!(windows) {
        home.join("local-appdata")
            .join("todo")
            .join("projects.json")
    } else {
        home.join("xdg-data").join("todo").join("projects.json")
    };
    let out = clean_todo_command(&home, &renderer)
        .args(["project", "add"])
        .output()
        .expect("run todo");
    assert_json_error_free(&out, 0);
    assert!(expected.is_file(), "register not at {}", expected.display());

    let out = clean_todo_command(&home, tmp.path())
        .args(["--json", "project", "list"])
        .output()
        .expect("run todo");
    let json = parse_json(&out.stdout);
    assert_eq!(json["data"]["register"], expected.display().to_string());
    assert_eq!(json["data"]["projects"][0]["name"], "renderer");
}

#[test]
fn a_register_that_another_todo_holds_is_busy_and_nothing_is_written() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let register = register_in(&tmp);
    fs::create_dir_all(register.parent().unwrap()).unwrap();
    let renderer = project_folder(tmp.path(), "renderer");

    // Another process holds the register's lock: take it here, as a second process would.
    let lock_path = register.with_file_name("projects.json.lock");
    let held = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .expect("open lock file");
    held.try_lock().expect("take the lock");

    let out = todo_command(&home, &renderer, &register)
        .env("TODO_TEST_LOCK_WAIT_MS", "200")
        .args(["--json", "project", "add"])
        .output()
        .expect("run todo");
    assert_json_error(&out, "BUSY", 3);
    assert!(!register.exists(), "a busy register must not be written");

    held.unlock().expect("release the lock");
    let out = run_project(&home, &renderer, &register, &["project", "add"]);
    assert_json_error_free(&out, 0);
    assert_eq!(names_in(&register), ["renderer"]);
}

#[test]
fn a_register_that_is_not_a_register_or_is_newer_is_refused_and_kept() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let register = register_in(&tmp);
    fs::create_dir_all(register.parent().unwrap()).unwrap();
    let renderer = project_folder(tmp.path(), "renderer");

    for (body, code, exit) in [
        ("not json at all", "UNSAFE_FILE", 4),
        (r#"{"format": 2, "projects": []}"#, "TOOL_TOO_OLD", 4),
    ] {
        fs::write(&register, body).unwrap();
        let out = run_project(&home, &renderer, &register, &["--json", "project", "list"]);
        assert_json_error(&out, code, exit);
        let out = run_project(&home, &renderer, &register, &["--json", "project", "add"]);
        assert_json_error(&out, code, exit);
        assert_eq!(
            fs::read_to_string(&register).unwrap(),
            body,
            "file must be kept"
        );
    }
}
