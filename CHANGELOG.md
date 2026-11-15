# Changelog

All notable changes to `todo` are documented in this file.

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
