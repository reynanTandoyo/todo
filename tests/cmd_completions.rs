//! Integration tests for `todo completions <shell>` (T-079; R-OUT-10).

mod common;

use common::{assert_todo, parse_json, stderr_text};
use tempfile::TempDir;

fn completions(shell: &str) -> std::process::Output {
    let home = TempDir::new().unwrap();
    assert_todo(home.path())
        .current_dir(home.path())
        .args(["completions", shell])
        .output()
        .unwrap()
}

#[test]
fn every_shell_gets_a_script_that_knows_the_commands() {
    let markers = [
        (
            "powershell",
            "Register-ArgumentCompleter -Native -CommandName 'todo'",
        ),
        ("bash", "complete -F _todo"),
        ("zsh", "#compdef todo"),
        ("fish", "complete -c todo"),
        ("elvish", "edit:completion:arg-completer[todo]"),
    ];
    for (shell, marker) in markers {
        let out = completions(shell);
        assert!(out.status.success(), "{shell}: {}", stderr_text(&out));
        let script = String::from_utf8(out.stdout).unwrap();
        assert!(
            script.contains(marker),
            "{shell}: marker {marker:?} missing"
        );
        for word in [
            "claim",
            "release",
            "import",
            "completions",
            "session",
            "agent",
        ] {
            assert!(script.contains(word), "{shell}: {word} missing");
        }
        // Hidden commands are not offered.
        assert!(
            !script.contains("session-end"),
            "{shell} lists a hidden command"
        );
    }
}

#[test]
fn unknown_shell_is_a_usage_error() {
    let out = completions("cmd");
    assert_eq!(out.status.code(), Some(2), "{}", stderr_text(&out));
    let out = completions("");
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn json_output_wraps_the_script() {
    let home = TempDir::new().unwrap();
    let out = assert_todo(home.path())
        .current_dir(home.path())
        .args(["--json", "completions", "powershell"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let json = parse_json(&out.stdout);
    assert_eq!(json["schema"], "todo.completions.v1");
    assert_eq!(json["data"]["shell"], "powershell");
    assert!(
        json["data"]["script"]
            .as_str()
            .unwrap()
            .contains("Register-ArgumentCompleter")
    );
}

/// The PowerShell script loaded the documented way completes command names (Windows only; skipped
/// when `powershell.exe` is missing).
#[cfg(windows)]
#[test]
fn powershell_profile_line_completes_commands_and_options() {
    let exe = std::path::Path::new(env!("CARGO_BIN_EXE_todo"));
    // The test binary has a hashed name; copy it to `todo.exe` in a temp folder so that the
    // completer, which is registered for `todo`, matches.
    let tmp = TempDir::new().unwrap();
    std::fs::copy(exe, tmp.path().join("todo.exe")).unwrap();
    let script = "todo completions powershell | Out-String | Invoke-Expression; \
        (TabExpansion2 'todo cla' 8).CompletionMatches.CompletionText; \
        (TabExpansion2 'todo release --ses' 18).CompletionMatches.CompletionText";
    let path = format!(
        "{};{}",
        tmp.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let Ok(out) = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env("PATH", path)
        .output()
    else {
        return;
    };
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.lines().any(|l| l.trim() == "claim"),
        "{text}\n{}",
        stderr_text(&out)
    );
    assert!(text.lines().any(|l| l.trim() == "--session"), "{text}");
}
