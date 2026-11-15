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

## 2. Who uses it, and the situations it must handle

- **Coordinator**: usually a main agent session. Plans work, adds tasks in bulk, hands tasks to
  other agents, reviews and closes them.
- **Worker agents**: Claude Code, Codex, opencode, Gemini and others, several at once, often in
  linked git worktrees. Short-lived; may crash or be cancelled mid-task.
- **People**: read `TODO.md` in an editor or on GitHub, sometimes edit it by hand, and answer
  questions agents raise.

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
| D11 | Settings are optional: one `todo.toml` next to `TODO.md`, no per-user settings. | New projects need no setup; projects that want rules can have them. |
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
<!-- todo-format: 1 -->
# Example project — tasks

## Now

### T-001 · Fix the save button
- status: claimed
- owner: agent-ui-1
- claimed: 2026-11-15T14:03+01:00
- priority: P1 · type: bug · area: ui
- accept: saving twice leaves one file
- note (2026-11-15, coordinator): added
```

(Invented example.) Known fields: `status`, `owner`, `claimed`, `priority`, `type`, `area`,
`depends`, `files`, `problem`, `fix`, `accept`, `commit`, and dated `note` lines. Any other
`- key: value` line is kept and shown in JSON. Everything outside task blocks — the preamble,
prose, tables, code — is never changed by task commands.

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
  the state folder, the lock and lease status, the format version and the number of tasks per
  status.

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
  markers, an unclosed fence or comment, an unknown format version; or, for the task being changed,
  a duplicate ID, a repeated field or an unknown status). Read commands still work and show the same
  messages as warnings.
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
  archive file instead).
- **R-READ-2 MUST** `show <task>` prints the task exactly as it appears in the file. With `--json` it
  returns every field, the notes, unknown fields, the section, the line numbers, the dependencies
  with their status, and the tasks that depend on it.
- **R-READ-3 MUST** `next` returns the task to take: status `open`, or `claimed` with a stale claim;
  every dependency finished (R-DEP-1); type not `question`; highest priority first, then earliest in
  the file. Filters: `--section`, `--type`, `--area`, `--where key=value`. With an agent name, if
  that agent already has a claimed task, `next` returns it first, marked as already theirs (`--new`
  skips this). `next --claim --agent X` chooses and claims in one locked step, so agents asking at
  the same moment get different tasks.
- **R-READ-4 MUST** Every read command supports `--json` (R-OUT-1).
- **R-READ-5 SHOULD** `summary` prints a session-start overview of at most 40 lines: tasks per
  status; open tasks per priority; claimed tasks with owner and age, stale ones marked; blocked
  tasks with their latest reason; the next five ready tasks; open questions; and a warning if the
  file was edited outside the tool.
- **R-READ-6 COULD** `search <text>` finds text in titles, fields and notes, printing each match
  with its task ID; `--archive` searches the archive file.
- **R-READ-7 COULD** `log <task>` shows the journal entries about one task (R-HIST-4).

### 5.7 Changing tasks — WRITE

Every write command needs an agent name (R-CLAIM-5). It prints one line such as `T-012: claim ok`,
or with `--json` the task as it is after the change.

- **R-WRITE-1 MUST** `add --title T [--section S] [--priority P] [--type X] [--area X]
  [--depends IDs] [--files T] [--problem T] [--fix T] [--accept T] [--field key=value]…` adds a task
  at the end of a section (format.md §10) and prints its ID. The section is found by its exact name,
  ignoring case and a leading number, or else by a unique part of the name; an unknown or ambiguous
  section is refused (exit 1) with the list of sections. Without `--section` the tool uses
  `default_section`, or the only section if there is just one, and refuses otherwise. Fields in
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
  done, dropped or reopened. Ownership rules (R-CLAIM-3) apply on top.
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

### 5.9 Dependencies — DEP

- **R-DEP-1 MUST** `depends: T-003, T-007` names tasks that must be finished first. A task is ready
  only when each of them is done or dropped. A dependency on an ID that doesn't exist keeps the task
  waiting. Whether a task is waiting is worked out when reading and never written into the file.
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
  `CHANGED_DURING_WRITE`, `UNSAFE_FILE`, `READ_ONLY`, `WRITE_FAILED`. New codes may be added; existing
  ones never change meaning.
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

### 5.11 History and recovery — HIST

- **R-HIST-1 MUST** A state folder per task file (layout in §7): inside git,
  `<git common folder>/todo/<path of TODO.md in the repository>/`, shared by all worktrees and never
  committed; outside git, `.todo/` next to `TODO.md`, containing a `.gitignore` with `*`.
- **R-HIST-2 SHOULD** Before each write, the old file is saved as a numbered snapshot, with an index
  line giving number, time, agent, command and hash. The newest 200 are kept (`snapshots_keep`).
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
  dependency problems of R-DEP-3; stale claims; blocked tasks without a reason note; values outside
  the configured lists; leftover temporary files; an outdated rules block (R-AGENT-3). Exit 0 if
  clean, 1 if anything was found.
- **R-CHECK-2 COULD** `check --fix` makes mechanical repairs only — canonical IDs and separators, a
  missing `status: open`, leftover temporary files, an outdated rules block — and reports the rest.

### 5.13 Archive — ARCH

- **R-ARCH-1 SHOULD** `archive [--older-than DAYS] --yes` moves done and dropped tasks into
  `TODO-archive.md` (format.md §14), under the same lock. `show`, `list --archive`,
  `search --archive` and dependency checks still find archived tasks.

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
  | `archive_file` | `"TODO-archive.md"` | archive file name |
  | `claim_ttl_hours` | `24` | when a claim becomes stale |
  | `lock_wait_seconds` | `30` | how long a write waits for the lock or a lease |
  | `lease_minutes` | `10` | default length of `todo lock` |
  | `lease_max_minutes` | `60` | longest allowed lease |
  | `snapshots_keep` | `200` | snapshots kept |
  | `worktree` | `"shared"` | `"local"` makes each worktree use its own file |

  An unknown key or a bad value is an error, reported as `todo.toml:<line>: message`.
- **R-CFG-3 MUST** Settings come from the command line first, then environment variables
  (`TODO_FILE`, `TODO_AGENT`, `NO_COLOR`), then `todo.toml`, then the defaults.

### 5.15 Agents — AGENT

- **R-AGENT-1 MUST** One rules text, built into the program, tells agents how to work with it: never
  edit `TODO.md` directly; use one unique agent name per agent (`--agent` or `TODO_AGENT`); start
  with `summary` or `next`; claim before working (`next --claim` does both); exit 1 means someone
  else has it, so pick another task; exit 3 means try again; add notes as you learn things; finish
  with `done --note … --commit …`; `add` problems you find instead of fixing them silently; `ask`
  what only a person can decide; start commit messages with the task ID (`T-012: …`).
  `todo help agents` prints it.
- **R-AGENT-2 SHOULD** `todo init` creates `TODO.md` with the format line, a title, the rules text
  between the managed-block markers (format.md §12) and a `## Tasks` section, and refuses if
  `TODO.md` already exists. It then prints the lines to add to `AGENTS.md`.
- **R-AGENT-3 SHOULD** `agents install [--into FILE] --yes` writes or updates the rules block in
  `AGENTS.md` (default), `CLAUDE.md` or another file, leaving the rest of the file byte for byte;
  `agents check` reports a missing or outdated block.
- **R-AGENT-4 COULD** `hooks install --yes` adds two Claude Code hooks to the project's
  `.claude/settings.json`: at session start, `todo summary` (at most 40 lines, within 2 seconds);
  before Edit, Write or MultiEdit, `todo guard`, which stops direct edits of `TODO.md` and the
  archive file and names the command to use instead (direct edits are allowed while a lease is
  live). It keeps a backup, preserves the file's formatting, marks its entries so `hooks uninstall`
  removes only those, and works from PowerShell.
- **R-AGENT-5 COULD** `skill install --yes` writes `.claude/skills/todo/SKILL.md` from the same rules
  text.

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
```

---

## 8. Non-functional requirements — NFR

- **NFR-1 MUST** One `todo.exe` for Windows 10/11 x64 with the C runtime linked in: no runtime to
  install, no administrator rights, runs from any folder on `PATH`.
- **NFR-2 MUST** The code also builds and passes its tests on Linux x64 (CI), to keep it portable.
- **NFR-3 MUST** `list` and `show` on a 600 KB file take under 100 ms; a write takes under 200 ms
  plus any lock wait (measured on a generated 5,000-task file).
- **NFR-4 MUST** No network access, telemetry, background process, database or installer.
- **NFR-5 MUST** git is optional; only `done --commit` uses it (`git rev-parse`), and only when git
  is on `PATH`. Everything else, including worktree detection, works without it.
- **NFR-6 MUST** No crashes: every failure ends with a message and an exit code. `unwrap`, `expect`
  and `panic!` are not used outside tests (enforced by Clippy).
- **NFR-7 MUST** Paths with spaces, non-ASCII characters and more than 260 characters work.
- **NFR-8 MUST** Command names and options, JSON schemas, error codes, exit codes and the file format
  are versioned interfaces. A breaking change needs a new major version and an entry in
  `CHANGELOG.md`; the file format has its own version line.
- **NFR-9 SHOULD** Few, well-known dependencies: `clap`, `serde`, `serde_json`, `toml`, `sha2`,
  `similar` and `chrono`; for tests `assert_cmd`, `predicates`, `tempfile`, `proptest` and `insta`.
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
  removed from the environment and `HOME`/`APPDATA` pointing at a temporary folder. JSON output is
  checked against snapshots, and a test checks that `todo help` lists every command.
- **TEST-9 MUST** A performance test on a generated 5,000-task file (NFR-3) runs in CI but not in
  every local `cargo test`.

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

---

## 12. Ideas, not planned

Recorded so they aren't lost; each becomes a requirement only by a decision recorded here.

- **I-1** An MCP server exposing the commands as tools.
- **I-2** A list of projects and an overview across them.
- **I-3** Claims tagged with an agent session, released by a Claude Code `SessionEnd` hook.
- **I-4** `todo schema`: a machine-readable description of commands, JSON shapes and exit codes.
- **I-5** `todo handoff`: a summary note of an agent's open claims for whoever continues.
- **I-6** `todo renumber`: change a task's ID and every reference to it.
- **I-7** A git merge driver that merges `TODO.md` task by task.
- **I-8** `todo align`: compare tasks with git history and report likely-finished or unproven tasks
  (exit 5 when the history is missing).
- **I-9** `todo undo`: revert an agent's last change if nothing has happened since.
- **I-10** PowerShell tab completion.
- **I-11** `todo scan`: find `TODO:` comments in source code and offer them as tasks.
- **I-12** Import from other task-list formats (checkbox lists, todo.txt).

---

## 13. Out of scope

- A terminal UI, desktop app or web server.
- Due dates, recurrence and computed priority scores.
- A database or index next to the file.
- Other record types (decision records, plans, issues).
- Syncing with GitHub Issues, Jira or other trackers.
- Locking across machines or real-time sync; git handles sharing between machines.
- Changing a task's status automatically, for example from git history.
- Network access of any kind, including self-update.
- macOS builds, an installer and code signing.
