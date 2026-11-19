//! `todo report --html --all` (T-094; R-REPORT-2, R-LOCK-5, TEST-8): one page for every registered
//! project. The projects are invented, registered with `todo project add` in a temporary folder, and
//! the register is a file in that folder (`TODO_REGISTER`), never the user's own.

mod common;

use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::{Duration, Instant};

use serde_json::Value;
use tempfile::TempDir;

use common::{FIXTURE_MARKER, clean_todo_command, parse_json, stderr_text, stdout_text};

/// The clock every test runs with, so the page does not depend on the day.
const NOW: &str = "2026-11-19T10:00:00+01:00";

/// A healthy project with every kind of entry: tasks in each status, a milestone with a task that
/// is nested below another one, a done milestone, a dropped milestone, a blocked task whose latest
/// reason differs from an earlier one, a dropped task and an open question about a milestone.
fn renderer_body() -> String {
    format!(
        "{FIXTURE_MARKER}
<!-- todo-format: 2 -->
# Renderer

## Tasks

### T-001 · Setup
- status: done
- owner: —
- priority: P1 · area: docs
- milestone: T-010

### T-010 · Develop the renderer
- status: open
- owner: —
- priority: P1 · type: milestone
- goal: the test scene renders

### T-011 · Lighting
- status: open
- owner: —
- priority: P1 · type: milestone
- milestone: T-010
- goal: point lights match the reference images

### T-012 · C shader
- status: claimed
- owner: alice
- claimed: 2020-01-01T00:00+00:00
- priority: P1 · area: gpu
- milestone: T-011

### T-013 · Compute pass
- status: blocked
- owner: —
- priority: P2 · area: gpu
- note (2026-11-15, planner): blocked — waiting for the key
- note (2026-11-16, planner): blocked — waiting for the vendor driver

### T-014 · Ship lighting
- status: open
- owner: —
- priority: P2 · area: docs

### T-015 · Old idea
- status: dropped
- owner: —
- priority: P3
- note (2026-11-16, planner): dropped — superseded by T-012

### T-020 · Audio
- status: done
- owner: —
- priority: P2 · type: milestone
- goal: sound plays with the scene

### T-021 · Mix the sound
- status: done
- owner: —
- priority: P2
- milestone: T-020

### T-030 · Old plan
- status: dropped
- owner: —
- priority: P3 · type: milestone
- goal: abandoned

### T-040 · Which format for the scene files?
- status: open
- owner: —
- priority: P2 · type: question
- about: T-011
"
    )
}

/// A small project with a milestone that has no task below it yet and one blocked task.
fn lamp_body() -> String {
    format!(
        "{FIXTURE_MARKER}
<!-- todo-format: 2 -->
# Lamp room

## Tasks

### T-010 · Light the room
- status: open
- owner: —
- priority: P1 · type: milestone
- goal: the lamp lights the room

### T-001 · Trim the wick
- status: open
- owner: —
- priority: P1 · area: ui
- milestone: T-010

### T-002 · Polish the glass
- status: blocked
- owner: —
- priority: P2 · area: ui
- note (2026-11-17, bob): blocked — waiting for the glass supplier

### T-020 · Brass base
- status: open
- owner: —
- priority: P2 · type: milestone
- goal: the base is polished
"
    )
}

/// A project with the test's own state folder and files, and the register it is listed in.
struct World {
    _tmp: TempDir,
    home: PathBuf,
    /// The per-user folder that holds the register and the default page.
    per_user: PathBuf,
    register: PathBuf,
    root: PathBuf,
}

fn world() -> World {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path().to_path_buf();
    let home = root.join("home");
    fs::create_dir_all(&home).expect("create home");
    let per_user = root.join("per user");
    World {
        _tmp: tmp,
        home,
        register: per_user.join("projects.json"),
        per_user,
        root,
    }
}

/// `todo args` run in `dir` with the register of `world` and the fixed clock.
fn todo_in(world: &World, dir: &Path, args: &[&str]) -> Output {
    clean_todo_command(&world.home, dir)
        .env("TODO_REGISTER", &world.register)
        .env("TODO_TEST_NOW", NOW)
        .args(args)
        .output()
        .expect("run todo")
}

/// Creates `<root>/<name>/TODO.md` with `body` and returns the file.
fn project_file(world: &World, name: &str, body: &str) -> PathBuf {
    let dir = world.root.join(name);
    fs::create_dir_all(&dir).expect("create project folder");
    let file = dir.join("TODO.md");
    fs::write(&file, body).expect("write TODO.md");
    file
}

/// Registers the project in `file` under `name`, by `todo project add` run in its folder.
fn register(world: &World, file: &Path, name: &str) {
    let folder = file.parent().expect("folder");
    let out = todo_in(world, folder, &["project", "add", "--name", name]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
}

/// Runs `todo report --html --all` with `extra` options from `dir`.
fn report_all(world: &World, dir: &Path, extra: &[&str]) -> Output {
    let mut args = vec!["report", "--html", "--all"];
    args.extend_from_slice(extra);
    todo_in(world, dir, &args)
}

/// The default page, `report-all.html` next to the register.
fn default_page(world: &World) -> PathBuf {
    world.per_user.join("report-all.html")
}

/// Reads the default page after a successful report.
fn default_html(world: &World) -> String {
    fs::read_to_string(default_page(world)).expect("the default page")
}

/// The part of `html` that is the card of the project named `name`: from its `<li class="card">` to
/// the next card, or to the end of the card list. The name is the one the card heading shows, linked
/// or not.
fn card_of(html: &str, name: &str) -> String {
    let linked = format!("\">{name}</a></h3>");
    let plain = format!("<h3>{name}</h3>");
    let heading = html
        .find(&linked)
        .or_else(|| html.find(&plain))
        .unwrap_or_else(|| panic!("no card for {name}"));
    let card_start = html[..heading]
        .rfind("<li class=\"card\">")
        .expect("the card starts");
    let after = &html[heading..];
    let next_card = after.find("<li class=\"card\">");
    let list_end = after.find("</ul>\n</section>");
    let end = [next_card, list_end]
        .into_iter()
        .flatten()
        .min()
        .expect("the card ends");
    html[card_start..heading + end].to_string()
}

/// The paths of the registered projects, in register order, as the register stores them.
fn registered_paths(world: &World) -> Vec<String> {
    let text = fs::read_to_string(&world.register).expect("the register");
    let json: Value = serde_json::from_str(&text).expect("register JSON");
    json["projects"]
        .as_array()
        .expect("projects")
        .iter()
        .map(|p| p["path"].as_str().expect("path").to_string())
        .collect()
}

/// Every file under `root`, with its bytes, so a test can see what a command left behind.
fn snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
    let mut folders = vec![root.to_path_buf()];
    while let Some(folder) = folders.pop() {
        for entry in fs::read_dir(&folder).expect("read folder") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                folders.push(path);
            } else {
                files.push((path.clone(), fs::read(&path).expect("read file")));
            }
        }
    }
    files.sort();
    files
}

/// A world with the two invented projects registered: the renderer first, the lamp room second.
fn two_projects() -> (World, PathBuf, PathBuf) {
    let world = world();
    let renderer = project_file(&world, "renderer", &renderer_body());
    let lamp = project_file(&world, "lamp room", &lamp_body());
    register(&world, &renderer, "renderer");
    register(&world, &lamp, "lamp room");
    (world, renderer, lamp)
}

#[test]
fn all_writes_one_page_next_to_the_register_and_leaves_the_projects_alone() {
    let (world, renderer, lamp) = two_projects();
    let before: Vec<Vec<u8>> = [&renderer, &lamp]
        .iter()
        .map(|file| fs::read(file).expect("read TODO.md"))
        .collect();
    let folders_before = snapshot(&world.root.join("renderer"));
    let lamp_before = snapshot(&world.root.join("lamp room"));

    let out = report_all(&world, &world.root, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    let printed = stdout_text(&out);
    assert_eq!(
        printed.trim_end(),
        default_page(&world).display().to_string()
    );
    assert!(default_page(&world).is_file());

    // Each TODO.md is as it was, and no state folder or other file appeared in a project.
    let after: Vec<Vec<u8>> = [&renderer, &lamp]
        .iter()
        .map(|file| fs::read(file).expect("read TODO.md"))
        .collect();
    assert_eq!(after, before);
    assert_eq!(snapshot(&world.root.join("renderer")), folders_before);
    assert_eq!(snapshot(&world.root.join("lamp room")), lamp_before);
    assert!(!world.root.join("renderer").join(".todo").exists());
    assert!(!world.root.join("lamp room").join(".todo").exists());
}

#[test]
fn each_project_has_a_card_and_then_its_own_lists_with_unique_anchors() {
    let (world, _renderer, _lamp) = two_projects();
    assert_eq!(report_all(&world, &world.root, &[]).status.code(), Some(0));
    let html = default_html(&world);

    let cards = html.find("<section id=\"projects\">").expect("cards");
    let first = html
        .find("<section class=\"project\" id=\"p1\">")
        .expect("p1");
    let second = html
        .find("<section class=\"project\" id=\"p2\">")
        .expect("p2");
    assert!(
        cards < first && first < second,
        "cards come first, then the projects"
    );

    // Each project's rows carry its own prefix, so the same ID in two projects does not clash.
    assert_eq!(html.matches("id=\"p1-e-T-010\"").count(), 1);
    assert_eq!(html.matches("id=\"p2-e-T-010\"").count(), 1);
    assert!(html.contains("id=\"p1-e-T-013\""));
    assert!(html.contains("id=\"p2-e-T-002\""));
    assert!(
        !html.contains("id=\"e-T-"),
        "an ID without a project prefix"
    );
    assert_eq!(html.matches("<section id=\"p1-work\">").count(), 1);
    assert_eq!(html.matches("id=\"p2-work\"").count(), 1);

    // The card links to its own project's section and rows, never to another project's.
    let renderer_card = card_of(&html, "renderer");
    assert!(renderer_card.contains("href=\"#p1\""));
    assert!(!renderer_card.contains("#p2"));
    let lamp_card = card_of(&html, "lamp room");
    assert!(lamp_card.contains("href=\"#p2\""));
    assert!(!lamp_card.contains("#p1-"));
}

#[test]
fn a_card_shows_tasks_per_status_milestones_with_progress_blocked_tasks_and_questions() {
    let (world, _renderer, _lamp) = two_projects();
    assert_eq!(report_all(&world, &world.root, &[]).status.code(), Some(0));
    let html = default_html(&world);
    let card = card_of(&html, "renderer");

    // Tasks per status, as todo overview counts them: T-001 and T-021 are done, T-012 claimed,
    // T-013 blocked, T-014 open and T-015 dropped. Milestones and questions are not tasks.
    for (n, label) in [
        (1, "open"),
        (1, "claimed"),
        (1, "blocked"),
        (2, "done"),
        (1, "dropped"),
    ] {
        let snippet = format!("<li><span class=\"n\">{n}</span> {label}</li>");
        assert!(card.contains(&snippet), "missing {snippet} in {card}");
    }

    // Top-level milestones only, with their progress over the tasks below them: T-011 sits below
    // T-010, and the dropped T-030 is left out. T-010 has T-001 done and T-012 open below it.
    assert!(card.contains("<a href=\"#p1-e-T-010\">T-010 · Develop the renderer</a>"));
    assert!(card.contains("<progress value=\"1\" max=\"2\">1 of 2 done</progress>"));
    assert!(
        card.contains("<a href=\"#p1-e-T-020\">T-020 · Audio</a> <span class=\"chip\">done</span>")
    );
    assert!(card.contains("<progress value=\"1\" max=\"1\">1 of 1 done</progress>"));
    assert!(!card.contains("T-011 ·"));
    assert!(!card.contains("T-030"));

    // The blocked task shows its latest reason, and links to its row in the lists.
    assert!(card.contains(
        "<a href=\"#p1-e-T-013\">T-013 · Compute pass</a> <span class=\"reason\">blocked: waiting for the vendor driver</span>"
    ));
    assert!(!card.contains("waiting for the key"));

    // The open question links to its row in the questions list.
    assert!(card.contains("<a href=\"#p1-e-T-040\">T-040 · Which format for the scene files?</a>"));
    assert!(html.contains("id=\"p1-e-T-040\""));
    assert_eq!(html.matches("id=\"p1-e-T-040\"").count(), 1);

    // The lamp room has one open task, one blocked task, and a milestone with nothing below yet.
    let lamp = card_of(&html, "lamp room");
    assert!(lamp.contains("<span class=\"n\">1</span> open"));
    assert!(lamp.contains("<progress value=\"0\" max=\"1\">0 of 1 done</progress>"));
    assert!(lamp.contains("<span class=\"n\">1</span> blocked"));
    assert!(lamp.contains("<span class=\"n\">0</span> done"));
    assert!(lamp.contains("no tasks below yet"));
    assert!(lamp.contains("No open questions."));
}

#[test]
fn the_snapshot_of_two_invented_projects_with_a_fixed_clock() {
    let (world, _renderer, _lamp) = two_projects();
    assert_eq!(report_all(&world, &world.root, &[]).status.code(), Some(0));
    let mut html = default_html(&world);
    // The project folders are temporary, so their paths are replaced by fixed names.
    let paths = registered_paths(&world);
    html = html.replace(&paths[0], "<renderer>/TODO.md");
    html = html.replace(&paths[1], "<lamp room>/TODO.md");
    assert!(!html.contains(world.root.to_str().expect("utf-8 root")));
    insta::assert_snapshot!("report_all_html", html);
}

#[test]
fn a_project_that_is_gone_shows_its_problem_and_the_others_their_lists() {
    let (world, renderer, _lamp) = two_projects();
    fs::remove_file(&renderer).expect("remove the renderer's TODO.md");

    let out = report_all(&world, &world.root, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    let html = default_html(&world);

    let renderer_card = card_of(&html, "renderer");
    assert!(renderer_card.contains(
        "<p class=\"problem\">This project cannot be shown: TODO.md not found <span class=\"chip\">NO_FILE</span></p>"
    ));
    assert!(
        !renderer_card.contains("counts"),
        "a problem card shows no lists"
    );
    assert!(!html.contains("<section class=\"project\" id=\"p1\">"));
    assert!(!html.contains("id=\"p1-"));

    // The other project is shown in full, and keeps its own section.
    assert!(html.contains("<section class=\"project\" id=\"p2\">"));
    assert!(card_of(&html, "lamp room").contains("<span class=\"n\">1</span> open"));
}

#[test]
fn a_project_in_a_newer_format_shows_tool_too_old_and_the_others_still_show() {
    let (world, renderer, _lamp) = two_projects();
    let newer = format!("{FIXTURE_MARKER}\n<!-- todo-format: 99 -->\n# Renderer\n\n## Tasks\n");
    fs::write(&renderer, newer).expect("write the newer file");

    let out = report_all(&world, &world.root, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    let html = default_html(&world);
    let renderer_card = card_of(&html, "renderer");
    assert!(renderer_card.contains("cannot be shown"));
    assert!(renderer_card.contains("<span class=\"chip\">TOOL_TOO_OLD</span>"));
    assert!(!html.contains("id=\"p1-"));
    assert!(html.contains("<section class=\"project\" id=\"p2\">"));
}

#[test]
fn a_project_that_is_not_utf8_shows_its_problem() {
    let (world, renderer, _lamp) = two_projects();
    let mut bytes = FIXTURE_MARKER.as_bytes().to_vec();
    bytes.extend_from_slice(b"\n# Renderer\n\xff\xfe broken\n");
    fs::write(&renderer, bytes).expect("write the broken file");

    let out = report_all(&world, &world.root, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    let html = default_html(&world);
    let renderer_card = card_of(&html, "renderer");
    assert!(renderer_card.contains("cannot be shown: invalid UTF-8 at byte"));
    assert!(renderer_card.contains("<span class=\"chip\">UNSAFE_FILE</span>"));
    assert!(!html.contains("id=\"p1-"));
}

#[test]
fn a_file_that_blocks_all_writes_shows_its_problem_as_overview_does() {
    let (world, renderer, _lamp) = two_projects();
    let conflicted = format!(
        "{FIXTURE_MARKER}\n<!-- todo-format: 2 -->\n# Renderer\n\n## Tasks\n\n\
### T-001 · Conflicted task\n<<<<<<< HEAD\n- status: open\n=======\n- status: claimed\n>>>>>>> feature\n"
    );
    fs::write(&renderer, conflicted).expect("write the conflicted file");

    let out = report_all(&world, &world.root, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    let html = default_html(&world);
    let renderer_card = card_of(&html, "renderer");
    assert!(renderer_card.contains("cannot be shown:"));
    assert!(renderer_card.contains("<span class=\"chip\">UNSAFE_FILE</span>"));
    assert!(!html.contains("Conflicted task"));
    assert!(!html.contains("id=\"p1-"));

    // The overview of the same register refuses the same file the same way.
    let overview = todo_in(&world, &world.root, &["overview"]);
    assert_eq!(
        overview.status.code(),
        Some(0),
        "{}",
        stderr_text(&overview)
    );
    assert!(stdout_text(&overview).contains("problem:"));
}

#[test]
fn a_warning_about_a_file_is_printed_with_its_path_and_its_lists_are_shown() {
    let (world, _renderer, _lamp) = two_projects();
    // T-001 is in the renderer's archive as well: a warning, not a reason to leave the lists out.
    let archive = format!(
        "{FIXTURE_MARKER}\n<!-- todo-format: 2 -->\n# Renderer\n\n## Archive\n\n\
### T-001 · Setup\n- status: done\n- owner: —\n"
    );
    fs::write(world.root.join("renderer").join("TODO-archive.md"), archive).expect("archive");

    let out = report_all(&world, &world.root, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    let html = default_html(&world);
    assert!(html.contains("<section class=\"project\" id=\"p1\">"));
    assert!(html.contains("<section class=\"project\" id=\"p2\">"));
    // The warning is printed on standard error with the path of the file it is about.
    let err = stderr_text(&out);
    let path = &registered_paths(&world)[0];
    assert!(err.contains(path.as_str()), "{err}");
}

#[test]
fn an_empty_or_missing_register_gives_a_page_that_says_how_to_register_in_words() {
    let world = world();
    // Missing register: the page is still written, next to where the register would be.
    let out = report_all(&world, &world.root, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    assert!(
        !world.register.exists(),
        "the report must not create the register"
    );
    let html = default_html(&world);
    assert!(html.contains("No project is registered yet."));
    assert!(html.contains("ask an agent working in that folder to register the project"));
    assert!(!html.contains("todo project"), "no copy-ready command");
    assert!(!html.contains("<code"));
    assert!(!html.contains("<section class=\"project\""));

    // An empty register gives the same page.
    fs::create_dir_all(&world.per_user).expect("create the folder");
    fs::write(&world.register, "{\"format\": 1, \"projects\": []}\n").expect("write the register");
    let out = report_all(&world, &world.root, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    assert!(default_html(&world).contains("No project is registered yet."));
}

#[test]
fn out_writes_to_the_given_path_with_spaces_and_non_ascii_and_not_next_to_the_register() {
    let (world, _renderer, _lamp) = two_projects();
    let folder = world.root.join("Tâche finale");
    fs::create_dir_all(&folder).expect("create folder");
    let target = folder.join("Alle Projekte.html");

    let out = report_all(
        &world,
        &world.root,
        &["--out", target.to_str().expect("utf-8 path")],
    );
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    assert_eq!(stdout_text(&out).trim_end(), target.display().to_string());
    let html = fs::read_to_string(&target).expect("the page");
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("<section class=\"project\" id=\"p2\">"));
    assert!(!default_page(&world).exists());
}

#[test]
fn out_into_a_missing_folder_fails_and_writes_nothing() {
    let (world, _renderer, _lamp) = two_projects();
    let target = world.root.join("no such folder").join("all.html");

    let out = todo_in(
        &world,
        &world.root,
        &[
            "--json",
            "report",
            "--html",
            "--all",
            "--out",
            target.to_str().expect("utf-8 path"),
        ],
    );
    assert_ne!(out.status.code(), Some(0));
    let json = parse_json(&out.stdout);
    assert_eq!(json["ok"], false);
    assert_eq!(json["error"]["code"], "WRITE_FAILED");
    assert!(!target.exists());
}

#[test]
fn all_without_html_is_a_usage_error() {
    let (world, _renderer, _lamp) = two_projects();
    let out = todo_in(&world, &world.root, &["report", "--all"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr_text(&out).contains("--html"),
        "{}",
        stderr_text(&out)
    );
    assert!(!default_page(&world).exists());
}

#[test]
fn json_reports_the_path_the_size_and_whether_the_page_was_opened() {
    let (world, _renderer, _lamp) = two_projects();
    let out = todo_in(
        &world,
        &world.root,
        &["--json", "report", "--html", "--all"],
    );
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    let json = parse_json(&out.stdout);
    assert_eq!(json["schema"], "todo.report.v1");
    assert_eq!(json["data"]["opened"], false);
    assert!(
        json["data"]["path"]
            .as_str()
            .expect("path")
            .ends_with("report-all.html")
    );
    assert_eq!(
        json["data"]["bytes"].as_u64().expect("bytes"),
        fs::metadata(default_page(&world)).expect("page").len()
    );
}

#[test]
fn all_works_from_a_folder_without_a_task_list_and_ignores_the_current_project() {
    let (world, _renderer, _lamp) = two_projects();
    let empty = world.root.join("nowhere");
    fs::create_dir_all(&empty).expect("create folder");
    let out = report_all(&world, &empty, &[]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    assert!(default_html(&world).contains("<section class=\"project\" id=\"p2\">"));
}

#[test]
fn all_takes_no_lock_even_while_the_register_lock_is_held() {
    let (world, _renderer, _lamp) = two_projects();
    let lock_path = world.per_user.join("projects.json.lock");
    let lock = File::options()
        .read(true)
        .write(true)
        .open(&lock_path)
        .expect("the register's lock file");
    lock.try_lock().expect("the test holds the register lock");

    let started = Instant::now();
    let out = report_all(&world, &world.root, &[]);
    let waited = started.elapsed();
    drop(lock);

    assert_eq!(out.status.code(), Some(0), "{}", stderr_text(&out));
    assert!(
        waited < Duration::from_secs(20),
        "waited {waited:?} for the lock"
    );
}

#[test]
fn the_text_of_every_project_is_escaped() {
    let world = world();
    let body = format!(
        "{FIXTURE_MARKER}\n<!-- todo-format: 2 -->\n# Harbour <i>lights</i>\n\n## Now\n\n\
### T-001 · <script>alert(\"x\")</script>\n- status: open\n- owner: —\n- priority: P1 · area: <b>ui</b>\n"
    );
    let file = project_file(&world, "harbour", &body);
    register(&world, &file, "harbour <b>one</b>");
    assert_eq!(report_all(&world, &world.root, &[]).status.code(), Some(0));
    let html = default_html(&world);
    assert!(html.contains("&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt;"));
    assert!(html.contains("harbour &lt;b&gt;one&lt;/b&gt;"));
    assert!(!html.contains("<script"));
    assert!(!html.contains("<b>"));
    assert!(!html.contains("<i>"));
}

#[test]
fn the_page_has_no_script_no_network_load_and_no_controls() {
    let (world, _renderer, _lamp) = two_projects();
    assert_eq!(report_all(&world, &world.root, &[]).status.code(), Some(0));
    let html = default_html(&world);
    for banned in [
        "<script",
        "<link",
        "<img",
        "<iframe",
        "<object",
        "<embed",
        "<form",
        "<button",
        "<input",
        "<select",
        "<textarea",
        "@import",
        "url(",
        "src=",
        "http://",
        "https://",
    ] {
        assert!(!html.contains(banned), "the page contains {banned:?}");
    }
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("<style>"));
    assert!(html.contains("prefers-color-scheme: dark"));
    assert!(html.contains("max-width: 600px"));
}

#[test]
fn archived_entries_are_listed_under_their_own_project() {
    let (world, _renderer, _lamp) = two_projects();
    let archive = format!(
        "{FIXTURE_MARKER}\n<!-- todo-format: 2 -->\n# Renderer\n\n## Archive\n\n\
### T-090 · Archived done task\n- status: done\n- owner: —\n- note (2026-11-01, alice): done\n"
    );
    fs::write(world.root.join("renderer").join("TODO-archive.md"), archive).expect("archive");

    assert_eq!(report_all(&world, &world.root, &[]).status.code(), Some(0));
    let html = default_html(&world);
    let renderer_done = html
        .split("<section class=\"project\" id=\"p2\">")
        .next()
        .expect("renderer part");
    assert!(renderer_done.contains("Archived done task"));
    assert!(renderer_done.contains("<span class=\"chip\">archived</span>"));
    assert!(
        !html
            .split("id=\"p2\">")
            .nth(1)
            .unwrap_or_default()
            .contains("Archived done task")
    );
}
