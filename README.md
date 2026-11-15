# todo

`todo` is a command-line tool (`todo.exe`) that lets several AI agents and people share one
`TODO.md` task list safely on the same machine.

Instead of letting multiple agents edit `TODO.md` by hand and overwrite each other's changes,
every write command takes an OS file lock, checks ownership and status rules, edits only the
lines that changed (leaving every other byte untouched), and replaces the file atomically.

## Building

Requires [Rust](https://www.rust-lang.org/) 1.89 or newer:

```powershell
cargo build --release
```

This produces `target\release\todo.exe` with the C runtime statically linked, so it runs on
Windows 10/11 without installing extra Visual C++ redistributables.

## Installing (Windows PowerShell)

Copy `todo.exe` to `%LOCALAPPDATA%\Programs\todo` and add that folder to your user `PATH`:

```powershell
$installDir = Join-Path $env:LOCALAPPDATA "Programs\todo"
New-Item -ItemType Directory -Force -Path $installDir | Out-Null
Copy-Item .\target\release\todo.exe (Join-Path $installDir "todo.exe") -Force

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not ($userPath -split ";" | Where-Object { $_ -eq $installDir })) {
    $newPath = if ($userPath) { "$userPath;$installDir" } else { $installDir }
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
}
```

Open a new terminal window so the updated `PATH` takes effect, then verify:

```powershell
todo --version
```

## Using it

Every write command needs an agent name via `--agent <name>` or the `TODO_AGENT` environment
variable:

```powershell
$env:TODO_AGENT = "agent-1"
```

### Overview and help

```powershell
todo
todo help claim
todo help agents
todo --version
```

### Reading tasks

```powershell
todo list
todo list --ready
todo list --mine --agent agent-1
todo list --status all --json
todo show T-012
todo next --agent agent-1
todo next --claim --agent agent-1
```

### Adding and editing tasks

```powershell
todo add --title "Fix login timeout" --priority P1 --type bug --area auth --section Now --agent agent-1
todo set T-012 --priority P0 --area cli --field reqs=R-WRITE-8 --unset fix --agent agent-1
todo note T-012 "reproduced on Windows 11" --agent agent-1
```

### Working through a task

```powershell
todo claim T-012 --agent agent-1
todo release T-012 --agent agent-1
todo release --stale --agent coordinator
todo release --all --agent agent-1
todo block T-012 --note "waiting on API schema" --agent agent-1
todo unblock T-012 --note "schema merged" --agent agent-1
todo done T-012 --note "all tests pass" --commit HEAD --agent agent-1
todo drop T-013 --note "superseded by T-012" --agent agent-1
todo reopen T-013 --note "needed after all" --agent agent-1
todo move T-012 --section Done --agent agent-1
```

### Questions, batch operations, and inspection

```powershell
todo where
todo search "timeout"
todo search "timeout" --archive
todo log T-012
todo check
todo check --fix --agent agent-1
todo ask "Should retries use exponential backoff?" --agent agent-1
todo answer T-014 "Yes, cap at 30s" --agent human
Get-Content tasks.jsonl | todo add --from - --agent coordinator
Get-Content ops.jsonl | todo apply - --agent coordinator
```

Task arguments also match unique case-insensitive title substrings (excluding `done` and `dropped`
tasks). Text options accept `-` (standard input) or `@path` (a UTF-8 file) so quotes, backticks,
and `$` pass cleanly through PowerShell.

### Tab completion

`todo completions <shell>` prints a completion script for `powershell`, `bash`, `zsh`, `fish` or
`elvish`, generated from the command definitions: Tab completes commands and options. In
PowerShell, add this line to your profile (`notepad $PROFILE`; create the file first with
`New-Item -ItemType File -Force $PROFILE` if it does not exist):

```powershell
todo completions powershell | Out-String | Invoke-Expression
```

For bash use `source <(todo completions bash)` in `~/.bashrc`; zsh, fish and elvish scripts go into
the usual completion folders of those shells (`todo completions zsh > _todo` into a folder of
`$fpath`, `todo completions fish > ~/.config/fish/completions/todo.fish`). The PowerShell script
completes command and option names; it does not list option values such as `--status`.

### Importing a checkbox list

`todo import --from checkbox` turns a Markdown checkbox list (`- [ ]`, `- [~]`, `- [!]`, `- [x]`
lines under `##` sections) into a `TODO.md`. Without `--yes` it only prints the converted text, or a
diff when the target exists. With `--yes` it writes the result and first saves an existing target as
`<name>.pre-import`. All other lines stay byte for byte.

```powershell
todo import --from checkbox "C:\My Notes\plan.md"                       # preview
todo import --from checkbox "C:\My Notes\plan.md" --out TODO.md --yes   # write a new TODO.md
todo import --from checkbox TODO.md --yes                                # convert in place
```

| In the list | In `TODO.md` |
|---|---|
| `- [ ]`, `- [~]`, `- [!]`, `- [x]` | status `open`, `claimed`, `blocked`, `done` |
| `**Title** — rest` or `Title — rest` | title `Title`, field `description: rest` |
| `(added: agent, 2026-11-01)` | note `added` |
| `(claimed: agent, 2026-11-01)` | owner `agent`, claim time = time of the import (an old claim would be stale), the old date in a note |
| `(done: agent, 2026-11-05, abc1234)` | note `done` and `commit: abc1234` |
| `  - 2026-11-05 agent: text` | note by `agent` on that date |
| sections named like "Needs user decision" or "Questions" | `type: question` (so `todo next` skips them) |

Checkboxes in the preamble, in code fences or indented stay text. IDs are `T-001`, `T-002`, … in
file order. The command checks the result like `todo check`, compares the task count with the number
of converted lines and reports both. It does not take the write lock; use it on lists that agents
are not using yet.

### Claims tagged with an agent session

A claim can carry the agent session that made it, so claims of a session that has ended are not
left behind. `claim`, `next --claim` and `unblock` (back to claimed) write `- session: <id>` right
after the `claimed` line. The ID comes from `--session`, else the `TODO_SESSION` environment
variable, else `CLAUDE_CODE_SESSION_ID` (Claude Code sets this for every command an agent runs, and
it is the `session_id` it passes to hooks). Without any of them no `session` line is written. The
line is removed whenever the claim is (`release`, `done`, `drop`, …).

```powershell
todo claim T-012 --agent agent-1 --session 6f0c1c2e-1111-4222-8333-944455556666
todo release --session 6f0c1c2e-1111-4222-8333-944455556666 --agent coordinator
```

`release --session <id>` releases every claim of that session (a blocked task stays blocked) and
adds the note `released — session <id> ended`. `todo hooks install --yes` also installs a Claude
Code `SessionEnd` hook (`todo session-end`, a hidden command) that reads the hook's JSON from
standard input and does exactly that for the ending session; `todo hooks uninstall --yes` removes
it again. Test it from PowerShell with
`'{"session_id":"6f0c1c2e-1111-4222-8333-944455556666"}' | todo session-end`.

### Manual edit leases, editor integration, and Claude Code

```powershell
todo status
todo lock --minutes 10 --agent agent-1
todo unlock --agent agent-1
todo edit --agent agent-1
todo hooks install --yes
todo skill install --yes
```

Pass `--json` to any command for machine-readable output, `--dry-run` to preview a write command's
diff without modifying `TODO.md`, `--yes` to confirm bulk commands, `--file <path>` (or
`TODO_FILE`) to target a specific file, and `--force` to override another agent's claim. Colour
output is enabled only on terminals and is disabled when piped or when `NO_COLOR` is set.

## Configuration (`todo.toml`)

An optional `todo.toml` next to `TODO.md` can configure `id_prefix`, `id_width`, `priorities`,
`required_fields`, `types`, `areas`, `default_section`, `question_section`, `archive_file`,
`claim_ttl_hours`, `lock_wait_seconds`, `lease_minutes`, `lease_max_minutes`, `snapshots_keep`,
and `worktree` (`"shared"` or `"local"`). See [`docs/requirements.md`](docs/requirements.md) §5.14
for defaults.

## Adding it to a project

1. Create a `TODO.md` at the root of your repository with at least one `## ` section:

   ```markdown
   <!-- todo-format: 1 -->

   ## Now

   ### T-001 · First task
   - status: open
   - owner: —
   - priority: P1
   ```

2. Add these rules to your project's `AGENTS.md` (also printed by `todo help agents`):

   ```markdown
   # Working with TODO.md

   - Never edit `TODO.md` (or `TODO-archive.md`) directly; always use `todo`.
   - Use one unique agent name per agent via `--agent <name>` or `TODO_AGENT`.
   - Start by running `todo summary` or `todo next`.
   - Claim a task before working on it (`todo claim <id>`, or `todo next --claim` to pick and claim in one step).
   - Exit code `1` means someone else claimed it or the rule refused the change — pick another task.
   - Exit code `3` means the lock or lease is busy — try the command again.
   - Add notes as you learn things (`todo note <id> "..."`).
   - Finish a task with `todo done <id> --note "..." --commit`.
   - Use `todo add "..."` to record problems you find instead of fixing them silently.
   - Use `todo ask "..."` for questions only a person can decide.
   - Start every git commit message with the task ID (for example `T-012: short summary`).
   ```

## Limits

- **Single-machine lock:** The write lock uses OS file locking and a local state folder
  (`.git/todo/…` inside git, or `.todo/` outside git). It coordinates processes on one machine,
  not across different computers.
- **Cloud-synced folders:** Folders synced by OneDrive, Dropbox, Google Drive, or similar sync
  clients are **not** safe for parallel agents, because sync clients can create conflicted copies
  or lock files outside `todo`'s coordination.
- **Windows SmartScreen:** A self-built `todo.exe` is unsigned, so Windows SmartScreen may show a
  warning the first time you run it from another location; building from source locally with
  `cargo build --release` avoids downloading an untrusted binary.
- **Capturing output in Windows PowerShell 5.1:** `todo` writes UTF-8. Printed straight to the
  console it always shows correctly, but when a script captures it (`$tasks = todo list`),
  Windows PowerShell 5.1 decodes it with the console's code page, which can garble `·` and `—`
  on PCs not set to UTF-8. Run `[Console]::OutputEncoding = [Text.Encoding]::UTF8` first.

## Documents

- [`docs/requirements.md`](docs/requirements.md): what the tool does, and the decisions behind it.
- [`docs/format.md`](docs/format.md): the `TODO.md` file format.
- [`docs/research.md`](docs/research.md): what was studied before writing the requirements.
- [`TODO.md`](TODO.md): the work plan.
- [`AGENTS.md`](AGENTS.md): rules for agents working on this repository.

## Licence

MIT — see [`LICENSE`](LICENSE).
