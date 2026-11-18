//! Tests for `todo idea`, `todo promote`, and how ideas are kept apart from work in `next` and
//! `summary` (T-097; R-KIND-1, R-KIND-3).

mod common;

use std::fs;
use std::process::Output;

use serde_json::Value;
use tempfile::TempDir;

use common::{clean_todo_command, parse_json, read_text, stderr_text, stdout_text};

/// Fixed clock for note dates (`TODO_TEST_NOW`, honoured in test builds).
const NOW: &str = "2026-11-18T10:00:00+01:00";

/// A project folder with its own home folder, so no test reads the developer's settings.
struct Project {
    dir: TempDir,
    home: TempDir,
}

impl Project {
    fn new(todo_md: &str) -> Self {
        let dir = TempDir::new().expect("tempdir");
        let home = TempDir::new().expect("tempdir");
        fs::write(dir.path().join("TODO.md"), todo_md).expect("write TODO.md");
        Self { dir, home }
    }

    fn with_config(self, toml: &str) -> Self {
        fs::write(self.dir.path().join("todo.toml"), toml).expect("write todo.toml");
        self
    }

    /// `TODO.md` as it is on disk now.
    fn text(&self) -> String {
        read_text(&self.dir.path().join("TODO.md"))
    }

    /// Runs `todo --agent <agent> <args>` in the project folder; an empty `agent` passes no agent.
    fn run(&self, agent: &str, args: &[&str]) -> Output {
        let mut all = Vec::new();
        if !agent.is_empty() {
            all.extend(["--agent", agent]);
        }
        all.extend_from_slice(args);
        clean_todo_command(self.home.path(), self.dir.path())
            .env("TODO_TEST_NOW", NOW)
            .args(&all)
            .output()
            .expect("run todo")
    }

    /// Runs `todo --json --agent <agent> <args>` and returns the exit code and the parsed output.
    fn run_json(&self, agent: &str, args: &[&str]) -> (Option<i32>, Value) {
        let mut all = vec!["--json"];
        all.extend_from_slice(args);
        let out = self.run(agent, &all);
        let json = parse_json(&out.stdout);
        (out.status.code(), json)
    }
}

/// The error code of a failed `--json` command.
fn error_code(json: &Value) -> &str {
    json["error"]["code"].as_str().unwrap_or("")
}

const HEADER: &str = "\
<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 1 -->
# Renderer fixture
";

/// One open task in `## Work`, nothing else.
fn work_only() -> String {
    format!(
        "{HEADER}
## Work

### T-001 · Draw the test scene
- status: open
- owner: —
- priority: P1
- type: feature
- note (2026-11-15, planner): added
"
    )
}

/// The work of `work_only` plus an idea in `## Ideas`, written as `type: idea` with no priority.
fn with_idea(idea_block: &str) -> String {
    format!("{}\n## Ideas\n\n{idea_block}", work_only())
}

const PLAIN_IDEA: &str = "\
### T-002 · Cache parsed files
- status: open
- owner: —
- type: idea
- note (2026-11-15, planner): added
";

#[test]
fn idea_needs_no_priority_and_creates_the_ideas_section_at_the_end() {
    // `required_fields` names priority, so an entry without one would be refused for a task.
    let p = Project::new(&work_only()).with_config("required_fields = [\"priority\"]\n");
    let out = p.run(
        "alice",
        &["idea", "Hot-reload shaders while the scene runs"],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));
    assert_eq!(stdout_text(&out), "T-002: idea ok\n");

    let text = p.text();
    // Every write adds the tool stamp (R-VER-1), idea included.
    assert!(text.contains("<!-- todo-tool: "), "{text}");
    assert!(
        text.ends_with(
            "- note (2026-11-15, planner): added\n\n## Ideas\n\n\
             ### T-002 · Hot-reload shaders while the scene runs\n\
             - status: open\n- owner: —\n- type: idea\n- note (2026-11-18, alice): added\n"
        ),
        "{text}"
    );

    // A second idea goes to the end of the existing section, and the JSON answer names it.
    let (code, json) = p.run_json("alice", &["idea", "Procedural sky"]);
    assert_eq!(code, Some(0));
    assert_eq!(json["data"]["id"], "T-003");
    assert_eq!(json["data"]["type"], "idea");
    assert_eq!(json["data"]["section"], "Ideas");
    assert_eq!(json["data"]["status"], "open");
    assert!(json["data"]["priority"].is_null());
    assert_eq!(p.text().matches("## Ideas").count(), 1);
}

#[test]
fn idea_goes_to_idea_section_from_todo_toml() {
    let p = Project::new(&format!("{}\n## Parking Lot\n\n", work_only()))
        .with_config("idea_section = \"Parking Lot\"\n");
    let out = p.run("alice", &["idea", "Ray tracing for reflections"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));

    let text = p.text();
    assert!(!text.contains("## Ideas"), "{text}");
    let heading = text.find("## Parking Lot").expect("section kept");
    let entry = text
        .find("### T-002 · Ray tracing for reflections")
        .expect("idea added");
    assert!(entry > heading, "{text}");
    assert!(
        text.contains("- type: idea\n- note (2026-11-18, alice): added\n"),
        "{text}"
    );
}

#[test]
fn idea_about_names_an_existing_entry_and_refuses_an_unknown_one() {
    let p = Project::new(&work_only());

    // A lenient ID is written in canonical form.
    let out = p.run("alice", &["idea", "Cache parsed files", "--about", "t1"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));
    assert!(
        p.text()
            .contains("- type: idea\n- about: T-001\n- note (2026-11-18, alice): added\n"),
        "{}",
        p.text()
    );

    // An ID that doesn't exist is refused and leaves the file byte-identical.
    let before = p.text();
    let (code, json) = p.run_json("alice", &["idea", "Missing target", "--about", "T-099"]);
    assert_eq!(code, Some(1));
    assert_eq!(error_code(&json), "NOT_FOUND");
    assert_eq!(p.text(), before);
}

#[test]
fn idea_refuses_the_title_of_an_active_task_or_idea() {
    let p = Project::new(&work_only());
    let out = p.run("alice", &["idea", "Cache parsed files"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));

    let before = p.text();
    let (code, json) = p.run_json("alice", &["idea", "cache parsed files"]);
    assert_eq!(code, Some(1));
    assert_eq!(error_code(&json), "DUPLICATE_TITLE");

    let (code, json) = p.run_json("alice", &["idea", "Draw the test scene"]);
    assert_eq!(code, Some(1));
    assert_eq!(error_code(&json), "DUPLICATE_TITLE");
    assert_eq!(p.text(), before);
}

#[test]
fn next_never_returns_an_idea_even_one_the_agent_holds() {
    // The idea has the highest priority and the earliest position, and is claimed by alice.
    let claimed_idea = "\
### T-002 · Cache parsed files
- status: claimed
- owner: alice
- claimed: 2026-11-18T09:00+01:00
- priority: P0 · type: idea
- note (2026-11-15, planner): added
";
    let p = Project::new(&with_idea(claimed_idea));

    let (code, json) = p.run_json("alice", &["next"]);
    assert_eq!(code, Some(0));
    assert_eq!(json["data"]["task"]["id"], "T-001", "{json}");
    assert_ne!(json["data"]["task"]["id"], "T-002");

    let (_, json) = p.run_json("", &["next"]);
    assert_eq!(json["data"]["task"]["id"], "T-001", "{json}");

    // With only an idea left, nothing is handed out.
    let only_idea = format!("{HEADER}\n## Ideas\n\n{PLAIN_IDEA}");
    let q = Project::new(&only_idea);
    let (code, json) = q.run_json("alice", &["next"]);
    assert_eq!(code, Some(0));
    assert!(json["data"]["task"].is_null(), "{json}");
}

#[test]
fn summary_counts_ideas_apart_from_open_work() {
    let content = format!(
        "{HEADER}
## Work

### T-001 · Draw the test scene
- status: open
- owner: —
- priority: P1
- type: feature

### T-002 · Light the scene
- status: open
- owner: —
- priority: P1

### T-003 · Old task
- status: done
- owner: alice
- priority: P2

## Ideas

### T-004 · Hot-reload shaders
- status: open
- owner: —
- priority: P0
- type: idea

### T-005 · Procedural sky
- status: open
- owner: —
- type: idea

### T-006 · Dropped thought
- status: dropped
- owner: —
- type: idea
"
    );
    let p = Project::new(&content);

    let (code, json) = p.run_json("alice", &["summary"]);
    assert_eq!(code, Some(0));
    let counts = &json["data"]["counts"];
    assert_eq!(counts["total"], 3, "{json}");
    assert_eq!(counts["open"], 2, "{json}");
    assert_eq!(counts["done"], 1, "{json}");
    assert_eq!(counts["dropped"], 0, "{json}");
    assert_eq!(counts["ideas"], 2, "{json}");
    // The P0 idea is not counted as open work, and not listed as ready.
    assert_eq!(
        json["data"]["open_by_priority"],
        serde_json::json!([{"priority": "P1", "count": 2}]),
        "{json}"
    );
    let ready: Vec<&str> = json["data"]["next_ready"]
        .as_array()
        .expect("next_ready")
        .iter()
        .filter_map(|r| r["id"].as_str())
        .collect();
    assert_eq!(ready, ["T-001", "T-002"]);

    let out = p.run("alice", &["summary"]);
    let text = stdout_text(&out);
    assert_eq!(out.status.code(), Some(0));
    assert!(
        text.starts_with("TODO.md: 3 total (open 2, claimed 0, blocked 0, done 1, dropped 0)\n"),
        "{text}"
    );
    assert!(text.contains("\nideas: 2 (not work;"), "{text}");
    assert!(text.lines().count() <= 40, "{text}");
}

#[test]
fn summary_has_no_ideas_line_without_ideas() {
    let p = Project::new(&work_only());
    let out = p.run("alice", &["summary"]);
    assert!(
        !stdout_text(&out).contains("ideas:"),
        "{}",
        stdout_text(&out)
    );
    let (_, json) = p.run_json("alice", &["summary"]);
    assert_eq!(json["data"]["counts"]["ideas"], 0);
}

#[test]
fn promote_turns_an_idea_into_a_task_with_a_note() {
    let idea = "\
### T-002 · Cache parsed files
- status: open
- owner: —
- type: idea
- about: T-001
- note (2026-11-15, planner): added
";
    let p = Project::new(&with_idea(idea));
    let out = p.run("alice", &["promote", "T-002", "--priority", "P0"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));
    assert_eq!(stdout_text(&out), "T-002: promote ok\n");

    // `type: idea` is gone; the priority is set; `about` and the old notes stay; the note is new.
    assert!(
        p.text().ends_with(
            "### T-002 · Cache parsed files\n\
             - status: open\n\
             - owner: —\n\
             - priority: P0\n\
             - about: T-001\n\
             - note (2026-11-15, planner): added\n\
             - note (2026-11-18, alice): promoted from idea\n"
        ),
        "{}",
        p.text()
    );

    // It is now ordinary work: `next` hands it out first, as the only P0 task.
    let (_, json) = p.run_json("alice", &["next"]);
    assert_eq!(json["data"]["task"]["id"], "T-002", "{json}");
}

#[test]
fn promote_json_shows_a_plain_task_with_the_new_priority() {
    let p = Project::new(&with_idea(PLAIN_IDEA));
    let (code, json) = p.run_json("alice", &["promote", "T-002", "--priority", "P2"]);
    assert_eq!(code, Some(0), "{json}");
    assert_eq!(json["data"]["id"], "T-002");
    assert!(json["data"]["type"].is_null(), "{json}");
    assert_eq!(json["data"]["priority"], "P2");
    assert_eq!(json["data"]["status"], "open");
    let last_note = json["data"]["notes"]
        .as_array()
        .and_then(|n| n.last())
        .and_then(|n| n["text"].as_str());
    assert_eq!(last_note, Some("promoted from idea"), "{json}");
}

#[test]
fn promote_keeps_other_parts_of_a_combined_priority_line() {
    let idea = "\
### T-002 · Cache parsed files
- status: open
- owner: —
- priority: P3 · type: idea · area: cli
- note (2026-11-15, planner): added
";
    let p = Project::new(&with_idea(idea));
    let out = p.run("alice", &["promote", "T-002", "--priority", "p1"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));
    let text = p.text();
    assert!(text.contains("- priority: P1 · area: cli\n"), "{text}");
    assert!(!text.contains("type: idea"), "{text}");
}

#[test]
fn promote_refuses_an_entry_that_is_not_an_idea() {
    let p = Project::new(&work_only());
    let before = p.text();
    let (code, json) = p.run_json("alice", &["promote", "T-001", "--priority", "P1"]);
    assert_eq!(code, Some(1));
    assert_eq!(error_code(&json), "BAD_TRANSITION");
    assert_eq!(p.text(), before);
}

#[test]
fn promote_milestone_must_name_a_milestone_that_exists() {
    let content = format!(
        "{}
### T-010 · Develop the renderer
- status: open
- owner: —
- priority: P1 · type: milestone
- goal: the test scene renders
- note (2026-11-15, planner): added

## Ideas

{PLAIN_IDEA}",
        work_only()
    );
    let p = Project::new(&content);
    let before = p.text();

    let (code, json) = p.run_json(
        "alice",
        &[
            "promote",
            "T-002",
            "--priority",
            "P1",
            "--milestone",
            "T-001",
        ],
    );
    assert_eq!(code, Some(1));
    assert_eq!(error_code(&json), "BAD_VALUE");

    let (code, json) = p.run_json(
        "alice",
        &[
            "promote",
            "T-002",
            "--priority",
            "P1",
            "--milestone",
            "T-099",
        ],
    );
    assert_eq!(code, Some(1));
    assert_eq!(error_code(&json), "NOT_FOUND");
    assert_eq!(p.text(), before);

    let out = p.run(
        "alice",
        &[
            "promote",
            "T-002",
            "--priority",
            "P1",
            "--milestone",
            "T-010",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));
    assert!(
        p.text().contains(
            "- priority: P1\n- milestone: T-010\n- note (2026-11-15, planner): added\n\
             - note (2026-11-18, alice): promoted from idea\n"
        ),
        "{}",
        p.text()
    );
}

#[test]
fn promote_section_moves_the_task_to_the_end_of_that_section() {
    let content = format!(
        "{HEADER}
## Ideas

{PLAIN_IDEA}
## Backlog

### T-003 · Write the loader
- status: open
- owner: —
- priority: P2
- note (2026-11-15, planner): added
"
    );
    let p = Project::new(&content);
    let out = p.run(
        "alice",
        &[
            "promote",
            "T-002",
            "--priority",
            "P1",
            "--section",
            "backlog",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));

    let text = p.text();
    assert!(
        text.ends_with(
            "## Ideas\n\n## Backlog\n\n\
             ### T-003 · Write the loader\n\
             - status: open\n- owner: —\n- priority: P2\n\
             - note (2026-11-15, planner): added\n\n\
             ### T-002 · Cache parsed files\n\
             - status: open\n- owner: —\n- priority: P1\n\
             - note (2026-11-15, planner): added\n\
             - note (2026-11-18, alice): promoted from idea\n"
        ),
        "{text}"
    );
}

#[test]
fn promote_without_section_leaves_the_entry_where_it_is() {
    let p = Project::new(&with_idea(PLAIN_IDEA));
    let out = p.run("alice", &["promote", "T-002", "--priority", "P1"]);
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));
    let text = p.text();
    assert!(
        text.ends_with(
            "## Ideas\n\n### T-002 · Cache parsed files\n\
             - status: open\n- owner: —\n- priority: P1\n\
             - note (2026-11-15, planner): added\n\
             - note (2026-11-18, alice): promoted from idea\n"
        ),
        "{text}"
    );
}

#[test]
fn promote_checks_its_arguments_before_writing() {
    let p = Project::new(&with_idea(PLAIN_IDEA));
    let before = p.text();

    // `--priority` is required (usage error, exit 2).
    let out = p.run("alice", &["promote", "T-002"]);
    assert_eq!(out.status.code(), Some(2), "stderr: {}", stderr_text(&out));

    let (code, json) = p.run_json("alice", &["promote", "T-002", "--priority", "P9"]);
    assert_eq!(code, Some(1));
    assert_eq!(error_code(&json), "BAD_VALUE");

    let (code, json) = p.run_json(
        "alice",
        &["promote", "T-002", "--priority", "P1", "--section", "Nope"],
    );
    assert_eq!(code, Some(1));
    assert_eq!(error_code(&json), "UNKNOWN_SECTION");
    assert_eq!(p.text(), before);
}

#[test]
fn promote_of_an_idea_claimed_by_someone_else_needs_force() {
    let claimed = "\
### T-002 · Cache parsed files
- status: claimed
- owner: bob
- claimed: 2026-11-18T09:00+01:00
- type: idea
";
    let p = Project::new(&with_idea(claimed));
    let before = p.text();
    let (code, json) = p.run_json("alice", &["promote", "T-002", "--priority", "P1"]);
    assert_eq!(code, Some(1));
    assert_eq!(error_code(&json), "NOT_OWNER");
    assert_eq!(p.text(), before);

    let out = p.run(
        "alice",
        &["--force", "promote", "T-002", "--priority", "P1"],
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr_text(&out));
    let text = p.text();
    assert!(text.contains("- owner: bob\n"), "{text}");
    assert!(
        text.contains("- note (2026-11-18, alice): forced promote (owner was bob)\n"),
        "{text}"
    );
}
