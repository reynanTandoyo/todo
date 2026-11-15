<!-- todo-format: 1 -->
# todo — task list

The work plan for building `todo`. What each task must achieve is in
[`docs/requirements.md`](docs/requirements.md); the `reqs:` line names the requirements a task
covers, and `accept:` is the check that proves it is finished. The file follows
[`docs/format.md`](docs/format.md).

Until `todo` can manage this file, it is edited by hand. From v0.1 on, it is changed only through
`todo`, like any other project that uses it.

Priorities: P0 = v0.1 (MUST), P1 = v0.2 (SHOULD), P2 = v0.3 (COULD), P3 = idea, not planned.

## 1. Foundations

### T-001 · Test fixtures
- status: done
- owner: w1-hydrogen
- priority: P0 · type: test · area: core
- reqs: TEST-1
- accept: invented fixtures cover every rule in docs/format.md, each starting with the fixture line; one deliberately messy fixture (CRLF with byte-order mark, code fences, tables, fake tasks, trailing spaces); none contains real task data
- commit: 9931c66
- note (2026-11-15, coordinator): done — merged into dev and verified: fmt, clippy and 79 tests pass

### T-002 · Line model, edits and verified writes
- status: done
- owner: w1-hydrogen
- priority: P0 · type: feature · area: core
- reqs: R-EDIT-1, R-EDIT-2, R-EDIT-4, R-EDIT-5, R-EDIT-6, R-EDIT-7, TEST-3
- depends: T-001
- accept: every fixture reads and writes back byte-identical; the random-edit property test passes; a deliberately wrong edit is caught by the verification and nothing is written
- commit: b4347ba
- note (2026-11-15, coordinator): done — merged into dev and verified: fmt, clippy and 79 tests pass

### T-003 · Parser for format version 1
- status: done
- owner: w1-hydrogen
- priority: P0 · type: feature · area: core
- reqs: R-EDIT-3, R-EDIT-8, R-EDIT-9, R-ID-1, R-ID-3, R-DEP-1, R-OUT-4, REL-4, NFR-8, TEST-2
- depends: T-001, T-002
- accept: every rule and every refusal in docs/format.md has a test, including a file whose format line names version 2; problems are reported as `file:line: message`
- commit: 80833f9
- note (2026-11-15, coordinator): done — merged into dev and verified: fmt, clippy and 79 tests pass

### T-004 · Errors, exit codes, JSON envelope and safe output
- status: done
- owner: w2-hydrogen
- priority: P0 · type: feature · area: cli
- reqs: R-OUT-1, R-OUT-2, R-OUT-3, R-OUT-5, R-OUT-7, NFR-6, NFR-8
- accept: each error code and exit code has a test; every JSON schema name carries its version (`todo.<command>.v1`); output into a pipe that closes early ends with exit 0; Clippy rejects unwrap, expect and panic outside tests
- commit: 23335c5
- note (2026-11-15, coordinator): done — verified on dev: fmt, clippy and 21 tests pass

### T-005 · Command line: global options, agent name, help
- status: done
- owner: w2-hydrogen
- priority: P0 · type: feature · area: cli
- reqs: R-OUT-8, R-CLAIM-5, R-CFG-1, R-CFG-3, TEST-8
- depends: T-004
- accept: `todo`, `todo help <command>`, `todo --version`, `--file`, `--json`, `--agent` and `TODO_AGENT` (with name checks) are tested against the built binary; a test checks that the help lists every command
- commit: e1ea9a3
- note (2026-11-15, coordinator): done — verified on dev: fmt, clippy and 21 tests pass

### T-006 · Non-ASCII text in Windows consoles
- status: done
- owner: coordinator
- priority: P0 · type: test · area: cli
- reqs: R-OUT-6
- depends: T-005
- accept: `·`, `—` and Japanese text checked by hand in PowerShell 5, PowerShell 7, cmd and Windows Terminal, with the result recorded in a note; piped output is UTF-8 (automated test)
- commit: 8029ac3
- note (2026-11-15, coordinator): automated part done in 8029ac3 (test: piped output is UTF-8); still open: a person runs scripts/console-check.ps1 in PowerShell 5, PowerShell 7, cmd and Windows Terminal and records the result here
- note (2026-11-15, coordinator): done — the owner ran scripts/console-check.ps1 in PowerShell: '·', '—' and Japanese text render correctly in todo list and todo show; other shells not repeated because direct console output goes through the Windows console API whatever the shell

### T-007 · Settings file todo.toml
- status: done
- owner: w5-hydrogen
- priority: P1 · type: feature · area: config
- reqs: R-CFG-2
- depends: T-005, T-011
- accept: every key in requirements §5.14 has a test, including `worktree = "local"`; an unknown key or a bad value gives `todo.toml:<line>: message`
- commit: d893d29
- note (2026-11-15, w5-hydrogen): done — verified by coordinator: fmt, clippy and 128 tests pass; where, title matching, --dry-run and ask tried on a copy

## 2. Safety: finding, locking, writing

### T-010 · Find TODO.md
- status: done
- owner: w3-hydrogen
- priority: P0 · type: feature · area: find
- reqs: R-FIND-1, R-FIND-2, R-FIND-3, R-FIND-4, R-FIND-6, NFR-7, TEST-6
- depends: T-005
- accept: the search cases of TEST-6 pass, including a repository without TODO.md inside a parent folder that has one; paths with spaces, non-ASCII characters and more than 260 characters work
- commit: 9b71809
- note (2026-11-15, coordinator): done — verified on dev: fmt, clippy and 79 tests pass

### T-011 · Linked worktrees use the main checkout's file
- status: done
- owner: w3-hydrogen
- priority: P0 · type: feature · area: find
- reqs: R-FIND-5, NFR-5, TEST-6
- depends: T-010
- accept: a test creates a real linked worktree in a temporary folder and resolves to the main checkout's file, also from a subfolder; the fallback warning is tested; works with git removed from PATH
- commit: 2c6381c
- note (2026-11-15, coordinator): done — verified on dev: fmt, clippy and 79 tests pass

### T-012 · State folder
- status: done
- owner: w3-hydrogen
- priority: P0 · type: feature · area: lock
- reqs: R-HIST-1, R-LOCK-7
- depends: T-011
- accept: the main checkout, a worktree and different spellings of the path get the same state folder; outside git it is `.todo/` with a `.gitignore`
- commit: 15236de
- note (2026-11-15, coordinator): done — verified on dev: fmt, clippy and 79 tests pass

### T-013 · Write lock
- status: done
- owner: w3-hydrogen
- priority: P0 · type: feature · area: lock
- reqs: R-LOCK-1, R-LOCK-2, R-LOCK-3, R-LOCK-4, R-LOCK-5, TEST-5
- depends: T-012
- accept: tested through a test-only write command (cargo feature `test-hooks`): a killed holder delays others by less than 2 s; a held lock makes a command exit 3 naming the holder, without writing; reads never wait
- commit: 63ca72d
- note (2026-11-15, coordinator): done — verified on dev: fmt, clippy and 79 tests pass

### T-014 · Atomic replace
- status: done
- owner: w3-hydrogen
- priority: P0 · type: feature · area: io
- reqs: R-IO-1, R-IO-2, R-IO-3, R-IO-4, R-IO-5, TEST-7
- depends: T-002
- accept: the TEST-7 cases pass; no temporary file remains after any failure
- commit: 04a493f
- note (2026-11-15, coordinator): done — verified on dev: fmt, clippy and 79 tests pass

### T-015 · Lease: lock, unlock and status
- status: done
- owner: w3-hydrogen
- priority: P0 · type: feature · area: lock
- reqs: R-LOCK-6
- depends: T-013
- accept: a lease stops other agents but not its holder; it expires (tested with a shortened time); `unlock --force` names the previous holder; `status` shows the lock and the lease
- commit: 8fc7a05,55a816e
- note (2026-11-15, coordinator): done — verified on dev: fmt, clippy and 79 tests pass

### T-016 · todo where
- status: done
- owner: w5-hydrogen
- priority: P1 · type: feature · area: find
- reqs: R-FIND-7
- depends: T-012, T-015
- accept: every item listed in R-FIND-7 appears, in text and JSON
- commit: b429716
- note (2026-11-15, w5-hydrogen): done — verified by coordinator: fmt, clippy and 128 tests pass; where, title matching, --dry-run and ask tried on a copy

## 3. Task commands

### T-020 · list
- status: done
- owner: w4a-hydrogen
- priority: P0 · type: feature · area: cli
- reqs: R-READ-1, R-READ-4
- depends: T-003, T-010
- accept: every filter has a test; done and dropped tasks are hidden by default; JSON output matches its snapshot
- commit: 885136d
- note (2026-11-15, coordinator): done — verified on dev: fmt, clippy and 101 tests pass; list, show and next read this repository's TODO.md correctly

### T-021 · show
- status: done
- owner: w4a-hydrogen
- priority: P0 · type: feature · area: cli
- reqs: R-READ-2, R-DEP-2
- depends: T-003, T-010
- accept: text output equals the task's lines in the file; JSON includes unknown fields, notes, line numbers, dependencies with their status, and the tasks waiting on it
- commit: 45e4761
- note (2026-11-15, coordinator): done — verified on dev: fmt, clippy and 101 tests pass; list, show and next read this repository's TODO.md correctly

### T-022 · claim and release
- status: done
- owner: w4a-hydrogen
- priority: P0 · type: feature · area: cli
- reqs: R-WRITE-2, R-WRITE-3, R-WRITE-9, R-CLAIM-1, R-CLAIM-2, R-CLAIM-3, R-CLAIM-4, R-IO-6, TEST-4, TEST-5
- depends: T-013, T-014, T-020
- accept: 20 parallel claims of one task have exactly one winner; claiming again refreshes the claim time; a stale claim is taken over with a note; release by a non-owner needs --force; every claim and release case of the transition table has a test
- commit: 22b9400
- note (2026-11-15, coordinator): done — verified on dev: fmt, clippy and 101 tests pass; list, show and next read this repository's TODO.md correctly

### T-023 · next, including --claim
- status: done
- owner: w4a-hydrogen
- priority: P0 · type: feature · area: cli
- reqs: R-READ-3, R-DEP-1
- depends: T-022
- accept: 10 parallel `next --claim` get 10 different tasks; tasks with an unfinished or missing dependency and questions are never returned; an agent's own claim comes first unless --new
- commit: 816e8bc
- note (2026-11-15, coordinator): done — verified on dev: fmt, clippy and 101 tests pass; list, show and next read this repository's TODO.md correctly

### T-024 · done
- status: done
- owner: w4b-hydrogen
- priority: P0 · type: feature · area: cli
- reqs: R-WRITE-4, NFR-5, TEST-4
- depends: T-022
- accept: `--commit HEAD` is resolved before the lock is taken; an unknown commit is refused when git is available and accepted unchecked when it isn't; the accept line is printed; refusals leave the file byte-identical
- commit: c076de7
- note (2026-11-15, w4b-hydrogen): done — verified by coordinator: fmt, clippy and 110 tests pass

### T-025 · block, unblock and drop
- status: done
- owner: w4b-hydrogen
- priority: P0 · type: feature · area: cli
- reqs: R-WRITE-5, R-WRITE-6, TEST-4
- depends: T-022
- accept: a reason is required for block and drop; unblock returns to claimed or open; every remaining case of the transition table has a test
- commit: e3ef996
- note (2026-11-15, w4b-hydrogen): done — verified by coordinator: fmt, clippy and 110 tests pass

### T-026 · note
- status: done
- owner: w4b-hydrogen
- priority: P0 · type: feature · area: cli
- reqs: R-WRITE-7, R-IO-6, TEST-4
- depends: T-013, T-014
- accept: notes are added after the last line of the task, on any status, by any agent, with the local date
- commit: d795bc2
- note (2026-11-15, w4b-hydrogen): done — verified by coordinator: fmt, clippy and 110 tests pass

### T-027 · set
- status: done
- owner: w4b-hydrogen
- priority: P0 · type: feature · area: cli
- reqs: R-WRITE-8, R-CLAIM-3, TEST-4
- depends: T-022
- accept: every option, `--field` and `--unset` are tested; the combined priority line is edited in place; changing status, owner or claimed is refused
- commit: 655962f
- note (2026-11-15, w4b-hydrogen): done — verified by coordinator: fmt, clippy and 110 tests pass

### T-028 · add
- status: done
- owner: w4b-hydrogen
- priority: P0 · type: feature · area: cli
- reqs: R-WRITE-1, R-ID-2, TEST-4, TEST-5
- depends: T-013, T-014
- accept: 20 parallel adds lose nothing and get distinct IDs; unknown and ambiguous sections are refused with the list of sections; a duplicate title is refused; a highest ID deleted by hand is not given out again
- commit: ba355b8
- note (2026-11-15, w4b-hydrogen): done — verified by coordinator: fmt, clippy and 110 tests pass

### T-029 · Match tasks by title
- status: done
- owner: w5-hydrogen
- priority: P1 · type: feature · area: cli
- reqs: R-ID-4
- depends: T-022
- accept: a unique match works; several or no matches are refused with the candidates; done and dropped tasks are never matched
- commit: 11a40ca
- note (2026-11-15, w5-hydrogen): done — verified by coordinator: fmt, clippy and 128 tests pass; where, title matching, --dry-run and ask tried on a copy

### T-030 · reopen and move
- status: done
- owner: w5-hydrogen
- priority: P1 · type: feature · area: cli
- reqs: R-WRITE-10
- depends: T-028
- accept: reopen needs a reason and clears owner and claim time; move keeps every line of the task and leaves the rest of the file unchanged
- commit: 185fdab
- note (2026-11-15, w5-hydrogen): done — verified by coordinator: fmt, clippy and 128 tests pass; where, title matching, --dry-run and ask tried on a copy

### T-031 · ask and answer
- status: done
- owner: w5-hydrogen
- priority: P1 · type: feature · area: cli
- reqs: R-WRITE-11
- depends: T-028
- accept: the questions section is added when missing; next never returns a question; answer adds the answer note and marks the question done
- commit: abd5368
- note (2026-11-15, w5-hydrogen): done — verified by coordinator: fmt, clippy and 128 tests pass; where, title matching, --dry-run and ask tried on a copy

### T-032 · Text from standard input or a file
- status: done
- owner: w5-hydrogen
- priority: P1 · type: feature · area: cli
- reqs: R-WRITE-12
- depends: T-005
- accept: text with quotes, backticks and `$` arrives unchanged from PowerShell through `-` and `@path`
- commit: 0d297c2
- note (2026-11-15, w5-hydrogen): done — verified by coordinator: fmt, clippy and 128 tests pass; where, title matching, --dry-run and ask tried on a copy

### T-033 · release --stale and --all
- status: done
- owner: w5-hydrogen
- priority: P1 · type: feature · area: cli
- reqs: R-CLAIM-6
- depends: T-022
- accept: only stale claims are released by --stale; --all releases one agent's claims in one locked step; another agent needs --force
- commit: 5f89686
- note (2026-11-15, w5-hydrogen): done — verified by coordinator: fmt, clippy and 128 tests pass; where, title matching, --dry-run and ask tried on a copy

### T-034 · add --from: several tasks in one step
- status: done
- owner: w5-hydrogen
- priority: P1 · type: feature · area: cli
- reqs: R-WRITE-13
- depends: T-028
- accept: 30 tasks are added in one locked step; `@ref` dependencies resolve to the new IDs; one bad entry rejects the whole batch
- commit: 94e2aa2
- note (2026-11-15, w5-hydrogen): done — verified by coordinator: fmt, clippy and 128 tests pass; where, title matching, --dry-run and ask tried on a copy

### T-035 · --dry-run and --yes
- status: done
- owner: w5-hydrogen
- priority: P1 · type: feature · area: cli
- reqs: R-EDIT-10, R-WRITE-14
- depends: T-028
- accept: every write command prints a diff with --dry-run and leaves the file byte-identical; bulk commands change nothing without --yes
- commit: f09a8e2
- note (2026-11-15, w5-hydrogen): done — verified by coordinator: fmt, clippy and 128 tests pass; where, title matching, --dry-run and ask tried on a copy

### T-036 · apply: several operations in one step
- status: done
- owner: w7-hydrogen
- priority: P2 · type: feature · area: cli
- reqs: R-WRITE-15
- depends: T-034
- accept: all operations apply or none do, with one snapshot
- commit: 24995b5
- note (2026-11-15, w7-hydrogen): done — verified by coordinator: fmt, clippy (Windows and Linux) and 150 tests pass, also with an 8.3 short temp path

## 4. Recovery, checks and overview

### T-040 · Snapshots: history, diff and restore
- status: done
- owner: w6-hydrogen
- priority: P1 · type: feature · area: history
- reqs: R-HIST-2
- depends: T-012, T-014, T-035
- accept: every write leaves a snapshot; only the newest `snapshots_keep` remain; restore needs --yes, runs under the lock and can itself be undone
- commit: 0f379c9
- note (2026-11-15, w6-hydrogen): done — verified by coordinator: fmt, clippy (Windows and Linux) and 138 tests pass; check and summary run cleanly on this repository's TODO.md

### T-041 · Detect edits made outside the tool
- status: done
- owner: w6-hydrogen
- priority: P1 · type: feature · area: history
- reqs: R-HIST-3
- depends: T-040
- accept: a hand edit between two commands is saved and reported once; edits during a lease are recorded without a warning
- commit: f6db442
- note (2026-11-15, w6-hydrogen): done — verified by coordinator: fmt, clippy (Windows and Linux) and 138 tests pass; check and summary run cleanly on this repository's TODO.md

### T-042 · check
- status: done
- owner: w6-hydrogen
- priority: P1 · type: feature · area: check
- reqs: R-CHECK-1, R-DEP-3
- depends: T-003
- accept: a fixture for each problem listed in R-CHECK-1 is reported with its line; a clean fixture exits 0
- commit: 5eee24f
- note (2026-11-15, w6-hydrogen): done — verified by coordinator: fmt, clippy (Windows and Linux) and 138 tests pass; check and summary run cleanly on this repository's TODO.md

### T-043 · summary
- status: done
- owner: w6-hydrogen
- priority: P1 · type: feature · area: cli
- reqs: R-READ-5
- depends: T-020, T-023
- accept: at most 40 lines on a 5,000-task file; JSON output matches its snapshot
- commit: ff7111f
- note (2026-11-15, w6-hydrogen): done — verified by coordinator: fmt, clippy (Windows and Linux) and 138 tests pass; check and summary run cleanly on this repository's TODO.md

### T-044 · archive
- status: done
- owner: w6-hydrogen
- priority: P1 · type: feature · area: cli
- reqs: R-ARCH-1
- depends: T-024, T-025, T-035
- accept: done and dropped tasks move to the archive under the same section names; show, list --archive and dependency checks still find them; their IDs are never given out again
- commit: 876cbda
- note (2026-11-15, w6-hydrogen): done — verified by coordinator: fmt, clippy (Windows and Linux) and 138 tests pass; check and summary run cleanly on this repository's TODO.md

### T-045 · check --fix
- status: done
- owner: w7-hydrogen
- priority: P2 · type: feature · area: check
- reqs: R-CHECK-2
- depends: T-042
- accept: only the repairs listed in R-CHECK-2 are made; everything else is still reported
- commit: 208685b
- note (2026-11-15, w7-hydrogen): done — verified by coordinator: fmt, clippy (Windows and Linux) and 150 tests pass, also with an 8.3 short temp path

### T-046 · search
- status: done
- owner: w7-hydrogen
- priority: P2 · type: feature · area: cli
- reqs: R-READ-6
- depends: T-020
- accept: finds text in titles, fields and notes; --archive searches the archive file
- commit: d4c075a
- note (2026-11-15, w7-hydrogen): done — verified by coordinator: fmt, clippy (Windows and Linux) and 150 tests pass, also with an 8.3 short temp path

### T-047 · Journal and log
- status: done
- owner: w7-hydrogen
- priority: P2 · type: feature · area: history
- reqs: R-HIST-4, R-READ-7
- depends: T-040
- accept: each write adds one journal line; `log <task>` shows that task's entries
- commit: 2445fbc
- note (2026-11-15, w7-hydrogen): done — verified by coordinator: fmt, clippy (Windows and Linux) and 150 tests pass, also with an 8.3 short temp path

### T-048 · edit
- status: done
- owner: w7-hydrogen
- priority: P2 · type: feature · area: lock
- reqs: R-LOCK-8
- depends: T-015, T-042
- accept: the lease is taken before the editor opens and ended after it closes, with check and a snapshot in between
- commit: 886b41f
- note (2026-11-15, w7-hydrogen): done — verified by coordinator: fmt, clippy (Windows and Linux) and 150 tests pass, also with an 8.3 short temp path

### T-049 · Show tasks that became ready
- status: done
- owner: w7-hydrogen
- priority: P2 · type: feature · area: cli
- reqs: R-DEP-4
- depends: T-024, T-025
- accept: done and drop list the tasks whose last open dependency they finished
- commit: 095a83d
- note (2026-11-15, w7-hydrogen): done — verified by coordinator: fmt, clippy (Windows and Linux) and 150 tests pass, also with an 8.3 short temp path

## 5. Agent setup

### T-050 · Agent rules text and `todo help agents`
- status: done
- owner: w2-hydrogen
- priority: P0 · type: feature · area: agent
- reqs: R-AGENT-1
- depends: T-005
- accept: one built-in text covers every point in R-AGENT-1 and is what `todo help agents` prints
- commit: a88f6da
- note (2026-11-15, coordinator): done — verified on dev: fmt, clippy and 21 tests pass

### T-051 · init
- status: done
- owner: w6-hydrogen
- priority: P1 · type: feature · area: agent
- reqs: R-AGENT-2
- depends: T-042, T-050
- accept: the new file passes `todo check`; an existing TODO.md is never overwritten
- commit: c673250
- note (2026-11-15, w6-hydrogen): done — verified by coordinator: fmt, clippy (Windows and Linux) and 138 tests pass; check and summary run cleanly on this repository's TODO.md

### T-052 · agents install and agents check
- status: done
- owner: w6-hydrogen
- priority: P1 · type: feature · area: agent
- reqs: R-AGENT-3
- depends: T-035, T-050
- accept: the block is added or replaced without changing any other byte of the file; check reports missing and outdated blocks
- commit: 42e9c47
- note (2026-11-15, w6-hydrogen): done — verified by coordinator: fmt, clippy (Windows and Linux) and 138 tests pass; check and summary run cleanly on this repository's TODO.md

### T-053 · Claude Code hooks
- status: done
- owner: w7-hydrogen
- priority: P2 · type: feature · area: agent
- reqs: R-AGENT-4
- depends: T-043, T-050
- accept: a new session starts with the summary; an Edit or Write of TODO.md is stopped with the command to use; other files are unaffected; uninstall removes only the tool's entries; works from PowerShell
- commit: 231c68e
- note (2026-11-15, w7-hydrogen): done — verified by coordinator: fmt, clippy (Windows and Linux) and 150 tests pass, also with an 8.3 short temp path

### T-054 · Claude Code skill
- status: done
- owner: w7-hydrogen
- priority: P2 · type: feature · area: agent
- reqs: R-AGENT-5
- depends: T-050
- accept: the skill file is generated from the same rules text
- commit: 7fcf2f9
- note (2026-11-15, w7-hydrogen): done — verified by coordinator: fmt, clippy (Windows and Linux) and 150 tests pass, also with an 8.3 short temp path

## 6. Build, release and documentation

### T-060 · CI workflow
- status: done
- owner: w2-hydrogen
- priority: P0 · type: chore · area: release
- reqs: REL-1, NFR-1, NFR-2, NFR-4, NFR-9, NFR-10
- accept: format check, Clippy and tests run on Windows and Linux for every push; the Windows job fails if todo.exe needs a DLL outside Windows or reaches 5 MB; the dependency list matches NFR-9; Cargo.toml requires Rust 1.89
- commit: 2743431
- note (2026-11-15, coordinator): done — verified on dev: fmt, clippy and 21 tests pass

### T-061 · README
- status: done
- owner: w4b-hydrogen
- priority: P0 · type: docs · area: release
- reqs: REL-2, NFR-11
- depends: T-023, T-050
- accept: build, install, use, adding to a project and the limits are explained in plain language, and the steps were followed on a clean folder
- commit: 8d606fa
- note (2026-11-15, w4b-hydrogen): done — verified by coordinator: fmt, clippy and 110 tests pass

### T-062 · MIT licence
- status: done
- owner: w2-hydrogen
- priority: P0 · type: chore · area: release
- reqs: REL-3
- accept: LICENSE holds the MIT text; the README and Cargo.toml name the licence
- commit: 450c931
- note (2026-11-15, coordinator): done — verified on dev: fmt, clippy and 21 tests pass

### T-063 · docs/json.md and CHANGELOG.md
- status: done
- owner: w6-hydrogen
- priority: P1 · type: docs · area: release
- reqs: REL-5
- depends: T-004
- accept: every JSON shape and error code is documented with an example; CHANGELOG.md has an entry for each released version
- commit: 829e2ec
- note (2026-11-15, w6-hydrogen): done — verified by coordinator: fmt, clippy (Windows and Linux) and 138 tests pass; check and summary run cleanly on this repository's TODO.md

### T-064 · Release workflow
- status: done
- owner: w6-hydrogen
- priority: P1 · type: chore · area: release
- reqs: REL-6
- depends: T-060
- accept: a `v*` tag publishes todo.exe and the Linux binary with SHA-256 files; a tag that doesn't match Cargo.toml fails
- commit: c3ca7e3
- note (2026-11-15, w6-hydrogen): done — verified by coordinator: fmt, clippy (Windows and Linux) and 138 tests pass; check and summary run cleanly on this repository's TODO.md

### T-065 · Performance test
- status: done
- owner: w4b-hydrogen
- priority: P0 · type: test · area: core
- reqs: NFR-3, TEST-9
- depends: T-020, T-028
- accept: on a generated 5,000-task file, list and show take under 100 ms and a write under 200 ms; runs in CI
- commit: 40b4d67
- note (2026-11-15, w4b-hydrogen): done — verified by coordinator: fmt, clippy and 110 tests pass

### T-066 · Version with build commit
- status: done
- owner: w7-hydrogen
- priority: P2 · type: chore · area: release
- reqs: REL-7
- depends: T-005
- accept: `todo --version` prints the version and the commit
- commit: 571a960
- note (2026-11-15, w7-hydrogen): done — verified by coordinator: fmt, clippy (Windows and Linux) and 150 tests pass, also with an 8.3 short temp path

### T-067 · Warn about synced folders
- status: done
- owner: w7-hydrogen
- priority: P2 · type: feature · area: find
- reqs: NFR-12
- depends: T-016
- accept: a TODO.md inside a OneDrive or Dropbox folder produces a warning in where and in write commands
- commit: 770cece
- note (2026-11-15, w7-hydrogen): done — verified by coordinator: fmt, clippy (Windows and Linux) and 150 tests pass, also with an 8.3 short temp path

### T-068 · Colour output
- status: done
- owner: w7-hydrogen
- priority: P2 · type: feature · area: cli
- reqs: R-OUT-9
- depends: T-020
- accept: colour appears only on a terminal, never in piped output or with NO_COLOR set
- commit: 501c372
- note (2026-11-15, w7-hydrogen): done — verified by coordinator: fmt, clippy (Windows and Linux) and 150 tests pass, also with an 8.3 short temp path

### T-082 · Make writes fast again on large files
- status: done
- owner: w8-hydrogen
- priority: P0 · type: fix · area: core
- problem: after v0.3 a write takes 150 ms (perf test: add 199 ms against the 200 ms limit) while v0.1 took 59 ms; note --dry-run takes 93 ms although it writes nothing, so the write path does several full parses or other repeated work; release todo.exe grew from 0.7 MB to 4.2 MB
- accept: on a generated 5,000-task file (about 1 MB) in a release build, add and note take under 100 ms and list and show under 60 ms on this PC, with the perf test asserting the NFR-3 limits; the release todo.exe size is explained and reduced if easy
- commit: 6bc8845
- reqs: NFR-3, TEST-9
- note (2026-11-15, coordinator): added
- note (2026-11-15, w8-hydrogen): done — verified by coordinator: fmt, clippy (Windows and Linux) and 150 tests pass; perf test list 41 ms, show 36 ms, note 72 ms, add 73 ms (5,000 tasks); release todo.exe 4.2 MB -> 3.06 MB (clap/toml/sha2 default features off, panic=abort; help, suggestions and error context kept)

### T-083 · Refactor: remove duplication without changing behaviour
- status: open
- owner: —
- priority: P1 · type: refactor · area: core
- problem: about 16,300 lines in src/ and 7,900 in tests/, with the same logic repeated across commands, batch paths and pipeline variants
- accept: same features, output, exit codes and error codes; all tests pass unchanged; clear shared abstractions instead of repeated code; src/ and tests/ noticeably smaller; docs/refactoring.md records what was consolidated
- reqs: NFR-6, NFR-8
- note (2026-11-15, coordinator): added

## 7. Ideas, not planned

### T-070 · MCP server
- status: open
- owner: —
- priority: P3 · type: idea · area: agent
- reqs: I-1

### T-071 · Overview across several projects
- status: open
- owner: —
- priority: P3 · type: idea · area: cli
- reqs: I-2

### T-072 · Claims tagged with an agent session
- status: open
- owner: —
- priority: P3 · type: idea · area: agent
- reqs: I-3

### T-073 · todo schema
- status: open
- owner: —
- priority: P3 · type: idea · area: agent
- reqs: I-4

### T-074 · todo handoff
- status: open
- owner: —
- priority: P3 · type: idea · area: agent
- reqs: I-5

### T-075 · todo renumber
- status: open
- owner: —
- priority: P3 · type: idea · area: cli
- reqs: I-6

### T-076 · Git merge driver for TODO.md
- status: open
- owner: —
- priority: P3 · type: idea · area: git
- reqs: I-7

### T-077 · todo align
- status: open
- owner: —
- priority: P3 · type: idea · area: git
- reqs: I-8

### T-078 · todo undo
- status: open
- owner: —
- priority: P3 · type: idea · area: history
- reqs: I-9

### T-079 · PowerShell tab completion
- status: open
- owner: —
- priority: P3 · type: idea · area: cli
- reqs: I-10

### T-080 · todo scan
- status: open
- owner: —
- priority: P3 · type: idea · area: cli
- reqs: I-11

### T-081 · Import from other task-list formats
- status: open
- owner: —
- priority: P3 · type: idea · area: cli
- reqs: I-12
