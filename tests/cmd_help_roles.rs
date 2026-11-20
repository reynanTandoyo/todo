//! `todo help worker`, `todo help planner` and `todo help agents` (T-166; R-AGENT-1): each role's text
//! is printed as it is built into the program, the worker's text is at most ten lines and the common
//! rules at most fifteen, and `--json` gives the topic with its text. Every test runs in a temporary
//! folder and reads nothing of the repository.

mod common;

use common::{parse_json, run_in, stderr_text, stdout_text};
use tempfile::TempDir;
use todo::agents::{
    COMMON_RULES_MAX_LINES, PLANNER_TEXT, RULES_TEXT, WORKER_TEXT, WORKER_TEXT_MAX_LINES,
};

/// Runs `todo help <topic>` in a fresh folder and returns its standard output.
fn help_text(topic: &str) -> String {
    let dir = TempDir::new().expect("tempdir");
    let out = run_in(dir.path(), &["help", topic]);
    assert!(out.status.success(), "help {topic}: {}", stderr_text(&out));
    stdout_text(&out)
}

#[test]
fn help_worker_prints_the_worker_text_within_its_line_limit() {
    let text = help_text("worker");
    assert_eq!(
        text, WORKER_TEXT,
        "`todo help worker` prints the built-in text"
    );
    assert!(
        text.lines().count() <= WORKER_TEXT_MAX_LINES,
        "the worker text has {} lines",
        text.lines().count()
    );
    // The finish path a worker follows (R-AGENT-1): take work, finish or give it back.
    for needle in [
        "todo next --claim --max-difficulty 1 --brief",
        "--commit HEAD",
        "todo stuck <id> --note",
        "todo brief <id>",
    ] {
        assert!(text.contains(needle), "`todo help worker` lacks `{needle}`");
    }
}

#[test]
fn help_planner_prints_the_planner_rules() {
    let text = help_text("planner");
    assert_eq!(
        text, PLANNER_TEXT,
        "`todo help planner` prints the built-in text"
    );
    for needle in [
        "todo plan",
        "todo split",
        "--final",
        "todo verify --wave",
        "todo waves",
        "todo help update",
        "todo migrate --yes",
    ] {
        assert!(
            text.contains(needle),
            "`todo help planner` lacks `{needle}`"
        );
    }
}

#[test]
fn help_agents_prints_the_common_rules_within_their_line_limit() {
    let text = help_text("agents");
    assert_eq!(
        text, RULES_TEXT,
        "`todo help agents` prints the common rules"
    );
    assert!(
        text.lines().count() <= COMMON_RULES_MAX_LINES,
        "the common rules have {} lines",
        text.lines().count()
    );
    assert!(text.contains("`todo help worker`") && text.contains("`todo help planner`"));
}

#[test]
fn help_role_topics_in_json_name_the_topic_and_carry_the_text() {
    let dir = TempDir::new().expect("tempdir");
    for (topic, text) in [
        ("worker", WORKER_TEXT),
        ("planner", PLANNER_TEXT),
        ("agents", RULES_TEXT),
    ] {
        let out = run_in(dir.path(), &["--json", "help", topic]);
        assert!(
            out.status.success(),
            "help {topic} --json: {}",
            stderr_text(&out)
        );
        let json = parse_json(&out.stdout);
        assert_eq!(json["schema"], "todo.help.v1", "{json}");
        assert_eq!(json["data"]["topic"], topic, "{json}");
        assert_eq!(json["data"]["rules"], text, "{json}");
    }
}
