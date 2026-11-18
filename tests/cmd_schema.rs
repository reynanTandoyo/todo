//! Integration tests for `todo schema` (T-073; ideas I-4).

mod common;

use clap::CommandFactory;
use common::{parse_json, run_in, run_todo, setup_fixture_proj, stderr_text, stdout_text};
use serde_json::Value;
use tempfile::TempDir;
use todo::cli::Cli;
use todo::error::{ErrorCode, ExitCode};

/// Runs `todo schema <args>` in a fresh folder and returns the parsed stdout.
fn schema_json(args: &[&str]) -> Value {
    let dir = TempDir::new().unwrap();
    let mut full = vec!["schema"];
    full.extend_from_slice(args);
    full.push("--json");
    let out = run_in(dir.path(), &full);
    assert!(out.status.success(), "{}", stderr_text(&out));
    parse_json(&out.stdout)
}

/// The `data` of the success envelope for `todo schema --json`.
fn full_schema() -> Value {
    schema_json(&[])["data"].clone()
}

/// Name words of every visible command, depth first, as clap defines them.
fn visible_command_names() -> Vec<String> {
    fn walk(cmd: &clap::Command, path: &[String], names: &mut Vec<String>) {
        for sub in cmd.get_subcommands().filter(|sub| !sub.is_hide_set()) {
            let mut sub_path = path.to_vec();
            sub_path.push(sub.get_name().to_string());
            names.push(sub_path.join(" "));
            walk(sub, &sub_path, names);
        }
    }
    let mut names = Vec::new();
    walk(&Cli::command(), &[], &mut names);
    names
}

#[test]
fn every_visible_command_and_subcommand_is_described() {
    let data = full_schema();
    let described: Vec<&str> = data["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();

    let expected = visible_command_names();
    assert!(expected.iter().any(|n| n == "hooks install"));
    for name in &expected {
        assert!(described.contains(&name.as_str()), "{name} is missing");
    }
    assert_eq!(described.len(), expected.len(), "unexpected extra commands");
    // Hidden commands are internal and not part of the description.
    assert!(!described.contains(&"session-end"));
}

#[test]
fn every_command_names_its_versioned_schema() {
    let data = full_schema();
    for command in data["commands"].as_array().unwrap() {
        let name = command["name"].as_str().unwrap();
        let top = name.split(' ').next().unwrap();
        // `overview` names its output `projects`: `todo.overview.v1` is `todo --json` (`json_name_of`).
        assert_eq!(
            command["schema"],
            format!("todo.{}.v1", todo::cli::json_name_of(top)),
            "schema name of {name}"
        );
    }
    assert_eq!(data["commands"][0]["schema"], "todo.add.v1");
}

#[test]
fn every_error_code_is_listed_with_its_meaning() {
    let data = full_schema();
    let listed = data["error_codes"].as_array().unwrap();
    assert_eq!(listed.len(), ErrorCode::ALL.len());
    for code in ErrorCode::ALL {
        let entry = listed
            .iter()
            .find(|e| e["code"] == code.as_str())
            .unwrap_or_else(|| panic!("error code {} is missing", code.as_str()));
        assert_eq!(entry["exit_code"], code.default_exit_code().as_i32());
        assert!(
            !entry["meaning"].as_str().unwrap().is_empty(),
            "{} has no meaning",
            code.as_str()
        );
    }
}

#[test]
fn every_exit_code_is_listed_with_its_meaning() {
    let data = full_schema();
    let codes: Vec<i64> = data["exit_codes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["code"].as_i64().unwrap())
        .collect();
    let expected: Vec<i64> = ExitCode::ALL
        .iter()
        .map(|c| i64::from(c.as_i32()))
        .collect();
    assert_eq!(codes, expected);
    assert_eq!(codes, [0, 1, 2, 3, 4, 5]);
    for entry in data["exit_codes"].as_array().unwrap() {
        assert!(!entry["meaning"].as_str().unwrap().is_empty());
    }
}

#[test]
fn envelope_task_fields_and_global_options_are_described() {
    let data = full_schema();
    assert_eq!(data["version"], env!("CARGO_PKG_VERSION"));
    assert!(data["envelope"]["success"]["schema"].is_string());
    assert!(data["envelope"]["error"]["error"]["code"].is_string());

    let fields: Vec<&str> = data["task"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap())
        .collect();
    for field in ["id", "title", "status", "owner", "notes", "line_start"] {
        assert!(fields.contains(&field), "task field {field} missing");
    }

    let globals: Vec<&str> = data["global_options"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["name"].as_str().unwrap())
        .collect();
    assert!(globals.contains(&"--agent"));
    assert!(globals.contains(&"--json"));
    assert!(!globals.contains(&"--help"));
}

#[test]
fn task_fields_match_show_json() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _) = setup_fixture_proj(&tmp, "clean_basic.md");
    let out = run_todo(&home, &work, &["show", "T-001", "--json"], &[]).unwrap();
    assert!(out.status.success(), "{}", stderr_text(&out));
    let shown = parse_json(&out.stdout);

    let mut show_keys: Vec<String> = shown["data"].as_object().unwrap().keys().cloned().collect();
    show_keys.sort();

    let mut schema_keys: Vec<String> = full_schema()["task"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap().to_string())
        .collect();
    schema_keys.sort();
    assert_eq!(schema_keys, show_keys);
}

#[test]
fn show_describes_one_command_with_its_arguments() {
    let data = schema_json(&["show"])["data"].clone();
    assert_eq!(data["name"], "show");
    assert_eq!(data["schema"], "todo.show.v1");
    assert!(data.get("commands").is_none(), "only one command expected");
    let arg = &data["arguments"][0];
    assert_eq!(arg["name"], "task");
    assert_eq!(arg["value_name"], "TASK");
    assert_eq!(arg["required"], true);
    assert_eq!(arg["repeatable"], false);
    assert!(!arg["help"].as_str().unwrap().is_empty());
}

#[test]
fn subcommand_words_describe_that_subcommand() {
    let data = schema_json(&["hooks", "install"])["data"].clone();
    assert_eq!(data["name"], "hooks install");
    assert_eq!(data["schema"], "todo.hooks.v1");
}

#[test]
fn repeatable_options_are_marked() {
    let data = schema_json(&["add"])["data"].clone();
    let field = data["options"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["name"] == "--field")
        .expect("add has --field");
    assert_eq!(field["repeatable"], true);
    assert_eq!(field["value_name"], "KEY=VALUE");
}

#[test]
fn text_output_is_pretty_json_equal_to_the_json_data() {
    let dir = TempDir::new().unwrap();
    let out = run_in(dir.path(), &["schema", "show"]);
    assert!(out.status.success(), "{}", stderr_text(&out));
    let text = stdout_text(&out);
    assert!(text.contains("\n  "), "expected pretty-printed output");
    let from_text = parse_json(text.as_bytes());
    assert_eq!(from_text, schema_json(&["show"])["data"]);
}

#[test]
fn json_output_is_one_envelope_with_schema_name() {
    let dir = TempDir::new().unwrap();
    let out = run_in(dir.path(), &["schema", "--json"]);
    assert!(out.status.success(), "{}", stderr_text(&out));
    assert_eq!(stdout_text(&out).lines().count(), 1);
    let envelope = parse_json(&out.stdout);
    assert_eq!(envelope["schema"], "todo.schema.v1");
    assert_eq!(envelope["ok"], true);
    assert_eq!(envelope["warnings"], serde_json::json!([]));
}

#[test]
fn unknown_command_is_a_usage_error() {
    let dir = TempDir::new().unwrap();
    let out = run_in(dir.path(), &["schema", "no-such-command"]);
    assert_eq!(out.status.code(), Some(2), "{}", stderr_text(&out));
    assert!(out.stdout.is_empty());

    let out = run_in(dir.path(), &["schema", "show", "extra", "--json"]);
    assert_eq!(out.status.code(), Some(2));
    let envelope = parse_json(&out.stdout);
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["error"]["code"], "USAGE");
}
