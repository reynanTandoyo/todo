//! `todo help update`: the steps an agent runs to update the todo executable (R-OUT-11).
//!
//! todo never connects to the network (NFR-4). This module only describes the steps; the agent
//! runs them. The asset names must match `.github/workflows/release.yml` (REL-6), which a test in
//! `tests/cmd_help_update.rs` checks.

use std::path::Path;

use serde::Serialize;

use crate::output::Output;
use crate::version::RUNNING_VERSION;

/// The Windows release asset, as `.github/workflows/release.yml` names it (REL-6).
pub const WINDOWS_ASSET: &str = "todo.exe";

/// The Linux x64 release asset, as `.github/workflows/release.yml` names it (REL-6).
pub const LINUX_ASSET: &str = "todo-linux-x86_64";

/// Shown in the commands when the running executable's path cannot be read.
const UNKNOWN_EXE: &str = "<path of the todo executable>";

/// The sentence on the network, shown first (NFR-4).
const NETWORK_NOTE: &str = "todo never connects to the network. The agent runs these steps itself, one after another, and stops at the first failure.";

/// The repository page, from `repository` in `Cargo.toml` (R-OUT-11).
#[must_use]
pub fn repository() -> &'static str {
    env!("CARGO_PKG_REPOSITORY").trim_end_matches('/')
}

/// The latest-release page of the repository.
#[must_use]
pub fn latest_release_url() -> String {
    format!("{}/releases/latest", repository())
}

/// A release file and the checksum file that goes with it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Asset {
    /// Platform the file is built for, such as `windows-x86_64`.
    pub platform: &'static str,
    /// Release asset name.
    pub file: &'static str,
    /// Checksum file name: the asset name plus `.sha256`.
    pub checksum: String,
}

/// One numbered step: what it does, then the commands that do it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Step {
    /// What the step does, in plain English.
    pub text: String,
    /// Commands to run in order, one per line.
    pub commands: Vec<String>,
}

/// The steps for one shell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Platform {
    /// The shell the commands are written for.
    pub shell: &'static str,
    /// The steps, in order.
    pub steps: Vec<Step>,
}

/// Everything `todo help update` prints as text, or emits as `data` with `--json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdateHelp {
    /// Always `update`.
    pub topic: &'static str,
    /// The running version, `MAJOR.MINOR.PATCH`.
    pub version: String,
    /// The full path of the running executable, or `null` when it cannot be read.
    pub executable: Option<String>,
    /// The repository page.
    pub repository: &'static str,
    /// The latest-release page.
    pub latest_release: String,
    /// The release files for each platform.
    pub assets: Vec<Asset>,
    /// The sentence on the network.
    pub network: &'static str,
    /// The Windows steps, in PowerShell.
    pub windows: Platform,
    /// The Linux steps, in a POSIX shell.
    pub linux: Platform,
}

impl UpdateHelp {
    /// Builds the help for the running executable at `executable`, or `None` when its path cannot
    /// be read.
    #[must_use]
    pub fn new(executable: Option<&Path>) -> Self {
        let shown = executable.map(|path| path.to_string_lossy().into_owned());
        let target = shown.as_deref().unwrap_or(UNKNOWN_EXE);
        let latest = latest_release_url();
        let download = format!("{}/releases/latest/download", repository());
        let version = RUNNING_VERSION.to_string();
        Self {
            topic: "update",
            windows: windows_platform(&powershell_quote(target), &version, &latest, &download),
            linux: linux_platform(&sh_quote(target), &version, &latest, &download),
            version,
            executable: shown,
            repository: repository(),
            latest_release: latest,
            assets: vec![
                Asset {
                    platform: "windows-x86_64",
                    file: WINDOWS_ASSET,
                    checksum: format!("{WINDOWS_ASSET}.sha256"),
                },
                Asset {
                    platform: "linux-x86_64",
                    file: LINUX_ASSET,
                    checksum: format!("{LINUX_ASSET}.sha256"),
                },
            ],
            network: NETWORK_NOTE,
        }
    }
}

/// Writes `todo help update`: plain text on standard output, or the `data` of a `todo.help.v1`
/// envelope with `--json`.
pub fn emit(out: &mut Output, executable: Option<&Path>) {
    let help = UpdateHelp::new(executable);
    if out.is_json() {
        out.emit_json_ok("help", &help);
    } else {
        write_text(out, &help);
    }
}

fn write_text(out: &mut Output, help: &UpdateHelp) {
    out.write_line("todo help update: how to update the todo executable");
    out.write_line("");
    out.write_line(&format!("Running version: {}", help.version));
    out.write_line(&format!(
        "Running executable: {}",
        help.executable
            .as_deref()
            .unwrap_or("unknown (its path could not be read)")
    ));
    out.write_line(&format!("Repository: {}", help.repository));
    out.write_line(&format!("Latest release: {}", help.latest_release));
    out.write_line("");
    out.write_line(help.network);
    out.write_line("Release files:");
    for asset in &help.assets {
        out.write_line(&format!(
            "  {}: {} and {}",
            asset.platform, asset.file, asset.checksum
        ));
    }
    write_platform(out, "Windows (PowerShell)", &help.windows);
    write_platform(out, "Linux (POSIX shell)", &help.linux);
}

fn write_platform(out: &mut Output, title: &str, platform: &Platform) {
    out.write_line("");
    out.write_line(&format!("{title}:"));
    for (index, step) in platform.steps.iter().enumerate() {
        out.write_line(&format!("  {}. {}", index + 1, step.text));
        for command in &step.commands {
            out.write_line(&format!("       {command}"));
        }
    }
}

fn step(text: String, commands: Vec<String>) -> Step {
    Step { text, commands }
}

/// The Windows steps. `exe` is the executable path already quoted for PowerShell.
fn windows_platform(exe: &str, version: &str, latest: &str, download: &str) -> Platform {
    let file = WINDOWS_ASSET;
    let checksum = format!("{WINDOWS_ASSET}.sha256");
    let steps = vec![
        step(
            format!(
                "Read the latest version. Its tag starts with v. If it is not newer than {version}, no update is needed. Without gh, open {latest}."
            ),
            vec!["gh release view --repo reynanTandoyo/todo --json tagName --jq .tagName".into()],
        ),
        step(
            format!(
                "Download {file} and {checksum} from the latest release into a new empty folder. Without gh, fetch {download}/{file} and {download}/{checksum} with Invoke-WebRequest -UseBasicParsing -OutFile."
            ),
            vec![
                "$dir = Join-Path $env:TEMP ('todo-' + [guid]::NewGuid().ToString('N'))".into(),
                "New-Item -ItemType Directory -Path $dir | Out-Null".into(),
                format!(
                    "gh release download --repo reynanTandoyo/todo --pattern {file} --pattern {checksum} --dir $dir"
                ),
            ],
        ),
        step(
            format!(
                "Compare the SHA-256 of {file} with its checksum file. Continue only when the last command prints True."
            ),
            vec![
                format!(
                    "$expected = (Get-Content -LiteralPath (Join-Path $dir '{checksum}') | Select-Object -First 1) -split '\\s+' | Select-Object -First 1"
                ),
                format!("(Get-FileHash -LiteralPath (Join-Path $dir '{file}') -Algorithm SHA256).Hash -eq $expected"),
            ],
        ),
        step(
            "Replace the executable at the path shown above. Windows does not overwrite a running .exe, so move the old file aside first; if the copy fails, move it back. Delete the .old file later, once no terminal runs it.".into(),
            vec![
                format!("Move-Item -LiteralPath {exe} -Destination ({exe} + '.old')"),
                format!("Copy-Item -LiteralPath (Join-Path $dir '{file}') -Destination {exe}"),
            ],
        ),
        step(
            "Only when todo is not installed yet: skip the replacement above, copy the file to %LOCALAPPDATA%\\Programs\\todo and add that folder to the user PATH.".into(),
            vec![
                "$dest = Join-Path $env:LOCALAPPDATA 'Programs\\todo'".into(),
                "New-Item -ItemType Directory -Force -Path $dest | Out-Null".into(),
                format!("Copy-Item -LiteralPath (Join-Path $dir '{file}') -Destination (Join-Path $dest '{file}')"),
                "$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')".into(),
                "$newPath = (@($userPath, $dest) | Where-Object { $_ }) -join ';'".into(),
                "if (-not (($userPath -split ';') -contains $dest)) { [Environment]::SetEnvironmentVariable('Path', $newPath, 'User') }".into(),
            ],
        ),
        step(
            "Open a new terminal if the PATH changed. Then check the version: it must print the version from the first step.".into(),
            vec!["todo --version".into()],
        ),
        step(
            "In each project folder that has a TODO.md, bring its files up to date with the new todo.".into(),
            vec!["todo migrate --yes".into()],
        ),
    ];
    Platform {
        shell: "PowerShell",
        steps,
    }
}

/// The Linux steps. `exe` is the executable path already quoted for a POSIX shell.
fn linux_platform(exe: &str, version: &str, latest: &str, download: &str) -> Platform {
    let file = LINUX_ASSET;
    let checksum = format!("{LINUX_ASSET}.sha256");
    let steps = vec![
        step(
            format!(
                "Read the latest version. Its tag starts with v. If it is not newer than {version}, no update is needed. Without gh, open {latest}."
            ),
            vec!["gh release view --repo reynanTandoyo/todo --json tagName --jq .tagName".into()],
        ),
        step(
            format!(
                "Download {file} and {checksum} from the latest release into a new empty folder. Without gh, fetch {download}/{file} and {download}/{checksum} with curl -fsSL -o into that folder."
            ),
            vec![
                "dir=$(mktemp -d)".into(),
                format!(
                    "gh release download --repo reynanTandoyo/todo --pattern {file} --pattern {checksum} --dir \"$dir\""
                ),
            ],
        ),
        step(
            "Compare the SHA-256. Continue only when it prints OK.".into(),
            vec![format!("(cd \"$dir\" && sha256sum -c {checksum})")],
        ),
        step(
            "Replace the executable at the path shown above. mv replaces the file by renaming it, so this works while todo runs.".into(),
            vec![
                format!("chmod +x \"$dir/{file}\""),
                format!("mv -f \"$dir/{file}\" {exe}"),
            ],
        ),
        step(
            "Only when todo is not installed yet: skip the replacement above, put the file in ~/.local/bin and add that folder to PATH. Add the export line to your shell profile to keep it.".into(),
            vec![
                format!("chmod +x \"$dir/{file}\""),
                format!("mkdir -p \"$HOME/.local/bin\" && mv -f \"$dir/{file}\" \"$HOME/.local/bin/todo\""),
                "export PATH=\"$HOME/.local/bin:$PATH\"".into(),
            ],
        ),
        step(
            "Check the version: it must print the version from the first step.".into(),
            vec!["todo --version".into()],
        ),
        step(
            "In each project folder that has a TODO.md, bring its files up to date with the new todo.".into(),
            vec!["todo migrate --yes".into()],
        ),
    ];
    Platform {
        shell: "POSIX sh",
        steps,
    }
}

/// Quotes `text` as a PowerShell single-quoted string. PowerShell reads several Unicode quote marks
/// as single quotes, so each of them is doubled, as an apostrophe is.
fn powershell_quote(text: &str) -> String {
    let mut quoted = String::with_capacity(text.len() + 2);
    quoted.push('\'');
    for c in text.chars() {
        if matches!(c, '\'' | '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}') {
            quoted.push(c);
        }
        quoted.push(c);
    }
    quoted.push('\'');
    quoted
}

/// Quotes `text` for a POSIX shell: single quotes, with each `'` written as `'\''`.
fn sh_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn powershell_quote_doubles_apostrophes_and_curly_quotes() {
        assert_eq!(
            powershell_quote("C:\\Bü ro's\\todo.exe"),
            "'C:\\Bü ro''s\\todo.exe'"
        );
        assert_eq!(powershell_quote("a\u{2019}b"), "'a\u{2019}\u{2019}b'");
    }

    #[test]
    fn sh_quote_escapes_single_quotes() {
        assert_eq!(sh_quote("/home/a b/todo"), "'/home/a b/todo'");
        assert_eq!(sh_quote("it's"), "'it'\\''s'");
    }

    #[test]
    fn latest_release_page_comes_from_cargo_repository() {
        assert_eq!(
            latest_release_url(),
            "https://github.com/reynanTandoyo/todo/releases/latest"
        );
    }

    #[test]
    fn steps_quote_the_executable_path_they_are_given() {
        let help = UpdateHelp::new(Some(Path::new("C:\\Tools\\Bü ro's\\todo.exe")));
        assert_eq!(help.executable.as_deref(), Some("C:\\Tools\\Bü ro's\\todo.exe"));
        let replace = &help.windows.steps[3].commands[1];
        assert_eq!(
            replace,
            "Copy-Item -LiteralPath (Join-Path $dir 'todo.exe') -Destination 'C:\\Tools\\Bü ro''s\\todo.exe'"
        );
    }

    #[test]
    fn unknown_executable_is_a_placeholder_and_null() {
        let help = UpdateHelp::new(None);
        assert_eq!(help.executable, None);
        assert!(help.linux.steps[3].commands[1].contains(UNKNOWN_EXE));
    }
}
