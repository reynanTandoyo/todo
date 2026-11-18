# Changelog

All notable changes to `todo` are documented in this file.

## [Unreleased]

### Fixed
- A failed replace of `TODO.md` after `archive` or a migration no longer leaves the moved tasks in both files: a read-only `TODO.md` is refused before `TODO-archive.md` is written, and a replace that fails afterwards puts the archive back as it was (`R-ARCH-1`, `R-MIG-1`).

## [0.6.0] - 2026-11-18

### Changed — file format version 2 (read this before updating)
- `TODO.md` and `TODO-archive.md` are now format version 2: the format line is `<!-- todo-format: 2 -->`,
  and a tool line `<!-- todo-tool: 0.6.0 -->` records the newest todo that has written the file
  (`R-VER-1`, format.md §4 and §18). Format 1 files are migrated on the first write, or at once with
  `todo migrate --yes`; the migration changes only the format and tool lines (`R-MIG-1`).
- **todo 0.5 and older can no longer write a format 2 file.** They read it with a warning but refuse
  to write it. Update every todo that writes the same `TODO.md` before the first write by 0.6.
- A todo that meets a format it does not know stops every command that reads task content with exit 4
  and the error `TOOL_TOO_OLD`, naming the stamped version and the running one and pointing at
  `todo help update` (`R-VER-2`). `where`, `help`, `schema`, `status`, `history` and the other commands
  that never read tasks keep working, and nothing is written.
- `summary` and `where` count tasks only: `total` and the status counts leave out milestones, questions
  and ideas, which are counted apart (`R-KIND-3`, `R-MILE-3`).
- The agent rules text (`todo help agents`, `R-AGENT-1`) now says: put each new task under a milestone
  with a `why`; read the goals above a task before starting it; close a milestone when its last task is
  finished; use `idea` for ideas and `ask --about` for questions; run `todo migrate --yes` after updating
  todo; and follow `todo help update` when todo is too old for a file. `todo migrate --yes` refreshes
  the rules block, skill and hooks that a project has installed.

### Added
- Milestones (`R-MILE-1` to `R-MILE-6`): an entry with `type: milestone` and a `goal`, a tree of
  milestones and tasks linked by the `milestone` field, at any depth. `todo tree [<milestone>]` shows
  the tree with progress `done/total`; `list --milestone` and `list --kind` filter it; `drop --tree`
  drops a milestone with everything below it. `show`, `next`, `claim` and `next --claim` print the goals
  of the milestones above a task. A milestone cannot be done while work below it is open (`--force`
  overrides), and `done` reports it as ready to close (`ready_to_close` in JSON) when its last task is
  finished. `next` hands out tasks in milestones that have already started first.
- Kinds of entry (`R-KIND-1` to `R-KIND-3`): `todo idea` records an idea without a priority, `todo promote`
  turns it into a task, `ask --about <ID>` and `idea --about <ID>` name the entry they are about, `show`
  lists the questions about an entry, and `answer` shows the entry a question is about.
- `why` and `difficulty` fields (`R-WRITE-17`, `R-WRITE-18`): the purpose of a task (`add --why`) and how
  hard it is (`add --difficulty`, one of the configured `difficulties`); `next --difficulty` and
  `--max-difficulty` hand out tasks by how hard they are.
- `todo migrate [--dry-run] [--yes]`: moves `TODO.md` and the archive to format 2 and brings the installed
  rules block, Claude Code skill and hooks up to date, without installing anything that was not there
  (`R-MIG-2`).
- `todo help update` (and `--json`): the steps an agent runs to update todo from the official release;
  todo itself never connects to the network (`R-OUT-11`).
- `todo check` reports milestone and reference problems (`R-MILE-5`): a missing or non-milestone parent,
  a cycle, a finished milestone with open entries below it, a milestone without a goal, an `about` naming
  a missing ID. It also warns about a difficulty outside the configured ones.
- `todo archive` keeps milestone trees whole and lists the finished entries it kept back, with the reason
  (`R-ARCH-2`).
- `todo where` shows the format version, the tool stamp and the running version (`R-FIND-7`).

### Fixed
- The combined priority line reads `status`, `owner`, `claimed`, `why`, `problem`, `fix`, `accept`,
  `files`, `depends` and `commit` as their fields, edits unknown parts in place and keeps a part that is
  not `key: value` (T-113, T-114, T-117, T-133, T-134).
- `hooks install` and `hooks uninstall` keep every other byte of `.claude/settings.json` (T-126, T-130);
  the agent and skill commands use the folder of `TODO.md` (T-127).
- `archive` writes `TODO-archive.md` after the snapshot of `TODO.md` (T-121).
- The milestone tree is built in one pass, and a 5,000-task performance test covers it (T-105, T-125).
- `ask` writes the note `added` as format.md §9 says (T-115); `summary` and `where` counts (T-116, T-128).
- The JSON examples in `docs/json.md` are checked against the real output of their commands (T-111,
  T-112, T-128, T-129); the error envelope shows its fields beside `code` and `message`, as the output
  does, and `TOOL_TOO_OLD` has its own example.

## [0.5.0] - 2026-11-18

### Added
- `todo align`: compare `TODO.md` with the git history and report open tasks that already have commits (`COMMITTED_NOT_DONE`), done tasks without a commit (`DONE_WITHOUT_COMMIT`) and `commit` fields naming commits that don't exist (`COMMIT_MISSING`); exit 1 with findings, exit 5 with the new error code `NO_GIT_HISTORY` when there is no usable history (`R-GIT-1`).
- `todo merge-driver install|uninstall|run`: a git merge driver that merges `TODO.md` task by task during `git pull`, `merge`, `rebase` and `cherry-pick`, keeps notes from both sides, and marks real conflicts per task block; it falls back to `git merge-file` (`R-GIT-2`, `R-GIT-3`). Run `todo merge-driver install` once in each clone.
- `todo scan [<path>…] [--add]`: find `TODO:` and `FIXME:` comments in the project and optionally add them as tasks with `files: <path>:<line>` (`R-DISC-1`).
- `todo schema [<command>]`: a machine-readable JSON description of every command, option, exit code, error code and task field, generated from the command definitions (`R-SCHEMA-1`).
- `by` field: `done` records who finished a task, from the commit author or the git user name, mapped through the repository's `.mailmap` so each contributor has one name; `list --by <name>` filters by it and `check` warns about names missing from `.mailmap`. No email address is written (`R-WRITE-16`).

### Changed
- Exit code 5 is now used (by `align`); `todo --json` lists the new commands.
- Tests run `todo` without the developer's global git configuration and with a fixed clock where claims must not expire.

## [0.4.0] - 2026-11-16

### Added
- `todo import --from checkbox <source> [--out <path>] [--yes]`: convert a Markdown checkbox task list into format version 1 with a preview, a `<name>.pre-import` backup, and a `todo check` plus task-count proof of the result (`R-IMP-1`).
- Claims tagged with an agent session: `claim`, `next --claim` and `unblock` write `- session: <id>` from `--session`, `TODO_SESSION` or `CLAUDE_CODE_SESSION_ID`; `release --session <id>` releases every claim of that session; `todo hooks install` adds a Claude Code `SessionEnd` hook (the hidden command `todo session-end`) that does this when a session ends; `hooks uninstall` removes it (`R-CLAIM-7`).
- `todo completions <shell>`: tab-completion scripts for powershell, bash, zsh, fish and elvish, generated with `clap_complete` (new dependency, `NFR-9`); hidden commands are left out (`R-OUT-10`).

### Changed
- The `session` field is written right after `claimed` (format.md §10); the global option `--session <ID>` is new.
- `todo hooks install` now installs three hooks (`SessionStart`, `PreToolUse` and `SessionEnd`); run it again in a project to add the new one. The release `todo.exe` is 3.1 MB.
- Tests run `todo` without `TODO_SESSION` and `CLAUDE_CODE_SESSION_ID`, so they give the same results inside Claude Code.

## [0.3.0] - 2026-11-15

### Added
- `todo apply <file|->`: run multiple operations from JSON lines in one locked, all-or-nothing step with a single snapshot (`R-WRITE-15`).
- `todo check --fix`: mechanical repairs only (canonical IDs and separators, missing `status: open`, leftover temporary files, outdated rules blocks) while still reporting everything else (`R-CHECK-2`).
- `todo search <text> [--archive]`: search task titles, fields, and notes (`R-READ-6`).
- Write journal (`journal.jsonl`) and `todo log <task>`: record every write with timestamp, agent, command, arguments, tasks, and before/after hashes (`R-HIST-4`, `R-READ-7`).
- `todo edit --agent <name>`: acquire a manual lease, open `VISUAL`/`EDITOR`/Notepad, run `check`, record a snapshot, and end the lease (`R-LOCK-8`).
- `done` and `drop` now list tasks whose last open dependency was just satisfied (`R-DEP-4`).
- `todo hooks install/uninstall` and `todo guard`: Claude Code `SessionStart` (`todo summary`) and `PreToolUse` (`todo guard`) hooks with backup, formatting preservation, and PowerShell compatibility (`R-AGENT-4`).
- `todo skill install`: generate `.claude/skills/todo/SKILL.md` from the built-in rules text (`R-AGENT-5`).
- `todo --version`: include the build commit hash (`REL-7`).
- Synced-folder warning in `where` and write commands when `TODO.md` is inside OneDrive or Dropbox (`NFR-12`).
- Terminal-only ANSI colour output, disabled when piped or with `NO_COLOR` set (`R-OUT-9`).

## [0.2.0] - 2026-11-15

### Added
- `todo where`: print resolved `TODO.md`, state folder, archive, and config paths (`R-FIND-6`).
- `todo summary`: compact session-start overview capped at 40 lines (`R-READ-5`).
- `todo check`: validate format, dependencies, cycles, claims, config values, leftover temp files, and managed rules blocks (`R-CHECK-1`, `R-DEP-3`).
- `todo history`, `todo diff`, `todo restore`: numbered pre-write snapshots with `snapshots_keep` pruning and locked restore (`R-HIST-2`).
- Outside-edit detection: hashes recorded in `last-write.json`, unexpected edits snapshotted and warned once, lease edits snapshotted without warning (`R-HIST-3`).
- `todo archive`: move `done` and `dropped` tasks (optionally `--older-than DAYS`) to `TODO-archive.md` under matching sections (`R-ARCH-1`).
- `todo reopen`, `todo move`, `todo ask`, `todo answer`, `todo release --stale/--all`, `todo add --from`, and `-` / `@path` text inputs.
- `--dry-run` unified diff previews for write commands and `--yes` confirmation for bulk commands (`R-EDIT-10`, `R-WRITE-14`).
- `todo.toml` project configuration (`R-CFG-2`) and title-substring matching (`R-ID-4`).
- `todo init`, `todo agents install`, and `todo agents check` (`R-AGENT-2`, `R-AGENT-3`).
- `docs/json.md` JSON and error code reference (`REL-5`), plus automated release workflow (`REL-6`).

## [0.1.0] - 2026-11-14

### Added
- Initial release of `todo` (`todo.exe`) with line-preserving `TODO.md` parser and verified whole-line editor (`R-EDIT-1`, `R-EDIT-7`, `R-EDIT-8`).
- Exclusive OS file locking, manual editing leases (`lock`, `unlock`, `status`), and atomic crash-safe writes (`R-LOCK-1..8`, `R-IO-1..5`).
- Core read and write commands: `list`, `show`, `next`, `add`, `claim`, `release`, `done`, `drop`, `block`, `unblock`, `note`, `set`, `help`, `--version`.
- Git worktree discovery (`R-FIND-1..5`), `--json` output envelopes (`R-OUT-1..8`), and built-in agent rules (`todo help agents`, `R-AGENT-1`).
- MIT license (`LICENSE`), README, and CI workflow (Windows & Linux).
