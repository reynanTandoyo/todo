# todo — requirements

Version 1, 2026-11-15. **Complete:** every decision is made (§3) and nothing waits on an answer.
Companion documents: [format.md](format.md) (the exact file format), [research.md](research.md)
(what was studied and why), and the work plan [`../TODO.md`](../TODO.md). Changing a requirement is
itself a task in `TODO.md`.

| Priority | Meaning | Release | `TODO.md` priority |
|---|---|---|---|
| **MUST** | needed for the first usable version | v0.1 | P0 |
| **SHOULD** | part of the complete tool | v0.2 | P1 |
| **COULD** | valuable extras | v0.3 | P2 |
| Idea (§12) | recorded, not planned | — | P3 |

---

## 1. What `todo` is

`todo` is a command-line program, `todo.exe`, that lets several AI agents and people share one
Markdown task list, `TODO.md`, without corrupting it or losing each other's changes. It finds the
file, takes a lock, changes only the lines it has to, and replaces the file in one step. Agents use
it instead of editing `TODO.md` themselves, and get plain text or JSON back.

- This repository stands alone. It doesn't depend on, change, or plan work in any other project.
- One `TODO.md` per repository or folder. Windows is the target platform.
- Entries are kept apart by kind (R-KIND-3): tasks; milestones, which are pieces of work with a goal,
  divided into tasks and nested to any depth (R-MILE-1); questions; and ideas.

## 2. Who uses it, and the situations it must handle

- **Coordinator**: usually a main agent session. Plans work, adds tasks in bulk, hands tasks to
  other agents, reviews and closes them.
- **Worker agents**: Claude Code, Codex, opencode, Gemini and others, several at once, often in
  linked git worktrees. Short-lived; may crash or be cancelled mid-task.
- **People**: look at the HTML report, read `TODO.md` in an editor or on GitHub, sometimes edit it
  by hand, and answer questions agents raise.

Command output, plain text and JSON alike, is written for agents. The one view meant for people is
the HTML report (`todo report --html`, R-REPORT-1, R-REPORT-2): a read-only page, built only when
someone asks for it, that never changes anything. A person who wants a change asks an agent, or edits
by hand under a lease (R-LOCK-6).

| # | Situation | Requirements |
|---|---|---|
| S1 | An agent starts a session and needs an overview without reading a large file | R-READ-5, R-ARCH-1 |
| S2 | Two agents claim the same task at the same moment: exactly one succeeds | R-CLAIM-2, TEST-5 |
| S3 | Two agents ask for the next task at the same moment: they get different tasks | R-READ-3 |
| S4 | An agent crashes while holding the write lock: the others continue within seconds | R-LOCK-2 |
| S5 | An agent abandons a claim: the task becomes available again | R-CLAIM-4 |
| S6 | A person edits `TODO.md` by hand while agents work | R-LOCK-6, R-IO-2, R-HIST-3 |
| S7 | An agent works in a linked git worktree | R-FIND-5 |
| S8 | The coordinator adds 30 tasks at once | R-WRITE-13 |
| S9 | Finished tasks pile up | R-ARCH-1 |
| S10 | Antivirus or an editor holds `TODO.md` open while it is being replaced | R-IO-3 |
| S11 | A task needs another task finished first | R-DEP-1 |
| S12 | A question only a person can answer | R-WRITE-11 |
| S13 | An agent passes text with quotes, backticks or `$` through PowerShell | R-WRITE-12 |
| S14 | An agent takes a task and needs to know what larger piece of work it serves, and why | R-MILE-4, R-WRITE-17 |
| S15 | A coordinator plans a piece of work ("Develop the renderer") as tasks and nested milestones | R-MILE-1, R-MILE-3, R-MILE-6 |
| S16 | A person wants to see the state of the work without reading `TODO.md` | R-REPORT-1 |
| S17 | A person wants one overview of all their projects | R-PROJ-1, R-PROJ-2, R-REPORT-2 |
| S18 | todo is updated: each `TODO.md` and the installed rules, skill and hooks must follow | R-VER-1, R-MIG-1, R-MIG-2 |
| S19 | An older todo meets a `TODO.md` written in a newer format | R-VER-2, R-OUT-11 |
| S20 | A small model should take only easy tasks | R-WRITE-18 |

---

## 3. Decisions

| # | Decision | Why |
|---|---|---|
| D1 | One `TODO.md` per repository. Tasks are `### T-012 · Title` blocks with one `- key: value` line per field, under free-form `##` sections ([format.md](format.md)). | One file stays readable in any editor and on GitHub and can be protected by one lock. The block format gives stable IDs, room for fields and one-line diffs, and has handled hundreds of tasks in practice. One file per task was rejected ([research.md](research.md) §3). |
| D2 | Five statuses: `open`, `claimed`, `blocked`, `done`, `dropped`. | `dropped` keeps "won't do" out of the done count and out of the way. |
| D3 | IDs are a prefix and a number (`T-001`), chosen as the highest so far plus one, under the lock, and never reused. | Short, sortable and easy to say. Random IDs only help when tasks are added on separate branches, which D5 avoids. |
| D4 | A claim records owner and time; it goes stale after 24 hours; claiming again refreshes it. | Agent sessions last hours, so abandoned work frees up the next day without taking active work away. No separate heartbeat command is needed. |
| D5 | Linked git worktrees use the main checkout's `TODO.md` and lock. | Otherwise agents in worktrees work on stale copies and their locks protect nothing. |
| D6 | Each write command takes an operating-system file lock. People editing by hand take a 10-minute lease with `todo lock`. | The operating system frees the lock when a process dies, so crashes never block others. The lease gives people the 10-minute manual lock. |
| D7 | Lock, lease and snapshots live in the git common folder (`.git/todo/…`), or in `.todo/` outside git. | Shared by every worktree, never committed, and no `.gitignore` changes needed. |
| D8 | Finished tasks stay where they are; `todo archive` moves them to `TODO-archive.md` on request. | Context stays visible while work goes on; the file shrinks when someone decides it should. |
| D9 | Every write needs an agent name (`--agent` or `TODO_AGENT`); there's no default. | A shared default name would make claims meaningless. |
| D10 | `--json` on every command, one envelope; exit codes 0–5 with fixed meanings. | Agents parse one stream and branch on the exit code. |
| D11 | Settings are optional: one `todo.toml` next to `TODO.md`, no per-user settings. The one per-user file is the project register (R-PROJ-1), which lists the user's projects and holds no settings. *(Amended by the owner's decision of 2026-11-18.)* | New projects need no setup; projects that want rules can have them. The register is per user because it spans that user's projects in different folders, and it lets todo show all of them without searching the PC. |
| D12 | Windows 10/11 x64 is the target. Linux is built and tested in CI to keep the code portable. No macOS builds. | The tool is wanted as a Windows `.exe`; Linux CI is cheap; nothing is available to test macOS. |
| D13 | Distributed as a single unsigned `todo.exe`, from GitHub Releases or built from source, copied into a folder on `PATH`. No installer. | Needs no administrator rights. |
| D14 | Every agent learns the tool from one rules text (`todo help agents`, `AGENTS.md`). Claude Code hooks and a skill are extras. No MCP server. | Every agent reads `AGENTS.md`; hooks only exist in Claude Code; an MCP server is another protocol to maintain. |
| D15 | No network access, background process, database or index. git is optional. | Fewer ways to fail; works offline and in any folder. |
| D16 | Rust 1.89 or newer, edition 2024, a short list of approved crates (NFR-9). | `File::try_lock` is in the standard library from Rust 1.89. |
| D17 | MIT licence. | Simple and permissive, like most comparable tools. |
| D18 | UTF-8 only. Each line keeps its own line ending; bytes the tool doesn't change stay identical. | Files edited on Windows mix line endings, and preserving bytes is what makes the tool safe to use on hand-written files. |

---

## 4. The file in brief

The full grammar is in [format.md](format.md). In short:

```markdown
<!-- todo-format: 2 -->
<!-- todo-tool: 0.6.0 -->
# Example project — tasks

## Now

### T-001 · Develop the renderer
- status: open
- owner: —
- priority: P1 · type: milestone · area: renderer
- goal: the test scene renders with lighting at 60 frames per second
- note (2026-11-15, coordinator): added

### T-002 · C shader
- status: claimed
- owner: agent-ui-1
- claimed: 2026-11-15T14:03+01:00
- priority: P1 · difficulty: hard · type: feature · area: renderer
- milestone: T-001
- why: the renderer has no shading stage yet
- accept: the test scene shows lit surfaces
- note (2026-11-15, coordinator): added
```

(Invented example.) Known fields: `status`, `owner`, `claimed`, `session`, `priority`,
`difficulty`, `type`, `area`, `milestone`, `about`, `goal`, `why`, `depends`, `files`, `problem`,
`fix`, `accept`, `commit`, `by`, and dated `note` lines. Any other `- key: value` line is kept and
shown in JSON. Everything outside task blocks — the preamble, prose, tables, code — is never changed
by task commands; only the format and tool lines are kept up to date (R-VER-1, R-MIG-1).

---

## 5. Functional requirements

### 5.1 Finding the file — FIND

- **R-FIND-1 MUST** `--file <path>` uses that file and skips the search.
- **R-FIND-2 MUST** Otherwise the `TODO_FILE` environment variable is used.
- **R-FIND-3 MUST** Otherwise the tool walks up from the current folder and uses the first
  `TODO.md` it finds.
- **R-FIND-4 MUST** The walk stops at the root of the git repository it started in (the first folder
  with a `.git` file or folder), so a repository without a `TODO.md` never picks up one from a
  parent folder. Outside a repository it goes up to the drive root.
- **R-FIND-5 MUST** In a linked git worktree, the search runs in the main checkout instead: the
  current folder's path relative to the worktree root is mapped onto the main checkout and the walk
  continues from there. The tool says on standard error which file it uses. It finds the main
  checkout from the worktree's `.git` file and the `commondir` file it points to, without needing
  git. Setting `worktree = "local"` turns this off. When the main checkout can't be found (a bare
  repository, an unusual layout), the worktree's own file is used, with a warning.
- **R-FIND-6 MUST** When no file is found, the command exits 4 and suggests `todo init`. Only `init`
  ever creates `TODO.md`.
- **R-FIND-7 SHOULD** `todo where` prints the file in use and how it was found, the repository root,
  the state folder, the lock and lease status, the format version, the tool version stamped in the
  file and the running version (R-VER-1), and the number of tasks per status; milestones, questions
  and ideas that are not done or dropped are counted apart from the tasks (R-KIND-3).

### 5.2 Reading and changing the file — EDIT

- **R-EDIT-1 MUST** The file is read as bytes and handled as a list of lines, each keeping its own
  line ending. Commands change the file only by replacing, inserting or removing whole lines; every
  other byte stays the same.
- **R-EDIT-2 MUST** Reading a file and writing it back unchanged gives identical bytes.
- **R-EDIT-3 MUST** Text the tool doesn't use is kept exactly: the preamble, sections without tasks,
  prose and lists inside tasks, unknown fields, tables, code blocks and comments. Headings inside
  code fences and HTML comments are never taken as tasks or sections.
- **R-EDIT-4 MUST** Added lines use the file's usual line ending; a missing final newline stays
  missing; a byte-order mark stays (format.md §2).
- **R-EDIT-5 MUST** Values the tool writes are single lines: line breaks in input become spaces.
- **R-EDIT-6 MUST** A command that changes nothing doesn't write the file.
- **R-EDIT-7 MUST** Before writing, the tool checks its own result: every byte outside the changed
  lines equals the original, and the new text parses to the same tasks apart from the intended
  change. If the check fails, nothing is written and the command exits 4.
- **R-EDIT-8 MUST** Writing is refused in the cases listed in format.md §13 (invalid UTF-8, conflict
  markers, an unclosed fence or comment; or, for the task being changed, a duplicate ID, a repeated
  field or an unknown status). Read commands still work and show the same messages as warnings. A
  format version the tool doesn't know is different: from v0.6 on it stops reads as well as writes
  (R-VER-2).
- **R-EDIT-9 MUST** Lenient reading, canonical writing (format.md §15): the alternative heading
  separators and the different ways of writing "nobody" are all understood; the tool writes the
  canonical form only on lines it changes.
- **R-EDIT-10 SHOULD** Every write command accepts `--dry-run`: it prints the diff it would apply
  and changes nothing.

### 5.3 Locking — LOCK

- **R-LOCK-1 MUST** A write command takes the write lock, reads the file, changes it, writes it, and
  releases the lock. It never works from a copy read before the lock was taken.
- **R-LOCK-2 MUST** The write lock is an operating-system lock (Rust's `File::try_lock`) on
  `write.lock` in the state folder. The operating system releases it when the process ends, so a
  crashed command never blocks the others.
- **R-LOCK-3 MUST** A command waits up to 30 seconds for the lock (`lock_wait_seconds`), trying
  again every 40–160 ms at random. It then exits 3 ("busy, try again") and names the holder — agent,
  command, process ID and start time — from the `holder.json` that every holder writes after
  locking.
- **R-LOCK-4 MUST** The lock is held only while reading, changing and writing the file. Slow work,
  such as asking git about a commit or waiting for an editor, happens outside it.
- **R-LOCK-5 MUST** Read commands never take the lock; replacing the file in one step makes that
  safe.
- **R-LOCK-6 MUST** `todo lock --agent X [--minutes N]` gives agent X a lease for editing by hand:
  10 minutes by default (`lease_minutes`), at most 60 (`lease_max_minutes`). While a lease is live,
  other agents' write commands wait as for the write lock and then exit 3, naming the holder and the
  expiry time; the holder's own commands work normally. `todo unlock --agent X` ends the lease; an
  expired lease counts as ended; `unlock --force` ends someone else's lease and says whose it was.
  `todo status` shows the write lock and the lease. Edits made during a lease are recorded as that
  agent's edit (R-HIST-3), without a warning.
- **R-LOCK-7 MUST** The lock and lease belong to the resolved file, so every worktree and every way
  of writing the path (relative, absolute, different letter case on Windows) uses the same lock.
- **R-LOCK-8 COULD** `todo edit --agent X` takes a lease, opens the file in `VISUAL`, `EDITOR` or
  Notepad, waits until the editor closes, runs `check`, records a snapshot and ends the lease.

### 5.4 Writing to disk — IO

- **R-IO-1 MUST** The new content goes into a temporary file in the same folder
  (`TODO.md.<process id>.<random>.tmp`), is flushed to disk, and replaces `TODO.md` by a rename.
- **R-IO-2 MUST** Just before the rename, the tool checks that `TODO.md` still has the size,
  modification time and hash it read. If not, another program changed it despite the lock: the tool
  deletes its temporary file, writes nothing, and exits 3.
- **R-IO-3 MUST** On Windows, the rename is retried for up to 2 seconds while another program
  (antivirus, an editor, the search indexer) holds the file; after that, the temporary file is
  deleted and the command exits 4. No temporary file is ever left behind.
- **R-IO-4 MUST** Invalid UTF-8 is reported with its byte position.
- **R-IO-5 MUST** A read-only `TODO.md` is refused with exit 4 and a clear message.
- **R-IO-6 MUST** Note dates are local dates (`2026-11-15`). The `claimed` time is local time with
  its UTC offset (`2026-11-15T14:03+01:00`), and ages are compared in UTC, so time zones and
  daylight saving don't break stale-claim checks.

### 5.5 Task identity — ID

- **R-ID-1 MUST** IDs are a prefix, a hyphen and a number (`T-001`). Prefix (`id_prefix`, default
  `T`) and padding (`id_width`, default 3) are configurable; numbers may outgrow the padding.
- **R-ID-2 MUST** `add` uses the number after the highest found in `TODO.md`, the archive file and
  the state folder's record of the highest number ever given out, all read under the lock. Numbers
  are never reused, even after a task was archived or deleted by hand on this machine.
- **R-ID-3 MUST** `12`, `t12`, `T-12` and `T-012` all name task 12.
- **R-ID-4 SHOULD** A task argument that isn't an ID is matched, ignoring case, as part of the title
  of tasks that aren't done or dropped. Exactly one task must match; otherwise the command exits 1
  and lists the candidates, or says there are none.

### 5.6 Reading tasks — READ

- **R-READ-1 MUST** `list` prints one line per task — ID, priority, status, owner if any, and title
  shortened to the terminal width unless `--wide` — and then a count. Done and dropped tasks are
  hidden by default. Filters: `--status` (a comma-separated list, or `all`), `--priority`,
  `--section`, `--type`, `--area`, `--owner`, `--where key=value` (any field, unknown fields
  included; repeatable; values compared ignoring case), `--ready` (tasks `next` could hand out),
  `--mine` (claimed by this agent), `--stale` (claims past the timeout), `--archive` (list the
  archive file instead). `--milestone`, `--kind` and `--difficulty` are added by R-MILE-3 and
  R-WRITE-18.
- **R-READ-2 MUST** `show <task>` prints the task exactly as it appears in the file. With `--json` it
  returns every field, the notes, unknown fields, the section, the line numbers, the dependencies
  with their status, and the tasks that depend on it.
- **R-READ-3 MUST** `next` returns the task to take: status `open`, or `claimed` with a stale claim;
  every dependency finished (R-DEP-1); of kind task, so never a milestone, question or idea
  (R-KIND-3); highest priority first, then earliest in the file. Milestones add to readiness and
  order (R-MILE-6). Filters: `--section`, `--type`, `--area`, `--where key=value`, `--milestone`
  (R-MILE-6), `--difficulty` and `--max-difficulty` (R-WRITE-18). With an agent name, if
  that agent already has a claimed task, `next` returns it first, marked as already theirs (`--new`
  skips this). `next --claim --agent X` chooses and claims in one locked step, so agents asking at
  the same moment get different tasks.
- **R-READ-4 MUST** Every read command supports `--json` (R-OUT-1).
- **R-READ-5 SHOULD** `summary` prints a session-start overview of at most 40 lines: tasks per
  status; open tasks per priority; claimed tasks with owner and age, stale ones marked; blocked
  tasks with the reason of their latest `blocked — <reason>` note (the same reason the HTML report
  shows, R-REPORT-1; `docs/format.md` §9); the next five ready tasks; open questions; and a warning if the
  file was edited outside the tool. From v0.6 it also shows the top-level milestones that are open or blocked, with
  progress (R-MILE-3), milestones ready to close (R-MILE-6), the number of ideas apart from open
  tasks (R-KIND-1), and a line naming `todo migrate --yes` while project files are out of date
  (R-MIG-2), still within 40 lines.
- **R-READ-6 COULD** `search <text>` finds text in titles, fields and notes, printing each match
  with its task ID; `--archive` searches the archive file.
- **R-READ-7 COULD** `log <task>` shows the journal entries about one task (R-HIST-4).

### 5.7 Changing tasks — WRITE

Every write command needs an agent name (R-CLAIM-5). It prints one line such as `T-012: claim ok`,
or with `--json` the task as it is after the change.

- **R-WRITE-1 MUST** `add --title T [--section S] [--priority P] [--type X] [--area X]
  [--depends IDs] [--files T] [--problem T] [--fix T] [--accept T] [--field key=value]…` adds a task
  at the end of a section (format.md §10) and prints its ID. The section is found by its exact name,
  ignoring case and a leading number, or else by a unique part of the name. An unknown section is
  refused (exit 1, `UNKNOWN_SECTION`) and an ambiguous one (a name that matches several) is refused
  (exit 1, `AMBIGUOUS_SECTION`), both with the list of sections. Without `--section` the tool uses
  `default_section`, or the only section if there is just one; with several sections and no
  `default_section` it is refused (exit 1, `UNKNOWN_SECTION`) with the list of sections. Fields in
  `required_fields` (default `priority`) must be given; values must be in the configured lists. A
  title equal to that of a task that isn't done or dropped is refused.
- **R-WRITE-2 MUST** `claim <task>` claims a task (R-CLAIM-1 to R-CLAIM-4).
- **R-WRITE-3 MUST** `release <task>` gives a task back: a claimed task becomes open; a blocked task
  stays blocked. The owner and claim time are removed. Only the owner may release, or another agent
  with `--force`.
- **R-WRITE-4 MUST** `done <task> [--note T] [--commit HASH|HEAD]` marks the task done, makes the
  agent its owner, and adds the note `done — T`. `--commit` goes into the `commit` field; when git is
  available, the commit must exist and `HEAD` is replaced by the current commit's hash (asked before
  the lock is taken). The output repeats the task's `accept` line as a reminder.
- **R-WRITE-5 MUST** `block <task> --note T` marks the task blocked, with the reason as a note.
  `unblock <task> [--note T]` makes it claimed again if it has an owner (refreshing the claim time),
  open otherwise.
- **R-WRITE-6 MUST** `drop <task> --note T` marks a task that won't be done as dropped, with the
  reason as a note.
- **R-WRITE-7 MUST** `note <task> <text>` (or `--text T`) adds a dated note. Anyone may add notes,
  to any task in any status.
- **R-WRITE-8 MUST** `set <task>` changes `--title`, `--priority`, `--type`, `--area`, `--depends`,
  `--files`, `--problem`, `--fix` and `--accept`, any other field with `--field key=value`, and
  removes a field with `--unset key`. `status`, `owner` and `claimed` change only through the status
  commands.
- **R-WRITE-9 MUST** Status changes follow this table. Anything else is refused (exit 1,
  `BAD_TRANSITION`) with the current status and the commands allowed from it.

  | Current | `claim` | `release` | `block` | `unblock` | `done` | `drop` | `reopen` |
  |---|---|---|---|---|---|---|---|
  | `open` | → claimed | refused | → blocked | refused | → done | → dropped | refused |
  | `claimed` | owner: refreshes the claim; others: refused unless stale | → open | → blocked | refused | → done | → dropped | refused |
  | `blocked` | only with `--force` | stays blocked, owner removed | refused | → claimed or open | only with `--force` | → dropped | refused |
  | `done` | refused | refused | refused | refused | refused | refused | → open |
  | `dropped` | refused | refused | refused | refused | refused | refused | → open |

  The `claimed` time is kept while a task is claimed or blocked, and removed when it is released,
  done, dropped or reopened. Ownership rules (R-CLAIM-3) apply on top, and milestones have rules of
  their own (R-MILE-2).
- **R-WRITE-10 SHOULD** `reopen <task> --note T` puts a done or dropped task back to open.
  `move <task> --section S` moves a task with all its lines to the end of another section.
- **R-WRITE-11 SHOULD** `ask <question>` adds a task of type `question` to the section named by
  `question_section` (default `Questions`, added at the end of the file if missing). `next` never
  hands out questions. `answer <task> <text>` adds the answer as a note and marks the question done.
- **R-WRITE-12 SHOULD** Every text option also accepts `-` (read standard input) or `@path` (read a
  file), so text with quotes, backticks or `$` arrives intact from PowerShell.
- **R-WRITE-13 SHOULD** `add --from <file|->` adds several tasks given as JSON lines in one locked
  step — all or none — and prints their IDs. An entry may name itself with `"ref"`, and other entries
  can depend on it with `"@ref"`.
- **R-WRITE-14 SHOULD** Commands that change a lot at once — `archive`, `restore`, `agents install`,
  `hooks install`, `skill install` — show what they would do and only do it with `--yes`.
- **R-WRITE-15 COULD** `apply <file|->` runs several operations given as JSON lines in one locked
  step: all apply or none do, with one snapshot.
- **R-WRITE-16 COULD** `done` (and `apply`'s `done` operation) records the person behind the work
  in a `by` field, so a task shows which contributor finished it, not only which agent.
  - With `--commit`, the name is the author of that commit as git shows it with `.mailmap` applied
    (`git log -1 --format=%aN <commit>`). Without `--commit`, it is `git config user.name` with
    `user.email`, mapped through `.mailmap` (`git check-mailmap`).
  - A `.mailmap` file at the repository root (git's own format, committed and shared) is how a team
    gives every contributor one name, for example their GitHub username, however their computers
    are set up. No email address is ever written to `TODO.md`.
  - When git isn't on `PATH`, the folder isn't in a repository, or no name is set, `by` is left out
    and `done` works as before (NFR-5). An existing `by` line is replaced; `reopen` removes it.
  - `list --by <name>` lists tasks whose `by` equals the name (ignoring case); it implies
    `--status all`.
  - When `.mailmap` exists, `check` warns about every `by` value that isn't a name on the left side
    of `.mailmap` (written before the mapping was added, or a missing entry). The warning doesn't
    change the exit code.
- **R-WRITE-17 COULD** The `why` field says what a task is for, so the agent that takes it knows the
  point of it. `add --why T`, `set --why T` and `"why"` in `add --from` and `apply` write
  `- why: <text>`, which comes before `problem` in the field order (format.md §10); `-` and `@path`
  work (R-WRITE-12). `show --json` and `schema` list it. A `why` field written by hand before v0.6
  was an unknown field and is now read as this field, with nothing to change.
- **R-WRITE-18 COULD** The `difficulty` field says how hard a task is, for choosing who takes it: a
  small model takes the easy tasks and leaves the hard ones to a stronger one. `priority` says how
  urgent a task is, not how hard.
  - `add --difficulty D`, `set --difficulty D` and `"difficulty"` in `add --from` and `apply` write
    it into the combined priority line, like `type` and `area` (format.md §8).
  - Allowed values come from `difficulties` in `todo.toml` (default `easy`, `medium`, `hard`,
    easiest first); anything else is refused (exit 1, `BAD_VALUE`).
  - `list --difficulty <list>` filters. `next --difficulty <list>` hands out only tasks with one of
    those values, and `next --max-difficulty D` only tasks of difficulty D or easier, in the order
    of `difficulties`. A task without a difficulty is handed out only when no difficulty option is
    given.
  - `tree` and `list` show it; `show --json` and `schema` list it.
  - It is a plain field: nothing is computed from it, and it doesn't change the order of `next`.

  ```powershell
  todo next --claim --max-difficulty medium --agent small-model-1
  ```

### 5.8 Claims — CLAIM

- **R-CLAIM-1 MUST** `claim <task>` sets `status: claimed`, the owner, and the claim time.
- **R-CLAIM-2 MUST** A task claimed by another agent is refused (exit 1, `ALREADY_CLAIMED`) with the
  owner and the claim's age. Claiming a task one already holds succeeds and refreshes the claim
  time, which is how long-running work keeps its claim.
- **R-CLAIM-3 MUST** On a task claimed by another agent, every command except `note` needs
  `--force`; a forced change adds a note saying who forced it (format.md §9).
- **R-CLAIM-4 MUST** A claim older than `claim_ttl_hours` (default 24) is stale. Another agent may
  then claim the task; the takeover adds a note naming the previous owner and the claim's age. A
  claimed task without a valid claim time counts as stale.
- **R-CLAIM-5 MUST** Every write command needs an agent name, from `--agent` or the `TODO_AGENT`
  environment variable; otherwise it exits 2 (`NO_AGENT`). Names follow format.md §11.
- **R-CLAIM-6 SHOULD** `release --stale` releases every stale claim; `release --all --agent X`
  releases all of agent X's claims (another agent needs `--force`).
- **R-CLAIM-7 COULD** Claims carry the agent session. `claim`, `next --claim` and `unblock` (back to
  claimed) write `- session: <id>`, taken from `--session`, else the `TODO_SESSION` environment
  variable, else `CLAUDE_CODE_SESSION_ID` (Claude Code exports this to every command an agent runs;
  it is the same ID Claude Code passes to hooks as `session_id`). Without any of them, no session
  field is written. `release --session <id>` releases every claim of that session, adding the note
  `released — session <id> ended`. `todo hooks install` also adds a Claude Code `SessionEnd` hook
  running a hidden command that reads the hook's JSON from standard input and releases that
  session's claims; `hooks uninstall` removes it. Hook commands must work from PowerShell.

### 5.9 Dependencies — DEP

- **R-DEP-1 MUST** `depends: T-003, T-007` names tasks that must be finished first. A task is ready
  only when each of them is done or dropped. A dependency on an ID that doesn't exist keeps the task
  waiting. Whether a task is waiting is worked out when reading and never written into the file.
  The milestones above a task add their own dependencies (R-MILE-6).
- **R-DEP-2 MUST** `show` lists the task's dependencies with their status, and the tasks waiting on
  it.
- **R-DEP-3 SHOULD** `check` reports dependencies on missing IDs, dependencies on dropped tasks (the
  waiting task may need rethinking), and cycles.
- **R-DEP-4 COULD** `done` and `drop` print the tasks that have just become ready.

### 5.10 Output — OUT

- **R-OUT-1 MUST** With `--json`, every command — read, write, or failed — prints exactly one JSON
  object on standard output:
  `{"schema":"todo.<command>.v1","ok":true,"data":…,"warnings":[…]}` or
  `{"schema":"todo.<command>.v1","ok":false,"error":{"code":"…","message":"…"},"warnings":[…]}`.
  Errors may carry extra details, such as `task`, `owner` or `candidates`.
- **R-OUT-2 MUST** Exit codes: `0` done; `1` refused by a rule (claimed by someone else, not allowed
  in this status, no or ambiguous match, problems found by `check`); `2` wrong usage; `3` busy, try
  again (lock or lease held, or the file changed during the write); `4` file problem (not found,
  unsafe to write, can't be written); `5` reserved for "a check couldn't run".
- **R-OUT-3 MUST** Error codes are stable names: `NO_FILE`, `NO_AGENT`, `USAGE`, `NOT_FOUND`,
  `AMBIGUOUS`, `ALREADY_CLAIMED`, `NOT_OWNER`, `BAD_TRANSITION`, `DUPLICATE_TITLE`,
  `UNKNOWN_SECTION`, `AMBIGUOUS_SECTION`, `MISSING_FIELD`, `BAD_VALUE`, `BUSY`, `LEASE_HELD`,
  `CHANGED_DURING_WRITE`, `UNSAFE_FILE`, `READ_ONLY`, `WRITE_FAILED`, `NO_GIT_HISTORY`,
  `TOOL_TOO_OLD` (R-VER-2), `DUPLICATE_PROJECT` (R-PROJ-1). New codes may be added; existing ones
  never change meaning. Each code always exits with the same code: the table in `docs/json.md`
  gives it, `todo schema` reports it, and a test checks both. `BAD_VALUE` exits 1, as R-WRITE-18 and
  R-MILE-1 say for an invalid value.
- **R-OUT-4 MUST** Problems in the file are reported as `TODO.md:123: message`.
- **R-OUT-5 MUST** Results go to standard output, warnings and errors to standard error (with
  `--json`, see R-OUT-1). Messages are plain English.
- **R-OUT-6 MUST** Non-ASCII text (`·`, `—`, Japanese) shows correctly in Windows PowerShell 5 and 7,
  cmd and Windows Terminal; piped output is UTF-8.
- **R-OUT-7 MUST** When the reader of the output goes away (`todo list | Select-Object -First 3`),
  the command ends quietly with exit 0 instead of crashing.
- **R-OUT-8 MUST** `todo` without arguments prints a short overview; `todo help <command>` explains
  one command with examples; `todo help agents` prints the agent rules (R-AGENT-1);
  `todo --version` prints the version.
- **R-OUT-9 COULD** Colour on a terminal only: never when piped, never with `NO_COLOR` set.
- **R-OUT-10 COULD** `todo completions <shell>` prints a tab-completion script for powershell, bash,
  zsh, fish or elvish, generated from the command definitions with `clap_complete`. The README
  explains how to add it to the PowerShell profile
  (`todo completions powershell | Out-String | Invoke-Expression`).
- **R-OUT-11 COULD** `todo help update` (and `help update --json`) tells an agent how to update todo.
  todo itself never connects to the network (NFR-4); the agent runs the commands. It prints:
  - the running version and the full path of the running executable;
  - the repository, https://github.com/reynanTandoyo/todo, and its latest-release page, both taken
    from `repository` in `Cargo.toml`;
  - the steps for Windows, in PowerShell: read the latest version (`gh release view`, or the
    latest-release page); download `todo.exe` and `todo.exe.sha256` from that release; compare the
    SHA-256 (`Get-FileHash`); replace the executable at the path shown, or, when todo isn't
    installed yet, copy it to `%LOCALAPPDATA%\Programs\todo` and add that folder to the user `PATH`
    (REL-2); run `todo --version`; then run `todo migrate --yes` in each project (R-MIG-2);
  - the same steps for Linux, with `todo-linux-x86_64` and `todo-linux-x86_64.sha256`.

  A test checks the printed asset names against the release workflow (REL-6). The `TOOL_TOO_OLD`
  message (R-VER-2), the warning that the tool is behind the file (R-VER-1) and the agent rules
  (R-AGENT-1) all name `todo help update`.

### 5.11 History and recovery — HIST

- **R-HIST-1 MUST** A state folder per task file (layout in §7): inside git,
  `<git common folder>/todo/<path of TODO.md in the repository>/`, shared by all worktrees and never
  committed; outside git, `.todo/` next to `TODO.md`, containing a `.gitignore` with `*`.
- **R-HIST-2 SHOULD** Before each write, the old file is saved as a numbered snapshot, with an index
  line giving number, time, agent, command and hash. The newest 200 are kept (`snapshots_keep`).
  A write refused because `TODO.md` is read-only (R-IO-5) saves no snapshot.
  `history` lists them, `diff <n> [<m>]` shows what changed, and `restore <n> --yes` puts one back
  under the lock; the restore takes its own snapshot first, so it can be undone too.
- **R-HIST-3 SHOULD** After each write, the tool records the file's hash. If the next command finds
  a different hash, the file was edited outside the tool: that version is saved as a snapshot marked
  "outside edit", a warning is shown once, and the command continues.
- **R-HIST-4 COULD** A journal (JSON lines) records every write: time, agent, command, arguments,
  tasks, and the hash before and after.

### 5.12 Checking — CHECK

- **R-CHECK-1 SHOULD** `check` reports, as `file:line: message`: everything that blocks writing
  (format.md §13); duplicate IDs; unknown statuses; repeated fields; a claimed task without owner or
  claim time; an open task with an owner; headings that look like tasks but don't parse; the
  dependency problems of R-DEP-3; stale claims; blocked tasks without a `blocked — <reason>` note
  (the reason R-READ-5 and R-REPORT-1 show; T-142); values outside
  the configured lists; leftover temporary files; an outdated rules block (R-AGENT-3). Exit 0 if
  clean, 1 if anything was found.
- **R-CHECK-2 COULD** `check --fix` makes mechanical repairs only — canonical IDs and separators, a
  missing `status: open`, leftover temporary files, an outdated rules block — and reports the rest.

### 5.13 Archive — ARCH

- **R-ARCH-1 SHOULD** `archive [--older-than DAYS] --yes` moves done and dropped tasks into
  `TODO-archive.md` (format.md §14), under the same lock. `show`, `list --archive`,
  `search --archive` and dependency checks still find archived tasks. A read-only `TODO.md` is
  refused with exit 4 before the archive is written. If `TODO.md` still cannot be replaced after
  the archive was written (another program changed it, or the rename fails), the archive is put
  back as it was (a new archive is removed), so a moved task is never in both files; the error says
  so if the archive itself cannot be put back.
- **R-ARCH-2 COULD** `archive` keeps milestone trees whole (R-MILE-1). An entry that has a milestone
  is moved only when its top-level milestone is done or dropped and nothing in that tree is still
  open, claimed or blocked; then the whole tree (the top-level milestone and every entry below it,
  at any depth) moves in the same step. Entries without a milestone and without entries below them
  are archived as before. An entry whose milestone chain does not reach a top-level milestone (its
  parent is missing or is not a milestone, or the chain loops) stays in `TODO.md`; so does a tree
  that an entry outside it still names, and a finished entry that other entries name as their
  milestone while it is not a milestone. `--older-than` is measured on the top-level milestone. The
  preview and the result say how many finished entries were kept back, and why. So every milestone still in
  `TODO.md` has all its entries there and its progress count is right. `show`, `tree --all` and
  `list --archive --milestone` find an archived tree.

### 5.14 Settings — CFG

- **R-CFG-1 MUST** Everything works without a settings file.
- **R-CFG-2 SHOULD** An optional `todo.toml` next to `TODO.md` can set:

  | Key | Default | Meaning |
  |---|---|---|
  | `id_prefix` | `"T"` | prefix of task IDs |
  | `id_width` | `3` | digits in new IDs |
  | `priorities` | `["P0", "P1", "P2", "P3"]` | allowed priorities, most urgent first |
  | `required_fields` | `["priority"]` | fields `add` requires |
  | `types` | `[]` | allowed types; empty allows any |
  | `areas` | `[]` | allowed areas; empty allows any |
  | `default_section` | `""` | section for `add` without `--section` |
  | `question_section` | `"Questions"` | section for `ask` |
  | `idea_section` | `"Ideas"` | section for `idea` (R-KIND-1) |
  | `difficulties` | `["easy", "medium", "hard"]` | allowed difficulties, easiest first (R-WRITE-18) |
  | `archive_file` | `"TODO-archive.md"` | archive file name |
  | `claim_ttl_hours` | `24` | when a claim becomes stale |
  | `lock_wait_seconds` | `30` | how long a write waits for the lock or a lease |
  | `lease_minutes` | `10` | default length of `todo lock` |
  | `lease_max_minutes` | `60` | longest allowed lease |
  | `snapshots_keep` | `200` | snapshots kept |
  | `worktree` | `"shared"` | `"local"` makes each worktree use its own file |

  An unknown key or a bad value is an error, reported as `todo.toml:<line>: message`.
- **R-CFG-3 MUST** Settings come from the command line first, then environment variables
  (`TODO_FILE`, `TODO_AGENT`, `NO_COLOR`), then `todo.toml`, then the defaults. `TODO_REGISTER`
  names the project register (R-PROJ-1), which isn't a setting of any one project.

### 5.15 Agents — AGENT

- **R-AGENT-1 MUST** One rules text, built into the program, tells agents how to work with it: never
  edit `TODO.md` directly; use one unique agent name per agent (`--agent` or `TODO_AGENT`); start
  with `summary` or `next`; claim before working (`next --claim` does both); exit 1 means someone
  else has it, so pick another task; exit 3 means try again; add notes as you learn things; finish
  with `done --note … --commit …`; `add` problems you find instead of fixing them silently; `ask`
  what only a person can decide; start commit messages with the task ID (`T-012: …`).
  From v0.6 it also says: put each new task under a milestone (`--milestone`) with a `--why`; read
  the goals of the milestones above a task before starting it; close a milestone with `done` when
  its last task is finished; record ideas with `idea`, and questions about an entry with
  `ask --about`; run `todo migrate --yes` after todo is updated; and when todo is too old for a
  file, follow `todo help update`. `todo help agents` prints it.
- **R-AGENT-2 SHOULD** `todo init` creates `TODO.md` with the format line, the tool line (R-VER-1),
  a title, the rules text between the managed-block markers (format.md §12) and a `## Tasks`
  section, and refuses if `TODO.md` already exists. It then prints the lines to add to `AGENTS.md`,
  and may suggest `todo project add`, but never registers the project itself (R-PROJ-1).
- **R-AGENT-3 SHOULD** `agents install [--into FILE] --yes` writes or updates the rules block in
  `AGENTS.md` (default), `CLAUDE.md` or another file, leaving the rest of the file byte for byte;
  `agents check` reports a missing or outdated block.
  - The project folder is the folder that holds `TODO.md`, found as `check` finds it, whatever the
    current folder is. `agents check`, `agents install`, `skill install` (R-AGENT-5) and
    `hooks install` (R-AGENT-4) all use it. `--into FILE` is relative to it; an absolute path is
    used as given. Without a `TODO.md` they stop with the not-found error (`todo init` creates one).
- **R-AGENT-4 COULD** `hooks install --yes` adds three Claude Code hooks to
  `.claude/settings.json` in the project folder (R-AGENT-3): at session start, `todo summary` (at most 40 lines, within 2 seconds);
  before Edit, Write or MultiEdit, `todo guard`, which stops direct edits of `TODO.md` and the
  archive file and names the command to use instead (direct edits are allowed while a lease is
  live); at session end, `todo session-end`, which releases the claims of the ending session
  (R-CLAIM-7). It keeps a backup; it changes only the managed entries and keeps every other byte of
  the file, key order and inline formatting included (as `migrate --yes` does, R-MIG-2). It marks
  its entries so `hooks uninstall` removes only those, and `hooks uninstall` keeps every other byte
  the same way; uninstall does not create the file when there is none. It works from PowerShell.
- **R-AGENT-5 COULD** `skill install --yes` writes `.claude/skills/todo/SKILL.md` in the project
  folder (R-AGENT-3), from the same rules text.

### 5.16 Import — IMP

- **R-IMP-1 COULD** `todo import --from checkbox <source> [--out <path>] [--yes]` converts a Markdown
  checkbox task list into the current format version (2, format.md §18). Without `--yes` it prints the converted text (or a diff
  when the target exists) and writes nothing.
  - Only top-level checkbox lines (`- [ ]`, `- [~]`, `- [!]`, `- [x]`) under `##` sections become
    tasks. Checkboxes in the preamble, in code fences, or indented stay as text.
  - Mark to status: space = open, `~` = claimed, `!` = blocked, `x` = done. A blocked task without a
    block-reason note (`blocked — <reason>`, `docs/format.md` §9) gets the note
    `blocked — imported from a checkbox list`, so `check` finds a reason (T-142).
  - Title: the bold part (`**…**`) if present, otherwise the text before ` — `. The rest of the line
    goes into a `description` field. Trailing `(added: agent, date)`, `(claimed: agent, date)` and
    `(done: agent, date[, commit])` become the added note; owner and claim time; and the done note
    plus the `commit` field.
  - Indented sub-bullets of the form `  - YYYY-MM-DD agent: text` become notes; other indented lines
    stay as text inside the task.
  - Tasks in a section whose name contains "decision" or "question" (any case) get `type: question`,
    so `next` skips them.
  - IDs are `T-001`, `T-002`, … in file order. Every other line (title, preamble, section headings,
    prose) is kept byte for byte, and a format line is added at the top.
  - The result must pass `todo check`, and the task count must equal the number of converted
    checkbox lines; the command reports both.
  - `--out` defaults to the source file itself (converting in place). When the target exists, the old
    file is first saved next to it as `<name>.pre-import`.

### 5.17 Git — GIT

- **R-GIT-1 COULD** `align` compares `TODO.md` with the git history of the current branch and only
  reports; it never changes the file (see §13). It reads every commit reachable from `HEAD`
  (`git log`) and links a commit to a task when the commit subject starts with the task's ID
  followed by `:` or a space (`T-012: …`; the prefix from `todo.toml`, any case). It reports three
  kinds of finding:
  - `COMMITTED_NOT_DONE`: an `open`, `claimed` or `blocked` task has at least one linked commit
    (lists the commits);
  - `DONE_WITHOUT_COMMIT`: a `done` task has no `commit` field and no linked commit;
  - `COMMIT_MISSING`: a `done` task's `commit` field names a commit that doesn't exist.
  `dropped` tasks and questions are ignored. Exit `0` when there are no findings, `1` when there
  are (like `check`), and `5` with error code `NO_GIT_HISTORY` when git isn't on `PATH`, the folder
  isn't in a git repository, or the repository is shallow or has no commits. Output is one line
  per finding, `TODO.md:<line>: T-012 <KIND> (<commits>)`; with `--json`, `data.findings` is a list of
  `{task, line, kind, commits}`.
- **R-GIT-2 COULD** `merge-driver run <base> <ours> <theirs> [<path>]` is a git merge driver for
  `TODO.md` (git calls it with `%O %A %B %P`). It merges task by task and writes the result into
  `<ours>`; exit `0` means merged cleanly, any other exit means conflicts are left in `<ours>`. It
  never takes the lock and never touches the real `TODO.md`.
  - Tasks are matched by ID. A task changed on one side only takes that side's block. A task
    changed on both sides is merged line by line: notes added on either side are all kept (ours
    first, then theirs); any other field set differently on both sides is a conflict.
  - A task added on one side is kept at the position it was added (after the same preceding line).
    The same ID added on both sides with different content is a conflict; the driver never
    renumbers.
  - A task removed on one side (for example archived) is removed when the other side left it
    unchanged, and is a conflict otherwise.
  - A conflict is written as git-style markers (`<<<<<<< ours` / `=======` / `>>>>>>> theirs`)
    around the whole task block, and the driver exits `1`.
  - Lines outside tasks (format line, preamble, section headings, prose) are taken from the side
    that changed them. When both sides changed them differently, when any input fails to parse,
    or when the format versions differ, the driver falls back to a plain three-way line merge
    (`git merge-file`) of the whole file, which is what git would have done without it.
  - Every output line is copied byte for byte from one of the inputs, line endings included
    (R-EDIT-1, D18). The one exception: a last line without a line ending that is no longer last
    gets one.
  - Task IDs are recognised with the default prefix `T`; with another prefix the file is merged as
    prose (`git merge-file`).
  - The tool line (R-VER-1) changed differently on both sides is not a conflict: the result takes
    the line with the higher version.
- **R-GIT-3 COULD** `merge-driver install [--path <pattern>]` registers the driver for the
  repository: `git config merge.todo.name` and
  `merge.todo.driver "todo merge-driver run %O %A %B %P"`, and a `<pattern> merge=todo` line
  (default `TODO.md`) in the `.gitattributes` at the repository root (added once, other lines kept
  byte for byte). `merge-driver uninstall` removes both. Both are safe to run twice.

### 5.18 Discovery — DISC

- **R-DISC-1 COULD** `scan [<path>…]` finds `TODO:` and `FIXME:` comments in source files and offers
  them as tasks.
  - Files: those listed by `git ls-files` (tracked and untracked but not ignored) when git is
    available, otherwise a walk of the folder that skips hidden folders, `target` and
    `node_modules`. `TODO.md`, `TODO-archive.md`, files over 1 MB and files that aren't valid UTF-8
    are skipped. Paths default to the folder that holds `TODO.md`.
  - A match is the marker (`TODO:` or `FIXME:`, case-sensitive, not part of a longer word, not
    directly wrapped in `` ` ``, `"` or `'`, which marks text about markers) and
    the rest of the line after it, trimmed and with a trailing `*/`, `-->` or `#}` removed; an empty
    rest is skipped.
  - A match is already known when it names a task ID (`TODO(T-012):`, `TODO: T-012 …`) or its text
    equals an existing task title (any case); known matches are not offered.
  - Without `--add` it only reports `path:line: text` per new match (exit `0`). With `--add` it adds
    every new match as an open task in one locked write, like `add --from` (R-WRITE-13): the text
    after the marker is the title (cut to 120 characters), `files: <path>:<line>`, plus `--section` and `--priority`
    when given. `--dry-run` works with `--add`.

### 5.19 Schema — SCHEMA

- **R-SCHEMA-1 COULD** `schema` prints a machine-readable description of the tool as JSON: the tool
  version; the JSON envelope (R-OUT-1); every exit code and error code with its meaning
  (R-OUT-2, R-OUT-3); every command with its arguments and options (name, value name, required,
  repeatable, help text) and its schema name `todo.<command>.v1`; and the fields of a task as
  `show --json` returns them. Commands and options are generated from the command definitions, so
  they can't fall out of date. `schema <command>` prints only that command. With `--json` the
  description is the `data` of a `todo.schema.v1` envelope.

### 5.20 Milestones — MILE

A milestone is a piece of work with a goal, divided into tasks: for example the milestone "Develop
the renderer", with the tasks "C shader" and "GPU compiler". A milestone is never a release or
version label such as "v0.6". Milestones nest to any depth, and todo manages the tree.

```powershell
todo add --type milestone --title "Develop the renderer" --goal "the test scene renders with lighting at 60 frames per second" --priority P1 --agent planner
todo add --title "C shader" --milestone T-010 --why "the renderer has no shading stage yet" --priority P1 --difficulty hard --agent planner
todo add --title "GPU compiler" --milestone T-010 --priority P1 --agent planner
todo tree T-010
```

- **R-MILE-1 COULD** A milestone is an entry with `type: milestone` and a `goal` field
  (`add --type milestone --goal T`, `set --goal T`); `add` refuses a milestone without a goal
  (exit 1, `MISSING_FIELD`).
  - Tasks and milestones name their parent milestone with `milestone: <ID>`: `--milestone <task>`
    on `add`, `set`, `add --from` (`"milestone"`, which may also be an `"@ref"`) and `apply`. A
    milestone may have a parent milestone, so milestones nest to any depth. Questions and ideas
    refer to an entry with `about` instead (R-KIND-1, R-KIND-2).
  - The tree is logical: entries stay where they are in the file, in any section, and only the
    `milestone` field links them (format.md §17).
  - `<task>` may be an ID or a title, as elsewhere; the file stores the ID. An empty value is
    refused (`BAD_VALUE`); `set --unset milestone` makes an entry top-level.
  - Refused with exit 1, leaving the file byte-identical: a parent that doesn't exist
    (`NOT_FOUND`); a parent that isn't a milestone (`BAD_VALUE`); a change that would make a
    milestone its own ancestor (`BAD_VALUE`); `set --type` that turns a milestone with entries
    below it into another kind (`BAD_VALUE`); a question or idea that names a milestone
    (`BAD_VALUE`).
- **R-MILE-2 COULD** Milestones in the status rules (R-WRITE-9):
  - A milestone is open, blocked, done or dropped, never claimed. `next` and `list --ready` never
    return one, and `claim` of one is refused (exit 1, `BAD_TRANSITION`).
  - `done` on a milestone is refused (exit 1, `BAD_TRANSITION`, listing them) while any entry below
    it, at any depth, is open, claimed or blocked. `--force` overrides this and adds a forced note
    (format.md §9). `drop` without `--tree` is refused in the same case and names `drop --tree`.
  - `drop --tree <milestone> --note T` drops the milestone and every entry below it that isn't done
    or dropped, in one locked step; the claim rules (R-CLAIM-3) apply to each entry, so another
    agent's claim needs `--force`. `--tree` on a task is refused (`BAD_VALUE`).
  - `set --milestone <task>` moves an entry, with everything below it, to another parent; only the
    entry's own `milestone` line changes. `set --unset milestone` makes it top-level.
  - `show --json` returns `goal`, `milestone`, the chain of parents (`milestones`, R-MILE-4) and the
    direct children (`children`: ID, title, kind and status of each).
- **R-MILE-3 COULD** Seeing the tree:
  - `todo tree [<milestone>]` prints milestones and their tasks, indented by depth: each milestone
    with its goal and its progress, and the area next to every entry that has one. Top-level tasks
    without a milestone come last. Done and dropped entries are hidden unless `--all`.
    `tree --area <list>` shows only the tasks in those areas, with the milestones above them.
    `tree --json` is nested. `tree <milestone>` always shows that milestone, even when it is done,
    with what is below it. Without a milestone or `--area`, the open questions and ideas follow the
    tree, each in its own list (R-KIND-3).
  - Progress is `done/total`, counted over every task below the milestone at any depth: `done`
    counts done tasks, `total` every task that isn't dropped. Milestones, questions and ideas
    aren't counted. `summary`, `overview` and the HTML report use the same count.
  - `list --milestone <task>` lists every entry below that milestone, at any depth; a task that is
    not a milestone is refused (`BAD_VALUE`).
    `list --kind <list>` filters by kind (`milestone`, `task`, `question`, `idea`; R-KIND-3).
  - `summary` shows the top-level milestones that are open or blocked (not done or dropped) with their
    progress, and stays within 40 lines.
- **R-MILE-4 COULD** An entry that has a milestone shows where it belongs. `show`, `next`, `claim`
  and `next --claim` print, after the task, the chain of milestones from the top down, each with ID,
  title and goal; their JSON carries `milestones: [{id, title, goal, status}]` in the same order.
  In text, `show` prints the chain after the questions about the entry (R-KIND-2), and `next`,
  `claim` and `next --claim` print it under the task's line (`claim` after `<ID>: claim ok`). The
  chain starts with `Milestones above <ID>, from the top down:`; each milestone takes two lines,
  `- <ID> · <title>` and `  goal: <goal>` (`goal: -` when it has none). Entries without a milestone
  print exactly what they printed before.
- **R-MILE-5 COULD** `check` reports, each with its line: a `milestone` field naming a missing ID, or
  an entry that isn't a milestone; a cycle of milestones; a done or dropped milestone with an entry
  below it still open, claimed or blocked; a milestone without a goal; an `about` field naming a
  missing ID (R-KIND-2). An ID is missing only when neither `TODO.md` nor `TODO-archive.md` holds it
  (R-DEP-3). The line is the one that holds the problem: the `milestone` or `about` line for a bad
  reference; the `milestone` line of the entry that closes a cycle; the heading of a milestone without
  a goal; the heading of a done or dropped milestone with unfinished entries below it (the message
  lists them); and the heading of an unfinished entry whose milestone is archived as done or dropped
  (R-ARCH-2).
- **R-MILE-6 COULD** Work is handed out along the tree:
  - A task is ready only when its own dependencies and those of every milestone above it are
    finished (R-DEP-1), and no milestone above it is blocked, done or dropped.
  - `depends` may name a milestone, which counts as finished only when the milestone itself is done
    or dropped. `check` finds dependency cycles that run through milestones (R-DEP-3). The waiting
    edges are: a task waits for the `depends` of every milestone above it (a milestone is never
    handed out, so it does not), and a milestone waits for the entries directly below it, since it
    is finished only when they are. A cycle is reported only when it contains a `depends` edge; a
    loop made of milestone parents alone is a milestone problem (R-MILE-5).
  - `next --milestone <task>` returns only tasks below that milestone, at any depth.
  - Among ready tasks, `next` orders by priority; then tasks in a milestone where work has already
    started (something below it at any depth is claimed, a stale claim included, or done) before
    tasks in untouched milestones and tasks outside any milestone; then file order. Parallel
    `next --claim` still hands out different tasks (TEST-5).
  - A milestone is ready to close when it is open or blocked, has entries below it, and every one of
    them is done or dropped. When `done` or `drop` finishes the last unfinished entry below a
    milestone, the output names the milestone as ready to close, with the command to use
    (`todo done <milestone>`); the JSON carries it as `ready_to_close`. `summary` lists milestones
    that are ready to close. A milestone is never closed automatically.

### 5.21 Kinds of entry — KIND

- **R-KIND-1 COULD** Ideas are kept without being handed out as work.
  `todo idea <text> [--about <task>]` adds an entry with `type: idea` and the note `added` to the
  section named by `idea_section` (default `Ideas`, added at the end of the file if missing). It
  needs no priority, whatever `required_fields` says. A title equal to that of an active task is
  refused (`DUPLICATE_TITLE`, as `add` and `ask` do), and an `--about` ID that doesn't exist is
  refused (exit 1, `NOT_FOUND`, as `ask --about` is, R-KIND-2); the file is left unchanged.
  `next` never returns an idea, not even one already claimed by the asking agent. `summary` counts
  the ideas that are not done or dropped apart from the work: they are left out of `total` and
  the status and priority counts, shown as `ideas: N` in text and as `counts.ideas` in JSON.
  `promote <idea> --priority P [--milestone <task>] [--section S]` turns an idea into an ordinary
  task: `--priority` is required and must be a configured priority; it removes `type: idea`, sets
  the priority and the milestone (which must name a milestone: `NOT_FOUND` when missing,
  `BAD_VALUE` when it names another kind of entry), keeps `about`, and adds the note
  `promoted from idea` (format.md §9). The status and owner don't change, and the ownership rule of
  `set` applies (R-CLAIM-3). `--section S` moves the entry to the end of section S in the same
  write, as `move` does; without it the entry stays where it is. `promote` of an entry that isn't
  an idea is refused (exit 1, `BAD_TRANSITION`) and the file is left unchanged.
- **R-KIND-2 COULD** Questions can refer to the entry they are about.
  `ask <question> --about <task>` writes `- about: <ID>`, and is refused (exit 1, `NOT_FOUND`) when
  that ID doesn't exist in `TODO.md` or `TODO-archive.md`. `--about` takes an ID (`T-002`, `t2`),
  written in canonical form; any other value is refused with `BAD_VALUE`, and the file is left
  alone. `show` of a milestone or task lists the questions about it, from both files, after the
  block in text (`Questions about <ID>:`, one line per question with its status, and `(archived)`
  for one in the archive) and in JSON as `questions`. `answer` prints the entry the question was
  about, after the result: in text the entry's block under `About <ID>:`; in JSON, under `about`,
  as its task JSON (only when the question has an `about` field). `list --where about=<ID>` works.
  A question without `--about` behaves as before.
- **R-KIND-3 COULD** Every entry has one kind, taken from its `type`: `milestone` is a milestone,
  `question` a question, `idea` an idea, and any other type, or none, a task (format.md §16).
  Milestones and tasks are work; questions and ideas are not. Commands keep the kinds apart: only
  tasks are handed out by `next` and counted as open work; `summary`, `tree`, `overview` and the HTML
  report show questions, ideas, done entries and dropped entries apart from open work; `summary`'s
  status counts and `total` count tasks only, and its `counts.questions` and `counts.ideas` count the
  open questions and ideas apart; and `list --kind <list>` filters by kind.

### 5.22 Versions and migration — VER, MIG

- **R-VER-1 COULD** `TODO.md` records the highest todo version that has written it, in the tool line
  `<!-- todo-tool: 0.6.0 -->` directly after the format line, or as the first line when the file has
  no format line (format.md §4).
  - `init` writes it. A write command adds it when it is missing, or raises it when its own version
    is higher, in the same write, and never lowers it; only a command that writes the file changes it
    (R-EDIT-6). Read commands never write it.
  - A value that is not `MAJOR.MINOR.PATCH` is kept as it is and reported as a warning.
  - Versions are compared number by number, so `0.10.0` is above `0.9.0`. A file without the line
    counts as written by todo 0.5.0 or older.
  - An older tool on a file stamped newer, in a format it knows, still works, and warns on standard
    error that it is behind, naming `todo help update` (R-OUT-11).
  - `where`, in text and JSON, shows the stamp and the running version (R-FIND-7).
  - The archive file gets the same line. The merge driver takes the higher stamp when both sides
    changed the line (R-GIT-2).
- **R-VER-2 COULD** A tool refuses a format it doesn't know. When the format line names a version
  the running tool doesn't know, every command that reads or writes tasks exits 4 with the error
  `TOOL_TOO_OLD`; the message names the todo version stamped in the file and the running version,
  and says to update the todo executable first (`todo help update`). Nothing is written. Commands
  that interpret or change task content refuse; the ones that never read it (`status`, `history`,
  `diff`, `lock`, `unlock`, `agents`, `skill`, `hooks`, `guard`, `session-end`, `merge-driver`) keep
  working, and so do `help`, `schema`, `completions` and `--version`. `where` keeps working: it
  shows the format version, the tool stamp and the running version, and counts no tasks. The
  per-user commands (`project`, `overview`, `report --html --all`) never open a project's tasks to
  write them; they show such a project with this problem and carry on with the others. A file in a
  known format with a higher stamp only warns (R-VER-1). (todo 0.5 and older read a newer format with a warning but
  refuse to write it; this rule starts with 0.6.)
- **R-MIG-1 COULD** After todo is updated, each `TODO.md` moves to the new format by itself, on the
  first write.
  - Migration steps are an ordered list in the code, each from one format version to the next, and
    each changes whole lines only (R-EDIT-1). The first step, format 1 to 2, raises the format line
    (adding it at the top of the file when it is missing) and adds the tool line; task lines stay as
    they are (format.md §18).
  - A write command on a file in an older format first applies every pending step, under the same
    lock (R-LOCK-1) and in the same write, after the usual snapshot (R-HIST-2), then makes its own
    change; the format line and the stamp are raised. The archive file is migrated in the same step,
    and if `TODO.md` then cannot be replaced, the migrated archive is put back as it was (R-ARCH-1).
  - The migrated text must parse without problems that block writing (format.md §13), the
    file-wide ones; a task problem keeps to the command that changes that task, as for any write.
    Otherwise nothing is written and the command exits 4. One line on standard error and one journal
    entry (R-HIST-4, with a `migrated` field such as `TODO.md: format 1 to 2`) say what was migrated.
  - A write that changes nothing writes nothing, so it migrates nothing (R-EDIT-6). New files (`init`,
    `import`, a new archive) are written in the current format.
  - `todo migrate` does the same without another change, prints one line (or, with `--dry-run`, the
    diff of `TODO.md` and of the archive), and a second run changes nothing.
  - Read commands read every older format and never write.
  - `restore` of a snapshot from before the migration works; the next write migrates it again.
  - Tests use LF, CRLF and byte-order-mark fixtures, with every byte outside the changed lines
    identical, and test a two-step chain with a test-only step behind the `test-hooks` feature.
- **R-MIG-2 COULD** `todo migrate` also brings the installed project files up to date.
  - It lists what is out of date in the project: the rules block in every file that has one
    (R-AGENT-3), the Claude Code skill (R-AGENT-5) and the Claude Code hooks (R-AGENT-4). Out of
    date means different from what the running tool would write, as `agents check` decides; it
    doesn't depend on the stamp, which the first write after an update has already raised.
  - The project files are those in the folder of `TODO.md`: `TODO.md` itself, `AGENTS.md` and
    `CLAUDE.md` (the files `todo check` reads), `.claude/skills/todo/SKILL.md` and
    `.claude/settings.json`. Another file named with `agents install --into` is not found.
  - A rules block counts as installed when its markers are there. The skill is compared with line
    endings ignored, as `agents check` does. A hook event counts as installed when its managed
    entry is there, and a missing event is not added (`hooks install` adds it).
  - `todo migrate --yes` updates exactly those that are already installed and never installs one
    that wasn't (R-WRITE-14). Each file changes only inside the parts todo owns: the lines between
    the markers, the whole skill file, and the managed hook entries, with every other byte kept.
    The settings file is copied to `.claude/settings.json.bak` first, as `hooks install` does.
  - A file that cannot be read or parsed is a warning on standard error, and is left alone.
  - `--dry-run` prints the diffs that `--yes` would write, and writes nothing.
  - `summary` shows one line naming `todo migrate --yes` while anything is out of date, within its
    40 lines. Afterwards `todo check` and `agents check` are clean, and a second run changes
    nothing.

```powershell
todo migrate --dry-run --agent maintainer
todo migrate --yes --agent maintainer
```

### 5.23 HTML report — REPORT

The report is the one view meant for people (§2). It is an overview only: no buttons, no forms, no
copy-ready commands; a person who wants a change asks an agent. It is built only when someone asks
for it, never automatically.

- **R-REPORT-1 COULD** `todo report --html [--out <path>] [--open]` writes one self-contained HTML
  file for the project.
  - CSS is inline; nothing is loaded from the network; no new dependency is needed. The default
    path is `report.html` in the state folder (§7), so it is never committed. The command prints
    the path; `--open` then opens the page with the system's default program for HTML files.
  - It is a read command: it takes no lock (R-LOCK-5) and never changes `TODO.md`.
  - The page keeps the kinds apart in separate lists: (1) work, as a tree of milestones, each with
    its goal and progress (R-MILE-3), with nested milestones and tasks indented below it (for
    example "Develop the renderer" with "C shader" and "GPU compiler" below), and top-level tasks
    last; (2) open questions, each with a link to the milestone or task it is about; (3) ideas;
    (4) done; (5) dropped, with the reason.
  - Each task shows its priority, difficulty and area; claimed tasks show owner and age, with stale
    claims marked; blocked tasks show the reason of their latest `blocked — <reason>` note
    (`docs/format.md` §9). A task opens to show its fields and notes. The page also gives the number
    of open tasks per area: every task that is open, claimed or blocked (that is, not done or
    dropped), counted by its area.
  - Done and dropped entries in `TODO-archive.md` are listed too, marked as archived. A done or
    dropped milestone appears in the work tree only while unfinished work sits below it; otherwise
    it is listed under done or dropped, so each entry appears once.
  - All text from the file is HTML-escaped (tested with a title containing `<script>`).
  - It is readable in light and dark mode and in a narrow window.
  - With a fixed clock, the output for a fixture matches its snapshot.
- **R-REPORT-2 COULD** `todo report --html --all [--out <path>] [--open]` puts every registered
  project on one page, built from the project register only (R-PROJ-1); it never searches folders
  and works from any folder.
  - A card per project (tasks per status, top-level milestones with progress, blocked tasks, open
    questions), followed by each project's lists as in R-REPORT-1.
  - The default path is `report-all.html` next to the register.
  - A project that is gone, can't be parsed or needs a newer todo shows its problem instead of its
    lists.
  - An empty register gives a page that says in words how a project is registered (by asking an
    agent to register it), without a copy-ready command.
  - A snapshot test uses two invented projects.

```powershell
todo report --html --open
todo report --html --all --out "$env:USERPROFILE\Desktop\Alle Projekte.html"
```

### 5.24 Projects — PROJ

todo never searches the PC for task lists. A project is known only when someone registers it.

- **R-PROJ-1 COULD** The project register is one JSON file per user (D11):
  `%LOCALAPPDATA%\todo\projects.json` on Windows, `$XDG_DATA_HOME/todo/projects.json` (else
  `~/.local/share/todo/projects.json`) on Linux; the `TODO_REGISTER` environment variable names
  another file.
  - `todo project add [<path>] [--name N]` registers the `TODO.md` found from that folder (default:
    the current one) by the usual search (R-FIND-3 to R-FIND-5). The name defaults to the name of
    the folder that holds the file. The same file, or the same name, registered twice is refused
    (exit 1, `DUPLICATE_PROJECT`); paths are compared as in R-LOCK-7.
  - `project remove <name|path>` removes an entry (exit 1, `NOT_FOUND`, when there is none) and
    `project list` lists them. Each entry records the name, the absolute path of the `TODO.md` and
    the time it was added.
  - The file is JSON: `{"format": 1, "projects": [{"name": …, "path": …, "added": …}]}`, the projects
    in the order they were added. A file in a newer format is refused with `TOOL_TOO_OLD` (R-VER-2).
    A file that is not a register is refused with `UNSAFE_FILE`; nothing is written in either case.
  - Names are compared ignoring case and surrounding spaces. A `remove` argument is matched as a name
    first; when no name matches, it is taken as the path of a `TODO.md` or of its folder, compared as
    R-LOCK-7 compares paths.
  - The register is written under its own operating-system lock (`<register file>.lock`, waiting as
    in R-LOCK-3: 30 seconds, then exit 3 `BUSY`). Unlike the write lock, the register has no holder
    file, so the message names no holder. The file is replaced in one step (R-IO-1), so parallel
    `project add`s lose nothing. These commands change no `TODO.md` and need no agent name.
  - `project add` finds the `TODO.md` from the folder as the other commands search (R-FIND-3 to
    R-FIND-5, including `worktree = "local"`); `--file` and `TODO_FILE` do not apply. A path that is
    not a folder is refused with `NO_FILE`.
  - `init` may suggest `todo project add`, but never registers a project by itself.
  - Paths with spaces and non-ASCII characters work (NFR-7). Tests point the register at a
    temporary folder.
- **R-PROJ-2 COULD** `todo overview` reads only the register, never searches folders, and works from
  any folder. It prints one line per project: name, path, tasks per status, top-level milestones
  done/total, stale claims, open questions, and the todo version stamped in the file (R-VER-1), so
  projects that still need a migration stand out. A registered path that is gone, can't be parsed
  or needs a newer todo is listed with its problem and doesn't stop the others. `--json` is
  supported.
  - The figures come from each project's `TODO.md` only (not its archive): tasks per status (R-KIND-3),
    the top-level milestones that are done out of those not dropped, the stale claims (R-CLAIM-4) and
    the open questions. A project is marked as needing a migration when its format is older than this
    todo's, or its stamp is missing or older than the running version; `todo migrate --yes` clears it.
  - The JSON schema is `todo.projects.v1`: `todo.overview.v1` is the output of `todo --json` with no
    command. Exit code 0 whenever the register can be read, also when some projects show a problem.
  - A register that is missing or empty gives a line that says so, and no project.

```powershell
todo project add "C:\Users\Example\Projekte\Grafik Engine" --name renderer
todo project list
todo overview
```

---

## 6. Commands at a glance

| Command | What it does | Kind | Priority |
|---|---|---|---|
| `list`, `show`, `next` | read tasks; `next --claim` also claims | read | MUST |
| `add`, `claim`, `release`, `done`, `drop`, `block`, `unblock`, `note`, `set` | change tasks | write | MUST |
| `lock`, `unlock`, `status` | 10-minute lease for hand edits; lock and lease status | lease | MUST |
| `help`, `--version` | help, agent rules, version | — | MUST |
| `where`, `summary`, `check`, `history`, `diff` | file in use, overview, problems, snapshots | read | SHOULD |
| `reopen`, `move`, `ask`, `answer`, `restore`, `archive`, `release --stale/--all`, `add --from` | more changes | write | SHOULD |
| `init`, `agents install`, `agents check` | set up a project | setup | SHOULD |
| `search`, `log` | find text, task history | read | COULD |
| `apply`, `check --fix` | batch changes, repairs | write | COULD |
| `edit` | lease plus editor | lease | COULD |
| `hooks install`, `hooks uninstall`, `guard`, `skill install` | Claude Code extras | setup | COULD |
| `import --from checkbox` | convert a Markdown checkbox list | write | COULD |
| `completions <shell>` | tab-completion script | — | COULD |
| `align` | compare tasks with git history | read | COULD |
| `merge-driver install`, `merge-driver uninstall`, `merge-driver run` | git merge driver for `TODO.md` | setup | COULD |
| `scan`, `scan --add` | find `TODO:` comments in code | read / write | COULD |
| `schema` | machine-readable description of the tool | read | COULD |
| `tree` | the milestone tree with progress | read | COULD |
| `drop --tree` | drop a milestone and everything unfinished below it | write | COULD |
| `idea`, `promote` | record an idea; turn it into a task | write | COULD |
| `migrate` | move `TODO.md` to the current format; update the rules block, skill and hooks | write / setup | COULD |
| `help update` | how to update the todo executable | — | COULD |
| `report --html`, `report --html --all` | the HTML page for people, for one project or all registered ones | read | COULD |
| `project add`, `project remove`, `project list`, `overview` | the per-user project register and an overview of it | register | COULD |

Options used by many commands: `--file`, `--json`, `--agent`, `--dry-run` (write commands), `--yes`
(bulk commands), `--force` (where allowed).

---

## 7. State folder

```
<state folder>/            (R-HIST-1)
  write.lock               target of the operating-system lock; stays empty
  holder.json              who holds the write lock: agent, command, process ID, start time
  lease.json               the manual lease: agent, start, expiry
  ids.json                 highest ID number ever given out
  last-write.json          size, modification time and hash after the tool's last write
  history/index.jsonl      one line per snapshot
  history/000001.md …      snapshots
  journal.jsonl            (COULD, R-HIST-4)
  report.html              the HTML report, when built (R-REPORT-1)
```

The one per-user folder (D11), outside every project:

```
%LOCALAPPDATA%\todo\       ($XDG_DATA_HOME/todo/ or ~/.local/share/todo/ on Linux; R-PROJ-1)
  projects.json            the project register
  projects.json.lock       target of the register's operating-system lock; stays empty
  report-all.html          the report for all projects, when built (R-REPORT-2)
```

With `TODO_REGISTER`, the lock and `report-all.html` sit next to the file it names.

---

## 8. Non-functional requirements — NFR

- **NFR-1 MUST** One `todo.exe` for Windows 10/11 x64 with the C runtime linked in: no runtime to
  install, no administrator rights, runs from any folder on `PATH`.
- **NFR-2 MUST** The code also builds and passes its tests on Linux x64 (CI), to keep it portable.
- **NFR-3 MUST** `list` and `show` on a 600 KB file take under 100 ms; a write takes under 200 ms
  plus any lock wait (measured on a generated 5,000-task file). On a variant of that file with a
  milestone tree four levels deep, `tree`, `next`, `list --milestone` and `show` also take under
  100 ms and a write under 200 ms; `report --html` takes under 500 ms and writes a page under 5 MB;
  `report --html --all` with ten such projects takes under 5 s (release build).
- **NFR-4 MUST** No network access, telemetry, background process, database or installer.
- **NFR-5 MUST** git is optional and used only when it is on `PATH`: by `done` (`git rev-parse`
  for `--commit`, the `by` name of R-WRITE-16), `align` (R-GIT-1), `merge-driver` (R-GIT-2, R-GIT-3) and `scan` (R-DISC-1, file
  list). Everything else, including worktree detection, works without it.
- **NFR-6 MUST** No crashes: every failure ends with a message and an exit code. `unwrap`, `expect`
  and `panic!` are not used outside tests (enforced by Clippy).
- **NFR-7 MUST** Paths with spaces, non-ASCII characters and more than 260 characters work.
- **NFR-8 MUST** Command names and options, JSON schemas, error codes, exit codes and the file format
  are versioned interfaces. A breaking change needs a new major version and an entry in
  `CHANGELOG.md`; the file format has its own version line.
- **NFR-9 SHOULD** Few, well-known dependencies: `clap`, `serde`, `serde_json`, `toml`, `sha2`,
  `similar`, `chrono` and `clap_complete`; for tests `assert_cmd`, `predicates`, `tempfile`, `proptest` and `insta`.
  Anything else needs a reason in its commit message. The release `todo.exe` stays under 5 MB.
- **NFR-10 MUST** Builds with stable Rust 1.89 or newer, edition 2024.
- **NFR-11 SHOULD** The README explains that the lock protects one machine, that folders synced by
  OneDrive or Dropbox aren't safe for parallel agents, and the SmartScreen warning for a self-built
  `.exe`.
- **NFR-12 COULD** `where` and write commands warn when `TODO.md` is inside a OneDrive or Dropbox
  folder.

---

## 9. Testing — TEST

- **TEST-1 MUST** Test fixtures are small, invented, and start with
  `<!-- TEST FIXTURE: invented data, not a real task list -->`. Tests run in temporary folders and
  never touch this repository's own `TODO.md`.
- **TEST-2 MUST** Parser tests cover every rule in format.md: code fences, comments, preamble,
  unknown fields, the combined priority line, notes, CRLF, byte-order mark, no final newline, mixed
  line endings, non-ASCII text, every lenient form, and every refusal in format.md §13.
- **TEST-3 MUST** Round trip: every fixture reads and writes back byte-identical. A property test
  applies random sequences of edits to generated files and checks that bytes outside the edits never
  change, and that a change followed by its reverse restores the original bytes. A deliberately messy
  fixture (CRLF with byte-order mark, code fences, tables, fake tasks, trailing spaces) is included.
- **TEST-4 MUST** Every write command has a test for each successful case (expected file after) and
  each refusal (exit code, error code, and a byte-identical file).
- **TEST-5 MUST** Locking, with separate processes: 20 parallel `add`s lose nothing and get distinct
  IDs; 20 parallel claims of one task have exactly one winner; 10 parallel `next --claim`s get 10
  different tasks; a process killed while holding the lock delays the others by less than
  2 seconds; a held lock makes a command exit 3 without writing; a lease stops other agents but not
  its holder, and expires. A test-only hook, behind the cargo feature `test-hooks`, makes a command
  pause while holding the lock.
- **TEST-6 MUST** File search: from the root and from a subfolder; a repository without `TODO.md`
  under a parent folder that has one (the search must stop at the repository root); `--file`;
  `TODO_FILE`; a real linked worktree created with git in a temporary folder (resolves to the main
  file); `worktree = "local"`; git missing from `PATH`.
- **TEST-7 MUST** Windows: replacing the file while another handle holds it (the write retries and
  succeeds once released); a read-only file is refused; a change by another program during the
  write is detected (R-IO-2).
- **TEST-8 MUST** Command-line tests run the built `todo.exe` with `TODO_FILE` and `TODO_AGENT`
  removed from the environment and `HOME`/`APPDATA` (and, from v0.7, `LOCALAPPDATA`,
  `XDG_DATA_HOME` and `TODO_REGISTER`) pointing at a temporary folder. JSON output is
  checked against snapshots, and a test checks that `todo help` lists every command.
- **TEST-9 MUST** A performance test on a generated 5,000-task file (NFR-3) runs in CI but not in
  every local `cargo test`. The milestone-tree and report limits of NFR-3 are tested the same way.

---

## 10. Build, release, documentation — REL

- **REL-1 MUST** GitHub Actions CI on every push and pull request: `cargo fmt --check`, Clippy with
  warnings as errors, and the tests on Windows and Linux. On Windows it also builds the release
  `todo.exe` and fails if the program needs a DLL that isn't part of Windows, or is 5 MB or larger.
- **REL-2 MUST** The README explains, in plain language: what the tool is; how to build it
  (`cargo build --release`); how to install it (copy `todo.exe` to `%LOCALAPPDATA%\Programs\todo`
  and add that folder to `PATH`, with the PowerShell commands); how to use it (the common commands,
  with examples); how to add it to a project (`todo init`, and the lines for `AGENTS.md`); and its
  limits (NFR-11).
- **REL-3 MUST** MIT licence: a `LICENSE` file, named in the README and in `Cargo.toml`.
- **REL-4 MUST** `docs/format.md` is the reference for the file format; any change to the format
  updates it in the same commit.
- **REL-5 SHOULD** `docs/json.md` documents every JSON shape and error code with an example;
  `CHANGELOG.md` lists the changes in each version.
- **REL-6 SHOULD** Pushing a `v*` tag runs a release workflow: it checks that the tag matches the
  version in `Cargo.toml`, runs the tests, builds `todo.exe` (Windows x64) and the Linux x64 binary,
  and publishes both with SHA-256 checksums on GitHub Releases.
- **REL-7 COULD** `todo --version` also prints the commit the program was built from.

---

## 11. Releases

- **v0.1** — every MUST: the commands `list`, `show`, `next`, `add`, `claim`, `release`, `done`,
  `drop`, `block`, `unblock`, `note`, `set`, `lock`, `unlock` and `status`, with file search,
  worktree support, locking, atomic and verified writes, JSON output, the agent rules, tests, CI,
  README and licence.
- **v0.2** — every SHOULD: `where`, `summary`, `check`, snapshots and restore, outside-edit
  detection, `archive`, questions, bulk add, title matching, `--dry-run`, `todo.toml`, `init`,
  `agents install`, release builds.
- **v0.3** — every COULD: `search`, the journal and `log`, `apply`, `check --fix`, `edit`, Claude
  Code hooks and skill, colour, the synced-folder warning.
- **v0.4** — ideas the owner promoted on 2026-11-16: `import --from checkbox` (R-IMP-1), session-tagged
  claims (R-CLAIM-7) and tab completion (R-OUT-10).
- **v0.5** — ideas the owner promoted on 2026-11-17: `align` (R-GIT-1), the merge driver
  (R-GIT-2, R-GIT-3), `scan` (R-DISC-1) and `schema` (R-SCHEMA-1); plus the `by` field
  (R-WRITE-16).
- **v0.6** — decided by the owner on 2026-11-18: format version 2; the tool version stamp
  (R-VER-1) and refusing a format the tool doesn't know (R-VER-2); migration on the first write and
  `todo migrate` (R-MIG-1, R-MIG-2); milestones as a tree of pieces of work with goals (R-MILE-1 to
  R-MILE-6) and archiving them whole (R-ARCH-2); kinds of entry: ideas, questions about an entry
  (R-KIND-1 to R-KIND-3); the `why` and `difficulty` fields (R-WRITE-17, R-WRITE-18); and
  `todo help update` (R-OUT-11).
- **v0.7** — decided by the owner on 2026-11-18: the HTML report for people (R-REPORT-1), the
  project register and `overview` (R-PROJ-1, R-PROJ-2), and the report for all registered projects
  (R-REPORT-2).

---

## 12. Ideas, not planned

Recorded so they aren't lost; each becomes a requirement only by a decision recorded here.

- **I-1** An MCP server exposing the commands as tools.
  *Dropped by the owner's decision of 2026-11-17: agents already use the CLI with `--json` (D10),
  and D14 rules out an MCP server.*
- **I-2** A list of projects and an overview across them.
  *Dropped by the owner's decision of 2026-11-17: work happens in each project's own session, where
  `todo summary` already shows that project's state. Revived and moved to requirements (R-PROJ-1,
  R-PROJ-2, R-REPORT-2) by the owner's decision of 2026-11-18, without scanning: projects are
  registered by command in a per-user file.*
- **I-3** Claims tagged with an agent session, released by a Claude Code `SessionEnd` hook.
  *Moved to requirements (R-CLAIM-7) by the owner's decision of 2026-11-16.*
- **I-4** `todo schema`: a machine-readable description of commands, JSON shapes and exit codes.
  *Moved to requirements (R-SCHEMA-1) by the owner's decision of 2026-11-17.*
- **I-5** `todo handoff`: a summary note of an agent's open claims for whoever continues.
  *Dropped by the owner's decision of 2026-11-17: `todo note` already does this.*
- **I-6** `todo renumber`: change a task's ID and every reference to it.
  *Dropped by the owner's decision of 2026-11-17: commit messages keep the old ID, so renumbering
  breaks the link to the history.*
- **I-7** A git merge driver that merges `TODO.md` task by task.
  *Moved to requirements (R-GIT-2, R-GIT-3) by the owner's decision of 2026-11-17.*
- **I-8** `todo align`: compare tasks with git history and report likely-finished or unproven tasks
  (exit 5 when the history is missing).
  *Moved to requirements (R-GIT-1) by the owner's decision of 2026-11-17.*
- **I-9** `todo undo`: revert an agent's last change if nothing has happened since.
  *Dropped by the owner's decision of 2026-11-17: `reopen`, `release`, `set` and `restore` already
  reverse any change.*
- **I-10** PowerShell tab completion.
  *Moved to requirements (R-OUT-10) by the owner's decision of 2026-11-16.*
- **I-11** `todo scan`: find `TODO:` comments in source code and offer them as tasks.
  *Moved to requirements (R-DISC-1) by the owner's decision of 2026-11-17.*
- **I-12** Import from other task-list formats (checkbox lists, todo.txt).
  *Checkbox lists moved to requirements (R-IMP-1) by the owner's decision of 2026-11-16; todo.txt
  stays an idea.*
- **I-13** Rewrite the HTML report after every write, and let the page reload itself.
  *Dropped by the owner's decision of 2026-11-18: the report is built only when someone asks for
  it. The ID R-REPORT-3 that was planned for it stays unused.*

---

## 13. Out of scope

- A terminal UI, desktop app or web server. A generated HTML file, written only on request
  (R-REPORT-1, R-REPORT-2), is allowed; any control on that page (buttons, forms, scripts that
  change something, copy-ready commands) stays out, and so does rewriting it automatically.
- Due dates, recurrence and computed priority scores; nothing is computed from `difficulty`.
- A database or index next to the file.
- Other record types (decision records, plans, issues). Milestones, questions and ideas are kinds
  of entry in `TODO.md` (R-KIND-3), not separate records.
- Syncing with GitHub Issues, Jira or other trackers.
- Locking across machines or real-time sync; git handles sharing between machines.
- Changing a task's status automatically, for example from git history (`align` only reports); a
  milestone is never closed automatically (R-MILE-6).
- Searching the PC for task lists: todo never does, and knows other projects only through the
  register (R-PROJ-1).
- Network access of any kind, including self-update: todo only prints the update steps
  (`todo help update`, R-OUT-11), and the agent runs them.
- macOS builds, an installer and code signing.
