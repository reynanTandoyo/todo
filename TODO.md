# todo — task list

The work plan for building `todo` itself. The reasons behind every task are in
[`docs/requirements.md`](docs/requirements.md); each task lists the requirement IDs it covers in
its `reqs:` line.

**Until `todo` can manage this file, it is edited by hand.** Once the claim and write commands pass
their tests (section 3), this file is managed with `todo`, like every other project.

The format is the airline-ops-web block format, the candidate canonical format (decision F1 in
`docs/requirements.md` §7). If F1 picks another format, this file is converted with the importer
and becomes that importer's first real test.

Fields: `status` is `open`, `claimed`, `blocked` or `done`. `priority`: P0 = needed for the first
usable version, P1 = needed before rollout to your projects, P2 = valuable later, P3 = idea.
`reqs` = requirement IDs. `depends` = tasks that must be done first. `accept` = the check that
proves the task is finished.

## 0. Decisions for the owner

### T-001 · Decide the file layout and canonical format (F1)
- status: open
- owner: —
- priority: P0 · type: decision · area: format
- reqs: §7, F1
- accept: the choice is written into docs/requirements.md §12 and T-010 can start
- note (2026-11-15, claude): options and a recommendation are in docs/requirements.md §7; all parser work waits for this

### T-002 · Decide claim timeout, done-task placement, worktree mode and a "won't do" status (F2–F5)
- status: open
- owner: —
- priority: P1 · type: decision · area: behaviour
- reqs: F2, F3, F4, F5
- accept: the four answers are written into docs/requirements.md §12

### T-003 · Choose a licence for the public repository (F6)
- status: open
- owner: —
- priority: P1 · type: decision · area: release
- reqs: F6, REL-5
- accept: LICENSE file committed and named in the README

### T-004 · Answer the questions about the office projects
- status: open
- owner: —
- priority: P1 · type: decision · area: rollout
- reqs: §12 office questions
- accept: answers written into docs/requirements.md §12 and any changed requirements updated

### T-005 · Decide which agents get hooks, skills or an MCP server (F7, F8)
- status: open
- owner: —
- priority: P2 · type: decision · area: agent
- reqs: F7, F8
- accept: answers written into docs/requirements.md §12

## 1. Foundations

### T-010 · Write docs/format.md: the exact grammar of the canonical format
- status: open
- owner: —
- priority: P0 · type: docs · area: format
- reqs: §7.3, REL-4
- depends: T-001
- accept: every line shape in the fixtures is covered by the grammar; the owner has reviewed it

### T-011 · Create invented, labelled test fixtures
- status: open
- owner: —
- priority: P0 · type: test · area: format
- reqs: TEST-9, TEST-1
- accept: small fixtures for the canonical format and both old formats, each starting with a test-fixture notice, covering CRLF, byte-order mark, no newline at end, code fences, rule examples in the preamble, unknown fields and non-ASCII text

### T-012 · Line-span document model with a byte-exact round trip
- status: open
- owner: —
- priority: P0 · type: feature · area: core
- reqs: R-EDIT-1, R-EDIT-2, R-EDIT-5, R-EDIT-9, R-EDIT-10, TEST-2
- depends: T-011
- accept: parse and write back gives identical bytes for every fixture; the random-edit property test passes; a write whose result fails the outside-bytes check is refused

### T-013 · Task parser for the canonical format
- status: open
- owner: —
- priority: P0 · type: feature · area: core
- reqs: R-EDIT-3, R-EDIT-4, R-EDIT-6, R-EDIT-7, R-EDIT-11, R-ID-1, R-ID-3, R-DEP-1, R-OUT-8, TEST-1
- depends: T-010, T-012
- accept: parser unit tests pass; an ambiguous fixture and a fixture with conflict markers are refused as `file:line: message`

### T-014 · Errors, exit codes and the JSON envelope
- status: open
- owner: —
- priority: P0 · type: feature · area: cli
- reqs: R-OUT-1, R-OUT-2, R-OUT-5, NFR-5, NFR-10
- accept: docs/json.md lists every error code; Clippy rejects unwrap and expect outside tests; output piped into a command that stops early ends quietly

### T-015 · Command-line skeleton: global options and help
- status: open
- owner: —
- priority: P0 · type: feature · area: cli
- reqs: R-OUT-6, R-CFG-1, R-CFG-4, R-CLAIM-6
- depends: T-014
- accept: `todo`, `todo help <command>`, `--file`, `--json`, `--agent` with `TODO_AGENT` fallback, and `--dry-run` are parsed and tested

### T-016 · Correct non-ASCII output in Windows consoles
- status: open
- owner: —
- priority: P0 · type: feature · area: cli
- reqs: R-OUT-3, R-OUT-4
- accept: `·`, `—` and Japanese text display correctly in PowerShell 5, PowerShell 7 and cmd; piped output is plain UTF-8

## 2. Safety: finding, locking, writing

### T-020 · Find TODO.md: --file, TODO_FILE, walk up to the repository root
- status: open
- owner: —
- priority: P0 · type: feature · area: find
- reqs: R-FIND-1, R-FIND-2, R-FIND-3, R-FIND-4, R-FIND-7, R-FIND-8, NFR-7, TEST-5
- accept: search tests pass, including "no TODO.md in this repository" not climbing into a parent folder, and paths with spaces, non-ASCII characters and more than 260 characters

### T-021 · Linked worktrees use the main checkout's TODO.md
- status: open
- owner: —
- priority: P0 · type: feature · area: find
- reqs: R-FIND-5, NFR-4, TEST-5
- depends: T-020
- accept: a test creates a real worktree in a temporary folder and resolves to the main file; `--local` opts out; works without git.exe on PATH

### T-022 · State folder in the git common directory
- status: open
- owner: —
- priority: P0 · type: feature · area: lock
- reqs: R-HIST-0, R-LOCK-6
- depends: T-021
- accept: the main checkout and a worktree get the same state folder; outside git it is .todo/ next to the file

### T-023 · Write lock: operating-system file lock with wait, timeout and holder info
- status: open
- owner: —
- priority: P0 · type: feature · area: lock
- reqs: R-LOCK-1, R-LOCK-2, R-LOCK-3, R-LOCK-8, R-LOCK-9, TEST-4
- depends: T-022
- accept: parallel add and claim tests across processes pass; a killed holder blocks others for under 2 s; a held lock makes a command exit 3 without writing

### T-024 · Atomic replace with flush and Windows retry
- status: open
- owner: —
- priority: P0 · type: feature · area: io
- reqs: R-IO-1, R-IO-2, R-IO-3, R-IO-4, R-IO-6, R-IO-7, NFR-7, TEST-6
- accept: replacing while another handle holds the file retries and then succeeds; temporary files never remain after a failure; invalid UTF-8 and read-only files are refused; a change made by another program during the write makes the command exit 3 without writing

### T-025 · Manual lease: lock, unlock, lock status
- status: open
- owner: —
- priority: P0 · type: feature · area: lock
- reqs: R-LOCK-4
- depends: T-023
- accept: a lease blocks other agents but not its holder; it expires after 10 minutes (tested with a shortened time); unlock --force is recorded

### T-026 · todo where
- status: open
- owner: —
- priority: P1 · type: feature · area: find
- reqs: R-FIND-6
- depends: T-022
- accept: prints file, how it was found, state folder, lock and lease status; --json tested

## 3. Commands: parity with todo.js and todo.py

### T-030 · list with filters and --json
- status: open
- owner: —
- priority: P0 · type: feature · area: cli
- reqs: R-READ-1, R-READ-7
- depends: T-013, T-015
- accept: every filter has a test; default hides done tasks; JSON snapshot test

### T-031 · show
- status: open
- owner: —
- priority: P0 · type: feature · area: cli
- reqs: R-READ-2, R-DEP-2
- depends: T-013, T-015
- accept: raw output equals the task's bytes; JSON includes unknown fields, notes and dependency status

### T-032 · claim and release, with stale takeover and --force
- status: open
- owner: —
- priority: P0 · type: feature · area: cli
- reqs: R-WRITE-2, R-WRITE-3, R-CLAIM-1, R-CLAIM-2, R-CLAIM-3, R-CLAIM-4, R-CLAIM-6, R-IO-5, TEST-3
- depends: T-023, T-024, T-030
- accept: parallel claims from separate processes have exactly one winner; re-claim by the owner is a no-op; a stale claim is taken over with a note

### T-033 · next, with dependencies, model filter and --claim
- status: open
- owner: —
- priority: P0 · type: feature · area: cli
- reqs: R-READ-3, R-DEP-1
- depends: T-032
- accept: N processes running `next --claim` get N different tasks; a task with an unfinished dependency is never returned

### T-034 · done with --note and --commit
- status: open
- owner: —
- priority: P0 · type: feature · area: cli
- reqs: R-WRITE-4, R-DEP-4, NFR-4, TEST-3
- depends: T-032
- accept: refusals for non-owner and blocked tasks tested; --commit checked when git is available; accept line printed

### T-035 · block and unblock
- status: open
- owner: —
- priority: P0 · type: feature · area: cli
- reqs: R-WRITE-5, R-WRITE-6, TEST-3
- depends: T-032
- accept: block requires a reason; unblock returns to claimed or open; ownership rules tested

### T-036 · note
- status: open
- owner: —
- priority: P0 · type: feature · area: cli
- reqs: R-WRITE-7, R-IO-5, TEST-3
- depends: T-023, T-024
- accept: notes append after the last note, also on done tasks; date is the local date

### T-037 · set
- status: open
- owner: —
- priority: P0 · type: feature · area: cli
- reqs: R-WRITE-8, TEST-3
- depends: T-032
- accept: every option and --field key=value tested; the combined priority line is updated in place

### T-038 · add: section matching, duplicate check, ID allocation
- status: open
- owner: —
- priority: P0 · type: feature · area: cli
- reqs: R-WRITE-1, R-ID-2, TEST-3
- depends: T-023, T-024
- accept: ambiguous section refused; duplicate title refused; parallel adds get distinct IDs

### T-039 · --dry-run on every write command, preview by default for large changes
- status: open
- owner: —
- priority: P1 · type: feature · area: cli
- reqs: R-EDIT-8, R-OUT-9
- depends: T-038
- accept: every write command prints a diff and leaves the file byte-identical; restore, import, archive and the install commands change nothing without --yes

### T-040 · Matching by title and alias
- status: open
- owner: —
- priority: P1 · type: feature · area: cli
- reqs: R-ID-4, R-ID-5
- depends: T-032
- accept: an ambiguous match is refused with the candidates listed; rule examples in the preamble never match

### T-041 · move and reopen
- status: open
- owner: —
- priority: P1 · type: feature · area: cli
- reqs: R-WRITE-9
- depends: T-038
- accept: the task moves with all its lines; reopen writes a note

### T-042 · ask and answer: questions for the owner
- status: open
- owner: —
- priority: P1 · type: feature · area: cli
- reqs: R-WRITE-10
- depends: T-038
- accept: next never returns a question; answer records the reply and closes or reopens it

### T-043 · Option to move done tasks to a Done section
- status: open
- owner: —
- priority: P1 · type: feature · area: cli
- reqs: R-WRITE-11
- depends: T-002, T-034
- accept: with done_moves_to set, the task and its notes move to the top of that section

### T-044 · Text from standard input or @file
- status: open
- owner: —
- priority: P1 · type: feature · area: cli
- reqs: R-WRITE-13
- depends: T-015
- accept: text with quotes, backticks and $ arrives unchanged from PowerShell via - and @file

### T-045 · release --all
- status: open
- owner: —
- priority: P2 · type: feature · area: cli
- reqs: R-CLAIM-7
- depends: T-032
- accept: frees every claim of one agent in one locked step

### T-046 · Project configuration file todo.toml
- status: open
- owner: —
- priority: P1 · type: feature · area: config
- reqs: R-CFG-2
- depends: T-015
- accept: every setting has a test; an unknown setting is an error, not ignored

## 4. Recovery, checks and bulk work

### T-050 · Snapshots: history, diff, restore
- status: open
- owner: —
- priority: P1 · type: feature · area: history
- reqs: R-HIST-1
- depends: T-022, T-024
- accept: every write leaves a snapshot; restore is locked and itself snapshotted; old snapshots are pruned

### T-051 · Journal and log
- status: open
- owner: —
- priority: P2 · type: feature · area: history
- reqs: R-HIST-2, R-READ-6
- depends: T-050
- accept: each write appends one journal line; log <id> shows the task's history

### T-052 · Detect edits made outside the tool
- status: open
- owner: —
- priority: P1 · type: feature · area: history
- reqs: R-HIST-3
- depends: T-050
- accept: a hand edit between two commands is snapshotted and reported once

### T-053 · check
- status: open
- owner: —
- priority: P1 · type: feature · area: check
- reqs: R-CHECK-1, R-DEP-3
- depends: T-013
- accept: a fixture with each listed problem is reported with line numbers; a clean fixture exits 0

### T-054 · check --fix
- status: open
- owner: —
- priority: P2 · type: feature · area: check
- reqs: R-CHECK-2
- depends: T-053
- accept: only the listed safe fixes are applied; everything else is still reported

### T-055 · summary
- status: open
- owner: —
- priority: P1 · type: feature · area: cli
- reqs: R-READ-4
- depends: T-030
- accept: under 40 lines on a 5,000-task file; JSON version tested

### T-056 · search
- status: open
- owner: —
- priority: P2 · type: feature · area: cli
- reqs: R-READ-5
- depends: T-030
- accept: finds text in fields and notes; --archive includes the archive file

### T-057 · archive
- status: open
- owner: —
- priority: P1 · type: feature · area: cli
- reqs: R-ARCH-1
- depends: T-034
- accept: done tasks move to TODO-archive.md; show, search --archive and depends still find them; IDs are not reused

### T-058 · Bulk add from JSON
- status: open
- owner: —
- priority: P1 · type: feature · area: cli
- reqs: R-WRITE-12
- depends: T-038
- accept: 30 tasks are added in one locked step with consecutive IDs; one invalid entry rejects the whole batch

### T-059 · todo edit
- status: open
- owner: —
- priority: P2 · type: feature · area: lock
- reqs: R-LOCK-5, R-CHECK-3
- depends: T-025, T-053
- accept: takes a lease, opens the editor, checks and snapshots after it closes, releases the lease

### T-060 · apply: several operations in one locked step
- status: open
- owner: —
- priority: P2 · type: feature · area: cli
- reqs: R-WRITE-14
- depends: T-058
- accept: all operations apply or none do; one snapshot per batch

## 5. Agent integration

### T-070 · todo init
- status: open
- owner: —
- priority: P1 · type: feature · area: agent
- reqs: R-AGENT-1
- depends: T-071
- accept: creates TODO.md with preamble, rules block, sections and format line; refuses if TODO.md exists

### T-071 · The rules text: help agents and the managed block
- status: open
- owner: —
- priority: P1 · type: feature · area: agent
- reqs: R-AGENT-5, R-AGENT-2
- depends: T-030
- accept: one source text feeds `todo help agents`, the AGENTS.md block and the TODO.md header block

### T-072 · agents install and agents check
- status: open
- owner: —
- priority: P1 · type: feature · area: agent
- reqs: R-AGENT-2
- depends: T-071
- accept: installs or updates the marked block without touching other text; reports drift

### T-073 · Claude Code hooks: session summary and blocking direct edits of TODO.md
- status: open
- owner: —
- priority: P2 · type: feature · area: agent
- reqs: R-AGENT-3
- depends: T-005, T-055, T-071
- accept: a new session starts with the summary (at most 40 lines, under 2 s); an Edit or Write on TODO.md is refused with the command to use; other files are unaffected; preview without --yes; uninstall removes only the tool's own entries; works from PowerShell

### T-074 · Claude Code skill
- status: open
- owner: —
- priority: P2 · type: feature · area: agent
- reqs: R-AGENT-4
- depends: T-071
- accept: `todo skill install` writes .claude/skills/todo/SKILL.md generated from the same rules text

## 6. Migration and rollout

### T-080 · Import the MusicDownloader checkbox format
- status: open
- owner: —
- priority: P1 · type: feature · area: migrate
- reqs: R-MIG-1
- depends: T-001, T-038
- accept: the checkbox fixture converts with every item, claim, note and commit kept; preview by default, writes only with --yes

### T-081 · Honour the old scripts' locks during the switch-over
- status: open
- owner: —
- priority: P1 · type: feature · area: migrate
- reqs: R-LOCK-7
- depends: T-023
- accept: with legacy_lock set, todo waits while todo.js or todo.py holds its lock, and they wait while todo holds it

### T-082 · Accept both old calling styles
- status: open
- owner: —
- priority: P1 · type: feature · area: migrate
- reqs: R-MIG-3, TEST-10
- depends: T-040
- accept: the command examples from both projects' AGENTS.md run unchanged (apart from the program name) against fixtures

### T-083 · Switch-over plan for airline-ops-web
- status: open
- owner: —
- priority: P1 · type: docs · area: rollout
- reqs: R-MIG-2
- depends: T-033, T-081, T-082
- accept: written plan with before/after task counts and ID lists; carried out only after the owner approves

### T-084 · Switch-over plan for MusicDownloader
- status: open
- owner: —
- priority: P1 · type: docs · area: rollout
- reqs: R-MIG-2
- depends: T-080, T-081, T-082
- accept: written plan with before/after item counts; carried out only after the owner approves

### T-085 · First office project
- status: open
- owner: —
- priority: P2 · type: docs · area: rollout
- reqs: §12 office questions
- depends: T-004, T-070
- accept: one office project uses todo for a week without a lost update

## 7. Build, release, documentation

### T-090 · CI workflow
- status: open
- owner: —
- priority: P0 · type: chore · area: release
- reqs: REL-1, TEST-7, NFR-1, NFR-3, NFR-6
- accept: format check, Clippy with warnings as errors, and tests run on Windows and Linux for every push; CI also fails if todo.exe needs a DLL outside Windows itself, grows past 5 MB, or gains a networking dependency

### T-091 · Full README
- status: open
- owner: —
- priority: P0 · type: docs · area: release
- reqs: REL-3, R-LOCK-10, NFR-9
- depends: T-033
- accept: build, install, use and the AGENTS.md snippet are explained in plain language and tried by following them on a clean folder

### T-092 · docs/json.md and CHANGELOG.md
- status: open
- owner: —
- priority: P1 · type: docs · area: release
- reqs: REL-4, NFR-8
- depends: T-014
- accept: every JSON shape and error code is documented with an example

### T-093 · Release workflow with checksums
- status: open
- owner: —
- priority: P2 · type: chore · area: release
- reqs: REL-2
- depends: T-090
- accept: a version tag produces todo.exe and its SHA-256 file; a tag that doesn't match Cargo.toml fails

### T-094 · Performance test with 5,000 tasks
- status: open
- owner: —
- priority: P1 · type: test · area: core
- reqs: NFR-2, TEST-8
- depends: T-030, T-038
- accept: list under 100 ms and a write under 200 ms on a generated 5,000-task file

### T-095 · Version output with the build commit
- status: open
- owner: —
- priority: P2 · type: chore · area: release
- reqs: REL-6
- accept: `todo --version` prints version and commit

## 8. Later / ideas

### T-100 · MCP server
- status: open
- owner: —
- priority: P3 · type: feature · area: agent
- reqs: R-AGENT-6
- depends: T-005

### T-101 · Several projects: project list and overview
- status: open
- owner: —
- priority: P3 · type: feature · area: cli
- reqs: R-XPROJ-1, R-CFG-3

### T-102 · align: compare tasks with git history
- status: open
- owner: —
- priority: P3 · type: feature · area: git
- reqs: R-GIT-1, R-GIT-2

### T-103 · renumber
- status: open
- owner: —
- priority: P3 · type: feature · area: cli
- reqs: R-ID-6

### T-104 · handoff note
- status: open
- owner: —
- priority: P3 · type: feature · area: agent
- reqs: R-AGENT-7

### T-105 · touch: refresh a claim
- status: open
- owner: —
- priority: P3 · type: feature · area: cli
- reqs: R-CLAIM-5

### T-106 · PowerShell tab completion
- status: open
- owner: —
- priority: P3 · type: feature · area: cli
- reqs: R-OUT-7

### T-107 · undo
- status: open
- owner: —
- priority: P3 · type: feature · area: history
- reqs: R-HIST-4
- depends: T-051

### T-108 · Import from other formats
- status: open
- owner: —
- priority: P3 · type: feature · area: migrate
- reqs: R-MIG-4
- depends: T-080

### T-109 · Claims tagged with a session, released by a SessionEnd hook
- status: open
- owner: —
- priority: P3 · type: feature · area: agent
- reqs: R-CLAIM-8
- depends: T-073
- note (2026-11-15, claude): first find out how an agent's own commands can learn their session ID

### T-110 · todo schema: the tool describes itself
- status: open
- owner: —
- priority: P3 · type: feature · area: agent
- reqs: R-AGENT-8
- depends: T-092

### T-111 · Git merge driver for TODO.md
- status: open
- owner: —
- priority: P3 · type: feature · area: git
- reqs: R-GIT-3

### T-112 · scan: find untracked checkboxes and TODO comments
- status: open
- owner: —
- priority: P3 · type: feature · area: migrate
- reqs: R-MIG-5
