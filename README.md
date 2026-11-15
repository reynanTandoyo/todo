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

### Questions, batch add, and inspection

```powershell
todo where
todo ask "Should retries use exponential backoff?" --agent agent-1
todo answer T-014 "Yes, cap at 30s" --agent human
Get-Content tasks.jsonl | todo add --from - --agent coordinator
```

Task arguments also match unique case-insensitive title substrings (excluding `done` and `dropped`
tasks). Text options accept `-` (standard input) or `@path` (a UTF-8 file) so quotes, backticks,
and `$` pass cleanly through PowerShell.

### Manual edit leases and lock status

```powershell
todo status
todo lock --minutes 10 --agent agent-1
todo unlock --agent agent-1
```

Pass `--json` to any command for machine-readable output, `--dry-run` to preview a write command's
diff without modifying `TODO.md`, `--yes` to confirm bulk commands, `--file <path>` (or
`TODO_FILE`) to target a specific file, and `--force` to override another agent's claim.

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

## Documents

- [`docs/requirements.md`](docs/requirements.md): what the tool does, and the decisions behind it.
- [`docs/format.md`](docs/format.md): the `TODO.md` file format.
- [`docs/research.md`](docs/research.md): what was studied before writing the requirements.
- [`TODO.md`](TODO.md): the work plan.
- [`AGENTS.md`](AGENTS.md): rules for agents working on this repository.

## Licence

MIT — see [`LICENSE`](LICENSE).
