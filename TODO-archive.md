<!-- todo-format: 3 -->
<!-- todo-tool: 0.8.1 -->
# Archive

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

### T-085 · done records who finished a task (by, via .mailmap)
- status: done
- owner: v5-by
- priority: P2 · type: feature · area: cli
- accept: done and apply's done write by: from the commit author (%aN) or git user.name mapped via git check-mailmap; no git or no name leaves it out; existing by replaced, reopen removes it; list --by filters; check warns about by values not on the .mailmap left side without changing the exit code; tests use temp repos with a .mailmap
- commit: 4e8a7bd
- reqs: R-WRITE-16
- note (2026-11-17, claude-main): added
- note (2026-11-17, v5-by): done — by: from commit author (%aN) or git user.name via check-mailmap; reopen removes it; list --by; check warns about names not in .mailmap; verified in a throwaway repo with two identities mapped to one name

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
- status: dropped
- owner: —
- priority: P1 · type: refactor · area: core
- problem: about 16,300 lines in src/ and 7,900 in tests/, with the same logic repeated across commands, batch paths and pipeline variants
- accept: same features, output, exit codes and error codes; all tests pass unchanged; clear shared abstractions instead of repeated code; src/ and tests/ noticeably smaller; docs/refactoring.md records what was consolidated
- reqs: NFR-6, NFR-8
- note (2026-11-15, coordinator): added
- note (2026-11-15, coordinator): dropped — stopped by the owner after measuring: only 2.3% duplicated code (jscpd). Kept: shared task operations for claim/release/done/drop/block/unblock/reopen/answer/note/set and apply/add --from, shared test helpers (tests/common), shared section lookup and read loading, answer note per format.md (about 520 lines removed, behaviour unchanged, 152 tests pass). See docs/refactoring.md

### T-084 · cmd_apply test fails once its fixed claim date is over 24 hours old
- status: done
- owner: claude-main
- priority: P1 · type: bug · area: test
- commit: ed481d5
- note (2026-11-17, claude-main): added
- note (2026-11-17, claude-main): done — TODO_TEST_NOW pinned in tests/cmd_apply.rs; the full suite passes again on dev

## 7. Ideas, not planned

### T-070 · MCP server
- status: dropped
- owner: —
- priority: P3 · type: idea · area: agent
- reqs: I-1
- note (2026-11-17, claude-main): dropped — Owner decision 2026-11-17: no MCP server. Agents already use the CLI with --json (D10, D14); an MCP server adds dependencies and a second interface. I-1 marked dropped in docs/requirements.md.

### T-071 · Overview across several projects
- status: dropped
- owner: —
- priority: P3 · type: idea · area: cli
- reqs: I-2
- note (2026-11-17, claude-main): dropped — Owner decision 2026-11-17: work happens per project session; todo summary already shows each project's state (I-2 dropped).

### T-072 · Claims tagged with an agent session
- status: done
- owner: r4-sonnet
- priority: P2 · type: idea · area: agent
- accept: claim, next --claim, unblock write session from --session / TODO_SESSION / CLAUDE_CODE_SESSION_ID; release --session; hidden SessionEnd hook command; hooks install/uninstall handle it
- commit: a870b0a
- reqs: R-CLAIM-7
- note (2026-11-16, r4-sonnet): done — session field after claimed; --session/TODO_SESSION/CLAUDE_CODE_SESSION_ID; release --session; hidden session-end hook command; hooks install/uninstall SessionEnd

### T-073 · todo schema
- status: done
- owner: v5-schema
- priority: P2 · type: feature · area: agent
- accept: todo schema prints JSON with version, envelope, exit codes, error codes, task fields and every command with its options; a test checks every clap subcommand and every error code appears; schema <command> prints one; --json wraps it in todo.schema.v1
- commit: 205b193
- reqs: R-SCHEMA-1
- note (2026-11-17, v5-schema): done — todo schema generated from clap at run time; ErrorCode::ALL/ExitCode::ALL with meanings, guarded by a unit test that counts enum variants; task fields from the show --json type

### T-074 · todo handoff
- status: dropped
- owner: —
- priority: P3 · type: idea · area: agent
- reqs: I-5
- note (2026-11-17, claude-main): dropped — Owner decision 2026-11-17: todo note already does this (I-5 dropped).

### T-075 · todo renumber
- status: dropped
- owner: —
- priority: P3 · type: idea · area: cli
- reqs: I-6
- note (2026-11-17, claude-main): dropped — Owner decision 2026-11-17: commit messages keep the old ID, so renumbering breaks the link to history (I-6 dropped).

### T-076 · Git merge driver for TODO.md
- status: done
- owner: v5-merge
- priority: P2 · type: feature · area: git
- accept: merge-driver run merges tasks changed on different sides cleanly (exit 0), keeps notes from both sides, conflicts (exit 1, markers around the task block) on the same field changed both ways and on the same new ID; falls back to git merge-file for prose conflicts or unparsable input; output bytes come only from inputs (CRLF kept); merge-driver install/uninstall are idempotent and a real git merge in a temp repo uses the driver
- commit: 582e47b
- reqs: R-GIT-2, R-GIT-3
- note (2026-11-17, v5-merge): done — merge-driver run/install/uninstall; tasks merged by ID over a git merge-file skeleton; notes from both sides kept; conflicts marked per task block; falls back to git merge-file; end-to-end git merge test. Limitation: default prefix T only

### T-077 · todo align
- status: done
- owner: v5-align
- priority: P2 · type: feature · area: git
- accept: todo align reports COMMITTED_NOT_DONE, DONE_WITHOUT_COMMIT and COMMIT_MISSING against a temp git repo; exit 0/1/5 as specified (5 + NO_GIT_HISTORY outside git and in a shallow clone); --json envelope todo.align.v1; never changes TODO.md
- commit: 6a0a937
- reqs: R-GIT-1
- note (2026-11-17, v5-align): done — todo align with COMMITTED_NOT_DONE, DONE_WITHOUT_COMMIT, COMMIT_MISSING; NO_GIT_HISTORY exit 5; ok envelope with exit 1 via Output::set_exit; questions skipped at merge

### T-078 · todo undo
- status: dropped
- owner: —
- priority: P3 · type: idea · area: history
- reqs: I-9
- note (2026-11-17, claude-main): dropped — Owner decision 2026-11-17: reopen, release, set and restore already reverse any change (I-9 dropped).

### T-079 · PowerShell tab completion
- status: done
- owner: r4-sonnet
- priority: P2 · type: idea · area: cli
- accept: todo completions <shell> prints scripts for powershell, bash, zsh, fish, elvish via clap_complete; README explains the PowerShell profile line
- commit: 6fce0b7
- reqs: R-OUT-10
- note (2026-11-16, r4-sonnet): done — todo completions <shell> (powershell, bash, zsh, fish, elvish) with clap_complete; README profile line

### T-080 · todo scan
- status: done
- owner: v5-scan
- priority: P2 · type: feature · area: cli
- accept: todo scan lists new TODO:/FIXME: matches as path:line: text, skips known ones (task ID or same title), TODO.md, binaries and big files; works with and without git; scan --add adds them in one locked write with files: path:line; --dry-run writes nothing
- commit: 51a1dfa
- reqs: R-DISC-1
- note (2026-11-17, v5-scan): done — todo scan and scan --add (shared add_batch_text with add --from); git ls-files or folder walk; title is the text after the marker (changed at merge)
- note (2026-11-17, claude-main): follow-up commit f6981d9: markers wrapped in a backtick or quote are skipped

### T-081 · Import from other task-list formats
- status: done
- owner: r4-sonnet
- priority: P2 · type: idea · area: cli
- accept: Checkbox list converts to format 1 per R-IMP-1: markers, title/description, suffixes, notes, question sections; non-task lines byte for byte; todo check passes; counts reported; preview without --yes; .pre-import backup
- commit: 93e0466
- reqs: R-IMP-1
- note (2026-11-16, r4-sonnet): done — todo import --from checkbox: preview/diff, --yes write with .pre-import backup, verified with todo check rules and task count; src/import.rs, src/cli/import.rs

## 8. v0.6: milestones and the HTML view for people

### T-095 · Keep the HTML report current after every write
- status: dropped
- owner: —
- priority: P2 · type: feature · area: report
- depends: T-092
- accept: with `report_file = "<path>"` in todo.toml, every successful write command rewrites the report after releasing the lock; a failure to write it is a warning and never changes the exit code; the page reloads itself every 30 seconds while open; without the setting nothing is written; the NFR-3 write limit still holds on the 5,000-task file
- reqs: R-REPORT-3, R-LOCK-4, NFR-3
- why: people only look at the page, so it must not go stale while agents work
- note (2026-11-18, claude-main): added
- note (2026-11-18, claude-main): dropped — Owner decision 2026-11-18: the HTML is built only when someone asks for it (`report --html`); no automatic rewrite after writes and no self-reloading page.

## v0.6: version stamp, migration, milestones, kinds of entry

### T-086 · v0.6 requirements: milestone tree, kinds of entry, why field, HTML report, project register
- status: done
- owner: t086-opus
- priority: P1 · type: docs · area: docs
- accept: docs/requirements.md defines R-MILE-1 to R-MILE-6, R-ARCH-2, R-KIND-1 to R-KIND-3, R-WRITE-17, R-WRITE-18, R-VER-1, R-VER-2, R-MIG-1, R-MIG-2, R-OUT-11 (v0.6) and R-REPORT-1, R-REPORT-2, R-PROJ-1, R-PROJ-2 (v0.7) as the tasks describe them; section 2 says the HTML report is the read-only view for people, built only on request, and command output is written for agents; D11 is amended: the project register is the one per-user file; section 13 allows a generated HTML file (terminal UI, desktop app, web server and any control on the page stay out), says todo never searches the PC for task lists, and keeps "no network access, no self-update": todo only prints the update steps; `difficulties` is added to the settings table; I-2 is marked moved to requirements; section 11 lists v0.6 and v0.7; docs/format.md becomes format version 2: `type: milestone`, `type: idea`, `goal`, `milestone`, `about`, `why`, `difficulty`, the `todo-tool` line, and the rule that a tool refuses a format it does not know
- commit: cbdd96d
- reqs: REL-4, NFR-8
- note (2026-11-18, claude-main): added
- note (2026-11-18, claude-main): Owner 2026-11-18: a milestone is a piece of work with a goal, divided into tasks, for example milestone 'Develop the renderer' with the tasks 'C shader' and 'GPU compiler'. It is NOT a release or version label such as 'v0.6'; write the requirements and every example that way.
- note (2026-11-18, claude-main): Owner 2026-11-18: (1) milestones may be nested to any depth; todo manages the tree. (2) All-projects overview: no scanning; projects are registered by command and kept in a per-user file. (3) Entries are kept apart by kind: work items (milestones, nested milestones, single tasks), questions (each may refer to a milestone or task), ideas; done and dropped are shown apart too.
- note (2026-11-18, claude-main): Owner 2026-11-18: todo knows its own version and TODO.md records the latest todo version that wrote it (T-099); after an update the file migrates automatically and cleanly on the first write (T-100), and `todo migrate --yes` refreshes the rules block, skill and hooks (T-101).
- note (2026-11-18, claude-main): Owner 2026-11-18: split into v0.6 (version stamp, migration, milestones, kinds) and v0.7 (HTML pages, project register). The format becomes version 2 in v0.6, so todo 0.5 can no longer write such a file (it already refuses writes to a newer format), and from v0.6 on a tool refuses every task command on a format it does not know and asks for an update (T-104). R-REPORT-3 (automatic rewrite) is not wanted.
- note (2026-11-18, claude-main): Owner 2026-11-18: add a difficulty field next to priority (T-108); no computed score. todo must be able to tell an agent how to update it: repository, download of the latest release, placing it on PATH (T-109).
- note (2026-11-18, t086-opus): done — requirements.md defines R-MILE-1..6, R-ARCH-2, R-KIND-1..3, R-WRITE-17/18, R-VER-1/2, R-MIG-1/2, R-OUT-11 (v0.6), R-REPORT-1/2, R-PROJ-1/2 (v0.7); §2, D11, §13, settings (difficulties, idea_section), I-2/I-13, §11 updated; format.md is format version 2 (tool line, kinds, new fields, TOOL_TOO_OLD, migration §18). Clarifications noted on T-087, T-088, T-094, T-097, T-101, T-103, T-104, T-108.

### T-087 · Milestones: a tree of milestones with goals, divided into tasks
- status: done
- owner: t087-haiku
- priority: P1 · type: feature · area: core
- depends: T-086
- accept: a task with `type: milestone` carries `goal:` (`add --type milestone --goal T`, `set --goal`); `add`, `set`, `add --from` and `apply` take `--milestone <task>` naming the parent, on tasks and on milestones, so milestones nest to any depth; a parent that is missing or not a milestone, and a change that would make a milestone its own ancestor, are refused (exit 1); `set --milestone` moves an entry with everything below it to another parent and `set --unset milestone` makes it top-level; `next` and `list --ready` never return a milestone and `claim` of one is refused; `done` on a milestone is refused without --force while anything below it is open, claimed or blocked; `drop --tree` drops a milestone and everything open below it in one locked step; `show --json` returns `goal`, `milestone`, the chain of parents and the direct children; every refusal leaves the file byte-identical
- commit: e330c88650fff00ce3bddf275d30a23c75a5600f
- reqs: R-MILE-1, R-MILE-2, TEST-4
- why: agents and people need to see which larger piece of work a task belongs to and what that work is for
- note (2026-11-18, claude-main): added
- note (2026-11-18, claude-main): Owner 2026-11-18: a milestone is a piece of work such as 'Develop the renderer', divided into tasks such as 'C shader' and 'GPU compiler'; never a release label. Use examples of that kind in help text, tests and fixtures. Once this works, group the remaining v0.6 tasks under milestones of that kind (for example 'HTML view for people', 'Overview of all projects').
- note (2026-11-18, claude-main): Owner 2026-11-18: nesting is allowed (this replaces the earlier one-level rule). The tree is logical: entries stay where they are in the file and only the `milestone` field links them.
- note (2026-11-18, t086-opus): T-086 clarifications in R-MILE-1/R-MILE-2: add refuses a milestone without --goal (MISSING_FIELD); missing parent = NOT_FOUND, non-milestone parent or cycle = BAD_VALUE; set --type on a milestone with entries below is refused; claim/done refusals use BAD_TRANSITION; plain drop of a milestone with unfinished entries is refused and names drop --tree; drop --tree covers open, claimed and blocked entries (claims of others need --force); add --from accepts "milestone" as an "@ref". Field order and notes: format.md §9, §10, §17.
- note (2026-11-18, t087-haiku): R-MILE-1 and R-MILE-2 clarified in docs/requirements.md: <task> may be an ID or a title and is stored as the ID; an empty value is BAD_VALUE; a question or idea naming a milestone is BAD_VALUE; drop --tree on a task is BAD_VALUE. Built: milestone and goal are parsed fields (also in the combined priority line); format.md section 10 order now has milestone and goal after area. Merge note for T-091 and T-108: both add parser fields and CANONICAL_FIELD_ORDER entries next to these, and known_field_index only tracks keys of 9 characters or fewer (difficulty is 10, so it needs the length guard raised); KNOWN_FIELD_SLOTS in parser.rs must grow with them.
- note (2026-11-18, t087-haiku): done — Milestones: goal and milestone fields (also in the combined priority line), add/set/add --from/apply take the parent with @ref in batches, tree refusals before any write, next/list/claim exclude milestones, done and drop refuse while entries below are open, claimed or blocked (--force on done notes the count), drop --tree in one locked write, show --json gains goal, milestone, milestones and children. Tests: tests/cmd_milestone.rs (14), doc_parser order updated; fmt, clippy (Windows and Linux) and 248 tests pass.
- note (2026-11-18, t087-haiku): Merged dev (T-098, T-099, T-108) into reynanTandoyo/t087 as b501a51. Conflicts in set.rs, doc/mod.rs, parser.rs, read.rs, task_ops.rs and doc_parser.rs were resolved so both sides work. The pre-merge dev head had cmd_difficulty tests that expected no tool stamp, while T-099 adds one on every write without one; those expectations now include the stamp (written helper), and the milestone tests do the same (stamped helper). Dev should be re-checked: its head (ce13d36) fails tests/cmd_difficulty.rs until this branch is merged.
- note (2026-11-18, t087-haiku): Correction to the note above: the claim that dev's head fails tests/cmd_difficulty.rs is inferred, not run. The failures appeared only on the merged tree; the stamp code is dev's and this branch does not change it. Run cmd_difficulty on dev to confirm.

### T-088 · todo tree: the milestone tree with progress; list --milestone and --kind; milestones in summary
- status: done
- owner: t088-haiku
- priority: P1 · type: feature · area: cli
- depends: T-087
- accept: `todo tree [<milestone>]` prints milestones and their tasks indented by depth, each milestone with its goal and done/total counted over every task below it, top-level tasks without a milestone last; done and dropped entries are hidden unless `--all`; `list --milestone <task>` lists everything below a milestone and `list --kind milestone|task|question|idea` filters by kind; `summary` shows the top-level open milestones with done/total and stays within 40 lines; `tree --json` is nested and matches its snapshot
- commit: 7c17d1fb6a9cf18d4bba188ef56f9f9a7435da0f
- reqs: R-MILE-3, R-READ-4
- note (2026-11-18, claude-main): added
- note (2026-11-18, claude-main): Owner 2026-11-18: categories are the existing `area` field. `tree --area <list>` shows only matching tasks with the milestones above them; `tree` prints the area next to each entry that has one.
- note (2026-11-18, t086-opus): T-086: progress done/total (R-MILE-3, format.md §17) counts tasks only, at any depth; total excludes dropped tasks. tree shows work only (milestones and tasks); questions and ideas are listed apart (R-KIND-3).
- note (2026-11-18, claude-main): Moved from T-108 (coordinator 2026-11-18): tree lines also show each task's difficulty (R-WRITE-18), like list does.
- note (2026-11-18, t103-haiku): From T-103 (archive keeps trees whole, commit 0c04fa6): tree --all and list --archive --milestone must also find archived entries, read from TODO-archive.md. An archived milestone tree is complete in the archive file (all its entries are there), so entries_below and children_of work on the archive document. Entries of a tree are archived only when the whole tree is finished.
- note (2026-11-18, t088-haiku): done — todo tree (text, nested --json, --area, --all also reads archived entries from TODO-archive.md, a milestone argument); list --milestone and list --kind (comma-separated); summary lists the top-level milestones that are open or blocked with done/total. R-MILE-3 and R-READ-5 clarified in docs/requirements.md; docs/json.md has the tree shape. 15 tree tests; the merged tree passes fmt, clippy (native and Linux) and 375 tests.

### T-089 · Show the milestone and its goal with each task
- status: done
- owner: t089-haiku
- priority: P1 · type: feature · area: cli
- depends: T-087
- accept: for an entry with a milestone, `show`, `next`, `claim` and `next --claim` print the chain of milestones from the top down, each with ID, title and goal, after the task, and their JSON carries `milestones: [{id, title, goal, status}]` in that order; entries without a milestone print exactly what they print today
- commit: 7af9e9d3237606feb7918c17291a9d8e116f0b30
- reqs: R-MILE-4
- why: an agent that takes a task should learn the point of it without a second command
- note (2026-11-18, claude-main): added
- note (2026-11-18, t089-haiku): R-MILE-4 clarified in docs/requirements.md: the text layout is fixed (the chain follows the questions in show; next, claim and next --claim print it under the task line; heading 'Milestones above <ID>, from the top down:', then '- <ID> · <title>' and '  goal: <goal>', goal: - when none). IDs unchanged.
- note (2026-11-18, t089-haiku): done — show, next, claim and next --claim print the milestones above an entry, top down, with ID, title and goal (text after the task; JSON milestones unchanged). Entries without a milestone print as before. Tests: tests/cmd_milestone_chain.rs (10 new); tests/cmd_show.rs expectation updated for the chain. fmt, clippy (Windows and Linux) and 313 tests pass (303 before, 0 failed).

### T-090 · check: milestone and reference problems
- status: done
- owner: t090-haiku
- priority: P1 · type: feature · area: check
- depends: T-087, T-098
- accept: a fixture for each problem is reported with its line: a `milestone` field naming a missing ID or an entry that is not a milestone; a cycle of milestones; a done or dropped milestone with anything below it still open, claimed or blocked; a milestone without a goal; an `about` field naming a missing ID; a clean fixture with three levels exits 0
- commit: 75b5700ed5c65e4449b9098dace86c397b1b983b
- reqs: R-MILE-5
- note (2026-11-18, claude-main): added
- note (2026-11-18, t090-haiku): R-MILE-5 clarified in docs/requirements.md (IDs unchanged): an ID is missing only when neither TODO.md nor TODO-archive.md holds it (R-DEP-3); each problem is reported at the line that holds it (the milestone or about line for a bad reference, the milestone line of the entry that closes a cycle, the heading of a milestone without goal, the heading of a done or dropped milestone with unfinished entries below it, and the heading of an unfinished entry below an archived done or dropped milestone, R-ARCH-2).
- note (2026-11-18, t090-haiku): done — check reports milestone and reference problems (R-MILE-5): a milestone field naming a missing ID or a non-milestone, a milestone cycle, a done or dropped milestone with unfinished entries below it, a milestone without a goal, an about field naming a missing ID, and an unfinished entry below an archived done or dropped milestone, each with its line; one invented fixture per problem plus a clean three-level fixture (tests/cmd_check_milestones.rs, 7 tests); R-MILE-5 clarified (lines, archive). Full suite 310 passed; fmt, clippy (Windows and Linux) clean. Filed T-120: existing cmd_check tests inherit TODO_FILE.

### T-091 · why field: the purpose of a task
- status: done
- owner: t091-haiku
- priority: P1 · type: feature · area: cli
- depends: T-086
- accept: `add --why T`, `set --why T` and `"why"` in `add --from` write `- why:` before `problem` in the field order; `-` and `@path` work; `show --json` and `schema` list it; existing files with a hand-written `why` field keep working
- commit: 3d199112e9ef038c6cc6b33f9165e9934644406d
- reqs: R-WRITE-17, TEST-4
- note (2026-11-18, claude-main): added
- note (2026-11-18, t091-haiku): done — why field built: add --why, set --why, why in add --from and apply write - why: before depends and problem; show --json and schema list it; list --where why= matches; a hand-written why line is read as the field. Tests: tests/cmd_why.rs (10) and tests/doc_parser.rs (2 new). Found and recorded: T-117 (combined priority line parts other than type and area), T-118 (repeated-field check skips keys over 8 characters).

### T-096 · v0.6 documentation, agent rules and release 0.6.0
- status: done
- owner: claude-main
- priority: P1 · type: docs · area: release
- depends: T-089, T-090, T-091, T-097, T-098, T-101, T-102, T-103, T-104, T-105, T-108, T-109
- accept: the rules text tells agents to put each new task under a milestone with a why, to read the goals above a task before starting, to close a milestone when its last task is finished, to use `idea` for ideas and `ask --about` for questions, and to run `todo migrate --yes` after an update; the README explains milestones, kinds, the version stamp and what happens when todo is too old for a file; docs/json.md documents every new JSON shape and `TOOL_TOO_OLD`; CHANGELOG.md has the 0.6.0 entry including the format change; Cargo.toml is 0.6.0; `agents install` and `skill install` were re-run in this repository and `todo check` is clean
- commit: c4eb3ff9b18e03e5755e7c9c992a4b4c7f1ee248
- reqs: R-AGENT-1, REL-2, REL-5
- by: Reynan Henry Tandoyo
- note (2026-11-18, claude-main): added
- note (2026-11-18, claude-main): Moved from T-109 (coordinator 2026-11-18): after the 0.6.0 release is published, run 'todo help update' by hand on this PC, follow its steps, and confirm todo --version shows 0.6.0. Also covers T-123 (README format 1 example) and T-124 (rules text line 'todo migrate --yes'), both already in this accept line.
- note (2026-11-18, t096-haiku): Local part finished on reynanTandoyo/t096 in commit 619da76 (T-123 done with it): rules text, README, docs/json.md (tests/docs_json.rs checks the examples against real output), CHANGELOG 0.6.0, Cargo.toml and Cargo.lock 0.6.0, AGENTS.md and the skill regenerated by the 0.6.0 binary with --file on the worktree copy. Full suite 505 passed, 0 failed (2 perf tests ignored, CI runs them separately); fmt and clippy clean for Windows and Linux. Still for the owner, so T-096 stays open: tag and push, publish the GitHub release, install 0.6.0 on this PC, todo migrate --yes on the shared TODO.md, and the by-hand todo help update run from the T-109 note. Check on the shared list with the new binary: one problem, AGENTS.md:65 outdated agent rules block in C:\Project\todo (todo migrate --yes or agents install --yes after the install clears it).
- note (2026-11-18, claude-main): Released 2026-11-18: tag v0.6.0 on c4eb3ff, GitHub release published by the Release workflow. Followed todo help update by hand on this PC: gh release view gave v0.6.0, downloaded todo.exe and todo.exe.sha256, SHA-256 matched, replaced C:\Users\reyna\AppData\Local\Programs\todo\todo.exe (was 0.4.0), todo --version prints 'todo 0.6.0 (c4eb3ff)'. todo migrate --yes moved TODO.md and TODO-archive.md to format 2; todo check is clean.
- note (2026-11-18, claude-main): forced done (owner was t096-haiku)
- note (2026-11-18, claude-main): done — v0.6 docs, agent rules and 0.6.0 released and installed.

### T-097 · Ideas as their own kind: todo idea and promote
- status: done
- owner: t097-haiku
- priority: P1 · type: feature · area: cli
- depends: T-086
- accept: `todo idea <text> [--about <task>]` adds an entry of `type: idea` to the section named by `idea_section` (default `Ideas`, added at the end of the file if missing) and needs no priority; `next` never returns an idea; `summary` counts ideas apart from open tasks; `promote <idea> --priority P [--milestone <task>] [--section S]` turns it into an ordinary task with a note; a test covers each case
- commit: 38dff941a676cd63af603fc8dc1c4336ecf5e344
- reqs: R-KIND-1, R-KIND-3
- why: ideas must be kept without being handed out as work or counted as open tasks
- note (2026-11-18, claude-main): added
- note (2026-11-18, t086-opus): T-086: R-KIND-1 details: promote removes type: idea, sets priority/milestone, keeps 'about', moves only when --section is given, adds the note 'promoted from idea'; promote of a non-idea is BAD_TRANSITION; idea ignores required_fields.
- note (2026-11-18, t097-haiku): R-KIND-1 clarified in docs/requirements.md (same commit): idea refuses a title equal to an active task (DUPLICATE_TITLE, as add and ask) and an --about ID that doesn't exist (NOT_FOUND, as ask --about); promote needs --priority, --milestone must name a milestone (NOT_FOUND when missing, BAD_VALUE otherwise), keeps status and owner (ownership rule of set), and moves the entry only when --section is given. summary: ideas that are not done or dropped are counted in counts.ideas and an 'ideas: N' line, and left out of total and the open counts.
- note (2026-11-18, t097-haiku): Decisions outside the accept line, left as built: claim still accepts an idea by explicit ID (next never hands it out, and next never returns an idea even one the agent holds); promote of a done or dropped idea keeps that status (no extra refusal). The owner may want claim refused for ideas, as for milestones.
- note (2026-11-18, t097-haiku): done — idea and promote built: idea adds type idea to idea_section (no priority, DUPLICATE_TITLE and NOT_FOUND --about refusals); promote --priority [--milestone] [--section] turns an idea into a task with note 'promoted from idea'; next and readiness hand out only kind task (EntryKind in doc/kind.rs); summary counts ideas apart (counts.ideas); section_move_edits shared with move; 16 new tests, snapshots updated; fmt, clippy (Windows and Linux) and cargo test --all-features pass (251 passed).

### T-098 · Questions that refer to a milestone or task
- status: done
- owner: t098-haiku
- priority: P1 · type: feature · area: cli
- depends: T-086
- accept: `ask <question> --about <task>` writes `- about: <ID>` (refused when the ID does not exist); `show` of a milestone or task lists the questions about it with their status, in text and JSON; `answer` prints the entry the question was about; `list --where about=<ID>` works; questions without `--about` behave as today
- commit: ecf8400731e4b103416338ebc5708938d9be0687
- reqs: R-KIND-2
- why: a person answering a question needs to see what it is about, and an agent on that task needs to see the open question
- note (2026-11-18, claude-main): added
- note (2026-11-18, t098-haiku): R-KIND-2 clarified in docs/requirements.md (ecf8400): --about takes an ID only (other values BAD_VALUE, file untouched); an entry in TODO-archive.md counts as existing; show lists questions from both files, archived ones marked; answer prints the about entry after the result (text: block under 'About <ID>:'; JSON: under about). docs/json.md updated.
- note (2026-11-18, t098-haiku): done — ask --about writes about: <ID> (NOT_FOUND for unknown IDs, BAD_VALUE for non-IDs, file untouched); show lists questions about an entry from TODO.md and the archive, in text and as questions in JSON; answer prints the about entry (text block, JSON about); list --where about=<ID> works; R-KIND-2 and docs/json.md updated. 244 tests pass; fmt, clippy (Windows and Linux) clean.

### T-099 · Tool version stamp in TODO.md
- status: done
- owner: t099-haiku
- priority: P1 · type: feature · area: core
- depends: T-086
- accept: a line `<!-- todo-tool: 0.6.0 -->` directly after the format line records the highest todo version that has written the file; `init` writes it; a write command adds or raises it in the same write when its own version is higher (compared number by number, so 0.10.0 is above 0.9.0) and never lowers it; read commands never write it; a file without the line counts as written by 0.5.0 or older; an older tool on a file stamped newer with a format it knows still works and warns on standard error that it is behind; `where` (text and JSON) shows the stamp and the running version; the archive file gets the same line; the merge driver takes the higher stamp when both sides changed the line instead of reporting a conflict; docs/format.md describes the line
- commit: 59508b46329627ad84c1a3388b4fc3d7aa507b4a
- reqs: R-VER-1, R-EDIT-7, NFR-8
- why: the file must say which todo version last shaped it, so a newer tool knows what to migrate and an older tool knows it is behind
- note (2026-11-18, claude-main): added
- note (2026-11-18, t099-haiku): Decisions: the stamp is the running version from Cargo.toml (0.5.0 until T-096 sets 0.6.0). A write adds a missing stamp and raises a lower one; a newer stamp is kept and warned about, and an invalid value is kept and reported. Without a format line the stamp goes first. The merge driver takes the higher stamp but does not raise it to the running version (R-GIT-2: merged lines come from the inputs).
- note (2026-11-18, t099-haiku): Clarified R-VER-1 and docs/format.md section 4: a write adds the stamp when it is missing (not only when its version is higher) and raises it when lower; the no-format-line placement and the invalid-value rule are now written down. The warning names todo help update, which T-109 adds; until then the warning points at a command that does not exist yet.
- note (2026-11-18, t099-haiku): done — Every write adds or raises the tool stamp <!-- todo-tool: X --> in the same write (init, archive file, restore, import, apply and task commands); reads never write it. A newer stamp warns on standard error or in JSON warnings and is never lowered; an invalid value is kept and reported. where shows the stamp and the running version; the merge driver takes the higher stamp instead of a conflict. Tests: tests/cmd_tool_stamp.rs, tests/doc_tool_stamp.rs, unit tests in src/version.rs. Full suite 265 passed, 0 failed; fmt, clippy (Windows and Linux) clean.

### T-100 · Migrations: automatic on the first write, and todo migrate
- status: done
- owner: t100-haiku
- priority: P1 · type: feature · area: core
- depends: T-099
- accept: migration steps are an ordered list in the code, each from one format version to the next, each changing whole lines only; the first real step is format 1 to 2 (raise the format line, add the stamp; task lines stay as they are); a write command on a file in an older format first applies every pending step under the same lock and in the same write, after the usual snapshot, then raises the format line and the stamp; the archive file is migrated in the same step; the migrated text must parse without write-blocking problems, otherwise nothing is written and the command exits 4; one line on standard error and one journal entry say what was migrated; `todo migrate` does the same without another change and `--dry-run` prints the diff; a second run changes nothing; read commands read every older format and never write; `restore` of a snapshot from before the migration works; tested on LF, CRLF and byte-order-mark fixtures with every byte outside the changed lines identical, and a two-step chain is tested with a test-only step behind the `test-hooks` feature
- commit: ba2267855bcb0702a6e36c2c76f3a27fdcc1db6b
- reqs: R-MIG-1, R-EDIT-7, R-HIST-2, R-LOCK-1
- why: after todo is updated, each TODO.md must move to the new format by itself, without hand edits and without losing a byte it should keep
- note (2026-11-18, claude-main): added
- note (2026-11-18, t100-haiku): t100-haiku: built. Format 2 is now current (CURRENT_FORMAT_VERSION in src/doc/parser.rs); format 1 is read and migrated; 0 and 3+ stay unknown (BlockAllWrites, TOOL_TOO_OLD is left to T-104). Steps: src/doc/migrate.rs (STEPS, FORMAT_1_TO_2, migrate_document); each step edits the preamble only and is checked byte for byte. src/ops/migrate.rs plans TODO.md and the archive; the pipeline, apply and migrate apply it inside the lock, with the archive written after the TODO.md snapshot (lock.rs MigrationWrite). The archive command migrates its archive inside its own write.
- note (2026-11-18, t100-haiku): t100-haiku: decisions: a write that changes nothing migrates nothing (R-EDIT-6); todo migrate writes whenever something is pending. The blocking check on the migrated text covers file-wide problems; task problems keep to the command that changes the task, as for any write. init, import and a new archive write format 2 (R-IMP-1 updated). R-MIG-1 clarified. docs/json.md: todo.migrate.v1 and the journal field migrated. The unknown-version fixture bad_version_2.md is renamed bad_version_99.md because format 2 is known now; T-104 should use the 99 fixture.
- note (2026-11-18, t100-haiku): t100-haiku: fixed on this branch, not caused by T-100: tests/cmd_difficulty.rs failed on dev (T-108 expected no tool stamp, T-099 adds one); its doc() helper now writes the current format with the stamp.
- note (2026-11-18, t100-haiku): done — Format 2 is current; format 1 is read and migrated by the first write (lock, snapshot, same write; archive too) and by todo migrate (--dry-run prints the diff). Steps are whole preamble lines, checked byte for byte; a blocked result exits 4 with nothing written. One stderr line and a journal migrated field. Tests: tests/cmd_migrate.rs (13), tests/doc_migrate.rs (10, the two-step chain under test-hooks), unit tests in src/ops/migrate.rs (4); full suite 315 passed, 1 timing flake (lock_lease, T-123); fmt and clippy clean on Windows and Linux.

### T-101 · todo migrate also brings the installed project files up to date
- status: done
- owner: t101-haiku
- priority: P1 · type: feature · area: agent
- depends: T-100
- accept: when the stamp is older than the running tool, `todo migrate` lists what is out of date in the project: the rules block in every file that has one, the Claude Code skill and the Claude Code hooks; `todo migrate --yes` updates exactly those that are already installed and never installs one that was not; each file changes only inside the parts todo owns; `summary` shows one line naming `todo migrate --yes` while anything is out of date, within the 40 lines; after it runs, `todo check` and `agents check` are clean; a second run changes nothing
- commit: 20e17b8881b0bd0b51cfa89240b5c971443a06b7
- reqs: R-MIG-2, R-AGENT-3, R-AGENT-4, R-AGENT-5, R-WRITE-14
- why: after an update, the rules block, the skill and the hooks in each project are stale until someone re-runs three install commands
- note (2026-11-18, claude-main): added
- note (2026-11-18, t086-opus): T-086: T-100 and T-101 conflict: any write (and plain 'todo migrate') raises the stamp first, so 'when the stamp is older' would never trigger afterwards. R-MIG-2 decides out-of-date by comparing with what the running tool would write (as 'agents check' does), not by the stamp.
- note (2026-11-18, t101-haiku): t101-haiku: built. migrate lists out-of-date project files; --yes updates them. Decisions: the project folder is the folder of TODO.md (the rule check already uses for AGENTS.md and CLAUDE.md), and the skill and settings file sit under it. The rules block of TODO.md is updated in the same locked write as the format migration, and only with --yes. Hooks are updated per event, only where the managed entry exists: the entry text is replaced in place, the other bytes are kept, and .claude/settings.json.bak is written first; a missing event is not added. The skill is compared with CRLF ignored, as agents check does. Without --yes, migrate still moves TODO.md's format (as before) and lists the rest; --dry-run shows the diffs that --yes would write. R-MIG-2 clarified; R-AGENT-4 said two hooks, hooks install adds three (SessionEnd, R-CLAIM-7): fixed in docs/requirements.md.
- note (2026-11-18, t101-haiku): done — built: migrate lists the out-of-date rules blocks (TODO.md, AGENTS.md, CLAUDE.md), the skill and the hooks, and --yes updates exactly those installed; hooks are edited per event in place with every other byte kept (backup first); summary names todo migrate --yes on line 2. Checks: cargo fmt --check clean; clippy -D warnings clean on Windows and Linux; cargo test --all-features 454 passed, 0 failed (9 new in tests/cmd_migrate_project.rs, 45 lib unit tests incl. the hooks scanner). Spec: R-MIG-2 clarified, R-AGENT-4 says three hooks (was two). Problems filed: T-126 (hooks install re-serializes settings.json, keys sorted), T-127 (project folder differs between check/migrate and the install commands).

### T-102 · Readiness and next follow the milestone tree
- status: done
- owner: t102-haiku
- priority: P1 · type: feature · area: core
- depends: T-087
- accept: a task is ready only when its own dependencies and those of every milestone above it are finished, and no milestone above it is blocked, done or dropped; `depends` may name a milestone, which counts as finished only when the milestone itself is done or dropped; dependency cycles through milestones are found by `check`; `next --milestone <task>` returns only tasks below that milestone; among ready tasks `next` orders by priority, then tasks in a milestone where work has already started (something below it is claimed or done) before untouched milestones, then file order; 10 parallel `next --claim` still get 10 different tasks; when `done` or `drop` finishes the last unfinished entry below a milestone, the output names the milestone as ready to close with the command to use, and `summary` lists milestones that are ready to close; a milestone is never closed automatically
- commit: 03bfaf937ed945e7b7f34b0ca767ae5ec7347e2d
- reqs: R-MILE-6, R-READ-3, R-DEP-1, R-DEP-3
- why: work must be handed out along the tree: finish what is started, respect what a whole milestone waits for, and never leave a finished milestone open unnoticed
- note (2026-11-18, claude-main): added
- note (2026-11-18, t102-haiku): done — Readiness follows the milestone tree: a task waits for the depends of every milestone above it and no milestone above it may be blocked, done or dropped; next gains --milestone and orders started milestones first; check finds dependency cycles through milestones; done and drop name milestones ready to close (JSON ready_to_close) and summary lists them, using the same order as next; now-ready output follows milestone dependencies. 9 tests in tests/cmd_ready_tree.rs; docs/requirements.md R-MILE-6 and docs/json.md updated.
- note (2026-11-18, t102-haiku): Decisions taken while building T-102 (requirements R-MILE-6 updated to say so): a cycle through milestones is reported only when it holds a depends edge, so a loop made of milestone parents alone is left to the milestone check (R-MILE-5). A milestone is ready to close when it is open or blocked, has entries below it, and all of them are done or dropped; an empty milestone is never listed. A claimed entry below a milestone counts as started, stale claims included, as the spec says claimed. Tasks outside any milestone count as untouched. next --milestone refuses a task that is not a milestone with BAD_VALUE and an unknown one with NOT_FOUND. The now-ready output of done and drop also follows milestone dependencies and a blocked milestone being unblocked, not only depends lines.

### T-103 · archive keeps unfinished milestones whole
- status: done
- owner: t103-haiku
- priority: P1 · type: feature · area: cli
- depends: T-087
- accept: `archive` moves an entry that has a milestone only when its top-level milestone is done or dropped, and then moves that whole tree in one step; entries without a milestone are archived as today; the preview and the result say how many finished entries were kept back and why; so every milestone still in TODO.md has all its entries there and its progress count is right; `show`, `tree --all` and `list --archive --milestone` find an archived tree; `--older-than` applies to the top-level milestone
- commit: 0c04fa6b74dca7d8808b5332c27e77919e0b79cf
- reqs: R-ARCH-2, R-ARCH-1
- why: archiving finished tasks out of an open milestone would make its progress wrong and hide what was done for it
- note (2026-11-18, claude-main): added
- note (2026-11-18, t086-opus): T-086: R-ARCH-2 adds one safety condition: a tree moves only when its top-level milestone is done or dropped AND nothing in the tree is still open, claimed or blocked (a milestone closed with --force keeps its open work in TODO.md).
- note (2026-11-18, t103-haiku): R-ARCH-2 clarified in docs/requirements.md for damaged files: an entry whose milestone chain does not reach a top-level milestone stays; so does a tree that an entry outside it still names, and a finished non-milestone that other entries name as their milestone. The JSON of archive gains kept_back (docs/json.md). NOT verified in this branch: 'tree --all' and 'list --archive --milestone' in the accept line, because tree and list --milestone come from T-088, which is not in dev yet. show and list --archive are tested (tests/cmd_archive_milestones.rs). T-088 must also read TODO-archive.md for tree --all and for list --archive --milestone; check that after T-088 lands.
- note (2026-11-18, t103-haiku): done — archive moves a milestone tree whole (R-ARCH-2): an entry in a milestone moves only when its top-level milestone is done or dropped and nothing in the tree is open, claimed or blocked, as one locked write; --older-than measures the top-level milestone. The preview and the result name the finished entries kept back and why; JSON gains kept_back. Damaged chains and trees named from outside stay. Tests: tests/cmd_archive_milestones.rs (10). Not verified here: tree --all and list --archive --milestone (T-088 owns tree and list --milestone).

### T-104 · Refuse to work on a TODO.md that needs a newer todo
- status: done
- owner: t104-haiku
- priority: P1 · type: feature · area: core
- depends: T-099
- accept: when the format line names a version the running tool does not know, every command that reads or writes tasks refuses with exit 4 and the new error code `TOOL_TOO_OLD`; the message names the todo version stamped in the file, the running version, and says to update the todo executable first; only `where`, `help`, `schema`, `completions` and `--version` still work; nothing is written; a file with a known format and a higher stamp only warns (T-099); docs/format.md section 4 and R-EDIT-8 are changed to say this; tested with a fixture whose format line names version 99
- commit: ff208ce2cb3f3b199660a71f5941447738b99ca2
- reqs: R-VER-2, R-OUT-3
- why: an old todo that half-understands a newer file would hand out milestones or ideas as ordinary tasks; the owner wants it to stop and ask for an update instead
- note (2026-11-18, claude-main): added
- note (2026-11-18, t086-opus): T-086: format.md §4/§13 and R-EDIT-8 already say this (written by T-086). Clarified in R-VER-2: the per-user commands (project, overview, report --html --all) are not blocked; they show a too-new project with TOOL_TOO_OLD and carry on with the others.
- note (2026-11-18, t104-haiku): commit 1d12d90 on reynanTandoyo/t104: a TODO.md in a format this todo does not know is refused by every command that reads or changes tasks (exit 4, TOOL_TOO_OLD, names the stamped and the running version); where shows the format and stamp without counting tasks; tests in tests/cmd_format_newer.rs with fixture format_newer_99.md. fmt and clippy (Windows and Linux) clean. cargo test --all-features: 296 passed, 3 failed, all in tests/cmd_difficulty.rs (T-119), which fail the same way on the untouched base. Not marked done until T-119 is fixed.
- note (2026-11-18, t104-haiku): decision (t104-haiku, to confirm): 'only where, help, schema, completions and --version still work' is read as: commands that interpret or change task content refuse. status, history, diff, lock, unlock, agents, skill, hooks, guard, session-end and merge-driver never read task content and keep working; merge-driver also falls back to a plain text merge (R-GIT-2). Written into format.md section 4 and R-VER-2.
- note (2026-11-18, claude-main): Coordinator 2026-11-18: scope confirmed. The accept line's 'only where, help, schema, completions, --version' means commands that interpret or change tasks. status, history, diff, lock, unlock, agents, skill, hooks, guard, session-end keep working; merge-driver falls back to a plain text merge. restore is refused.
- note (2026-11-18, t104-haiku): done — Refuses a TODO.md in a newer format with TOOL_TOO_OLD (exit 4) in every command that reads or changes task content, the milestone routes included; where shows the format and stamp without task counts; format.md section 4, R-VER-2 and json.md say which commands still work. Tests: tests/cmd_format_newer.rs (11) on fixture format_newer_99.md. dev (T-087) merged with no conflicts. fmt and clippy (Windows and Linux) clean; cargo test --all-features: 314 passed, 0 failed.

### T-105 · Performance test with a milestone tree
- status: done
- owner: t105-haiku
- priority: P1 · type: test · area: core
- depends: T-088, T-102
- accept: the generated 5,000-task file gets a variant with a milestone tree four levels deep; on it `tree`, `next`, `list --milestone` and `show` take under 100 ms and a write under 200 ms in a release build; the test runs in CI with the existing performance test and asserts those limits
- commit: 65f76ce80f97bfb83e182734df08525ec282955e
- reqs: NFR-3, TEST-9
- note (2026-11-18, claude-main): added
- note (2026-11-18, t105-haiku): Commit 65f76ce on t105 adds perf_5000_entry_milestone_tree_meets_nfr3_budgets (tests/perf.rs). Release timings, median of five, 5,000 entries (370 milestones in three nested levels, tasks at depth 4, 972,745 bytes): tree 358 ms (budget 100) FAILS; next 34, list --milestone 31, show 31, note 88, add --milestone 77 ms pass. Flat test (perf_5000_tasks...) passes: list 23, show 23, note 56, add 57 ms. Not done: the accept line is not met until tree is fixed (recorded as T-125). Left claimed; not merged.
- note (2026-11-18, t105-haiku): done — Perf test perf_5000_entry_milestone_tree_meets_nfr3_budgets (tests/perf.rs) added and passing in release: tree 34 ms, next 33, list --milestone 30, show 31, note 74, add --milestone 74 ms (all within budget). Flat test passes too. The tree defect it found is fixed by T-125 (5e613bd).

### T-108 · difficulty field: how hard a task is, for choosing who takes it
- status: done
- owner: claude-main
- priority: P1 · type: feature · area: cli
- depends: T-086
- accept: `add --difficulty D`, `set --difficulty D` and `"difficulty"` in `add --from` write the field, in the combined priority line like `type` and `area`; allowed values come from `difficulties` in todo.toml (default `easy`, `medium`, `hard`, easiest first) and anything else is refused (`BAD_VALUE`); `list --difficulty <list>` filters; `next --difficulty <list>` and `next --max-difficulty D` hand out only matching tasks, and a task without a difficulty is handed out only when no difficulty option is given; `tree` and `list` show it; `show --json` and `schema` list it; it is a plain field: nothing is computed from it and the order of `next` does not change
- commit: 5615fae1a23e1f571dd898c8f1ea86bbce449c7b
- reqs: R-WRITE-18, R-READ-1, R-READ-3, R-CFG-2
- why: a small model should take easy tasks and leave hard ones to a stronger one; priority says how urgent a task is, not how hard
- note (2026-11-18, claude-main): added
- note (2026-11-18, t086-opus): T-086: field order is priority, difficulty, type, area (format.md §10); a new difficulty goes into the combined line right after the priority.
- note (2026-11-18, t108-haiku): Committed on t108 as 5615fae; checks pass. NOT done: the accept line says tree shows difficulty, but todo tree is T-087's command and does not exist on this branch. When T-087 lands, its tree lines must show difficulty (R-WRITE-18); or make T-108 depend on T-087 and finish it there. Left claimed on purpose.
- note (2026-11-18, claude-main): forced done (owner was t108-haiku)
- note (2026-11-18, claude-main): done — difficulty field merged into dev. The 'tree shows it' part moved to T-088, since tree does not exist yet.

### T-109 · todo help update: tell an agent how to update todo
- status: done
- owner: claude-main
- priority: P1 · type: feature · area: agent
- depends: T-104
- accept: `todo help update` (and `help update --json`) prints: the running version and the full path of the running executable; the repository https://github.com/reynanTandoyo/todo and its latest-release page, taken from `repository` in Cargo.toml (added there); the steps for Windows in PowerShell: read the latest version (`gh release view` or the latest-release address), download `todo.exe` and `todo.exe.sha256` from that release, compare the SHA-256, replace the executable at the path shown (or copy it to `%LOCALAPPDATA%\Programs\todo` and add that folder to the user PATH when todo is not installed yet), run `todo --version`, then `todo migrate --yes` in each project; the same for Linux with `todo-linux-x86_64`; todo itself never connects to the network, the agent runs the commands; the `TOOL_TOO_OLD` message (T-104), the behind-version warning (T-099) and the agent rules text name `todo help update`; a test checks the printed asset names against .github/workflows/release.yml; the steps were followed once by hand on this PC and the result recorded in a note
- commit: a5c0681af4e0435517f5afc9122635ebda722225
- reqs: R-OUT-11, NFR-4, REL-6
- why: when a file needs a newer todo, the agent should be able to update the executable by itself from the official repository instead of guessing
- note (2026-11-18, claude-main): added
- note (2026-11-18, t109-haiku): t109-haiku: built todo help update (src/cli/help/update.rs; the topic is dispatched in src/cli/help.rs). Repository in Cargo.toml; the rules line in src/agents/rules.md, with AGENTS.md and .claude/skills/todo/SKILL.md refreshed to match; docs/json.md shape; tests/cmd_help_update.rs (9 tests); cmd_format_newer.rs asserts TOOL_TOO_OLD names it. Not done: the by-hand run on this PC. The only release is v0.5.0, so it waits for the v0.6 release the owner pushes. Step 7 (todo migrate --yes) needs T-100 and T-101 merged, because migrate is not in dev yet.
- note (2026-11-18, t109-haiku): t109-haiku: committed a5c0681 on reynanTandoyo/t109 (fmt, clippy for Windows and Linux, full suite 385 passed, 0 failed). Not marked done: the accept line's by-hand run (download, verify, replace, todo --version, todo migrate --yes on this PC) needs a v0.6 release, and the only release is v0.5.0. Run that step after the owner pushes v0.6, then todo done T-109 with the commit and the result.
- note (2026-11-18, claude-main): forced done (owner was t109-haiku)
- note (2026-11-18, claude-main): done — help update merged into dev (7e9465e). The by-hand run against a real release moved to T-096, since no v0.6 release exists yet.

### T-110 · T-091 accept line repeats the T-098 check (ask --about, show questions, answer, list --where about=) instead of checking the why field
- status: dropped
- owner: —
- priority: P1 · type: bug · area: docs
- note (2026-11-18, t098-haiku): added
- note (2026-11-18, claude-main): dropped — Not a bug: T-091's accept line checks the why field (add/set --why, add --from, show --json, schema); it does not repeat the T-098 check.

### T-111 · docs/json.md show example uses depends_on, waiting_on_this and found, but show --json emits dependencies and dependents
- status: done
- owner: p2-kinds-haiku
- priority: P2 · type: bug · area: docs
- commit: ab7f0de80825623b7785a7988d4a92b6ef0bc3eb
- note (2026-11-18, t098-haiku): added
- note (2026-11-18, p2-kinds-haiku): done — docs/json.md show example is the real output of show --json (fixture: milestone with parent, dependency, dependent, question, task below); block_lines, depends_on, waiting_on_this and found removed; the shared TaskJson section lists every key. tests/docs_json.rs compares the key paths of the example with show --json (failed before: 8 keys missing, 8 invented). fmt, clippy (Windows and Linux) and cargo test --all-features (457 passed) pass.
- note (2026-11-18, p2-kinds-haiku): Correction to the done note: tests/docs_json.rs failed before the fix with 8 paths only in the example and about 50 only in the output (the output had the full TaskJson, with nested keys).

### T-112 · docs/json.md: where example is stale; document tool_stamp and running_version (added by T-099)
- status: done
- owner: p2-kinds-haiku
- priority: P2
- commit: 78d2aab159934b032d06ee866344087cf5d4a9be
- note (2026-11-18, t099-haiku): added
- note (2026-11-18, p2-kinds-haiku): done — docs/json.md where example is the real where --json shape (file, found_by, repo_root, state_dir, write_lock, format_version, tool_stamp, running_version, counts); todo_file, archive_*, config_* removed; tool_stamp, running_version, lease and holder explained. tests/docs_json.rs compares the example with where --json (fails on the old example). fmt, clippy (Windows and Linux) and cargo test --all-features (458 passed) pass.

### T-113 · check: warn about a difficulty that is not in difficulties, as type and area are
- status: done
- owner: p2-parser-haiku
- priority: P2 · type: feature · area: cli
- problem: T-108 added the difficulty field. todo check warns about a type or area outside the configured list (check.rs) but not about a hand-written difficulty outside difficulties; such a task is then never handed out by next --max-difficulty.
- commit: 51997c64f45a235e3d971d451a54d12c2a9db362
- note (2026-11-18, t108-haiku): added
- note (2026-11-18, p2-parser-haiku): done — check now warns about a difficulty outside difficulties, as it does for type and area (test: check_warns_about_a_difficulty_that_is_not_configured_as_it_does_for_type_and_area)

### T-114 · set --field on a key that lives in the combined priority line writes a second copy (refused with exit 4)
- status: done
- owner: p2-parser-haiku
- priority: P2 · type: bug · area: core
- problem: found while building T-087: todo set T-001 --field custom_tag=b on '- priority: P1 · custom_tag: a' inserts a second custom_tag line, so edit verification refuses the write with exit 4; the file stays unchanged, but the field cannot be changed
- commit: 547743ea655a1c6a7d8f412950b1ed8e2f1b5632
- note (2026-11-18, t087-haiku): added
- note (2026-11-18, p2-parser-haiku): done — set --field and --unset on an unknown part of the combined priority line now edit or remove it in place (no second copy, no exit 4); session and by use the same path. Test: set_edits_an_unknown_part_of_the_combined_line_in_place in tests/cmd_combined_line.rs.

### T-115 · ask writes the note 'asked', but format.md section 9 says ask adds 'added'
- status: done
- owner: p2-kinds-haiku
- priority: P2 · type: bug · area: cli
- commit: 1324d4bd69e1488d9fa4cf9730e1729a82ab966c
- note (2026-11-18, t097-haiku): added
- note (2026-11-18, p2-kinds-haiku): done — ask writes the note 'added' (format.md section 9: add, ask and idea), not 'asked'; test ask_writes_the_note_added_as_format_section_9_says in tests/cmd_ask_answer.rs. fmt, clippy (Windows and Linux) and cargo test --all-features (455 passed) pass.

### T-116 · summary counts questions as open work and in total; R-KIND-3 says questions are apart from open work
- status: done
- owner: p2-kinds-haiku
- priority: P2 · type: bug · area: cli
- commit: ebc955c4b6c21620afa84b57718f17d784f4fb4a
- note (2026-11-18, t097-haiku): added
- note (2026-11-18, t088-haiku): Same rule for milestones (found by T-088): summary's open count, total and 'open by priority' also count milestones, though a milestone is work, not an open task (R-KIND-3). Only tasks should count as open work.
- note (2026-11-18, p2-kinds-haiku): done — summary counts tasks only: status counts, total and open by priority leave out milestones (listed under milestones) and questions; counts.questions and counts.ideas count the open ones apart; blocked and claimed lists are tasks only. R-KIND-3 and docs/json.md (summary counts) updated. New test summary_counts_only_tasks_and_counts_questions_and_ideas_apart; summary_json snapshot updated (open 2->1, total 7->6, questions 1, P1 line gone). fmt, clippy (Windows and Linux) and cargo test --all-features (456 passed) pass. Not fixed here: where counts every entry (milestones, questions, ideas) as a task; see the task filed for it.

### T-117 · Combined priority line: why, problem, fix, accept, files, depends are not read as their fields
- status: done
- owner: p2-parser-haiku
- priority: P2 · type: bug · area: cli
- accept: a part such as priority: P1 · why: x or · problem: x is read as that field, set and unset edit it in place, and a test covers it
- commit: f51184c20e1ed52d703a00c5b50f6b06f95314bd
- why: format.md §8 says every other part of the combined priority line counts as its field, but the parser reads only type and area that way; a part such as why or problem stays an unknown field (show gives null, --where finds nothing), and set refuses with exit 4 (no corruption) because the field is then repeated
- note (2026-11-18, t091-haiku): added
- note (2026-11-18, p2-parser-haiku): done — Decision (format.md §8: every other part of the combined priority line counts as its field): why, problem, fix, accept, files, depends and commit are now read as fields and edited in place; a value containing ' · ' moves to its own line. Still unknown parts on that line: status, owner and claimed (claim logic; filed as a new task). Tests: tests/cmd_combined_line.rs.

### T-118 · Repeated-field check skips keys longer than 8 characters (milestone, difficulty)
- status: dropped
- owner: p2-parser-haiku
- priority: P2 · type: bug · area: cli
- accept: a repeated milestone line and a repeated difficulty line each give a RepeatedField problem, as the other known fields do; a test covers both
- why: known_field_index in src/doc/parser.rs returns None for keys longer than 8 characters, so a repeated milestone (9) or difficulty (10) line is not reported as a repeated field; widen the guard or drop it when those fields are added
- note (2026-11-18, t091-haiku): added
- note (2026-11-18, p2-parser-haiku): dropped — Not a bug on dev: known_field_index allows keys up to 10 characters, so repeated milestone (9), difficulty (10) and goal lines are already RepeatedField problems. The length guard never changed a result: a key outside the known table is checked by name (seen_unknown), so a repeated milestone was reported even with the 8-character guard this task describes (checked by reinstating it temporarily; a test for milestone, difficulty and goal passed both ways). No code changed.

### T-119 · T-108 difficulty tests fail: they expect no todo-tool line, but writes add it (3 tests in tests/cmd_difficulty.rs)
- status: dropped
- owner: —
- priority: P1 · type: bug · area: core
- note (2026-11-18, t104-haiku): added
- note (2026-11-18, claude-main): dropped — Already fixed on dev: the T-087 merge (b501a51) updated tests/cmd_difficulty.rs for the todo-tool stamp; dev passes 303/0.

### T-120 · Existing check tests in tests/cmd_check.rs fail when TODO_FILE is set: common::run_in does not clear it, so the tests read the environment's task list (run_in / todo_bin should clear TODO_FILE as clean_todo_command does)
- status: done
- owner: p2-tests-haiku
- priority: P2
- commit: b490f30c049a6aa4e57acf956461e8802dabd9f1
- note (2026-11-18, t090-haiku): added
- note (2026-11-18, p2-tests-haiku): done — Test helpers (run_in, todo_bin, clean_todo_command, assert_todo) remove every TODO_* variable todo reads, not only the session ones; new test tests/cmd_env_isolation.rs runs the checks in a child process with TODO_FILE and TODO_AGENT set and fails without the fix.

### T-121 · archive: write the archive after the TODO.md snapshot, as the migration does (it is written inside the closure today, before the snapshot is recorded)
- status: done
- owner: p2-tests-haiku
- priority: P2
- commit: 8dadb1965695d932be424a9904bec799f1176a92
- note (2026-11-18, t100-haiku): added
- note (2026-11-18, p2-tests-haiku): done — archive builds TODO-archive.md in the write closure and the lock writes it after the TODO.md snapshot and before TODO.md is replaced, as a migrated archive is; a failed snapshot no longer leaves the archive changed. Tests in tests/cmd_archive.rs block the history folder and check the archive is unchanged or not created; both fail without the change.

### T-122 · tests/lock_lease.rs status_shows_active_write_lock_holder_without_waiting asserts under 500 ms for a debug todo.exe run; measured 100-900 ms here, so it fails at random; bound it to something that still proves there is no 30 s lock wait
- status: done
- owner: p2-tests-haiku
- priority: P2
- commit: c859b78e1358ef37e5c0141b9df2487db1bb78d1
- note (2026-11-18, t100-haiku): added
- note (2026-11-18, p2-tests-haiku): done — The status timing test no longer asserts under 500 ms. It sets a 10 s lock timeout for the status run while the holder keeps the lock for 20 s, and asserts status returns before that timeout. Checked: a status slowed by 10.5 s fails the test.

### T-123 · README still says the file format is unchanged and shows a format 1 example (lines ~66 and ~306); update with T-096
- status: done
- owner: t096-haiku
- priority: P2
- commit: 619da76477247f12d5c8db035935788bc5000400
- note (2026-11-18, t100-haiku): added
- note (2026-11-18, t096-haiku): done — README: format 2, milestones, kinds, versions, migration and TOOL_TOO_OLD; the format 1 example and unchanged-format sentence are gone (commit 619da76, T-096)

### T-124 · Agent rules text lacks the line that R-AGENT-1 requires: run todo migrate --yes after todo is updated
- status: dropped
- owner: —
- priority: P2 · type: chore · area: agent
- note (2026-11-18, t109-haiku): added
- note (2026-11-18, claude-main): dropped — Duplicate: T-096's accept line already requires the rules text to tell agents to run todo migrate --yes after an update.

### T-125 · tree takes about 360 ms on the 5,000-entry milestone-tree file (NFR-3 budget 100 ms)
- status: done
- owner: t105-haiku
- priority: P1 · type: bug · area: cli
- problem: T-105 perf test (release, 5,000 entries, 370 milestones in three nested levels, tasks at depth 4): tree median 358 ms against the 100 ms budget. next 34 ms, list --milestone 31 ms, show 31 ms, note 88 ms and add --milestone 77 ms are within budget. Cause: src/cli/tree.rs node() calls children_of for every milestone and src/doc/tree.rs progress() calls entries_below for every milestone; each call scans all entries and walks each entry's milestone ancestors, so the cost grows with milestones times entries.
- accept: todo tree on the T-105 variant file takes under 100 ms in a release build; cargo test --release --test perf perf_5000_entry_milestone_tree_meets_nfr3_budgets -- --ignored passes
- commit: 5e613bde0dac598e096048c1433842eaa1ede083
- note (2026-11-18, t105-haiku): added
- note (2026-11-18, t105-haiku): done — todo tree builds the milestone tree once (MilestoneIndex, src/doc/tree.rs) instead of scanning every entry per milestone; check and summary use the same index in their per-milestone loops. Output unchanged (tree, tree --json, summary snapshots pass). Release tree on the 5,000-entry milestone-tree file: 358 ms -> 34 ms. Unit test compares every index answer with a scan on a damaged tree. 439 tests pass; fmt, clippy (Windows, Linux) clean.

### T-126 · hooks install rewrites the whole .claude/settings.json: serde_json sorts keys, so the user's key order is re-serialized (migrate --yes keeps every byte outside the managed hook entries)
- status: done
- owner: p2-install-haiku
- priority: P2 · type: bug · area: agent
- commit: c401cee2837ec6784cc63b8d01bab979b57ac137
- note (2026-11-18, t101-haiku): added
- note (2026-11-18, p2-install-haiku): done — hooks install now splices only the managed entries into .claude/settings.json (outdated entry replaced in place, missing entry/event/hooks inserted), so key order and other bytes are kept; uninstall still rewrites the parsed file; tests: unit tests in hooks_file.rs and tests/cmd_hooks.rs hooks_install_keeps_key_order_and_every_other_byte_of_the_settings (fails on the old code); R-AGENT-4 wording states the contract

### T-127 · project folder differs by command: check and migrate use the folder of TODO.md for AGENTS.md, CLAUDE.md and .claude/, while agents check, agents install, skill install and hooks install use the current folder
- status: done
- owner: p2-install-haiku
- priority: P2 · type: bug · area: agent
- commit: 47f15f4227bb746a008cf831ba3bb3b7f1c1ecbb
- note (2026-11-18, t101-haiku): added
- note (2026-11-18, p2-install-haiku): done — agents check, agents install, skill install and hooks install (and hooks uninstall) now use the folder of TODO.md, found as check finds it, not the current folder; --into FILE is relative to it; R-AGENT-3/4/5 state the rule; tests/cmd_project_folder.rs runs each command from a subfolder and fails on the old code; tests that ran in folders without TODO.md create one first

### T-128 · where counts milestones, questions and ideas as tasks (summary counts tasks only since T-116)
- status: done
- owner: p2-kinds-haiku
- priority: P2 · type: bug · area: cli
- problem: where prints 'Tasks: N total (open ...)' and its JSON counts from doc.tasks, so a milestone, a question or an idea is counted as a task. summary no longer does (R-KIND-3, T-116). The where counts should follow the same rule, or the requirement should say where may count every entry.
- accept: where and where --json count only tasks, as summary does (milestones, questions and ideas are left out); a test covers it
- commit: 7f1647a7db729215f08d50de24ae4d7b19f4f483
- note (2026-11-18, p2-kinds-haiku): added
- note (2026-11-18, p2-kinds-haiku): done — where counts tasks only per status and in total (R-KIND-3, as summary after T-116). Milestones, questions and ideas that are not done or dropped are counted apart: counts.milestones, counts.questions, counts.ideas in JSON and a 'Not tasks, not done or dropped:' line in text. R-FIND-7 and docs/json.md updated. New test where_counts_tasks_and_counts_milestones_questions_and_ideas_apart in tests/cmd_where.rs (fails on the old counts). fmt, clippy (Windows and Linux) and cargo test --all-features (474 passed) pass.

### T-129 · docs/json.md next example is stale: next --json returns claimed and already_claimed, and the task as TaskJson
- status: done
- owner: p2-kinds-haiku
- priority: P3 · type: bug · area: docs
- problem: docs/json.md's todo next example shows found, claimed and already_theirs with a short task object; the real output is data.task (full TaskJson, as show), data.claimed and data.already_claimed. Found while fixing T-111.
- accept: the next example in docs/json.md has the keys of next --json
- commit: 1e5295c112145f610a5f622074ebe75de030d2b6
- note (2026-11-18, p2-kinds-haiku): added
- note (2026-11-18, p2-kinds-haiku): done — docs/json.md next example is the real next --json output for a fixture: data.task is the TaskJson shape (or null), data.claimed and data.already_claimed; found and already_theirs removed. tests/docs_json.rs checks the example against next --json (failed before on found, already_theirs and the task keys) and checks the null task when nothing is ready. fmt, clippy (Windows and Linux) and cargo test --all-features (476 passed) pass.

### T-130 · hooks uninstall rewrites the whole .claude/settings.json: serde_json sorts the keys and reformats the rest, as hooks install did before T-126
- status: done
- owner: p2-install-haiku
- priority: P2 · type: bug · area: agent
- problem: hooks uninstall parses settings.json and writes it back whole, so key order and formatting change; R-AGENT-4 says the file's formatting is kept. Only the managed entries should change.
- fix: remove the managed entries as text, the way hooks_file splices install them (T-126); drop an event array or the hooks object only when it is left empty
- accept: hooks uninstall --yes keeps every other byte of settings.json, with keys out of alphabetical order in a test (tests/cmd_hooks.rs)
- commit: 1bc7ae8410417b258437db086d67b05ccabb897f
- note (2026-11-18, p2-install-haiku): added
- note (2026-11-18, p2-install-haiku): done — hooks uninstall cuts only the managed entries out of settings.json (the T-126 splice): an event made only of managed entries goes with its key, hooks goes when nothing is left in it, and every other byte stays; install then uninstall returns the original text; uninstall no longer creates a settings file when there is none. Tests: unit tests in hooks_file.rs and tests/cmd_hooks.rs hooks_uninstall_keeps_every_other_byte_of_the_settings and hooks_uninstall_does_not_create_a_settings_file (both fail on the old code)

### T-131 · Raw Command calls in cli_basic, cmd_add_from, cmd_apply and cmd_hooks bypass the test helpers and inherit TODO_FILE
- status: done
- owner: p2-tests-haiku
- priority: P2
- commit: 5438fe80cb27cad9581e9bcc464f6d44a7ba2c2c
- note (2026-11-18, p2-tests-haiku): added
- note (2026-11-18, p2-tests-haiku): done — Every test process in tests/ now starts todo or git through common::todo_bin, assert_todo_bin, git_command or clear_todo_environment, so no test inherits TODO_* variables. With TODO_FILE and TODO_AGENT set to an outside file the suite failed 24 tests before and passes after; the file is unchanged.
- note (2026-11-18, p2-tests-haiku): Follow-up commit 0860bfc on the same branch routes two raw todo calls that came in with the newer dev merge (cmd_hooks, cmd_where); the done record above is 5438fe8.

### T-132 · archive and migration: a failed TODO.md replace after the archive write leaves moved tasks in both files
- status: done
- owner: t132-haiku
- priority: P3
- commit: 05a8438e1e812568914332d2b2d0782a07ce8443
- by: Reynan Henry Tandoyo
- note (2026-11-18, p2-tests-haiku): added
- note (2026-11-18, p2-tests-haiku): Window left open by T-121: archive writes TODO-archive.md after the TODO.md snapshot and before TODO.md is replaced (the migration order). If the replace of TODO.md then fails, the moved tasks stay in TODO.md and are also in TODO-archive.md. Reproduced with a read-only TODO.md in a scratch folder: archive exited 4 (READ_ONLY) after creating the archive with the task, and TODO.md still held the task. A CHANGED_DURING_WRITE failure of the replace takes the same path (not reproduced). Not fixed: a fix would check that TODO.md can be replaced before the archive is written, or undo the archive write; the next archive run after such a failure was not tested.
- note (2026-11-19, t132-haiku): R-ARCH-1, R-MIG-1 and format.md section 14 now say what a write does when TODO.md cannot be replaced after the archive was written: the archive is put back as it was (a new archive is removed), and a read-only TODO.md is refused before the archive is written. Requirement IDs are unchanged.
- note (2026-11-19, t132-haiku): done — A write that moves tasks to the archive (archive, or migrating the archive) now checks that TODO.md can be replaced before it writes the archive, so a read-only TODO.md is refused with exit 4 and nothing changes. If the replace of TODO.md then fails, the archive files are put back as they were (a new archive is removed). If an archive cannot be put back, the error says it still holds the moved tasks. Tests cover read-only and changed-during-write for archive and migration paths; checks pass (fmt, clippy Windows and Linux with all features, cargo test --all-features: 512 passed).

### T-133 · Combined priority line: status, owner and claimed there are read as unknown parts
- status: done
- owner: p2-parser-haiku
- priority: P1 · type: bug · area: cli
- problem: format.md §8 says every other part of the combined priority line counts as its field, but status, owner and claimed on that line are unknown parts: show gives status open and owner null, next can hand the task out, check reports 0 problems, and done is refused with exit 4. Found while doing T-117.
- fix: Read status, owner and claimed on the combined line as their fields, and let set, claim, release, done and reopen edit them there (as T-117 does for why and the others).
- accept: a task with '- priority: P1 · status: claimed · owner: alice · claimed: <time>' shows status claimed and owner alice, next does not hand it out, and done and release edit that line; a test covers each
- commit: c967aa4ce4a0e8b94d2f11fc3dca248a49d36ef6
- note (2026-11-18, p2-parser-haiku): added
- note (2026-11-18, p2-parser-haiku): done — status, owner and claimed on the combined priority line are read as their fields (format.md §8): readiness, next, claim, release, done, reopen and stale-claim detection see them, and set edits them in place; session was already read there. Tests: tests/cmd_combined_line.rs.

### T-134 · Combined priority line: a part without key: value is dropped when the line is rewritten
- status: done
- owner: p2-parser-haiku
- priority: P1 · type: bug · area: cli
- problem: set --priority P0 on '- priority: P1 · see the design notes · area: ui' writes '- priority: P0 · area: ui' and reports ok, so 'see the design notes' is lost without a warning. Verification does not catch it, because it compares only the parsed fields of the other tasks. Found while doing T-117; reproduced in a scratch folder.
- fix: Rebuild the combined line from its parts as written, so a part the tool does not read as a field is kept where it is; or refuse the write with an error when a part would be lost.
- accept: set --priority on a line with a part that has no key keeps that part; a test covers it
- commit: c05df974a695b30e4d34e6e03da5667389fffb0f
- note (2026-11-18, p2-parser-haiku): added
- note (2026-11-18, p2-parser-haiku): done — A segment of a combined priority line that is not key: value is kept as written in its place, so no rewrite drops it (set --priority, set --area, done ...). If the priority is removed, such segments stay on one line where it was. check reports each as a warning with its line (PartWithoutField). docs/format.md §8 updated. Tests: tests/cmd_combined_line.rs.

### T-135 · Combined priority line: a value with the separator ' · ' is split on read for type, area, milestone and goal
- status: dropped
- owner: —
- priority: P2 · type: bug · area: cli
- problem: set --type 'a · b' on '- priority: P1 · type: bug · area: ui' reports ok and writes '- priority: P1 · type: a · b · area: ui'; show then reads type 'a' and the 'b' part is dropped on the next rewrite. Verification does not check the new value. difficulty is refused with BAD_VALUE, but type, area, milestone and goal are not. T-117 moves the seven newly read fields' values with the separator to their own line; these four keep the old behaviour.
- fix: Apply the same move as T-117 (a value containing ' · ' gets its own line) to type, area, milestone and goal, or refuse the value with BAD_VALUE.
- accept: set --type, --area, --milestone and --goal with a value containing ' · ' either move the field to its own line or are refused; a test covers each
- note (2026-11-18, p2-parser-haiku): added
- note (2026-11-18, claude-main): dropped — By design: format.md section 8 splits the combined line at ' · ', so a value there cannot contain the separator. Writes already move such values to their own line (T-117). A stray part without key: value is kept and reported by T-134.

## v0.7: HTML view for people and project register

### T-092 · report --html: one page for people
- status: done
- owner: t092-haiku
- priority: P1 · type: feature · area: report
- milestone: T-136
- depends: T-088, T-091, T-097, T-098, T-096
- accept: `todo report --html [--out <path>] [--open]` writes one self-contained HTML file (inline CSS, nothing loaded from the network, no new dependency; default path in the state folder so it is never committed); it keeps the kinds apart in separate lists: (1) work, as a tree of milestones with goal and progress, nested milestones and tasks indented below, top-level tasks last, with owner and age on claimed tasks (stale marked) and the latest reason on blocked ones; (2) open questions, each with a link to the milestone or task it is about; (3) ideas; (4) done; (5) dropped with the reason; a task opens to show its fields and notes; all text is HTML-escaped (test: a title containing `<script>`); it takes no lock and never changes TODO.md; with a fixed clock the output for a fixture matches its snapshot; readable in light and dark mode and on a narrow window
- commit: 2a73c4ac2db2a817b80213559281f567218d65a0
- reqs: R-REPORT-1, R-LOCK-5, NFR-4
- by: Reynan Henry Tandoyo
- why: the HTML page is the one view meant for people; every other output is written for agents
- note (2026-11-18, claude-main): added
- note (2026-11-18, claude-main): Owner 2026-11-18: show milestones as a tree, the milestone with its goal on top and its tasks indented below it, like 'Develop the renderer' > 'C shader', 'GPU compiler'.
- note (2026-11-18, claude-main): Owner 2026-11-18: the page is an overview only. No buttons, no forms, no copy-ready commands; a person who wants a change asks an agent. It is built only when asked, never automatically.
- note (2026-11-18, claude-main): Show priority and difficulty on each task (T-108).
- note (2026-11-18, claude-main): Owner 2026-11-18: show the category of each entry (the existing `area` field, for example ui, numerics, math) and give the page a per-area count of open tasks.
- note (2026-11-19, t092-haiku): Decisions for the owner to confirm (docs/requirements.md R-REPORT-1 now says them): 'open tasks per area' counts open, claimed and blocked tasks; done and dropped entries in TODO-archive.md are listed and marked archived; a done or dropped milestone is shown in the work tree only while unfinished work sits below it, otherwise under done or dropped. --html is required; without it report is a usage error. The default file is report.html in the state folder.
- note (2026-11-19, t092-haiku): done — Added todo report --html (src/report for the page model and HTML, src/cli/report.rs for the command): one read-only page with the work tree, unfinished tasks per area, open questions linked to their entries, ideas, done and dropped entries with the reason of each drop, all text escaped, inline CSS for light and dark mode and narrow windows, default file report.html in the state folder, --out and --open, --json. 22 tests added; cargo fmt, clippy (Windows and Linux) and cargo test --all-features pass (527 passed, 0 failed). Not checked: --open on a real Windows desktop.
- note (2026-11-19, claude-main): Coordinator 2026-11-19: the R-REPORT-1 clarifications are accepted (open per area = open, claimed or blocked; archived done/dropped listed; a finished milestone shows only above unfinished work).

### T-093 · Project register: todo project add, remove, list; overview of registered projects
- status: done
- owner: t093-haiku
- priority: P1 · type: feature · area: cli
- milestone: T-137
- depends: T-088, T-096
- accept: `todo project add [<path>] [--name N]` registers the TODO.md found from that folder (default: the current one), `project remove <name|path>` and `project list` manage the register; it is one JSON file per user, `%LOCALAPPDATA%\todo\projects.json` on Windows and `$XDG_DATA_HOME/todo/projects.json` (else `~/.local/share/todo/projects.json`) on Linux, with `TODO_REGISTER` naming another file; it is written under its own lock by replacing the file, so parallel adds lose nothing; the same file registered twice is refused; `todo overview` reads only the register, never searches folders, and prints one line per project: name, path, tasks per status, top-level milestones done/total, stale claims, open questions; a registered path that is gone or cannot be parsed is listed with its problem and does not stop the others; `--json`; paths with spaces and non-ASCII characters work; tests point the register at a temporary folder
- commit: 9d00a9310f4446a5f4357b323b9d70b603d17dd3
- reqs: R-PROJ-1, R-PROJ-2, NFR-7
- by: Reynan Henry Tandoyo
- why: the owner wants one place to see all projects (idea I-2, revived 2026-11-18) without todo searching the PC
- note (2026-11-18, claude-main): added
- note (2026-11-18, claude-main): Owner 2026-11-18: todo must not scan the PC for TODO.md files. Projects are registered with a command and noted in the user app-data folder (and the Linux equivalent); the all-projects view is built only from that register. `init` may print a hint to run `todo project add` but does not register by itself.
- note (2026-11-18, claude-main): With T-099 in place, `overview` also shows the todo version stamped in each project, so projects that still need a migration stand out.
- note (2026-11-19, t093-haiku): Decisions (t093-haiku): the register is {"format": 1, "projects": [{name, path, added}]} at TODO_REGISTER or the user data folder; project add takes a folder and finds its TODO.md as other commands do (--file and TODO_FILE do not apply); add does not parse the file. The same file (compared as R-LOCK-7) or the same name (ignoring case) is DUPLICATE_PROJECT. An invalid name is USAGE (exit 2). remove matches a name first, then a path to a TODO.md or its folder; a missing register is NOT_FOUND without creating anything. JSON: todo.project.v1 for add, remove and list; todo.projects.v1 for overview, since todo.overview.v1 is todo --json with no command. overview reads only the register and each TODO.md (no lock, no state folder, no write); figures count TODO.md only; a project with a problem keeps its line and its figures are null; exit 0. needs_migration is true when the format is older than 2 or the stamp is missing or older than the running version. A busy register waits 30 s, then BUSY exit 3, and names no holder (the register has no holder file). The init hint was not added (R-AGENT-2 says may).
- note (2026-11-19, t093-haiku): Clarified R-PROJ-1 and R-PROJ-2 in docs/requirements.md (file layout, remove selector order, lock message, schema names, overview figures, needs_migration, exit code) and docs/json.md (three project shapes, the overview shape, DUPLICATE_PROJECT, error rows for the register). No requirement ID changed.
- note (2026-11-19, t093-haiku): done — Added todo project add, remove and list (per-user JSON register: TODO_REGISTER or the user data folder; locked, replaced in one step; same file or name refused as DUPLICATE_PROJECT) and todo overview (one line per registered project: tasks, top-level milestones, stale claims, open questions, stamp, problems; reads only the register and each TODO.md). New error code DUPLICATE_PROJECT; docs/json.md has todo.project.v1 and todo.projects.v1 shapes; R-PROJ-1/2 clarified. Tests: 537 passed in full suite, fmt, clippy Windows and Linux clean.

### T-094 · report --html --all: every registered project on one page
- status: done
- owner: t094-haiku
- priority: P1 · type: feature · area: report
- milestone: T-136
- depends: T-092, T-093
- accept: `todo report --html --all [--out <path>] [--open]` builds one self-contained page from the project register only: a card per project (tasks per status, top-level milestones with progress, blocked tasks, open questions) followed by each project's lists as in the single-project report; the default path is next to the register; a project that is gone or cannot be parsed shows its problem instead of its lists; an empty register gives a page that says how to register a project; snapshot test with two invented projects
- commit: 3bfa4a1b4e32bd966943f4a7eba92062ad8a96d9
- reqs: R-REPORT-2
- by: Reynan Henry Tandoyo
- note (2026-11-18, claude-main): added
- note (2026-11-18, claude-main): Owner 2026-11-18: built only when asked (`report --html --all`); overview only, no buttons or commands on the page.
- note (2026-11-18, t086-opus): T-086: conflict between accept ('an empty register gives a page that says how to register a project') and the owner note (no commands on the page). Followed the owner: R-REPORT-2 says the empty page explains in words that a project is registered by asking an agent, without a copy-ready command.
- note (2026-11-19, t094-haiku): R-REPORT-2 clarified (docs/requirements.md): the card counts as todo overview counts (TODO.md only, top-level milestones not nested and not dropped); a project that overview refuses for a problem that blocks all writes shows its problem here too, other problems are warnings; reads take no state folder and write nothing in a project.
- note (2026-11-19, t094-haiku): done — report --html --all: one page with a card per registered project (tasks per status, top-level milestones with progress, blocked tasks, open questions), then each project's lists with per-project anchors; problems of unreadable projects instead of lists; default report-all.html next to the register; empty register explains registration in words; 19 integration tests and 2 unit tests, snapshot of two invented projects

### T-106 · Performance test for the HTML report
- status: done
- owner: t094-haiku
- priority: P1 · type: test · area: report
- milestone: T-136
- depends: T-094
- accept: on the generated 5,000-task file with a milestone tree, `report --html` finishes in under 500 ms in a release build and the page is under 5 MB; `report --html --all` with ten such projects finishes in under 5 s; the test runs in CI with the other performance tests
- commit: 04cbfce8f02802a5c2bb079ec48f09f460b6a7ce
- reqs: NFR-3, TEST-9
- by: Reynan Henry Tandoyo
- note (2026-11-18, claude-main): added
- note (2026-11-19, t094-haiku): done — Added two ignored release perf tests in tests/perf.rs for the HTML report on the 5,000-entry milestone tree: report --html (median 51 ms, page 3,086,950 bytes, budget 500 ms and 5 MB) and report --html --all over ten projects in a temp register (median 429 ms, page 30,990,183 bytes, budget 5 s). Budgets pass, so the report code needed no change; all four perf tests pass in release.

### T-107 · v0.7 documentation and release 0.7.0
- status: done
- owner: claude-main
- priority: P1 · type: docs · area: release
- depends: T-094, T-106
- accept: the README has a short part for people: register a project, ask an agent for the report, open the page, and that every change goes through an agent; docs/json.md documents `project`, `overview` and `report`; CHANGELOG.md has the 0.7.0 entry; Cargo.toml is 0.7.0; `todo check` is clean
- commit: 2c2fdd9f0db8c5a0daaebc2ffc92c98d52b61497
- reqs: REL-2, REL-5
- by: Reynan Henry Tandoyo
- note (2026-11-18, claude-main): added
- note (2026-11-19, t094-haiku): Local part finished in commit 0bbe49e on reynanTandoyo/t094: version 0.7.0 in Cargo.toml and Cargo.lock, README part for people, CHANGELOG 0.7.0 entry, docs/json.md checked (report example added and tested). Checks pass; todo check with the new release binary reports no problems. Tagging, pushing and installing are left to the coordinator, who marks T-107 done after the release.
- note (2026-11-19, session-end): released — session 79d7d817-2673-48b1-aecd-b31d881f709e ended
- note (2026-11-19, claude-main): Released 2026-11-19: tag v0.7.0 on 2c2fdd9, GitHub release published by the Release workflow. Installed on this PC with todo help update: SHA-256 matched, todo --version prints 'todo 0.7.0 (2c2fdd9)'; todo migrate --yes raised the stamp to 0.7.0.
- note (2026-11-19, claude-main): done — v0.7 docs and 0.7.0 released and installed.

### T-136 · HTML view for people
- status: done
- owner: claude-main
- priority: P1 · type: milestone
- goal: A person can see the state of one project, or of every registered project, on a single read-only page built on request
- why: People read the task list without running commands; every change still goes through an agent
- by: Reynan Henry Tandoyo
- note (2026-11-19, claude-main): added
- note (2026-11-19, claude-main): done — report --html (T-092), report --html --all (T-094) and their performance tests (T-106) merged into dev. Release medians: 51 ms for a 5,000-entry tree (3.1 MB page), 429 ms for ten projects (31 MB page).

### T-137 · Overview of all projects
- status: done
- owner: claude-main
- priority: P1 · type: milestone
- goal: Projects are registered by command and their state can be listed together, without scanning the PC
- why: One place to see which projects have stale claims, open questions or need a migration
- by: Reynan Henry Tandoyo
- note (2026-11-19, claude-main): added
- note (2026-11-19, claude-main): done — Project register and overview (T-093) merged into dev; T-141 and T-143 fixed the exit-code docs found on the way.

### T-138 · A write refused for a read-only TODO.md still saves a history snapshot
- status: done
- owner: t132-haiku
- priority: P3
- why: The snapshot (R-HIST-2) is recorded before TODO.md is replaced, so a write that the read-only check then refuses leaves a snapshot of the unchanged file in .todo/history. Seen with todo archive on a read-only TODO.md (T-132). The check could run before the snapshot is recorded.
- commit: da287a53931cfa09a3eba92bdeb79689965e1e2a
- by: Reynan Henry Tandoyo
- note (2026-11-19, t132-haiku): added
- note (2026-11-19, t132-haiku): done — A write refused because TODO.md is read-only now checks replaceability before the history snapshot is recorded, so it saves no snapshot (R-HIST-2 now says so). Tests: a refused note adds no snapshot to an existing history, and refused note and migrate on a fresh folder leave no history; both fail on the previous lock.rs. Checks pass: fmt, clippy Windows and Linux with all features, clippy without features, cargo test --all-features.

### T-139 · Tests without test-hooks: unused imports in cmd_next.rs and lock_write.rs fail cargo clippy --all-targets
- status: done
- owner: t132-haiku
- priority: P3
- why: Without the test-hooks feature, tests/cmd_next.rs (read_text, TaskId) and tests/lock_write.rs (parse_json, read_text, stderr_text) have unused imports, so cargo clippy --all-targets -D warnings fails. CI runs --all-features, so it passes there. Found while doing T-132.
- commit: 8ac7c0bde8c5197e30760935d75ab508a543e33e
- by: Reynan Henry Tandoyo
- note (2026-11-19, t132-haiku): added
- note (2026-11-19, t132-haiku): done — Imports used only by test-hooks tests are gated with cfg(feature = test-hooks) in tests/cmd_next.rs, tests/lock_write.rs and tests/lock_lease.rs. cargo clippy --all-targets -- -D warnings now passes without features (it failed before on these three files), and the all-features clippy and cargo test --all-features still pass.

### T-140 · summary and report give different reasons for a blocked task
- status: done
- owner: t092-haiku
- priority: P2
- milestone: T-136
- why: summary shows the latest non-empty note of a blocked task as its reason, so an unrelated later note replaces the reason; report shows the reason of the latest 'blocked — <reason>' note (docs/format.md §9). One rule should serve both pages and the other reason readers.
- commit: f666e868dbef2b8f8f34f8989f13bef2bd06c158
- by: Reynan Henry Tandoyo
- note (2026-11-19, t092-haiku): added
- note (2026-11-19, t092-haiku): done — summary and report now read the blocked reason from one function, Task::blocked_reason (src/doc/reason.rs): the text after the dash of the latest 'blocked — <reason>' note. summary used to show the latest non-empty note; a test with several block notes and an unrelated later note fails on the old code and passes now, and summary --json, the text overview and the report agree. Behaviour change: summary's blocked reason is the reason alone, without the 'blocked — ' prefix; docs/json.md, R-READ-5 and the summary snapshot are updated.

### T-141 · docs/json.md gives exit 2 for BAD_VALUE (1 for check), but the program exits 1 for every BAD_VALUE, such as an unknown setting in todo.toml
- status: done
- owner: t093-haiku
- priority: P2 · type: bug · area: docs
- milestone: T-137
- why: the JSON reference and R-OUT-2 must say what the program does; found while documenting the project commands, not changed by T-093
- commit: 976a9ad6c0d762dd7054135659e08ccf2253a150
- by: Reynan Henry Tandoyo
- note (2026-11-19, t093-haiku): added
- note (2026-11-19, t093-haiku): Decided (t093-haiku): the requirements win, so BAD_VALUE exits 1 (R-WRITE-18 and R-MILE-1 both refuse an invalid value with exit 1); the program already does, so only docs/json.md changed. The other 21 codes match the requirements and the program (R-OUT-2 categories, R-CLAIM-2/5, R-IO-2/3/5, R-LOCK-3/6, R-VER-2, R-PROJ-1, R-FIND-6, R-EDIT-7, I-8). Added tests/docs_exit_codes.rs: the table must match todo schema, and 21 of 22 codes are produced by real commands with the table's exit; CHANGED_DURING_WRITE is covered by tests/lock_lease.rs. Also corrected two json.md rows: AMBIGUOUS_SECTION now describes a --section name matching several sections (its test is tests/cmd_add.rs), and BAD_TRANSITION no longer says done on open is refused (R-WRITE-9 allows it). R-OUT-3 now says each code has one fixed exit.
- note (2026-11-19, t093-haiku): done — BAD_VALUE exits 1 in docs/json.md (R-WRITE-18, R-MILE-1; the program already did). Two more rows corrected: AMBIGUOUS_SECTION now describes a name matching several sections, BAD_TRANSITION no longer refuses done on open (R-WRITE-9). New tests/docs_exit_codes.rs: the table equals todo schema, and 21 of 22 codes exit with the table's code from real commands (CHANGED_DURING_WRITE is in lock_lease.rs). R-OUT-3 says each code has one fixed exit. fmt, clippy (Windows and Linux) and cargo test --all-features (568 passed) pass. Recorded T-143: a no-section add gives UNKNOWN_SECTION, owner to decide the code.

### T-142 · check accepts a blocked task whose only note is not a block reason
- status: done
- owner: t092-haiku
- priority: P3
- milestone: T-136
- why: check warns about a blocked task only when it has no non-empty note, so a task whose only note is 'added' passes, while summary and the HTML report now show no reason for it. check should use Task::blocked_reason (src/doc/reason.rs) so that all three readers agree.
- commit: b2848b02ab624cad709f25d8e59d31d2e9e87ca8
- by: Reynan Henry Tandoyo
- note (2026-11-19, t092-haiku): added
- note (2026-11-19, t092-haiku): done — check reports a blocked task without a block-reason note (Task::blocked_reason, the reason summary and the report show) at its heading line, as a problem with exit 1 like the other task state problems (R-CHECK-1). The import of a checkbox list now gives a blocked task without a reason the note 'blocked — imported from a checkbox list', as it already did for a task with no notes, so its output passes check. Invented fixture check_blocked_reason.md: the old rule missed three of four blocked tasks; the import test checks the reason line; milestone_check_unfinished.md's blocked task now has a block note. cargo fmt, clippy -D warnings (Windows and Linux, with and without features), and cargo test --all-features (574 passed, 0 failed, 2 ignored) pass.

### T-143 · add with several sections and no --section fails with UNKNOWN_SECTION; docs/json.md first gave AMBIGUOUS_SECTION for that case, and R-OUT-3 does not say which
- status: done
- owner: t093-haiku
- priority: P3 · type: bug · area: cli
- milestone: T-137
- why: the error code of a refused add is part of the JSON contract (R-OUT-3); the owner decides which code it is, found while checking the error table against the program (T-141)
- commit: a2aa3a64d1fc3e22bf3700ef55e42892f10f63fa
- by: Reynan Henry Tandoyo
- note (2026-11-19, t093-haiku): added
- note (2026-11-19, t093-haiku): Decided with the owner: UNKNOWN_SECTION stays for add without --section in a file with several sections (returned since v0.1; existing codes never change meaning). R-WRITE-1 now names UNKNOWN_SECTION for that case and AMBIGUOUS_SECTION for an ambiguous name; the UNKNOWN_SECTION row of docs/json.md covers it with the message the program prints. tests/docs_exit_codes.rs has a case for it.
- note (2026-11-19, t093-haiku): done — Docs only: R-WRITE-1 names UNKNOWN_SECTION for add without --section in a file with several sections (kept, per owner) and AMBIGUOUS_SECTION for an ambiguous name; docs/json.md's UNKNOWN_SECTION row covers that case; tests/docs_exit_codes.rs checks it. fmt, clippy (Windows and Linux) and cargo test --all-features (574 passed, 0 failed) pass.

### T-144 · report --html and report --html --all have no timing test for NFR-3 (ten projects under 5 s)
- status: dropped
- owner: —
- priority: P2
- milestone: T-136
- why: NFR-3 sets a time budget for report --html (under 500 ms) and report --html --all (ten projects under 5 s, release build), but tests/perf.rs measures only list, show, write and tree; the report commands have no timing check.
- note (2026-11-19, t094-haiku): added
- note (2026-11-19, t094-haiku): dropped — duplicate of T-106, which already tests report --html and report --html --all against NFR-3

## v0.8: level-1 tasks and parallel work

### T-145 · Weak models finish the work in parallel
- status: done
- owner: claude-main
- priority: P1 · type: milestone
- goal: A planner splits the work until every task is level 1; any number of cheap agents then finish those tasks at the same time, without touching each other's files and without a false done
- why: the owner wants work finished faster by spawning many cheap agents (Haiku, DeepSeek flash) instead of a few strong ones
- by: Reynan Henry Tandoyo
- note (2026-11-19, claude-main): added
- note (2026-11-20, claude-main): done — v0.8 released as 0.8.0: levels 1 to 5, detailed level-1 tasks, split, verify, stuck, commit scope, file scopes, waves, plan and brief.
- note (2026-11-20, claude-main): reopened — Reopened: CI shows next at 156 ms (Linux) and 185 ms (Windows) on the T-165 milestone-tree perf test, budget 100 ms.
- note (2026-11-21, claude-main): done — v0.8 complete: 0.8.0 released; CI speed fixes T-185 to T-188 on main, CI green on Linux and Windows.

### T-146 · v0.8 requirements: levels, detailed tasks, split, verify, stuck, file scopes, waves
- status: done
- owner: t146-opus
- priority: P0 · difficulty: hard · type: docs · area: docs
- milestone: T-145
- why: every other v0.8 task builds what this one writes down; the owner's decisions of 2026-11-19 must be in the requirements first
- accept: docs/requirements.md defines R-LEVEL-1 to R-LEVEL-3, R-DETAIL-1 to R-DETAIL-3, R-SPLIT-1, R-VERIFY-1, R-VERIFY-2, R-STUCK-1, R-SCOPE-1, R-SCOPE-2, R-WAVE-1 and R-REPORT-3 as the v0.8 tasks describe them; section 2 describes the two roles, planner and worker; NFR-5 and section 13 say that todo runs one more kind of command, a task's `verify` line, and nothing else; the settings table gains `target_difficulty`, `level1_max_files`, `verify`, `verify_timeout_seconds`, `verify_attempts` and `verify_shell`, and the defaults of `difficulties` (`1` to `5`) and `required_fields` (`priority`, `difficulty`) change; no model name appears anywhere, only what each level means; section 11 lists v0.8; docs/format.md becomes format version 3: the fields `steps` (a numbered list on indented lines under `- steps:`), `read`, `verify`, `final` and `attempts`, `files` as the list of paths and patterns a task may change, and the migration step from format 2
- commit: f1d87c1a4a96e476e7bd7d897905a0b4a2511e73
- reqs: REL-4, NFR-8
- by: Reynan Henry Tandoyo
- note (2026-11-19, claude-main): added
- note (2026-11-19, claude-main): Also specify `verify_when` (`done` or `wave`, T-158), the `verified` field and `todo verify --wave`.
- note (2026-11-19, claude-main): R-AGENT-1 changes: the one rules text becomes a short common block (at most 15 lines) plus `todo help worker` and `todo help planner` (T-166).
- note (2026-11-20, t146-opus): done — requirements.md: R-LEVEL-1..3, R-DETAIL-1..3, R-SPLIT-1, R-VERIFY-1/2, R-STUCK-1, R-SCOPE-1/2, R-WAVE-1, R-REPORT-3 added; R-AGENT-1 split by role; section 2 planner/worker; NFR-5 and section 13 limit runs to the verify line; new settings and changed defaults; v0.8 in section 11; amended R-EDIT-5, R-READ-3/5, R-WRITE-1/4/8/9/10/18, R-CLAIM-2, R-OUT-3/8/11, R-CHECK-1, R-GIT-2, R-MIG-1, NFR-3, TEST-2/5/9, D14, I-13. format.md: version 3, new fields, steps list, notes, section 19 paths, migration 2 to 3. Decisions recorded on T-148..T-166.

### T-147 · Difficulty levels 1 to 5 and the planner's list
- status: done
- owner: claude-main
- priority: P1 · type: milestone
- milestone: T-145
- goal: Every task has a level from 1 (the weakest model finishes it without a mistake) to 5, and the planner sees at a glance what still has to be split or detailed
- why: the planner can only drive everything to level 1 if nothing is unrated and the remaining hard tasks are listed for it
- by: Reynan Henry Tandoyo
- note (2026-11-19, claude-main): added
- note (2026-11-20, claude-main): done — Levels 1 to 5 (T-148), todo plan (T-149) and final (T-150) merged into dev.

### T-148 · Levels 1 to 5: new default difficulties, difficulty required, old values migrated
- status: done
- owner: t148-haiku
- priority: P1 · difficulty: medium · type: feature · area: core
- milestone: T-147
- why: a task without a level cannot be given to the right agent, so an unrated task must not exist
- depends: T-146
- accept: the default `difficulties` is `1`, `2`, `3`, `4`, `5` and `todo help add` says what each level means without naming a model; `required_fields` defaults to `priority` and `difficulty`, applied to tasks only (milestones, ideas and questions need neither); `add` and `add --from` without a difficulty are refused with `MISSING_FIELD`; the format 2 to 3 migration rewrites `easy` to `2`, `medium` to `3` and `hard` to `4` on entries that are not done or dropped, only when todo.toml sets no `difficulties` of its own, and leaves open tasks without a difficulty for the planner's list; `next` never hands out a task without a difficulty; every refusal leaves the file byte-identical
- commit: ef67ed1692551538c32f5906f49dc51cba4b771d
- reqs: R-LEVEL-1, R-MIG-1, R-CFG-2, TEST-4
- by: Reynan Henry Tandoyo
- note (2026-11-19, claude-main): added
- note (2026-11-20, t146-opus): T-146 decision (R-CHECK-1, format.md section 18): check does not report a difficulty outside difficulties on done or dropped entries, which keep easy/medium/hard after the migration as a record.
- note (2026-11-20, t148-haiku): t148-haiku: the format 2 to 3 step changes task lines (the difficulty values), so the old rule that migrations touch only the preamble is gone; R-MIG-1 already says whole lines only, and the bytes outside the changed lines are still checked. R-DISC-1 now lists --difficulty next to --priority, since scan --add goes through add --from and that needs a level; docs/requirements.md is updated in this commit.
- note (2026-11-20, t148-haiku): t148-haiku: readiness: a task without a level is not ready, so next, list --ready, summary's next-ready list, show --json ready and the done/drop 'now ready' output all leave it out (one rule in is_task_ready, R-READ-3). T-169 records that promote and apply still create tasks without a level.
- note (2026-11-20, t148-haiku): done — Default difficulties are 1 to 5; required_fields is priority and difficulty for tasks; add and add --from refuse a task without a level (MISSING_FIELD); format 3 with the format 2 to 3 rewrite of easy/medium/hard to 2/3/4 on open entries unless todo.toml sets difficulties; next and list --ready skip unrated tasks; check ignores old names on done or dropped; todo help add and set print the levels; scan --add takes --difficulty. Tests in tests/cmd_levels.rs and tests/doc_migrate.rs; full suite 618 passed; all four clippy runs and fmt pass.

### T-149 · todo plan: what the planner still has to split, detail or fix
- status: done
- owner: t153-haiku
- priority: P1 · difficulty: medium · type: feature · area: cli
- milestone: T-147
- why: todo does not split tasks itself; it shows the planner exactly which ones need its attention
- depends: T-148, T-153, T-159
- accept: `todo plan [<milestone>]` lists, in this order: tasks a worker gave back (stuck), each with the problem; tasks above `target_difficulty` (default: the lowest level), hardest first; tasks at the target level that are not ready, each with what is missing; tasks without a difficulty; a last group, counted apart, holds tasks kept above the target on purpose (`final`); done, dropped and claimed entries, milestones, ideas and questions are left out; `summary` shows one line with the three counts within its 40 lines; `plan --json` matches its snapshot; exit 0 when the first four groups are empty, 1 otherwise
- commit: e30df2449cde01fd4b21727a925fd056ec8b61fc
- reqs: R-LEVEL-2, R-READ-4
- by: Reynan Henry Tandoyo
- note (2026-11-19, claude-main): added
- note (2026-11-20, t146-opus): T-146 decisions (R-LEVEL-2): T-158 asks plan to list unverified done tasks, so plan has a sixth group counted apart (done tasks with verify and no verified), the one exception to 'done entries are left out'; groups 5 and 6 don't change the exit code. Summary line: to split / to detail / without a level, plus kept on purpose; given-back tasks are counted on their own line (R-STUCK-1). Read-only, JSON schema todo.plan.v1.
- note (2026-11-20, t158-haiku): From T-158: plan must also list done tasks that have a verify line and no verified line (R-VERIFY-2, the counted-apart group 6 of R-LEVEL-2). T-158 built the warnings of check only; plan is not built yet.
- note (2026-11-20, t150-haiku): From T-150 (t150-haiku): the summary part is done. summary prints the line "kept above the target level on purpose: N" only when N is above 0, and JSON carries kept_on_purpose (left out at 0). A task counts when it has a final, is open or blocked, and its level is above the target (OpsConfig::is_above_target). plan group 5 should use the same rule and leave out claimed, done and dropped tasks. plan itself is not built: its listing, its exit code and the three-count summary line (to split, to detail, without a level) are still open for T-149. The target is OpsConfig::target_difficulty, the first of difficulties, until target_difficulty is read from todo.toml.
- note (2026-11-20, t153-haiku): Decisions: each task is in the first group it fits (given back, to split, to detail, without a level, kept on purpose, unverified), so the groups do not overlap. Group 6 is left out while verify = false, as check leaves it out. The groups live in ops::plan and summary's planner line reads them, so plan and summary cannot disagree. ops::verify::awaits_verified is the one rule for a done task whose verify has not passed; check and verify --wave use it. Found: a task whose difficulty is set but not configured is in no group, while next hands it out (T-181).
- note (2026-11-20, t153-haiku): done — todo plan: the six groups of R-LEVEL-2 in order, exit 0 only when the first four are empty, plan --json (todo.plan.v1) with a snapshot. summary's kept-on-purpose line is now the planner's one line with to split, to detail and without a level, and its JSON carries the three counts.

### T-150 · final: keep a task above level 1 on purpose
- status: done
- owner: t150-haiku
- priority: P2 · difficulty: easy · type: feature · area: cli
- milestone: T-147
- why: splitting is the intended way, but the planner may decide as a last resort that a task stays hard; that decision needs a reason and must stop the reminders
- depends: T-148
- accept: `set <task> --final <reason>` writes `- final: <reason>` and is refused on a task at the lowest level or without a reason; `set --unset final` removes it; `plan` and `summary` count such tasks apart (see the plan task); `check` reports a `final` on a milestone, idea or question; `show --json` and `schema` list the field
- commit: 1890a73df8ccedb88f3959a865961469e0e93ced
- reqs: R-LEVEL-3, TEST-4
- by: Reynan Henry Tandoyo
- note (2026-11-19, claude-main): added
- note (2026-11-20, t146-opus): T-146 decision (R-LEVEL-3): --final is refused at or below target_difficulty (default: the lowest level), and set --final on a milestone, question or idea is refused (BAD_VALUE) in addition to check reporting one written by hand.
- note (2026-11-20, t156-haiku): split (T-156) removes final by its key today: final is an unknown line to the parser, so the generic unset takes it out. When final gets its own field, split must remove it too: add final to FIELDS_OF_A_TASK in src/cli/split.rs, and keep the split test that expects no final line.
- note (2026-11-20, t150-haiku): done — set --final, --unset final, the refusals (empty reason, milestone, question, idea, target level), check, show --json, schema and the summary count; final is a typed task field; 11 tests in tests/cmd_final.rs; plan's part left as a note on T-149; attempts and verified field gap recorded as T-177

### T-151 · Level-1 tasks carry everything a weak model needs
- status: done
- owner: claude-main
- priority: P1 · type: milestone
- milestone: T-145
- goal: A level-1 task names its files, its steps and its check, todo refuses to hand it out until it does, and the worker gets all of it in one work order
- why: a weak model fails when it has to explore, guess or decide; the task must leave nothing open
- by: Reynan Henry Tandoyo
- note (2026-11-19, claude-main): added
- note (2026-11-20, claude-main): done — Detail fields (T-152), the level-1 ready rule (T-153) and todo brief (T-154) merged into dev.

### T-152 · Detail fields: steps, read and verify
- status: done
- owner: t152-haiku
- priority: P1 · difficulty: hard · type: feature · area: core
- milestone: T-151
- why: the steps, the files to look at and the check command are the detail a weak model needs, and they do not fit the existing one-line fields
- depends: T-146
- accept: `add`, `set`, `add --from` and `apply` take `--steps` (text, `-` or `@path`; one step per input line, written as numbered indented lines under `- steps:`), `--read` (paths the worker should look at but not change) and `--verify` (one command line); `set --steps` replaces the whole list and `--unset steps` removes it with its lines; the parser reads the indented lines as the value of `steps` and still keeps every other indented line as text; LF, CRLF and byte-order-mark fixtures round-trip byte-identical; the merge driver treats the list as part of the task block; `show --json` returns `steps` as a list; `schema` lists the three fields; the field order in docs/format.md section 10 is followed
- commit: 810fe9af96c51cfc9d40d21f225df0c82a5e0939
- reqs: R-DETAIL-1, R-EDIT-1, R-EDIT-7, TEST-2, TEST-3, TEST-4
- by: Reynan Henry Tandoyo
- note (2026-11-19, claude-main): added
- note (2026-11-20, t159-haiku): from T-159 (t159-haiku): todo stuck writes attempts through the generic unknown-field path, so it lands after the last known field, not before verified and commit as docs/format.md section 10 says; attempts is not in CANONICAL_FIELD_ORDER (src/doc/parser.rs). When adding steps, read, verify, final, attempts and verified to that order, check that a stuck on a task with a verify line puts attempts in place.
- note (2026-11-20, t152-haiku): Decisions: a steps part on the priority line is read as one step (check warns) and is moved below - steps: when steps are set or removed; format.md section 8 says so. --field steps=, add --from and apply fields take the list too (raw text, one step per line); set --steps with empty text removes the field. CANONICAL_FIELD_ORDER follows section 10 in full (final, attempts, verified are ordered but not parsed yet; T-150, T-158 and T-159 add those).
- note (2026-11-20, t152-haiku): Merge driver: steps are merged as lines of the task block by git merge-file (the textual fallback), not as a field group, so a steps change next to another field's change on the other side conflicts. Reproduced; recorded as a P2 problem.
- note (2026-11-20, t152-haiku): Command tests compare the text from the ## Now heading on and the fixtures have no format line, so the format 3 migration of T-148 does not change their expectations.
- note (2026-11-20, t152-haiku): done — steps list, read and verify fields: parsed and written with the edit builder, add/set/add --from/apply take them, show --json and schema list them, set --steps and --unset steps, section 10 order, verifier and merge driver cover them; tests and LF/CRLF/BOM fixtures; a steps part on the priority line is read as a step and moved below - steps: on write

### T-153 · Ready rule: a level-1 task is handed out only when it is complete
- status: done
- owner: t153-haiku
- priority: P1 · difficulty: medium · type: feature · area: core
- milestone: T-151
- why: calling a task level 1 is only a claim; the tool can at least check that the detail a weak model needs is there
- depends: T-152, T-148
- accept: a task at the lowest level is ready only when it has `why`, `accept`, `steps`, `verify` and `files`; `files` has at most `level1_max_files` entries (default 3) and none is a pattern or a folder; every path in `read` exists; no open question is about it; `next` never hands out a lowest-level task that fails the rule; `check` reports each such task with what is missing, as a warning that does not change the exit code; `show --json` carries `ready` and `missing`; tasks at other levels are not affected
- commit: b18c77fff732a6fd8e2df1ca9c7bcc12e74e5ea9
- reqs: R-DETAIL-2, R-READ-3, R-CHECK-1
- by: Reynan Henry Tandoyo
- note (2026-11-19, claude-main): added
- note (2026-11-20, t146-opus): T-146 decision (R-DETAIL-2): the ready rule applies to tasks at or below target_difficulty, which defaults to the lowest level, so by default exactly the lowest level as this task says; this keeps plan's group 'at the target level, not ready' meaningful when a project raises the target. claim by ID still works; next and list --ready skip failing tasks.
- note (2026-11-20, t163-haiku): T-163 added ops::waves::detail_gaps (src/ops/waves.rs): the why, accept, steps, verify and files part of R-DETAIL-2 and the level check, used by todo waves. T-153 should extend that one function with the read paths and open questions and use it for next, check and show, not write a second rule. level1_max_files (default 3) and target_difficulty are still constants there until their config keys are read.
- note (2026-11-20, t153-haiku): detail_gaps(task, doc, archive, cfg) also checks the read paths (they must exist in the folder of TODO.md; a :<line> suffix is not part of the path) and the open questions about the task; next, list --ready, show --json (ready, missing), check (warnings for unfinished tasks only) and waves all read it. target_difficulty (default: the first of difficulties) and level1_max_files (default 3, at least 1) come from todo.toml; load_next_to sets the folder that read paths are relative to, and a configuration without a file skips the read check. waits_on_task counts a question about a task, so done reports the task newly ready when its last open question is closed. R-DETAIL-2 and R-CHECK-1 now say that check reports unfinished tasks only.
- note (2026-11-20, t153-haiku): done — Ready rule for tasks at or below the target level: detail_gaps (ops::waves) checks why, accept, steps, verify, files (level1_max_files), read paths and open questions; next, list --ready, show --json (ready, missing), check (warnings) and waves use it. target_difficulty and level1_max_files are read from todo.toml; waits_on_task counts a question about a task for newly ready.
- note (2026-11-20, t154-haiku): T-154 (t154-haiku): target_difficulty and level1_max_files are read from todo.toml now. Use OpsConfig::is_at_or_below_target(difficulty) for the ready rule, not a rank of 0; cfg.level1_max_files replaces the LEVEL1_MAX_FILES constant, which is gone. detail_gaps keeps its checks; the read paths and open questions and the next/check/show wiring stay with this task, as the T-163 note says.

### T-154 · todo brief: one work order for the worker
- status: done
- owner: t154-haiku
- priority: P1 · difficulty: medium · type: feature · area: cli
- milestone: T-151
- why: a weak model should read one text and start, without running several commands to collect the context
- depends: T-152, T-158, T-159
- accept: `todo brief <task>` prints, in this order: the goals of the milestones above the task, top down; the title and the why; the files to read; the files it may change, with the sentence that no other file may be changed; the numbered steps; the verify command; for each finished dependency its done note and commit; the exact commands to finish (commit only the listed files, `todo done <id> --commit HEAD`) and to give up (`todo stuck <id> --note`), with the agent name filled in; `next --claim --brief` and `claim --brief` print the same after claiming; `brief --json` matches its snapshot; it reads only and takes no lock
- commit: 73d88c98fbd29f1abf38405747bb12df598212fd
- reqs: R-DETAIL-3, R-MILE-4, R-LOCK-5
- by: Reynan Henry Tandoyo
- note (2026-11-19, claude-main): added
- note (2026-11-20, t154-haiku): done — brief: todo brief <task> prints the R-DETAIL-3 work order (goals above, ID/title/why, read, files with no-other-file sentence, steps, verify and when done runs it, done dependencies with note and commit, finish and give-up commands); next --claim --brief, next --brief and claim --brief print it; --json todo.brief.v1; target_difficulty and level1_max_files read from todo.toml (defaults: first level, 3); OpsConfig::is_at_or_below_target replaces the rank-0 checks in the done commit rule and in waves detail_gaps; LEVEL1_MAX_FILES removed; R-DETAIL-3 item 7 limited to done dependencies. 21 new tests; full suite 850 passed, 0 failed; fmt, clippy (all features, none, Linux) clean. next/check/show wiring of detail_gaps and the read/question checks left to T-153.

### T-155 · Splitting a task into smaller tasks
- status: done
- owner: claude-main
- priority: P1 · type: milestone
- milestone: T-145
- goal: The planner replaces one hard task with several smaller ones in a single step, and nothing that depended on the old task breaks
- why: dividing is the planner's main move, so it must be one safe command and not a dozen
- by: Reynan Henry Tandoyo
- note (2026-11-19, claude-main): added
- note (2026-11-20, claude-main): done — todo split (T-156) merged into dev.

### T-156 · todo split: turn a task into a milestone with child tasks
- status: done
- owner: t156-haiku
- priority: P1 · difficulty: medium · type: feature · area: cli
- milestone: T-155
- why: splitting by hand takes one command per child plus a change of the parent, and a crash in between leaves half a split
- depends: T-152, T-148
- accept: `todo split <task> --from <file|->` takes child tasks as JSON lines in the shape of `add --from`; in one locked write with one snapshot the task becomes `type: milestone` with `goal` taken from `--goal` or else from its `accept`, loses `difficulty`, `files`, `steps`, `verify`, `final` and its claim, and every child is added after it with `milestone` set to it; children may depend on each other with `@ref`; tasks that depended on the split task still do and now wait for the whole milestone; all or nothing: one bad child changes nothing; refused on a done, dropped or milestone entry and on a task claimed by another agent without --force; a note on the parent names the children; `--dry-run` prints the diff
- commit: c913638e3731dc0b5aabac951da12320fa0b3c4e
- reqs: R-SPLIT-1, R-WRITE-13, R-MILE-1, TEST-4
- by: Reynan Henry Tandoyo
- note (2026-11-19, claude-main): added
- note (2026-11-20, t146-opus): T-146 decision (R-SPLIT-1): split also removes read and attempts; a blocked (given-back) task becomes open so its children can be handed out (a blocked milestone would hold them back, R-MILE-6); a child naming its own milestone is refused (BAD_VALUE); no --goal and no accept refuses with MISSING_FIELD; the parent note is 'split into T-021, T-022, ...'.
- note (2026-11-20, t156-haiku): done — split adds the children and makes the task a milestone in one locked write: goal from --goal or else accept; drops difficulty, files, read, steps, verify, final, attempts, verified and its claim; a blocked task becomes open; the note reads split into <IDs>. Children reuse add --from parsing and @ref handling (parse_batch_entries, check_batch_titles, resolve_batch_depends, entry_lines). Decided: a child with a section, or a question or idea child, is BAD_VALUE; verified is removed (format.md 16), R-SPLIT-1 now says so. 15 tests in tests/cmd_split.rs; cli_overview snapshot lists split. Found T-178 (set --goal with a separator). final is removed by its key until T-150 adds its field; noted on T-150.

### T-157 · A done that can be trusted
- status: done
- owner: claude-main
- priority: P1 · type: milestone
- milestone: T-145
- goal: A task counts as done only after its own check passed and its commit stayed inside its files; a worker that fails gives the task back with the problem
- why: weak models report success when the work is broken, and with many of them nobody can re-check every task by hand
- by: Reynan Henry Tandoyo
- note (2026-11-19, claude-main): added
- note (2026-11-20, claude-main): done — verify (T-158, T-173), todo stuck (T-159) and the commit-scope check (T-160, T-174, T-175) merged into dev.

### T-158 · Verify command: checked at done, or for the whole wave
- status: done
- owner: t158-haiku
- priority: P1 · difficulty: hard · type: feature · area: core
- milestone: T-157
- why: asking the worker whether its test passed relies on the one thing that cannot be trusted
- depends: T-152
- accept: a task's `verify` command is run by todo in the folder of TODO.md, outside the write lock, through `verify_shell` (default PowerShell on Windows, `sh` elsewhere), stopped after `verify_timeout_seconds` (default 300). `verify_when` in todo.toml picks the mode. Mode `done` (default): `done` runs the command first; exit 0 marks the task done and writes `- verified: <date>`; anything else refuses with exit 1 and the new code `VERIFY_FAILED`, leaves the task claimed, raises `attempts` by one, saves the full output in the state folder and puts its last line in a note; after `verify_attempts` failures (default 3) the task is given back as `stuck` does. Mode `wave`: `done` does not run the command and the task is done without `verified`; `todo verify --wave` then runs the command of every done task that has `verify` and no `verified`, one after another; a pass writes `verified`; a failure reopens the task and gives it back as `stuck` does, with the last line of the output as the problem; it prints passed, failed and the tasks given back, exit 0 only when all passed, and refuses to start while any task is claimed unless --force. In both modes: `todo verify <task>` runs one command without changing anything; `done --no-verify` needs --force and leaves a note; `verify = false` turns running off; a task without `verify` behaves as today; `check` and `plan` list done tasks that are still unverified; the command's output never breaks `--json`; tests use a command that passes, one that fails and one that outlives a short timeout, in each mode
- commit: cb9f26fa43fc4a217be0cf7a7fb27a7a485c4c9d
- reqs: R-VERIFY-1, R-VERIFY-2, R-LOCK-4, R-OUT-3, NFR-5, NFR-6
- by: Reynan Henry Tandoyo
- note (2026-11-19, claude-main): added
- note (2026-11-19, claude-main): Owner 2026-11-19: both modes wanted. Wave mode exists because in a compiled project one worker's half-finished edit breaks the build for every other worker's check; there the planner checks all tasks when the wave is finished.
- note (2026-11-20, t146-opus): T-146 decisions (R-VERIFY-1/2): in a linked worktree the command runs in the matching folder of that worktree, not the main checkout, so it checks the worker's own code; output goes to verify/<ID>.log in the state folder; the give-back after verify_attempts counts 'verify failed' notes since the latest unblocked/reopened/stuck note (attempts stays a lifetime total); a shell that cannot start, or verify=false on 'todo verify', is exit 5 with new code VERIFY_NOT_RUN and writes nothing; verify --wave with claimed tasks refuses with ALREADY_CLAIMED; set refuses verified and attempts (BAD_VALUE); reopen removes verified; default verify_shell is pwsh -NoProfile -NonInteractive -Command, else powershell.exe; each --wave result is its own locked write. Note texts are in format.md section 9.
- note (2026-11-20, t158-haiku): Decisions (T-158): a timeout's last line is 'timed out after N seconds'; a give-back after verify_attempts still exits 1 with VERIFY_FAILED; --no-verify without --force is USAGE (exit 2); apply's done runs no command (see T-173); check warns about unverified done tasks but not while verify = false; set and --unset refuse attempts and verified; the process-tree problem is T-172. The format 3 fixtures wait for T-148.
- note (2026-11-20, t158-haiku): done — done runs the verify command first; todo verify runs one check or the wave; verify settings and error codes VERIFY_FAILED and VERIFY_NOT_RUN; check warns about unverified done tasks; merged dev with T-148 as 25af8a7

### T-159 · todo stuck: give a task back to the planner with the problem
- status: done
- owner: t159-haiku
- priority: P1 · difficulty: easy · type: feature · area: cli
- milestone: T-157
- why: a weak model that cannot finish must have a clean way out, and the planner must learn what went wrong
- depends: T-146
- accept: `todo stuck <task> --note <problem>` sets the task to blocked, removes the owner and the claim, adds the note `stuck — <problem>` and raises `attempts` by one, in one write; the note is required; only the owner may do it, others need --force; the level is not changed; such a task is never handed out by `next` until the planner runs `unblock` or `split` on it; `summary` counts tasks given back; a test covers each case and each refusal
- commit: ce0f54bc7add4bd9757b8b3af0fc7e458e0bcdfb
- reqs: R-STUCK-1, R-WRITE-9, TEST-4
- by: Reynan Henry Tandoyo
- note (2026-11-19, claude-main): added
- note (2026-11-20, t146-opus): T-146 decision (R-STUCK-1): stuck works only on a claimed task (BAD_TRANSITION otherwise); a missing --note is exit 2 USAGE; a task is 'given back' when blocked and its latest blocked/stuck note is a stuck note, and that stuck note counts as the block reason for check, summary and the report (so check does not flag it).
- note (2026-11-20, t159-haiku): done — todo stuck <task> --note <problem>: claimed to blocked in one write (owner and claim removed, level kept, note stuck — <problem>, attempts +1); owner or --force only; BAD_TRANSITION unless claimed; summary lists tasks given back; stuck counts as the block reason; 7 integration tests and 4 unit tests added; 610 tests pass

### T-160 · done checks that the commit stayed inside the task's files
- status: done
- owner: t160-haiku
- priority: P1 · difficulty: medium · type: feature · area: core
- milestone: T-157
- why: in one shared checkout a worker that edits a file outside its task breaks another worker's task without anyone noticing
- depends: T-162
- accept: `done --commit` on a task with `files` compares the files that commit changed (from git, before the lock is taken) with the task's paths and patterns; a file outside them refuses with exit 1 and the new code `OUT_OF_SCOPE`, naming the files; `TODO.md` and the archive file are always allowed; --force overrides with a note; a lowest-level task with `files` cannot be finished without `--commit` while git is available; a task without `files`, or a folder without git, behaves as today; tests use a temporary repository with one commit inside and one outside the scope
- commit: 79e89eac664f5f86074564a4e9e9c2b9252cc603
- reqs: R-SCOPE-2, R-WRITE-4, NFR-5
- by: Reynan Henry Tandoyo
- note (2026-11-19, claude-main): added
- note (2026-11-20, t146-opus): T-146 decision (R-SCOPE-2): a task at or below the target level with files, finished without --commit while git is available, is refused with exit 1 MISSING_FIELD naming --commit HEAD; renames count both paths, merge commits are compared with their first parent; force note 'forced done outside files: <files>'.
- note (2026-11-20, t160-haiku): Decisions in the code: the target level is the first of difficulties (R-DETAIL-2) until target_difficulty exists from T-148; --force does not override the MISSING_FIELD rule (R-SCOPE-2 now says so); git is available when it is on PATH and the folder of TODO.md is inside a repository (R-SCOPE-2 now says so); apply does not check the commit's files yet (recorded as T-174).
- note (2026-11-20, t160-haiku): done — done --commit checks the commit's changed files against files (OUT_OF_SCOPE, --force notes them); a lowest-level task with files needs --commit (MISSING_FIELD); merge compared with first parent; renames count both paths; TODO.md and archive always allowed; 15 CLI tests, 9 path-rule tests, exit-code table case
- note (2026-11-20, t160-haiku): Order of the checks in done and apply after the merge with dev (verify, T-158/T-173): the commit scope is checked first, then the verify command runs; a commit outside the files, or a missing commit, runs no command. The scope is checked again under the lock (R-SCOPE-2, R-VERIFY-1).

### T-161 · Many workers at once without collisions
- status: done
- owner: claude-main
- priority: P1 · type: milestone
- milestone: T-145
- goal: Workers in one checkout never get tasks that touch the same file, and the planner sees how many workers are useful now and which chain of tasks limits the speed
- why: worktrees per agent cost disk space and a serial merge; file scopes let every agent work in the same folder
- by: Reynan Henry Tandoyo
- note (2026-11-19, claude-main): added
- note (2026-11-20, claude-main): done — File scopes (T-162, T-168, T-170) and todo waves (T-163, T-171) merged into dev.

### T-162 · File scopes: next never hands out tasks that share a file
- status: done
- owner: t162-haiku
- priority: P1 · difficulty: hard · type: feature · area: core
- milestone: T-161
- why: two tasks with no dependency between them still collide when both edit the same file
- depends: T-146
- accept: `files` is a comma-separated list of paths, folders (ending in `/`) and `*` patterns, relative to the folder of TODO.md, compared with `/` and `\` as the same and ignoring case on Windows; two tasks overlap when a path of one equals, lies under or can match a path of the other; `next` and `next --claim` skip a ready task that overlaps a task claimed by someone else whose claim is not stale; `claim <task>` on such a task is refused with exit 1 and the new code `FILES_BUSY`, naming the task in the way, unless --force; tasks without `files` are handed out as today; 10 parallel `next --claim` on tasks that pairwise share files hand out tasks that never overlap; `list --ready` marks tasks held back for this reason; `show --json` lists the overlapping claimed tasks
- commit: 2cb42c5dda393ccae5f6642ce578e614a8ee0fa7
- reqs: R-SCOPE-1, R-READ-3, R-CLAIM-2, TEST-5
- by: Reynan Henry Tandoyo
- note (2026-11-19, claude-main): added
- note (2026-11-20, t146-opus): T-146 decisions (R-SCOPE-1, format.md section 19): a task blocked with an owner and a non-stale claim also holds its files (an owner may have uncommitted edits); a trailing :<line> (as scan writes) is ignored; ** matches any number of folders; two patterns overlap unless their fixed parts before the first * differ (errs towards overlap); show --json field name files_busy; claim --force note 'forced claim (files held by <ID>)'.
- note (2026-11-20, t162-haiku): done — file scopes: files overlap rules (src/ops/scope.rs); next and next --claim skip files another agent holds; claim refused with FILES_BUSY unless --force (note: forced claim); show --json files_busy; list --ready marks held-back tasks; check warns on absolute or outside files entries; tests in tests/ops_scope.rs and tests/cmd_scope.rs, FILES_BUSY row in docs/json.md and the exit-code test

### T-163 · todo waves: how much can run in parallel, and the longest chain
- status: done
- owner: t163-haiku
- priority: P1 · difficulty: medium · type: feature · area: cli
- milestone: T-161
- why: the number of waves is the fewest rounds the work needs however many agents are spawned, so it is the number the planner should shrink
- depends: T-162
- accept: `todo waves [<milestone>]` groups the unfinished tasks into waves: wave 1 is everything ready now, wave n is what becomes ready when the waves before it are done, using each task's dependencies and those inherited from milestones; inside a wave, tasks that share files are shown as one lane because they must run one after another; it prints the number of waves, the size of each, the largest number of workers useful at once, and the longest dependency chain task by task; tasks waiting on a missing ID or in a cycle are listed apart; `waves --json` matches its snapshot; it reads only
- commit: 4fb0d313d1afd53f6e2c355f33578e6f92a309d8
- reqs: R-WAVE-1, R-DEP-1, R-MILE-6
- by: Reynan Henry Tandoyo
- note (2026-11-19, claude-main): added
- note (2026-11-20, t146-opus): T-146 decisions (R-WAVE-1): claimed tasks count in wave 1 as running; given-back and other blocked tasks, and tasks below a blocked milestone, are listed apart with the missing-ID and cycle ones; tasks not ready for lack of detail or level are placed by their dependencies and marked; workers useful at once = most lanes in any wave; JSON schema todo.waves.v1.
- note (2026-11-20, t163-haiku): done — todo waves [<milestone>]: waves, lanes (files overlap), the longest chain, workers at once and the tasks apart with reasons; JSON todo.waves.v1 in docs/json.md. Planner in src/ops/waves.rs (one Kahn pass), overlap_lanes in src/ops/scope.rs. Markers for missing level and detail (why, accept, steps, verify, files) placed by dependencies. Tests: cmd_waves (19 incl. snapshot), ops_scope lanes, docs_json example, overview snapshot updated. Merged dev (T-167, T-168) into the branch; merged tree passes fmt, clippy (all features, none, Linux) and cargo test --all-features (703 passed).

### T-164 · HTML report shows the planning state
- status: done
- owner: t164-haiku
- priority: P2 · difficulty: medium · type: feature · area: report
- milestone: T-145
- why: the page is the owner's only view, so it must show how far the planner is and where workers failed
- depends: T-149, T-163
- accept: each project page shows: open tasks per level; how many are not ready; tasks given back with the problem and their `attempts`; tasks kept above level 1 with the reason; the number of waves and the size of wave 1; each task shows its level, files and verify command when opened; the all-projects page shows open tasks per level on each card; snapshot tests are updated; still no button, form or command on the page
- commit: 973f8e90230034d0b4c7034192d28724a8d114e2
- reqs: R-REPORT-3, R-REPORT-1
- by: Reynan Henry Tandoyo
- note (2026-11-19, claude-main): added
- note (2026-11-20, t146-opus): T-146 decision: docs/requirements.md I-13 said R-REPORT-3 stays unused; T-146 and this task both name R-REPORT-3, so the ID is reused for the planning state and I-13 now says so.
- note (2026-11-20, t164-haiku): done — HTML report: each project page has a Planning section (open tasks per level, not ready for lack of detail with what is missing, given back with problem and attempts, kept above the target level with the final reason, waves and wave 1 size), from ops::plan::build_plan, ops::waves::plan_waves and detail_gaps, built once per page; an opened task shows level, files, verify and verified; each all-projects card shows open tasks per level from the page's count. R-REPORT-3 says open means open, claimed or blocked. Dev merged (T-149). Snapshots of both pages updated; tests/cmd_report_planning.rs (7) and one card test; no form, button or command; all text escaped. Release perf: single page 75 ms, 3.99 MB; all projects 582 ms. Full suite 904 passed, 0 failed; fmt and clippy clean on Windows and Linux.

### T-165 · Performance test for plan, waves and file scopes
- status: done
- owner: t163-haiku
- priority: P1 · difficulty: medium · type: test · area: core
- milestone: T-145
- why: with thousands of level-1 tasks, overlap and wave calculations must not make every `next` slow
- depends: T-149, T-163
- accept: on the generated 5,000-task milestone-tree file, extended so every task has files and a third of them share one, `next`, `plan`, `waves` and `brief` each take under 100 ms and `next --claim` under 200 ms in a release build; the test runs in CI with the other performance tests
- commit: 4a82f2a2716fc1286e9ec0a56158a3a6d2d51175
- reqs: NFR-3, TEST-9
- by: Reynan Henry Tandoyo
- note (2026-11-19, claude-main): added
- note (2026-11-20, t163-haiku): done — tests/perf.rs: a planning file from the T-105 generator extended to level 1 with files, why, steps and verify, a third of the tasks sharing one file; the T-105 tests keep their own file and budgets. New ignored release test: next, plan, waves and brief under 100 ms, next --claim --new under 200 ms; medians of five, release, 5,000 entries, 1.84 MB: next 82 ms, plan 83 ms, waves 92 ms, brief 41 ms, next --claim 146 ms. Causes found and fixed in T-182 (waves lanes) and T-183 (next). Whole perf set passes in release; cargo test --all-features: 896 passed.

### T-166 · v0.8 agent rules by role, documentation and release 0.8.0
- status: done
- owner: claude-main
- priority: P1 · difficulty: 3 · type: docs · area: release
- milestone: T-145
- why: the new commands only help when every agent knows which role it has and what to run
- depends: T-150, T-154, T-156, T-160, T-164, T-165
- accept: the rules are split by role so that no agent carries rules it does not need. The common rules block (AGENTS.md, TODO.md, the skill) is at most 15 lines and ends with one line per role naming its help text. `todo help worker` is at most 10 lines: take work with `todo next --claim --max-difficulty 1 --brief`, change only the listed files, commit only those files, `todo done <id> --commit HEAD`, `todo stuck <id> --note` when it cannot finish; `todo brief` repeats the last three with the task's ID filled in, so a worker needs no other text. `todo help planner` holds the planner's rules: give every task a level; split with `todo split` until `todo plan` is empty; write files, steps and verify for level-1 tasks; keep a higher level only with `--final`; read `todo waves` and break long chains; in wave mode run `todo verify --wave` when a wave is finished, otherwise the full test suite. A test fails when the common block grows past 15 lines or the worker text past 10. The README explains the roles, the levels, file scopes in one checkout and what the verify command is trusted to do; docs/json.md documents every new JSON shape and the codes `VERIFY_FAILED`, `OUT_OF_SCOPE` and `FILES_BUSY`; CHANGELOG.md has the 0.8.0 entry including the format change and the changed defaults; Cargo.toml is 0.8.0; `todo migrate --yes` was run in this repository and `todo check` is clean
- commit: e143d635e2656c0b3002f3b77c6c0dfcc45032ee
- reqs: R-AGENT-1, REL-2, REL-5
- by: Reynan Henry Tandoyo
- note (2026-11-19, claude-main): added
- note (2026-11-19, claude-main): Rules text for the planner: in wave mode run `todo verify --wave` when every worker of the wave has finished, before starting the next wave.
- note (2026-11-19, claude-main): Owner 2026-11-19: commands cost no context (nothing is loaded until `todo help <command>` is run), but the rules text does and it grew from 11 to 20 lines by v0.7. Keep the common rules short and move role rules into `todo help worker` and `todo help planner`. Lines moved out of the common block today (milestones, ideas, questions, migrate, update) go to the planner text or stay only where a refusal message already names the command.
- note (2026-11-20, t146-opus): T-146 decisions (R-AGENT-1): help update, migrate, milestones, ideas and questions move to the planner text; R-OUT-11 now says the planner rules name todo help update; D14 is marked as amended. docs/json.md must also document VERIFY_NOT_RUN (exit 5) besides VERIFY_FAILED, OUT_OF_SCOPE and FILES_BUSY, and the schemas todo.plan.v1, todo.brief.v1, todo.waves.v1.
- note (2026-11-20, t153-haiku): Local part finished on reynanTandoyo/t153 in 375a7d1 (after dev 9a45595): the common rules are 10 lines (src/agents/rules.md), todo help worker is 8 lines and todo help planner holds the planner rules; tests fail past 15 and 10 lines. README, docs/json.md (help topics, stuck, split, next --brief, done refusals, the report planning state), CHANGELOG 0.8.0 (format 3, the changed defaults, fixes T-167 to T-183, the report planning state and T-184) and Cargo.toml and Cargo.lock at 0.8.0 are done. The AGENTS.md block and the skill were reinstalled with the new binary in the worktree; CLAUDE.md imports AGENTS.md. Left for the coordinator: todo migrate --yes on the shared list (todo 0.8 check reports T-166 difficulty medium and a format 2 file until then), agents install and skill install in C:\Project\todo (its AGENTS.md block is outdated), tag, push, publish and install. T-166 stays claimed.
- note (2026-11-20, claude-main): Released 2026-11-20: tag v0.8.0 on e143d63, GitHub release published by the Release workflow. Installed on this PC with todo help update (SHA-256 matched, todo --version prints 'todo 0.8.0 (e143d63)'); todo migrate --yes moved TODO.md and TODO-archive.md to format 3; todo check is clean.
- note (2026-11-20, claude-main): forced done (owner was t153-haiku)
- note (2026-11-20, claude-main): done — v0.8 agent rules by role, docs and 0.8.0 released and installed.

### T-167 · Merge driver: a steps change next to another field's change conflicts
- status: done
- owner: t152-haiku
- priority: P2 · difficulty: medium
- milestone: T-145
- why: steps are merged as lines of the task block by git merge-file, so a change to the last step and a change to the line right after the list, made on different sides, conflict although they touch different fields; reproduced with merge-driver run on a base with steps and an accept line below them. A field-group merge for steps (like single-line fields) would avoid it.
- commit: 1ec53f93b28660043be2e4df62a9e59be0d51fb9
- by: Reynan Henry Tandoyo
- note (2026-11-20, t152-haiku): added
- note (2026-11-20, t152-haiku): done — merge driver: the steps list is one field (its field line and step lines) and is merged three-way like a single-line field; a steps change on one side and another field's change on the other merge cleanly, LF and CRLF; both sides changing the list differently conflicts; tests in tests/cmd_merge_driver.rs
- note (2026-11-20, claude-main): Coordinator 2026-11-20: accepted that two sides editing different lines of one steps list now conflict (the list is one field; a silent merge could reorder steps).

### T-168 · summary next ready list ignores file scopes
- status: done
- owner: t162-haiku
- priority: P2 · difficulty: medium
- milestone: T-145
- why: summary lists the next ready tasks with is_task_ready alone, so a task whose files another agent holds (R-SCOPE-1) appears there although next will not hand it out
- commit: 2f11e04c1c16e9e47fd3919a2fc4f0bf1b34b08c
- by: Reynan Henry Tandoyo
- note (2026-11-20, t162-haiku): added
- note (2026-11-20, t162-haiku): done — summary next ready leaves out tasks whose files another agent holds (R-SCOPE-1, R-READ-5): the same check as ready in show and list, applied after the sort so it stops at five; help text and docs/json.md say so; tests in tests/cmd_scope.rs

### T-169 · promote and apply make tasks without a level, which next never hands out
- status: done
- owner: t148-haiku
- priority: P2 · difficulty: medium
- milestone: T-145
- why: add refuses a task without a difficulty since T-148, but promote and apply still create one; it is never handed out until someone runs set --difficulty
- commit: 66493a07fea7486ec39671c3c1479f817416f1be
- by: Reynan Henry Tandoyo
- note (2026-11-20, t148-haiku): added
- note (2026-11-20, t148-haiku): done — promote takes --difficulty and refuses MISSING_FIELD without a level (the idea's own level is kept); apply refuses an add operation without a level when required_fields names difficulty, for the whole batch; tasks that are milestones, questions or ideas need none. Tests in tests/cmd_levels_entries.rs; full suite 706 passed; fmt and clippy (Windows, Linux, with and without all-features) pass.

### T-170 · done reports newly ready tasks that files another agent holds
- status: done
- owner: t162-haiku
- priority: P2 · difficulty: medium
- milestone: T-145
- why: newly_ready after done or drop uses is_task_ready alone (pipeline.rs), so a task that becomes dependency-free can be listed although next will not hand it out while an overlapping live claim of another agent holds its files (R-SCOPE-1)
- commit: 70df2d84bac9173017c3009bcc27254947a7c72a
- by: Reynan Henry Tandoyo
- note (2026-11-20, t162-haiku): added
- note (2026-11-20, t162-haiku): done — done and drop: newly_ready (text now-ready lines and JSON) leaves out a task whose files another agent's live claim holds, the same files_busy check as next and summary (R-DEP-4, R-SCOPE-1); docs/json.md says so; tests in tests/cmd_scope.rs
- note (2026-11-20, t162-haiku): after merging T-148, two file-scope tests failed because their newly ready task had no level; the fixtures now give level 2 and format 3 (commit c1ece85); no rule changed

### T-171 · waves places a task below a done or dropped milestone, which next never hands out
- status: done
- owner: t163-haiku
- priority: P2 · difficulty: medium
- milestone: T-145
- why: R-MILE-6 says such a task is not ready, but waves follows only the blocked-milestone rule from the T-146 note; the planner should decide whether waves lists it apart too
- commit: 4b45b97f97419e900a2325b12788ccfdf457abbd
- by: Reynan Henry Tandoyo
- note (2026-11-20, t163-haiku): added
- note (2026-11-20, t163-haiku): done — waves now holds back a task below a done, dropped or blocked milestone, listed apart with its reason, as next does (R-MILE-6). Shared rule: read::milestone_holds_back, used by is_task_ready and waves. R-WAVE-1, json.md and the waves help name all three states. Tests in cmd_waves: held-back reasons, dependents, done-milestone dependency, list --ready agreeing with wave 1 on two fixtures. fmt, clippy (Windows and Linux, with and without --all-features) and cargo test --all-features (705 passed) all pass.

### T-172 · A verify timeout stops only the shell, not the processes it started
- status: done
- owner: t158-haiku
- priority: P2 · difficulty: medium
- milestone: T-145
- why: R-VERIFY-1 says the command and the processes it started are stopped after verify_timeout_seconds. todo kills only the shell it started (std Child::kill). A build or test started by that shell keeps running after the timeout. Stopping the whole tree needs a Windows job object or a Unix process group, which needs unsafe code or a crate outside NFR-9; decide which.
- commit: e02dd3e93770e916af73bd7d82bb2735bcebd5b4
- by: Reynan Henry Tandoyo
- note (2026-11-20, t158-haiku): added
- note (2026-11-20, t158-haiku): Options (not implemented; needs an owner decision). Nothing in std stops a process tree: Windows needs a Job Object (KILL_ON_JOB_CLOSE) and Unix needs a process group sent a signal (process_group(0) is std, but kill needs libc). Cargo.lock already has libc 0.2.191 and windows-sys 0.61.2, but only as transitive dependencies. (1) Make libc and windows-sys direct dependencies: no new download, a change to the NFR-9 list, a few unsafe calls in todo. (2) Declare the few kernel32 and libc functions by hand: no dependency change, about 80 lines of unsafe FFI with hand-written signatures. (3) Keep the limit and document it. taskkill and kill as extra commands are excluded by NFR-5 and section 13.
- note (2026-11-20, t158-haiku): done — a verify timeout stops the whole process tree: a process group killed with killpg on Unix, a Job Object with KILL_ON_JOB_CLOSE on Windows; the tree is also stopped after a normal end; libc and windows-sys are direct dependencies (already in Cargo.lock; NFR-9 updated)

### T-173 · apply done runs no verify command, so the task has no passed check
- status: done
- owner: t158-haiku
- priority: P2 · difficulty: medium
- milestone: T-145
- why: apply writes every change under one lock, where no command may run (R-LOCK-4). A task finished with apply's done is done without verified until todo verify --wave checks it, while done itself would run the command. Decide whether apply should refuse done on a task that has a verify line.
- commit: dfbb44e9964d20193b92b9bd4ac5d72d73aaf1f0
- by: Reynan Henry Tandoyo
- note (2026-11-20, t158-haiku): added
- note (2026-11-20, t158-haiku): done — apply done runs the task's verify command before the lock, as done does; a failing or unrunnable check refuses the whole batch and writes nothing; tests in tests/cmd_verify.rs

### T-174 · todo apply does not check a done commit against the task's files
- status: done
- owner: t160-haiku
- priority: P2 · difficulty: medium
- milestone: T-145
- why: R-SCOPE-2 is checked by done --commit only. A done operation with a commit inside todo apply is not compared with the task's files, so a worker can change a file outside its task through apply
- commit: f7ce1a03c341db82f51ad96a5b7a4053c7cb3ed2
- by: Reynan Henry Tandoyo
- note (2026-11-20, t160-haiku): added
- note (2026-11-20, t160-haiku): done — apply checks a done commit's files as done does: OUT_OF_SCOPE refuses the batch, --force notes the files, MISSING_FIELD and unknown commits as done; 6 tests, four fail with the check disabled

### T-175 · apply writes a done commit as given: HEAD stays in TODO.md where done writes the hash
- status: done
- owner: t160-haiku
- priority: P2 · difficulty: medium
- milestone: T-145
- why: apply does not resolve its commit the way done does, so a done operation writes 'commit: HEAD', which names whatever HEAD is when it is read, not the commit that finished the task
- commit: 0ed6316b4dd9595c261647809e49d70fff31061f
- by: Reynan Henry Tandoyo
- note (2026-11-20, t160-haiku): added
- note (2026-11-20, t160-haiku): done — apply writes a done commit's value as done does: HEAD and names resolved to the short hash through the shared resolver, a full hash kept; three tests, two fail with resolution disabled

### T-177 · attempts and verified are read as unknown fields, not as typed task fields
- status: done
- owner: t150-haiku
- priority: P2 · difficulty: medium
- milestone: T-145
- why: format.md section 8 lists attempts and verified as task fields, but the parser keeps them as unknown fields: show --json lists them only under fields and schema does not list them as task fields. final is typed since T-150 and the other two should follow the same rule
- commit: a0a0f01c6c9eff67b47b896c865b8c7816e18368
- by: Reynan Henry Tandoyo
- note (2026-11-20, t150-haiku): added
- note (2026-11-20, t150-haiku): done — attempts and verified are typed task fields: the parser reads them on their own line and on the priority line, show --json and schema list them, writes go through the edit builder as before; 3 tests (parser, show and schema, split removes final)

### T-178 · set --goal with a separator on a combined priority line corrupts the line
- status: done
- owner: t156-haiku
- priority: P2 · difficulty: medium
- milestone: T-145
- why: set --goal 'lights · shadows' on a task whose goal sits on its priority line writes the value as it is: the goal reads as 'lights' and check reports 'shadows' as a part that is not key: value. set_part_field does not move such a value to a line of its own, as set_line_field does.
- commit: da7c236be31d861abfa6733b71232eecb68899ac
- by: Reynan Henry Tandoyo
- note (2026-11-20, t156-haiku): added
- note (2026-11-20, t156-haiku): done — set --goal, --type and --area put a value that holds the part separator on its own line instead of the combined priority line (edits.rs); add and ask do the same for a new type or area (NewTask, ask.rs). set --priority with the separator is refused (BAD_VALUE), since the priority is the first part of the line and cannot move. Merged dev 4aef723 into the branch (fast-forward, no conflicts). Six new tests in tests/cmd_combined_line.rs; they fail on the code before this commit and all 21 pass after it; format.md section 8 documents the rule. Recorded a new problem: set --priority accepts a value that is not configured.

### T-179 · set --priority accepts a value that is not a configured priority
- status: done
- owner: t156-haiku
- priority: P2 · difficulty: medium
- milestone: T-145
- why: set --priority P9 writes P9 with no check (set does not validate priority against the configured priorities), while add and promote refuse it with BAD_VALUE. Found while working on T-178.
- commit: b306792639e2f2f527ad873ed70b2ea6c0654d51
- by: Reynan Henry Tandoyo
- note (2026-11-20, t156-haiku): added
- note (2026-11-20, t156-haiku): done — set --priority, --field priority and apply's set now check the priority with OpsConfig::canonical_priority, the function add uses (also used by ask, promote and apply's add): a value that is not configured is refused with BAD_VALUE and the same message as add, and nothing is written. A configured priority is written in its configured spelling. The check in T-178 for the separator is replaced by this one. Merged dev e801fab (fast-forward). Five new tests (four in tests/cmd_set.rs, one in tests/cmd_apply.rs); five fail before the change. R-WRITE-8 says so.

### T-180 · README lists only some todo.toml keys: the verify and target keys are missing
- status: done
- owner: t153-haiku
- priority: P2 · difficulty: medium
- milestone: T-145
- why: a user reads the README for the settings a project can set, and the list leaves out the verify_* keys
- commit: 2e1c21e0ac77748e21e32e7fc29910203e4c11fa
- by: Reynan Henry Tandoyo
- note (2026-11-20, t153-haiku): added
- note (2026-11-20, t153-haiku): done — README configuration table lists all 24 todo.toml keys the code reads, with defaults matching docs/requirements.md R-CFG-2 (target_difficulty, level1_max_files, verify settings, difficulties, required_fields and the older ones).

### T-181 · plan leaves out a task whose difficulty is set but not configured, and next hands it out
- status: done
- owner: t153-haiku
- priority: P2 · difficulty: medium
- milestone: T-145
- why: a level that todo.toml does not configure is in no planner group, and next treats it as ready; check reports it but plan and next should agree
- commit: 17a637a55d84b3ba978c849c8e1c71c93a7dceae
- by: Reynan Henry Tandoyo
- note (2026-11-20, t153-haiku): added
- note (2026-11-20, t153-haiku): done — A difficulty that is set but not configured counts as no level: next, list --ready, show (ready), the claim filter of next, plan (without a level, with the reason), waves and summary read one rule (lacks_level with the config, level_problem for the reason). Tests in tests/cmd_unconfigured_level.rs; two docs fixtures moved to configured levels.

### T-182 · waves takes 118 ms on the 5,000-task milestone tree (budget 100 ms): lane grouping compares every file with every other file under src/
- status: done
- owner: t163-haiku
- priority: P1 · difficulty: medium
- milestone: T-145
- why: T-165's planning budget test fails on waves (median of five release runs, 118 ms). overlap_lanes buckets entries by their first path part, and most files start with src, so each entry is compared with all earlier ones in one bucket: quadratic for a wave of about 4,000 tasks. Exact-text lookup for single files fixes it; folders and patterns still compare with everything before them
- commit: 297767f96a34493436aead92468414a7349d0745
- by: Reynan Henry Tandoyo
- note (2026-11-20, t163-haiku): added
- note (2026-11-20, t163-haiku): done — overlap_lanes looks single files up by their text (a map), not by their first path part: a wave of about 4,000 tasks under src/ was quadratic. waves on the 5,000-task planning file: 118 ms before, 84 to 92 ms after, median of five in release (T-165). The lane tests in tests/ops_scope.rs compare with the pairwise rule and pass.

### T-183 · next takes 95 ms on the 5,000-task milestone tree, near its 100 ms budget: select_next_task computes files_busy for every ready task, which scans every task each time
- status: done
- owner: t163-haiku
- priority: P2 · difficulty: easy
- milestone: T-145
- why: found while measuring T-165: next evaluates files_busy inside min_by_key, so each ready task (about 4,600 here) scans all tasks. Sorting the ready tasks and checking files only for the first one that holds no files gives the same answer with one scan
- commit: 0ff614ed1a1b38094c9da86a7d697104b75878e0
- by: Reynan Henry Tandoyo
- note (2026-11-20, t163-haiku): added
- note (2026-11-20, t163-haiku): done — select_next_task: ready candidates sorted once (each key computed once, file index breaks ties), then the first one whose files no other agent holds is taken, so files_busy runs until one passes. Two commits: 077e5f7 (checks the files in order) and 0ff614e (keys once). next on the 5,000-task planning file: 95 to 137 ms before, 82 ms after (median of five, release). Next, claim, scope and ready-tree tests pass.

### T-184 · Report pages are large: the facts block of every task adds about 250 bytes, so ten projects of 5,000 entries make a 40 MB all-projects page
- status: done
- owner: t154-haiku
- priority: P2 · difficulty: medium
- milestone: T-145
- why: T-164 added the level, files, verify and verified block to every opened task (R-REPORT-3). The single page is 3.99 MB at 5,000 entries (budget 5 MB) and the all-projects page of ten such projects is 40 MB; both are within the time budgets, but a page this size is slow to open and to share. Shorter wording, or the block only for tasks that have a check, would cut it.
- commit: 82eb839cff913507ea50a59763564d1734f217fc
- by: Reynan Henry Tandoyo
- note (2026-11-20, t164-haiku): added
- note (2026-11-20, t154-haiku): done — Page size: all-projects page of ten 5,000-entry projects 40,079,554 -> 28,432,052 bytes (single page 3,995,518 -> 2,830,937); release report timings 74 ms and 579 ms, budgets 500 ms and 5 s. Each opened task shows Level/Files/Verify (and Verified when there is a check) on one line, none for missing parts; the per-task wrapper, the row breaks, the empty notes block, the task row classes and the priority/area chip titles are gone. Left: the fields lists (required, as written) and the dt/dd markup, about 17% of the page; omitting the optional end tags of dt/dd or dropping duplicate fields would take it further, left to the owner. report_basic.md moved to format 3 (todo migrate on a copy).

### T-185 · next is too slow on the 5,000-task milestone tree in CI: 156 ms Linux, 185 ms Windows, budget 100 ms
- status: done
- owner: t185-haiku
- priority: P1 · difficulty: 3 · type: bug · area: core
- milestone: T-145
- why: CI is red on main since 0.8.0: perf_5000_entry_milestone_tree_next_plan_waves_brief_meet_budgets fails on both runners; locally next takes 82 ms, so the slower CI machines show the real headroom is too small
- commit: 20a50e6f5dae36a3d7ae1279abc4b5253aa95198
- by: Reynan Henry Tandoyo
- note (2026-11-20, claude-main): added
- note (2026-11-21, t185-haiku): done — next judges readiness only in candidate order and stops at the first ready task whose files are free; open questions indexed once per selection (OpenQuestionTargets), detail rule checks read paths last. Output unchanged (byte-identical on 10 commands). Perf test medians, 5,000-entry milestone tree: next 72.2 ms before, 45.3 ms after; plan 77.6 -> 75.3; waves 81.0 -> 84.0; brief 38.9 -> 43.1 (noise); next --claim 129.4 -> 105.6. Checks: fmt, clippy (all features, default, linux), cargo test --all-features all pass, perf tests pass.

### T-186 · plan, waves, next and next --claim need about 2.5x headroom for the CI runners
- status: done
- owner: t185-haiku
- priority: P1 · difficulty: 4 · type: bug · area: core
- milestone: T-145
- why: CI on windows-latest measured plan 170 ms, waves 134 ms, next 185 ms and next --claim 196 ms (budgets 100/100/100/200, NFR-3); the runner is about 2.3x slower than the dev PC, so locally they must be under about 40 ms (next --claim under about 85 ms)
- commit: 8b18774c5733920cc5a12ce6c2c81f272921a41b
- by: Reynan Henry Tandoyo
- note (2026-11-21, claude-main): added
- note (2026-11-21, t185-haiku): done — plan and waves judge open questions through one index per command (OpenQuestionTargets, detail_gaps_with), and plan_waves groups levels in one pass; next, plan, waves and brief end with LoadedRead::leak_for_exit after output (read-only only). Output byte-identical to the T-185 binary on next, plan, waves, brief, list --ready, show --json, summary and tree, text and JSON. Perf-test medians, T-185 then T-186: next 45.3 -> 35.1 ms, plan 75.3 -> 34.7, waves 84.0 -> 43.1 (target 40, missed), brief 43.1 -> 31.7, next --claim 105.6 -> 95.6 (target 85, missed). The claim parses the file twice and drops two documents; reaching 85 needs the parser or the document model to change. Checks: fmt, clippy (all features, default, linux), cargo test --all-features all pass; perf tests pass on the rerun (first run failed on claim 200.1 ms under other agents' cargo load).

### T-187 · next --claim parses the file twice: 229 ms on windows-latest CI, budget 200 ms
- status: done
- owner: claude-main
- priority: P1 · difficulty: 4 · type: bug · area: core
- milestone: T-145
- why: After T-186 every other planning timing has headroom on both CI runners, but next --claim measured 229 ms on one windows-latest run (passed on another), so CI on main is red by chance; it reads and parses the 1.8 MB file twice and drops two documents
- commit: 620851f785d0e49ddde354600a6c37d55675d4c4
- by: Reynan Henry Tandoyo
- note (2026-11-21, claude-main): added
- note (2026-11-21, t185-haiku): Choice: next --claim already chooses its task under the write lock, from the one document that the write reads there, so no pre-lock parse exists to remove. Its second parse is the R-EDIT-7 check of the edited bytes, which stays. Change (commit 620851f): the claim keeps its two documents (the parsed input and the edited one) until the process exits, via edit_doc_keeping_documents; the file is still written and the lock released as before. In-process claim totals, 11 interleaved claims each: 148 ms before, 113 ms after. Perf-test next --claim medians: 95.6 ms before; 76 to 121 ms after across six runs under desktop load. Target 70 is not reached: the remaining cost is the two parses (about 18 and 15 ms), the snapshot (about 10), the atomic write (about 9) and a fresh-file read penalty (about 25 ms) after each write. Option for the planner: parse task blocks in parallel (needs approval, since it adds threads to the parser). Perf gate: the planning test passed in 2 of 6 runs today; the other runs failed on tree, show, add and waves, which this change does not touch, under desktop load.
- note (2026-11-21, claude-main): Coordinator 2026-11-21: CI green on dev and main after 0a90ef4; next --claim 135 ms and 117 ms on windows-latest (was 229). The 70 ms local target was not reached, but the CI budget is met with headroom; parallel parsing not pursued.
- note (2026-11-21, claude-main): forced done (owner was t185-haiku)
- note (2026-11-21, claude-main): done — next --claim keeps its documents until exit; CI windows-latest 135/117 ms (budget 200).

### T-188 · waves has no headroom on windows-latest CI: 99.3 ms of 100 ms
- status: done
- owner: claude-main
- priority: P2 · difficulty: 4 · type: bug · area: core
- milestone: T-145
- why: After T-187 CI is green, but one windows-latest run measured waves at 99.3 ms (the other 52.6 ms), so the perf test can fail by chance; waves takes about 43 ms locally and needs to get under about 30 ms
- commit: ae3902b26443692d17de15d10013d9f8d5bc7353
- by: Reynan Henry Tandoyo
- note (2026-11-21, claude-main): added
- note (2026-11-21, t188-haiku): Commit ae3902b (branch reynanTandoyo/t188), not marked done: waves 46.6 ms before and 39-40 ms after in the T-165 perf test (medians of 5; HEAD re-run 41.6 and 43.0 ms, same PC). Parsing takes about 19 ms and each planning command pays about 21 ms of load before any planning, so the 30 ms target needs waves' own work under about 3 ms; it is now about 6-8 ms. Left: flat planner arrays, readiness without re-parsing milestone IDs, text rendered from the plan, parsed file scopes reused for the gaps (about 3-4 ms in all). The larger lever is the parser: about 3 Strings per field line and one per line, which needs Task storage to borrow from the document (a redesign).
- note (2026-11-21, claude-main): Coordinator 2026-11-21: accepted at 39-40 ms locally. CI after 511ba0d is green on both runners, waves at most 71 ms (budget 100). The 30 ms target needs a parser redesign (borrowed lines and fields), left for the owner to decide.
- note (2026-11-21, claude-main): forced done (owner was t188-haiku)
- note (2026-11-21, claude-main): done — waves 46.6 to about 40 ms locally; CI waves at most 71 ms.

### T-189 · Release 0.8.1: CI speed fixes
- status: done
- owner: claude-main
- priority: P1 · difficulty: 2 · type: release · area: release
- why: the speed fixes T-185 to T-188 and the CI dependency list fix are on main but not in a release; the owner asked for 0.8.1 on 2026-11-21
- commit: 84c225b434d7141306a679b567e2ef309cf83bdd
- by: Reynan Henry Tandoyo
- note (2026-11-21, claude-main): added
- note (2026-11-21, claude-main): done — Released 2026-11-21: tag v0.8.1 on 84c225b, published by the Release workflow; installed on this PC with todo help update (SHA-256 matched, todo 0.8.1 (84c225b)); stamp raised to 0.8.1.

## Questions

### T-176 · T-172: a verify timeout should stop the whole process tree. Which option: (1) libc and windows-sys as direct dependencies (already in Cargo.lock, NFR-9 list changes), (2) hand-written FFI with no dependency change, or (3) keep the limit and document it? Details are in the note on T-172.
- status: done
- owner: claude-main
- type: question
- about: T-172
- note (2026-11-20, t158-haiku): added
- note (2026-11-20, claude-main): answer — Option 1: make libc (Unix) and windows-sys (Windows) direct dependencies, target-specific, with only the features needed; both are already in Cargo.lock. Add them to the NFR-9 list with the reason (stop a verify command's whole process tree on timeout, NFR-5 forbids running kill/taskkill). Keep the unsafe calls few, small and commented.
