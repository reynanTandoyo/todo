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

### Updating

Download `todo.exe` from the latest
[GitHub release](https://github.com/reynanTandoyo/todo/releases/latest) and copy it over the old
one:

```powershell
Copy-Item "$HOME\Downloads\todo.exe" (Get-Command todo).Source -Force
Unblock-File (Get-Command todo).Source
todo --version
```

`Unblock-File` removes the "downloaded from the internet" mark, so Windows doesn't show the
SmartScreen prompt for the unsigned `.exe`. If the copy fails with "file in use", a `todo` command
is still running; try again a moment later. Each release also has a `.sha256` file to compare with
`Get-FileHash todo.exe`.

On Linux, download `todo-linux-x86_64` and install it as `todo`:

```bash
install -m 755 ~/Downloads/todo-linux-x86_64 ~/.local/bin/todo
```

After an update, run `todo migrate --yes` in each project. It moves `TODO.md` and its archive to the
current file format (version 2 since todo 0.6) and brings the installed rules block, Claude Code skill
and hooks up to date. `todo help update` prints the steps for updating the executable itself, and
[Versions, format and migration](#versions-format-and-migration) explains what changes.

Colleagues may run different versions against the same `TODO.md`. Todo 0.5 and older can still read a
version 2 file, with a warning, but can no longer write it, so everyone who writes the file should update
before anyone migrates it. A todo that is too old for a file refuses to work on it (`TOOL_TOO_OLD`).

## For people

You do not need to read `TODO.md` to see where the work stands. Agents keep the list, and you read
the page they build from it.

1. **Register each project once**, from its folder: `todo project add --name renderer` (or ask an
   agent to do it). The register is one file per user, `%LOCALAPPDATA%\todo\projects.json`.
2. **Ask an agent for the report.** It runs `todo report --html --all`, which writes one page for
   every registered project to `report-all.html` next to the register.
3. **Open the page** in your browser, or ask the agent to open it: `todo report --html --all --open`.
   The page has a card per project, then each project's lists. A project that cannot be read shows
   its problem instead of its lists.
4. **Every change goes through an agent.** The page is read-only: it has no buttons, forms or
   commands. To change a task, ask an agent.

```powershell
todo project add --name renderer
todo project list
todo overview
todo report --html --all --open
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
todo help worker
todo help planner
todo help update
todo --version
```

`todo schema` prints a JSON description of this build: the commands with their arguments and options,
the JSON envelope, every exit code and error code with its meaning, and the task fields. Use it to
let an agent discover the interface; `todo schema show` describes one command, and `--json` wraps the
description in a `todo.schema.v1` envelope.

```powershell
todo schema
todo schema hooks install
todo schema --json
```

### Reading tasks

```powershell
todo summary
todo tree
todo list
todo list --ready
todo list --mine --agent agent-1
todo list --status all --json
todo show T-012
todo next --agent agent-1
todo next --claim --agent agent-1
```

### HTML report for people

```powershell
todo report --html
todo report --html --open
todo report --html --out "$env:USERPROFILE\Desktop\tasks.html"
todo report --html --all
```

`todo report --html` writes one read-only page: the planning state (open tasks per level, tasks not
ready for lack of detail, tasks given back with their attempts, tasks kept above the target level,
and the waves), the work as a tree of milestones with their goals and progress, the unfinished tasks
per area, open questions, ideas, and the done and dropped entries. An opened task shows its level,
files, check and whether the check passed.
It prints the path of `report.html` in the state folder, which git never sees. It takes no lock,
never changes `TODO.md`, and needs no network. `--out` writes the page elsewhere, and `--open` opens
it with the system's default program for HTML files.

`todo report --html --all` writes one page for every project in the project register (see
`todo project list`): a card per project with its task counts, open tasks per level, top-level
milestones, blocked tasks and open questions, then each project's lists. It prints the path of `report-all.html` next to the
register. A project that is gone or cannot be read shows its problem instead of its lists.

### Adding and editing tasks

```powershell
todo add --title "Fix login timeout" --priority P1 --difficulty 2 --type bug --area auth --section Now --agent agent-1
todo set T-012 --priority P0 --area cli --field reqs=R-WRITE-8 --unset fix --agent agent-1
todo note T-012 "reproduced on Windows 11" --agent agent-1
```

### Working through a task

```powershell
todo brief T-012 --agent agent-1
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

### Milestones

A milestone is a piece of work with a goal, divided into tasks: for example "Develop the renderer", with
the tasks "C shader" and "GPU compiler". A milestone is never a release or version label such as "v0.6".
Milestones nest to any depth, and every entry names its parent with `milestone`.

```powershell
todo add --type milestone --title "Develop the renderer" --goal "the test scene renders with lighting at 60 frames per second" --priority P1 --section Now --agent planner
todo add --title "C shader" --milestone T-001 --why "the renderer has no shading stage yet" --priority P1 --difficulty 4 --section Now --agent planner
todo add --title "GPU compiler" --milestone T-001 --priority P1 --difficulty 4 --section Now --agent planner
todo tree T-001
todo list --milestone T-001
```

`todo tree` shows each milestone with its goal and its progress (`done/total` over every task below it,
at any depth). Milestones are never claimed or handed out by `next`. `claim` and `next --claim` print the
goals of the milestones above a task, so the agent that takes it knows why it matters:

```text
T-002: claim ok
Milestones above T-002, from the top down:
- T-001 · Develop the renderer
  goal: the test scene renders with lighting at 60 frames per second
```

- `todo done T-001` is refused while anything below the milestone is open, claimed or blocked, and lists
  those entries. `--force` marks it done anyway and says so in a note.
- When the last entry below a milestone is finished, `done` names it as ready to close, with the command
  to close it. A milestone is never closed automatically.
- `todo drop --tree T-001 --note "superseded"` drops the milestone and every unfinished entry below it.
- `todo archive` keeps a milestone's tree together: finished entries under an open milestone stay in
  `TODO.md`, and the output says so.
- `next` prefers tasks in a milestone that has already started over tasks in untouched milestones.

### Kinds of entry

Every entry has one kind, taken from its `type`: a milestone (`milestone`), a question (`question`), an
idea (`idea`), or a task (any other type, or none). Only tasks are handed out by `next` and counted as
open work; `summary` and `tree` show questions and ideas apart from them.

```powershell
todo idea "Hot-reload shaders while the scene runs" --about T-001 --agent planner
todo ask "Should the C shader support shadows in its first version?" --about T-002 --agent agent-ui-1
todo list --kind question
todo promote T-004 --priority P2 --milestone T-001 --agent planner
```

- `idea` needs no priority and is never handed out. `promote` turns an idea into a task with a priority
  and a milestone, and adds the note `promoted from idea`.
- `ask --about` names the task or milestone a question is about. `show` lists the questions about an
  entry, and `answer` shows the entry that a question is about.

### Versions, format and migration

A `TODO.md` written by todo 0.6 starts with two lines:

```markdown
<!-- todo-format: 2 -->
<!-- todo-tool: 0.6.0 -->
```

- The **format line** is the file format version. Version 3 (todo 0.8 and later) adds the levels `1`
  to `5` as the default `difficulties`, the detail fields `steps`, `read` and `verify`, `final`,
  `attempts` and `verified`, and `files` as a list of paths and patterns. Version 2 (todo 0.6 and 0.7)
  adds milestones, the kinds of entry and the `why`, `difficulty`, `milestone`, `about` and `goal`
  fields; version 1 was written by todo 0.1 to 0.5 (`docs/format.md` §18). A file in version 3 stops
  todo 0.7 with `TOOL_TOO_OLD`: update todo first, then run `todo migrate --yes` in the project.
- The **tool line** is the newest todo that has written the file. A write by a newer todo raises it, and
  nothing lowers it. `todo where` shows both, and the running version.
A file in an older format is migrated by the first write of todo 0.6, under the lock and after a snapshot
(`todo history` and `todo restore` can go back). To migrate at once, look at the change first and then
write it:

```powershell
todo where
todo migrate --dry-run --agent agent-1
todo migrate --yes --agent agent-1
```

Migration changes only the format and tool lines; task lines stay as they are, and a second run changes
nothing. `migrate --yes` also refreshes the rules block, skill and hooks that the project has installed.

A todo that is **behind** a file, one that knows the file's format but was last written by a newer todo,
still works and warns on standard error, naming `todo help update`.

A todo that is **too old** for a file, because the file's format is newer than any format it knows, stops
every command that reads or changes tasks with exit 4 and the error `TOOL_TOO_OLD`. Nothing is written:

```text
error: TODO.md:1: format version 3 is newer than this todo 0.7.0 supports; the file was last written by todo 0.9.0. Update the todo executable first (`todo help update`).
```

`where`, `help`, `schema`, `status`, `history`, the lock commands and the other commands that never read
tasks keep working. Run `todo help update` for the steps that update the executable, then run
`todo migrate --yes` in each project.

### Who finished a task

`todo done` (and the `done` operation of `apply`) adds a `by` line with the person behind the work,
taken from git: the author of `--commit`, or without `--commit` the repository's `user.name`. Both go
through `.mailmap`, so a team can give every contributor one name however their computers are set
up. No email address is ever written to `TODO.md`. Without git, outside a repository or without a
name, `by` is left out and `done` works as before. `reopen` removes it.

```powershell
Set-Content .mailmap @'
# Proper Name <proper@email> [Commit Name] <commit@email>
Ada Lovelace <ada@example.org> <ada@laptop.invalid>
Ada Lovelace <ada@example.org> Ada L <ada@work.invalid>
'@
todo done T-012 --commit HEAD --agent agent-1
todo list --by "ada lovelace"
todo check
```

`list --by <name>` ignores case and implies `--status all`. When `.mailmap` exists, `todo check`
warns about every `by` value that is not a name in it (the exit code does not change).

### Questions, batch operations, and inspection

```powershell
todo where
todo search "timeout"
todo search "timeout" --archive
todo log T-012
todo check
todo check --fix --agent agent-1
todo align
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

### Merging `TODO.md` with git

Two branches that both add tasks or notes to `TODO.md` conflict in plain git even though nothing
overlaps. The optional merge driver merges the file task by task. Run this once in each clone
(`todo` must be on `PATH` when git merges):

```powershell
todo merge-driver install
git merge other-branch
todo merge-driver uninstall
```

`install` sets `merge.todo.name` and `merge.todo.driver` in `.git/config` and adds the line
`TODO.md merge=todo` to `.gitattributes` (commit that file so every clone uses the driver; other
lines are left alone; `--path <pattern>` uses another pattern). Tasks changed on different sides,
different fields of one task, and new tasks added at the same place merge cleanly, and notes added on
both sides are all kept, ours first. A task whose same field was changed differently on both sides,
or that was added with the same ID on both sides, gets `<<<<<<< ours` / `=======` / `>>>>>>> theirs`
markers around the whole task block; fix it by hand and `git add TODO.md`. Prose conflicts, input
that is not valid UTF-8 and files with different format versions are left to `git merge-file`. Task
IDs are recognised with the default prefix `T`. Run `todo help merge-driver` for details.

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
| `- [!]` with no `blocked — <reason>` sub-bullet | status `blocked`, with the note `blocked — imported from a checkbox list` |
| `(added: agent, 2026-11-01)` | note `added` |
| `(claimed: agent, 2026-11-01)` | owner `agent`, claim time = time of the import (an old claim would be stale), the old date in a note |
| `(done: agent, 2026-11-05, abc1234)` | note `done` and `commit: abc1234` |
| `  - 2026-11-05 agent: text` | note by `agent` on that date |
| sections named like "Needs user decision" or "Questions" | `type: question` (so `todo next` skips them) |

Checkboxes in the preamble, in code fences or indented stay text. IDs are `T-001`, `T-002`, … in
file order. The command checks the result like `todo check`, compares the task count with the number
of converted lines and reports both. It does not take the write lock; use it on lists that agents
are not using yet.

### Finding TODO comments in code

`todo scan` lists the `TODO:` and `FIXME:` comments in the folder that holds `TODO.md`, as
`path:line: text`. With git it looks at the files git knows about (tracked and not ignored); without
git it walks the folder, skipping hidden folders, `target` and `node_modules`. Comments that already
name a task ID or repeat a task title are left out.

```powershell
todo scan
todo scan --add --section Now --priority P2 --agent coordinator   # adds them in one locked write
todo scan --add --dry-run --priority P2 --agent coordinator       # shows what would be added
```

Each added task gets `files: <path>:<line>`, and its title is the comment text cut to 120 characters.

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

## Roles, levels and parallel work

Two roles use todo (`docs/requirements.md` §2). The **planner**, usually one strong agent or a person,
splits the work into tasks and gives each task a level, until every task can go to a worker. The
**workers** are any number of cheaper agents; each takes one ready task, finishes it and records the
result. Each role has its own rules text, so no agent carries rules it does not need:

- `todo help agents` prints the common rules, which the managed block of `AGENTS.md`, `TODO.md` and the
  Claude Code skill carry. `todo help worker` prints the worker's rules (at most ten lines), and
  `todo help planner` the planner's.
- A task has a **level** from `1` to `5`, its `difficulty`. Level `1` is the lowest: everything is
  written down (`files`, `read`, `steps`, `verify`, `why` and `accept`), so the weakest worker finishes
  it without a decision. Higher levels need more understanding, and level `5` is research or a design
  that the planner splits before anyone works on it. `todo help add` prints what each level means.
  `target_difficulty` in `todo.toml` sets the level the planner splits down to (by default the lowest),
  and `final` keeps a task above it on purpose, with a reason.
- `todo plan` lists what the planner still has to do: tasks given back, tasks above the target level
  (split them with `todo split`), tasks at the target level that lack their detail, and tasks without a
  usable level. A task is ready for `next` only when its detail is complete and its dependencies are
  done.
- A worker takes its work with `todo next --claim --max-difficulty 1 --brief`, which prints the work
  order (`todo brief`). It changes only the files the task lists, commits only those files, and finishes
  with `todo done <id> --commit HEAD`. A worker that cannot finish runs
  `todo stuck <id> --note "<problem>"`; the task then waits for the planner, which runs `todo unblock` or
  `todo split` on it, and `todo plan` lists it first.

### File scopes in one checkout

Workers share one checkout, so a task's `files` field is what it may change. Two tasks whose files
overlap never run at the same time:

- `next` skips a ready task that overlaps a task another agent holds, and `claim` refuses it with
  `FILES_BUSY` unless `--force`;
- `todo waves` puts overlapping tasks of one wave in one lane, so they run one after another, and shows
  how many workers can work at once and the longest chain of tasks;
- `done --commit` checks that the commit changed only the task's files and refuses a commit that changed
  others with `OUT_OF_SCOPE`, so one worker's edit does not break another worker's task unnoticed.

### What `verify` is trusted to do

A task's `verify` line is a command that todo runs in the task's folder, through `verify_shell`, outside
the write lock. todo trusts only its exit code: `0` passes, and anything else fails. After
`verify_timeout_seconds` the command and every process it started are stopped, and the check fails.
`done` runs the command before it marks the task done; a failure refuses it with `VERIFY_FAILED` and the
task stays claimed, and after `verify_attempts` failures the task is given back as stuck. With
`verify_when = "wave"`, `todo verify --wave` runs the checks of a finished wave instead.

The command is written in `TODO.md`, and it runs with the permissions of the agent that finishes the
task: review a `verify` line the way you review code. todo does not sandbox it, and a pass proves only
what the command checks. The output is kept in full in the state folder (`verify/<ID>.log`).

## Configuration (`todo.toml`)

An optional `todo.toml` next to `TODO.md` sets the project's options. Without it, every default below
applies. An unknown key or a bad value is an error, reported as `todo.toml:<line>: message`. The
requirements in [`docs/requirements.md`](docs/requirements.md) §5.14 (R-CFG-2) are the reference.

| Key | Default | Meaning |
|---|---|---|
| `id_prefix` | `"T"` | prefix of task IDs |
| `id_width` | `3` | digits in new IDs |
| `priorities` | `["P0", "P1", "P2", "P3"]` | allowed priorities, most urgent first |
| `required_fields` | `["priority", "difficulty"]` | fields `add` requires of a task; milestones, questions and ideas need none of them; `["priority"]` before v0.8 |
| `types` | `[]` | allowed types; empty allows any |
| `areas` | `[]` | allowed areas; empty allows any |
| `default_section` | `""` | section for `add` without `--section`; empty means none |
| `question_section` | `"Questions"` | section for `ask` |
| `idea_section` | `"Ideas"` | section for `idea` |
| `difficulties` | `["1", "2", "3", "4", "5"]` | allowed levels, easiest first; `["easy", "medium", "hard"]` before v0.8 |
| `target_difficulty` | the first of `difficulties` | the level the planner splits down to: tasks at or below it need complete detail before `next` hands them out, and `final` keeps a task above it |
| `level1_max_files` | `3` | most `files` entries a task at or below the target level may have |
| `verify` | `true` | `false`: todo never runs a task's `verify` command |
| `verify_when` | `"done"` | `"done"`: `done` runs the command; `"wave"`: `todo verify --wave` runs it after the wave |
| `verify_timeout_seconds` | `300` | seconds a `verify` command may run before it is stopped and counts as failed |
| `verify_attempts` | `3` | failed checks after which `done` gives the task back |
| `verify_shell` | `[]` | the program and arguments that run a `verify` line, which is added last; empty means `pwsh -NoProfile -NonInteractive -Command` (else `powershell.exe` with the same arguments) on Windows, and `sh -c` elsewhere |
| `archive_file` | `"TODO-archive.md"` | archive file name |
| `claim_ttl_hours` | `24` | hours before a claim becomes stale |
| `lock_wait_seconds` | `30` | how long a write waits for the lock or a lease |
| `lease_minutes` | `10` | default length of `todo lock` |
| `lease_max_minutes` | `60` | longest allowed lease |
| `snapshots_keep` | `200` | snapshots kept |
| `worktree` | `"shared"` | `"local"`: each worktree uses its own `TODO.md` |

## Adding it to a project

1. Run `todo init` in the repository root. It creates `TODO.md` in the current format, with the format line,
   the tool line, the rules block and a `## Tasks` section, and prints the lines for `AGENTS.md`. Or write
   the file yourself, with the format line and at least one `## ` section (the first write adds the tool
   line):

   ```markdown
   <!-- todo-format: 2 -->

   ## Now

   ### T-001 · Develop the parser
   - status: open
   - owner: —
   - priority: P1 · type: milestone
   - goal: the reader accepts every example in the manual
   - note (2026-11-18, planner): added

   ### T-002 · Write the tokenizer
   - status: open
   - owner: —
   - priority: P1 · difficulty: 2 · type: feature
   - milestone: T-001
   - why: the parser needs tokens first
   - note (2026-11-18, planner): added
   ```

2. Add these rules to your project's `AGENTS.md`, or run `todo agents install --yes` to write them between
   the `<!-- todo:begin -->` and `<!-- todo:end -->` markers. `todo help agents` prints the same text:

   ```markdown
   # Working with TODO.md

   - Never edit `TODO.md` (or `TODO-archive.md`) directly; always use `todo`.
   - Use one unique agent name per agent via `--agent <name>` or `TODO_AGENT`.
   - Start by running `todo summary` or `todo next`.
   - Claim a task before working on it (`todo claim <id>`, or `todo next --claim` to pick and claim in one step).
   - Before you start a task, read the goals of the milestones above it: `todo claim` and `todo show <id>` print them.
   - Exit code `1` means someone else claimed it or the rule refused the change — pick another task.
   - Exit code `3` means the lock or lease is busy — try the command again.
   - Add notes as you learn things (`todo note <id> "..."`).
   - Finish a task after committing: `todo done <id> --note "..." --commit HEAD`.
   - Close a milestone with `todo done <milestone-id>` once its last task is finished; `todo done` names a milestone when it is ready to close.
   - Put each new task under a milestone, with a `why`: `todo add --title "..." --milestone <id> --why "..." --priority P2 --agent <name>`. A milestone is a piece of work with a goal, such as "Develop the renderer", never a release label.
   - Use `todo add --title "..." --milestone <id> --why "..."` to record problems you find instead of fixing them silently.
   - Record ideas with `todo idea "..."`, and questions about a task or milestone with `todo ask "..." --about <id>`.
   - Use `todo ask "..."` for questions only a person can decide.
   - Start every git commit message with the task ID (for example `T-012: short summary`).
   - After todo is updated, run `todo migrate --yes` in each project.
   - When todo says a file needs a newer todo (`TOOL_TOO_OLD`, or a warning that it is behind), update the todo executable first: follow `todo help update`.
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
