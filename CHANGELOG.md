# Changelog

All notable changes to `todo` are documented in this file.

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
