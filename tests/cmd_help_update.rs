//! `todo help update` (T-109; R-OUT-11): the running version and executable, the repository, the
//! Windows and Linux steps, the release file names (REL-6) and the agent rules that name the
//! command. Every test runs in temporary folders; nothing here reads the repository's own TODO.md.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use common::{parse_json, stderr_text, stdout_text};
use serde_json::Value;
use tempfile::TempDir;
use todo::cli::help::update::{LINUX_ASSET, WINDOWS_ASSET};

const REPOSITORY: &str = "https://github.com/reynanTandoyo/todo";
const LATEST_RELEASE: &str = "https://github.com/reynanTandoyo/todo/releases/latest";

/// Runs `program` with `args` in `work`, with a clean environment and `home` as the user folder.
fn run_program(program: &Path, work: &Path, home: &Path, args: &[&str]) -> Output {
    std::process::Command::new(program)
        .current_dir(work)
        .env_remove("TODO_FILE")
        .env_remove("TODO_AGENT")
        .env_remove("TODO_SESSION")
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .env("HOME", home)
        .env("APPDATA", home)
        .env("NO_COLOR", "1")
        .args(args)
        .output()
        .expect("run todo")
}

/// Runs the `todo` built for these tests.
fn run_todo(args: &[&str]) -> Output {
    let home = TempDir::new().expect("home");
    let work = TempDir::new().expect("work");
    run_program(
        Path::new(env!("CARGO_BIN_EXE_todo")),
        work.path(),
        home.path(),
        args,
    )
}

/// The path after `Running executable: ` in the text output.
fn running_exe_in(text: &str) -> PathBuf {
    let line = text
        .lines()
        .find_map(|line| line.strip_prefix("Running executable: "))
        .expect("a `Running executable:` line");
    PathBuf::from(line)
}

fn same_file(a: &Path, b: &Path) -> bool {
    let a = fs::canonicalize(a).expect("canonicalize first path");
    let b = fs::canonicalize(b).expect("canonicalize second path");
    a == b
}

/// The text of the Windows and Linux step sections, each as one string.
fn platform_text(json: &Value, platform: &str) -> String {
    let steps = json["data"][platform]["steps"]
        .as_array()
        .expect("steps array");
    steps
        .iter()
        .flat_map(|step| {
            let text = step["text"].as_str().unwrap_or_default().to_string();
            let commands = step["commands"]
                .as_array()
                .map(|cmds| {
                    cmds.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            std::iter::once(text).chain(commands)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn text_shows_the_running_version_executable_and_repository() {
    let out = run_todo(&["help", "update"]);
    assert!(out.status.success(), "{}", stderr_text(&out));
    let text = stdout_text(&out);
    assert!(
        text.contains(&format!("Running version: {}", env!("CARGO_PKG_VERSION"))),
        "{text}"
    );
    assert!(
        text.contains(&format!("Repository: {REPOSITORY}")),
        "{text}"
    );
    assert!(
        text.contains(&format!("Latest release: {LATEST_RELEASE}")),
        "{text}"
    );
    let exe = running_exe_in(&text);
    assert!(
        same_file(&exe, Path::new(env!("CARGO_BIN_EXE_todo"))),
        "the printed path is the running executable: {}",
        exe.display()
    );
}

#[test]
fn the_repository_comes_from_cargo_toml() {
    let cargo_toml = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .expect("read Cargo.toml");
    assert!(
        cargo_toml
            .lines()
            .any(|line| line.trim() == format!("repository = \"{REPOSITORY}\"")),
        "Cargo.toml must name the repository"
    );
    assert_eq!(env!("CARGO_PKG_REPOSITORY"), REPOSITORY);
}

#[test]
fn json_carries_the_same_facts_as_text() {
    let out = run_todo(&["--json", "help", "update"]);
    assert!(out.status.success(), "{}", stderr_text(&out));
    let json = parse_json(&out.stdout);
    assert_eq!(json["schema"], "todo.help.v1", "{json}");
    assert_eq!(json["ok"], true, "{json}");
    assert_eq!(json["warnings"], serde_json::json!([]), "{json}");
    assert_eq!(json["data"]["topic"], "update", "{json}");
    assert_eq!(json["data"]["version"], env!("CARGO_PKG_VERSION"), "{json}");
    assert_eq!(json["data"]["repository"], REPOSITORY, "{json}");
    assert_eq!(json["data"]["latest_release"], LATEST_RELEASE, "{json}");
    let exe = json["data"]["executable"]
        .as_str()
        .expect("executable is the full path");
    assert!(
        same_file(Path::new(exe), Path::new(env!("CARGO_BIN_EXE_todo"))),
        "{json}"
    );
    assert_eq!(json["data"]["windows"]["shell"], "PowerShell", "{json}");
    assert_eq!(json["data"]["linux"]["shell"], "POSIX sh", "{json}");
}

#[test]
fn asset_names_match_the_release_workflow() {
    let workflow_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(".github")
        .join("workflows")
        .join("release.yml");
    let workflow = fs::read_to_string(&workflow_path).expect("read release.yml");

    let mut published: Vec<String> = workflow
        .lines()
        .filter_map(|line| line.trim().strip_prefix("asset_name:"))
        .map(|name| name.trim().to_string())
        .collect();
    published.sort();
    let mut expected = vec![WINDOWS_ASSET.to_string(), LINUX_ASSET.to_string()];
    expected.sort();
    assert_eq!(published, expected, "asset names in release.yml");
    assert!(
        workflow.contains("${{ matrix.asset_name }}.sha256"),
        "the workflow writes each checksum as <asset>.sha256"
    );

    let json = parse_json(&run_todo(&["--json", "help", "update"]).stdout);
    let assets = json["data"]["assets"].as_array().expect("assets array");
    let mut printed: Vec<String> = assets
        .iter()
        .map(|asset| asset["file"].as_str().unwrap_or_default().to_string())
        .collect();
    printed.sort();
    assert_eq!(printed, expected, "asset names printed by help update");
    for asset in assets {
        let file = asset["file"].as_str().unwrap_or_default();
        assert_eq!(
            asset["checksum"].as_str(),
            Some(format!("{file}.sha256").as_str()),
            "{asset}"
        );
    }

    let text = stdout_text(&run_todo(&["help", "update"]));
    for name in [WINDOWS_ASSET, LINUX_ASSET] {
        assert!(
            text.contains(&format!("{name} and {name}.sha256")),
            "release files in text: {text}"
        );
    }
}

#[test]
fn windows_steps_download_verify_replace_and_migrate() {
    let json = parse_json(&run_todo(&["--json", "help", "update"]).stdout);
    let steps = platform_text(&json, "windows");
    for needle in [
        "gh release view --repo reynanTandoyo/todo --json tagName --jq .tagName",
        "gh release download --repo reynanTandoyo/todo --pattern todo.exe --pattern todo.exe.sha256 --dir $dir",
        "Get-FileHash -LiteralPath (Join-Path $dir 'todo.exe') -Algorithm SHA256",
        "Move-Item -LiteralPath",
        "Copy-Item -LiteralPath (Join-Path $dir 'todo.exe') -Destination",
        "$dest = Join-Path $env:LOCALAPPDATA 'Programs\\todo'",
        "SetEnvironmentVariable('Path', $newPath, 'User')",
        "todo --version",
        "todo migrate --yes",
    ] {
        assert!(steps.contains(needle), "missing `{needle}` in:\n{steps}");
    }
    assert!(
        steps.contains("Invoke-WebRequest -UseBasicParsing -OutFile"),
        "{steps}"
    );
}

#[test]
fn linux_steps_use_the_linux_asset() {
    let json = parse_json(&run_todo(&["--json", "help", "update"]).stdout);
    let steps = platform_text(&json, "linux");
    for needle in [
        "gh release download --repo reynanTandoyo/todo --pattern todo-linux-x86_64 --pattern todo-linux-x86_64.sha256 --dir \"$dir\"",
        "sha256sum -c todo-linux-x86_64.sha256",
        "chmod +x \"$dir/todo-linux-x86_64\"",
        "mv -f \"$dir/todo-linux-x86_64\"",
        "todo --version",
        "todo migrate --yes",
    ] {
        assert!(steps.contains(needle), "missing `{needle}` in:\n{steps}");
    }
    assert!(
        !steps.contains("--pattern todo.exe"),
        "the Linux steps download no Windows file: {steps}"
    );
}

#[test]
fn the_path_shown_is_quoted_for_each_shell_when_it_has_spaces_quotes_and_non_ascii() {
    let home = TempDir::new().expect("home");
    let work = TempDir::new().expect("work");
    let tools = TempDir::new().expect("tools");
    let folder = tools.path().join("Bü ro's tools");
    fs::create_dir(&folder).expect("create folder");
    let copy = folder.join(format!("todo{}", std::env::consts::EXE_SUFFIX));
    fs::copy(env!("CARGO_BIN_EXE_todo"), &copy).expect("copy todo");

    let out = run_program(
        &copy,
        work.path(),
        home.path(),
        &["--json", "help", "update"],
    );
    assert!(out.status.success(), "{}", stderr_text(&out));
    let json = parse_json(&out.stdout);
    let shown = json["data"]["executable"]
        .as_str()
        .expect("executable path")
        .to_string();
    assert!(same_file(Path::new(&shown), &copy), "{json}");

    // PowerShell and sh single-quoted forms of the same path.
    let ps_path = format!("'{}'", shown.replace('\'', "''"));
    let sh_path = format!("'{}'", shown.replace('\'', "'\\''"));

    let text = stdout_text(&run_program(
        &copy,
        work.path(),
        home.path(),
        &["help", "update"],
    ));
    assert!(
        text.contains(&format!(
            "Copy-Item -LiteralPath (Join-Path $dir 'todo.exe') -Destination {ps_path}"
        )),
        "{text}"
    );
    assert!(
        text.contains(&format!("mv -f \"$dir/todo-linux-x86_64\" {sh_path}")),
        "{text}"
    );
    assert_eq!(running_exe_in(&text), PathBuf::from(&shown));
}

#[test]
fn help_update_writes_nothing_and_needs_no_task_list() {
    let home = TempDir::new().expect("home");
    let work = TempDir::new().expect("work");
    let cases: [&[&str]; 3] = [
        &["help", "update"],
        &["--json", "help", "update"],
        &["help", "UPDATE"],
    ];
    for args in cases {
        let out = run_program(
            Path::new(env!("CARGO_BIN_EXE_todo")),
            work.path(),
            home.path(),
            args,
        );
        assert_eq!(
            out.status.code(),
            Some(0),
            "{args:?}: {}",
            stderr_text(&out)
        );
    }
    assert_eq!(
        fs::read_dir(work.path()).expect("read work folder").count(),
        0,
        "no file is written in the working folder"
    );
    assert_eq!(
        fs::read_dir(home.path()).expect("read home folder").count(),
        0,
        "no file is written in the user folder"
    );
}

#[test]
fn agent_rules_name_help_update() {
    assert!(
        todo::agents::RULES_TEXT.contains("todo help update"),
        "the rules text must name `todo help update`"
    );
    let out = run_todo(&["help", "agents"]);
    assert!(stdout_text(&out).contains("todo help update"));
}
