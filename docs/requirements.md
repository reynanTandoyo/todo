# todo — requirements (draft 1, for review)

Brainstorm of 2026-11-15. Nothing here is final until you agree; open decisions are collected in
§12. The work plan derived from this document is [`../TODO.md`](../TODO.md).

**How to read the requirement lists.** Each requirement has an ID (`R-AREA-n`), a priority and a
source tag.

| Priority | Meaning |
|---|---|
| **MUST** | needed for the first usable version (replaces `todo.js` / `todo.py` safely) |
| **SHOULD** | needed before rolling the tool out to all your projects |
| **COULD** | valuable, build when the above is done |
| **LATER** | idea parking; not planned |

| Source tag | Where the idea comes from |
|---|---|
| **[AO]** | airline-ops-web: `scripts/todo.js` and how its `TODO.md` is used |
| **[MD]** | MusicDownloader: `src/music_dl/todo.py`, its tests, and how its `TODO.md` is used |
| **[td]** | github.com/arkan/td (MIT) |
| **[primum]** | github.com/aegea-dev/primum (Apache-2.0) |
| **[myco]** | github.com/9aia/myco (MIT) |
| **[lens]** | github.com/lens077/ToDo — **no licence: ideas only, no code may be copied** |
| **[new]** | not in any source; proposed here |

---

## 0. Summary: the twelve points that matter most

1. **One program for every project.** `todo.exe` replaces `scripts/todo.js` and `music_dl/todo.py`,
   works in any folder that has a `TODO.md`, and needs nothing installed. [AO, MD]
2. **Never lose an update.** Every write is: take the lock, re-read the file, change it, replace it
   atomically. Proven by tests that run many processes at once. [AO, MD]
3. **Never touch what it doesn't understand.** A command rewrites only the lines it changes; every
   other byte, including line endings, stays identical. If the file is ambiguous, the tool refuses
   instead of guessing. [AO, td, lens]
4. **One shared list per repository, even across git worktrees.** An agent in
   `.claude/worktrees/agent-…` must reach the same `TODO.md` and the same lock as the main checkout.
   None of the four projects does this; myco comes closest by keeping its lock in the git common
   folder. [new; evidence in §3.2 F3]
5. **Stable IDs** (`T-012`) for every task, with title-substring matching as a fallback. [AO, MD, td]
6. **"Give me the next task and claim it" in one locked step**, so two agents never get the same
   task. [myco]
7. **Claims can't get stuck**: they are dated, can be taken over after a configurable time, and an
   override leaves a note. [MD]
8. **A machine contract for agents**: `--json` on every command (errors too) and documented exit
   codes (0 ok, 1 refused, 2 usage, 3 busy-retry, 4 file problem, 5 check couldn't run). [brief,
   MD, lens]
9. **Recovery**: a snapshot before every write, `todo history` / `diff` / `restore`, and detection
   of edits made outside the tool. [lens, primum]
10. **`todo check`** finds broken files: duplicate IDs, bad statuses, missing dependencies, cycles,
    stale claims. [td, myco]
11. **Agent setup in one command**: `todo init` and `todo agents install` write the rules into
    `AGENTS.md` / `CLAUDE.md`, and optionally a Claude Code hook that blocks direct edits of
    `TODO.md`. [td, myco, lens]
12. **The file-format decision is still open** (§7). Recommendation: one `TODO.md` per repository,
    the airline block format as the single canonical format, and MusicDownloader converted once
    with `todo import`.

---

## 1. Goal

Build the task tool once and use it everywhere:

- **airline-ops-web**: 375 tasks, managed today by `scripts/todo.js`.
- **MusicDownloader**: 251 items, managed today by `src/music_dl/todo.py`.
- **Future projects and office projects**: setups not known yet (questions in §12).

The project is done when:

- both existing projects have switched to `todo.exe` without losing a task, a note or a claim;
- parallel agents can't corrupt the file or lose each other's changes (shown by tests);
- a new project is set up in under a minute (`todo init`, `todo agents install`).

---

## 2. Who uses it, and the situations it must handle

**Actors**

- **Coordinator**: usually the main Claude Code session. It plans, adds tasks in bulk, sends
  subagents off, reviews and closes tasks. In airline-ops-web the `model:` field already records
  which model tier should do a task (`sonnet` 65×, `haiku` 21×, `hydrogen-high` 15×).
- **Worker agents**: Claude Code subagents, Codex, opencode, Gemini. Several run at once, often in
  linked git worktrees, they are short-lived, and they can crash or be cancelled mid-task.
- **The owner (you)**: reads `TODO.md` in an editor or on GitHub, sometimes restructures it by hand,
  and answers questions that agents raise.
- **Office colleagues**: may read or edit `TODO.md` without agents, possibly on locked-down
  Windows PCs.

**Situations** (each one becomes at least one test):

| # | Situation | Requirement(s) |
|---|---|---|
| S1 | Session start: an agent needs an overview without reading a 590 KB file | R-READ-4, R-ARCH-1 |
| S2 | Two agents claim the same task at the same moment: exactly one wins | R-CLAIM-2, TEST-4 |
| S3 | Two agents ask for the next task at the same time: they get different tasks | R-READ-3 |
| S4 | An agent crashes while holding the write lock: others continue within seconds | R-LOCK-2 |
| S5 | An agent abandons a claim: the task becomes available again after a timeout | R-CLAIM-4 |
| S6 | The owner edits `TODO.md` by hand while agents work | R-LOCK-4, R-LOCK-5, R-HIST-3 |
| S7 | An agent runs in a linked git worktree | R-FIND-5 |
| S8 | The coordinator adds 30 tasks at once | R-WRITE-12 |
| S9 | Done tasks pile up (airline: 289 of 375 are done) | R-ARCH-1 |
| S10 | A project adopts the tool with an existing `TODO.md` in either old format | R-MIG-1 |
| S11 | Antivirus or an editor holds `TODO.md` open while the tool replaces it (Windows) | R-IO-2 |
| S12 | A task depends on another one that isn't done | R-DEP-1 |
| S13 | A question only the owner can answer | R-WRITE-10 |
| S14 | The tool is used before and after a switch-over while the old script still exists | R-LOCK-7, R-MIG-3 |

---

## 3. What your two existing tools teach us

### 3.1 Side by side

| | airline-ops-web (`todo.js`) | MusicDownloader (`todo.py`) |
|---|---|---|
| A task is | `### T-012 · Title` heading plus `- key: value` lines | one line: `- [ ] **Title** — text (added: claude, 2026-11-12)` |
| ID | `T-012` | none: matched by part of the title |
| Status | `- status: open/claimed/blocked/done` | `[ ]` open · `[~]` claimed · `[!]` blocked · `[x]` done |
| Owner | `- owner: name` (`—` = nobody) | `(claimed: agent, date)` appended to the line |
| Priority | `P0`–`P3`, with type and area: `- priority: P1 · type: bug · area: ui` | the section (Now / Next / Later) |
| Description | `files`, `problem`, `fix`, `accept` fields | text after the em dash |
| Dependencies | `- depends: T-103` (51 tasks), plus prose in the header | prose only |
| Notes | `- note (2026-11-13, agent): text` | indented `  - 2026-11-13 agent: text` |
| Finishing | status → `done`, note `done — …`, stays in place | `[x]`, `(done: agent, date, commit)`, moved to top of `## Done` |
| Questions for the owner | none (two tasks say `model: owner decision`) | `## Needs user decision` section |
| Lock | `.todo.lock/` folder + `owner.json`; one-shot locks broken after 30 s, manual `lock` after 10 min, waits up to 60 s | `TODO.md.lock` file; broken after 60 s; waits up to 30 s |
| Finding the file | fixed: the repo the script lives in | `MUSIC_DL_TODO`, then walk up from the current folder, then the package's repo |
| Exit codes | 1 for every error | 0 ok · 1 refused · 2 usage or lock timeout |
| Commands | list, show, next, claim, release, done, block, note, set, add, lock, unlock, status | list, add, claim, release, note, block, done |
| Size today | 375 tasks, 34 sections, 4,475 lines, 590 KB | 251 items, 7 sections, 513 lines, 253 KB |

### 3.2 Findings from the real files and code

- **F1 — The files are big.** airline-ops-web's `TODO.md` is 590 KB (longest line 3,043 characters);
  MusicDownloader's is 253 KB. An agent that reads the whole file at session start spends roughly
  150k and 60k tokens. Agents need a short summary command and done tasks need an archive.
- **F2 — Done tasks dominate.** 289 of 375 airline tasks are done, and 111 of 251 MusicDownloader
  items.
- **F3 — Worktrees hold stale copies.** MusicDownloader has 7 linked worktrees in
  `.claude/worktrees/`, each with its own `TODO.md`, between 34 and 993 lines behind main. No update
  was lost (the copies have no uncommitted edits and no unmerged commits touching `TODO.md`), but
  `todo.py` run from inside a worktree would find the stale copy first, because it walks up from
  the current folder (`todo.py:60-68`). A lock on that copy protects nothing.
- **F4 — Fields beyond the JS tool are in use.** `depends:` (51 tasks) and `model:` (103 tasks).
  `todo.js next` ignores both (`todo.js:296-304`), so it can hand out a task whose prerequisite is
  open.
- **F5 — Both projects need "question for the owner".** MusicDownloader has a section for it;
  airline-ops-web has no support and works around it in the `model:` field.
- **F6 — Title codes act like IDs but aren't unique.** 168 of 251 MusicDownloader items start with a
  code such as `[GUI-P4]` or `[CORE-3]` (142 distinct codes), but some codes appear on more than one
  item (`[GUI-N1]`, `[DB-1]`, …). They can be aliases, not IDs.
- **F7 — The two tools write different dates.** `todo.js` writes the UTC date (`todo.js:237-239`),
  `todo.py` the local date (`todo.py:121-122`). Around midnight they disagree.
- **F8 — The Python tool rewrites line endings.** It always writes LF and normalises the end of the
  file (`todo.py:105, 299`), which turns a CRLF file into a whole-file change. `todo.js` keeps the
  file's line ending (`todo.js:97`).
- **F9 — Reading doesn't need the lock.** `todo.py list` takes the write lock (`todo.py:350`);
  `todo.js` reads without it. Atomic replacement makes lock-free reads safe.
- **F10 — The two lock protocols don't know about each other**, and neither would see a lock taken
  by a new tool. During a switch-over, two tools could write at once.
- **F11 — `next` then `claim` is a race.** Two agents calling `next` get the same task; one `claim`
  fails and must start over.
- **F12 — Exit codes matter.** `todo.py`'s 0/1/2 lets an agent tell "someone else has it" from
  "try again" (`todo.py:20-21`). `todo.js` returns 1 for everything.
- **F13 — The rules are written down three times and drift.** Both repos repeat the usage rules in
  the `TODO.md` header and in `AGENTS.md` (and `CLAUDE.md` in MusicDownloader).
- **F14 — No way to unblock.** Neither tool has `unblock`; airline-ops-web has 25 blocked tasks.
- **F15 — Ownership rules differ.** `todo.js block` requires the owner, `todo.py block` doesn't;
  both let anyone add a note.
- **F16 — Matching safety differs.** `todo.py` refuses ambiguous title matches and lists the
  candidates (`todo.py:153-168`) and never matches the example item in the rules header
  (`todo.py:139-142`). `todo.js` would treat a `### T-…` example in the header as a real task.
- **F17 — `todo.js add --section` takes the first heading that contains the text**
  (`todo.js:361`). With sections such as `## 20. Phase 3 — overview` and `## 21. Phase 3.1 — Icons`,
  `--section "Phase 3"` silently picks the first one.
- **F18 — Both files keep a hand-written preamble** (title, how-to, rules, phase guidance) above the
  first task section. It must survive every write unchanged.

---

## 4. What the GitHub projects contribute

Each was read at source level (file references below are into each repository). td was also built
and run on copies of both your `TODO.md` files: it listed 0 tasks for both and refused every write.
**None of the four can read either of your files**, so the value is in their ideas, not their code.

### 4.1 At a glance

| Concern | td | primum | myco | lens077/ToDo | Your tools |
|---|---|---|---|---|---|
| Storage | one file, checkbox lines | one file per item, front matter | `TODO.md` lines plus one file per record, on an orphan git branch | one file, checkbox lines | one file |
| IDs | random 8 hex, hidden comment | number = file name, highest + 1 | `T-001`, counter per type | random base36, hidden comment | `T-012` / none |
| Claims | none | none (`start` checks nothing) | owner + status, `--steal`, sessions, `reap` | none (every agent gets the same `next`) | AO owner; MD claim with 2-day expiry |
| Lock | OS lock, path hash in config folder | OS lock, no timeout, held during network calls | OS lock in git common folder, no waiting | OS lock on sibling file, 2 s wait | lock folder / lock file, time-based |
| Unknown text | refuses to write | parse error | parse error; rewrites whole file | byte-exact splice, verified | AO keeps; MD keeps, forces LF |
| History | none | git commit per change | git commit per change, `undo` | numbered snapshots | none |
| Worktrees | n/a | n/a | lock shared, but file not found from a worktree | `./TODO.md` only | MD finds the stale copy |
| Agent setup | `SKILL.md` | one line in docs | `AGENTS.md` block, skill, `manifest` | `AGENTS.md` block, SessionStart hook | rules in `AGENTS.md` |
| Windows | CI builds it | Linux and macOS only | not checked | POSIX-only hook command | yes |

### 4.2 arkan/td — MIT, Rust, ~6.5k lines, 32 commits, v0.1.1

A personal task manager for one Obsidian `TODO.md` (checkbox lines under `## Tasks`, `## Backlog`,
`## Archive`), with a CLI, a terminal UI and an agent skill.

Take:
- OS file lock held across load and save; the lock file is named by a hash of the canonical path
  (`file.rs:100-127, 326-334`) → R-LOCK-2, R-LOCK-6.
- Refuse to write when the file holds content it can't round-trip (`file.rs:181-199`) → R-EDIT-4.
- Match by ID prefix or by text, listing the candidates when ambiguous (`task_match.rs`) → R-ID-4.
- An agent skill shipped with the tool (`skills/td/SKILL.md`) → R-AGENT-4.
- CI with format, Clippy and tests on three systems; a release workflow that checks the tag matches
  the Cargo version and publishes SHA-256 checksums → REL-1, REL-2.

Leave: whole-file re-rendering, automatic "migrations" that move done tasks and rename sections,
IDs hidden in HTML comments, due dates and recurrence, the terminal UI, a default file in the home
folder.

### 4.3 aegea-dev/primum — Apache-2.0, Rust, ~4.8k lines, 100 commits, v0.2.1

A backlog kept as one Markdown file per item (`items/87.md` with front matter), ranked by a
computed score, with a generated `BOARD.md`, a local web board, and history in a separate bare git
repository; sync between machines merges field by field.

Take:
- Lock, re-read, change only the fields named, write (`store.rs:147-159, 324-356`) → R-LOCK-1,
  R-WRITE-8.
- Edits made outside the tool are recorded as their own history entry before the next change
  (`history.rs:191-195`) → R-HIST-3.
- Text only through `--file` or `-` for standard input (`cli.rs:697-705`) → R-WRITE-13.
- Without a terminal, risky commands need `--yes` (`cli.rs:806-811`) → R-OUT-9.
- `next` shows work in progress first, then the top unblocked items; "blocked" is worked out when
  reading, not stored (`cli.rs:826-852`, `model.rs:112-120`) → R-READ-3, R-DEP-1.
- IDs never reused; a duplicate from a merge is renumbered and its references rewritten
  (`sync.rs:150-181`) → R-ID-2, R-ID-6. Field-by-field merge rules (`merge.rs`) → R-GIT-3.
- Errors as `file:line: message` (`item_file.rs:29-33`); migrations that convert everything before
  writing and can be rerun (`migrate.rs:116-249`) → R-OUT-8, R-MIG-1.
- Tests: the real binary with a cleared environment, non-ASCII test data, "a failed change writes
  nothing", "a closed pipe ends quietly", help text checked against the code → TEST-3, TEST-7,
  NFR-10.

Leave: one file per item (decision F1), whole-file regeneration and rejecting unknown fields (one
bad file stops every command), a lock with no timeout held during network calls, `start` without
any check, no owner field, WSJF scoring, the web board, Unix-only assumptions and no Windows CI.

### 4.4 9aia/myco — MIT, Rust, ~57k lines, 148 commits, v0.2.0

A "records engine" for agent work: todos, issues, proposals, plans, decision records and handoffs in
a `project/` folder that is a worktree of a separate orphan branch, with one git commit per write.

Take:
- The lock file in the git common folder, shared by every worktree and never committed; an OS lock
  via Rust's standard `File::try_lock` (`myco-ops/ops.rs:18-90`) → R-LOCK-2, R-LOCK-6, R-HIST-0.
- `next --claim` picks and claims under one lock (`cli/next.rs:131-170`) → R-READ-3.
- Claim guard with `--steal` that leaves a "taken over from X" entry; `interrupt --session` and
  `reap --idle-days 3` free abandoned claims → R-CLAIM-2, R-CLAIM-4, R-CLAIM-7, R-CLAIM-8.
- IDs: highest seen across open, done and archived records plus one, under the lock; never reused
  (`store-md/store.rs:191-215`) → R-ID-2.
- `status --prompt`, a capped session-start summary → R-READ-4. A status transition table whose
  errors list the valid next statuses (`core/status.rs:74-96`) → §6.2.
- `check --fix` in several passes; an `AGENTS.md` block whose drift check simulates the update
  (`agentmd.rs:178-200`) → R-CHECK-2, R-AGENT-2.
- `manifest` and `schema` so agents can ask the tool about itself; `batch` for many operations →
  R-AGENT-8, R-WRITE-14. Handoff log per task and `resume` → R-AGENT-7.
- Golden snapshot tests, property-based round trips, lock race tests → TEST-2, TEST-4.

Leave: strict parsing with a canonical whole-file rewrite (drops unknown keys, turns CRLF into LF),
git as the transaction log (fetch and push on every write), a lock that fails at once instead of
waiting, seven statuses, a folder walk that stops at the worktree root (its ledger isn't found from
`.claude/worktrees/*` without a registry), and a guard that lets a non-owner complete a claimed task.

### 4.5 lens077/ToDo — no licence (ideas only), Rust + Tauri, 238 commits, MVP

"A local task reconciler for AI-assisted development": `TODO.md` with YAML front matter, checkbox
tasks with hidden random IDs and emoji metadata, plus a CLI, desktop app, HTTP server and
agent-session feed. Its `NOTICE.md` reserves all rights, so nothing below may be copied as code.

Take (as ideas):
- Changes as byte-range splices, then a check that every byte outside them is unchanged and the
  result still parses, before anything is written (`write.rs:119-135, 746-778`) → R-EDIT-10.
- Re-check hash and modification time just before writing to catch editors that ignore the lock
  (`lock.rs:54-65`) → R-IO-7. Refuse files with git conflict markers (`parse.rs:180-191`) →
  R-EDIT-11.
- Numbered snapshots with an index, restore that previews and can itself be undone
  (`history.rs`) → R-HIST-1.
- A versioned JSON envelope with `ok`/`data`/`error`, and a distinct exit code for "checks couldn't
  run" (`main.rs:3105-3142`) → R-OUT-1, R-GIT-1.
- Managed `AGENTS.md` block, preview by default, backups; a SessionStart hook printing at most 40
  lines within 2 s (`hooks.rs`) → R-AGENT-2, R-AGENT-3.
- `reconcile`: tasks checked against git evidence, reported as confirmed or suspected, never
  changing completion → R-GIT-1. `scan` for untracked checkboxes and TODO comments → R-MIG-5.
- Property tests over random edits, messy CRLF/BOM fixtures, multi-process races, a 10,000-task
  benchmark → TEST-2, TEST-4, TEST-8.

Leave: only `./TODO.md` (no parent search, no `--file`; no reason is given in its docs), no claims
at all, one replaceable note per task, `init --adopt` rewriting the whole file without a restore
point, heavy dependencies (SQLite, HTTP client) even for `list`, spec and code that disagree, and
hooks that assume a POSIX shell.

---

## 5. Principles

- **P1 — No lost updates.** Lock, re-read, change, atomic replace. Always.
- **P2 — Don't touch what you don't understand.** Byte-exact preservation outside the changed
  lines; refuse rather than guess.
- **P3 — The file stays human-first Markdown.** Readable in any editor and on GitHub; line-based
  diffs; no hidden database.
- **P4 — Agents get a stable contract.** IDs, JSON, exit codes and error codes are documented and
  versioned.
- **P5 — One binary, nothing else.** No runtime, no daemon, no network, no index database.
- **P6 — Report, don't decide.** The tool may suggest (stale claims, possibly-finished tasks) but
  never changes a task's status on its own.
- **P7 — Small and boring.** Every feature beyond the core must earn its place against the cost of
  testing it on Windows.

---

## 6. Data model (independent of the file format)

### 6.1 Task fields

| Field | Content | [AO] | [MD] |
|---|---|---|---|
| `id` | `T-012`; prefix configurable; never reused | yes | — |
| `title` | one line | yes | bold part |
| `status` | `open` · `claimed` · `blocked` · `done` | yes | checkbox mark |
| `owner` | agent name, or none | yes | claim suffix |
| `claimed` | date and time of the claim (for stale claims) | — | date only |
| `priority` | `P0`–`P3` | yes | implied by section |
| `section` | the `##` heading the task sits under | yes | yes |
| `type`, `area` | free text; optional allowed lists in config | yes | title code |
| `alias` | human codes such as `GUI-P4`; not necessarily unique | — | title code |
| `depends` | list of task IDs | yes (51) | prose |
| `model` | which agent or model tier should do it | yes (103) | — |
| `files` | paths involved | yes | — |
| `problem`, `fix`, `accept` | one line each | yes | description text |
| `added` | agent and date | first note | yes |
| `done` | agent, date, commit(s) | note | yes |
| `notes` | append-only list of (date, agent, text) | yes | yes |
| any other `key: value` | kept unchanged, exposed in JSON | `depends`, `model` today | — |

### 6.2 Status transitions

| Current status | `claim` | `release` | `block` | `unblock` | `done` | `reopen` |
|---|---|---|---|---|---|---|
| `open` | → `claimed` | refused | → `blocked` | refused | → `done` | refused |
| `claimed` | same agent: no change; other agent: refused unless stale | → `open` | → `blocked` | refused | → `done` | refused |
| `blocked` | only with `--force` | refused | refused | → `claimed` if it has an owner, else `open` | only with `--force` | refused |
| `done` | refused | refused | refused | refused | refused | → `open`, with a note saying why |

- Ownership rules (R-CLAIM-2) apply on top of this table: on a task claimed by someone else, every
  change except `note` needs `--force`.
- A refusal names the current status and lists the commands that are allowed from it. [myco
  `core/status.rs:74-96`]
- `reopen` is new; neither of your tools has it.
- Whether to add a fifth status for "won't do" is decision F2 (airline-ops-web's rule 5 says: mark
  it done with a note).

---

## 7. File layout and format — decision F1

### 7.1 Options

| Criterion | A. One `TODO.md`, airline block format | B. One `TODO.md`, checkbox lines | C. One file per task | D. Support A and B both |
|---|---|---|---|---|
| Readable in an editor or on GitHub | good: headings give an outline | best: a checklist | poor: needs `todo list` or a generated index | as today |
| Stable IDs | yes | no (would need adding) | yes (file name) | mixed |
| Rich fields (priority, depends, model, accept) | yes, one per line | awkward, inline | yes (front matter) | mixed |
| Git diffs | one field per line | whole long line changes | one file per change | as today |
| Merge conflicts between branches | possible; mostly avoided by R-FIND-5 | possible | rare, except two branches picking the same new ID | as today |
| File growth | needs an archive (R-ARCH-1) | needs an archive | none | needs archive ×2 |
| Existing data | airline as is; MusicDownloader converted once | MusicDownloader as is; airline converted (loses fields) | both converted | none converted |
| Build and test cost | one parser | one parser | new store, index, ID allocation across branches | two parsers, two writers, every command and test twice, forever |

### 7.2 Recommendation (for you to confirm)

**Option A.** One `TODO.md` per repository, the airline block format as the only canonical format,
and MusicDownloader converted once with `todo import --from checkbox` (R-MIG-1).

- A is a superset: every MusicDownloader item maps onto it (mark → status, claim suffix → owner and
  claimed date, section kept, notes kept, `(done: …, commit)` → done note and `commit` field, title
  code → `alias`). The reverse loses `priority`, `depends`, `model`, `accept`.
- C mainly solves merge conflicts between branches. R-FIND-5 (every worktree writes to the main
  checkout's file) removes most of those, at a fraction of the cost.
- D costs double in every command and test for as long as the tool lives.
- MusicDownloader's strengths are kept as behaviour, not as syntax: `todo summary` gives the
  at-a-glance view, `## Needs user decision` becomes `todo ask`, and "done moves to Done" becomes an
  option (decision F4).

### 7.3 Format details to settle in `docs/format.md` (if A is chosen)

- A version line near the top, such as `<!-- todo-format: 1 -->`, so later format changes can be
  migrated safely. [td migrations, new]
- Heading separators: the canonical `·` (middle dot) is hard to type by hand, so reading also accepts
  ` - `, ` — ` and `:`. Writing uses `·`. [new]
- The combined line `- priority: P1 · type: bug · area: ui` stays (375 tasks use it); a task may also
  use separate `- type:` and `- area:` lines.
- New fields this tool needs: `claimed` (date-time), `alias`, `commit`.
- A task block ends at the next `##` or `###` heading (as in `todo.js:148`). Text inside the block
  that isn't a field is kept untouched.

---

## 8. Functional requirements

### 8.1 Finding the file — FIND

- **R-FIND-1 MUST** `--file <path>` overrides everything. [brief, MD, td]
- **R-FIND-2 MUST** Otherwise the `TODO_FILE` environment variable. [td; MD used `MUSIC_DL_TODO`]
- **R-FIND-3 MUST** Otherwise walk up from the current folder to the first folder that has a
  `TODO.md`, like git does. [brief, MD]
- **R-FIND-4 MUST** The walk stops at the repository root (the first folder with a `.git`), so a
  project without a `TODO.md` never picks up an unrelated one in a parent folder such as
  `C:\Project\`. Outside a repository it walks to the drive root. [new]
- **R-FIND-5 MUST** In a linked git worktree, use the main checkout's `TODO.md` (same relative
  path), so every agent shares one list and one lock. Print a one-line notice on stderr.
  `--local` (or config `worktree = "local"`) opts out. Detect this by reading the `.git` file and
  `commondir`, so `git.exe` isn't required. [new; F3. myco's own folder walk stops at the
  worktree root and needs a registry to find its ledger from `.claude/worktrees/*`
  (`discover.rs:253-317`); lens only reads `./TODO.md`.]
- **R-FIND-6 MUST** `todo where` prints the resolved file, how it was found, the state folder and
  the lock and lease status; supports `--json`. [new]
- **R-FIND-7 MUST** No file found: exit 4 with a hint to run `todo init`. Never create a file
  implicitly. [td, MD]
- **R-FIND-8 SHOULD** The file name is configurable (default `TODO.md`).

### 8.2 Reading and editing the file — EDIT

- **R-EDIT-1 MUST** The file is read as bytes and split into lines that keep their own line ending.
  A task is a span of lines. A write replaces, inserts or removes specific lines only; all other
  bytes stay identical. [AO approach, lens]
- **R-EDIT-2 MUST** Parsing and writing back without a change gives identical bytes, for every
  fixture.
- **R-EDIT-3 MUST** Unknown fields, free text inside a task, sections without tasks, the preamble,
  tables and code blocks are kept and never parsed as tasks. Headings and checkboxes inside fenced
  code blocks and HTML comments are ignored. [MD `todo.py:139-142`, td; F16, F18]
- **R-EDIT-4 MUST** Refuse instead of guessing: if a command's target is ambiguous (duplicate ID,
  task heading inside an unclosed code fence, two `status:` lines in one task), refuse with exit 4,
  name the line, and point to `todo check`. [td]
- **R-EDIT-5 MUST** Inserted lines use the file's main line ending; existing lines keep theirs; a
  missing newline at the end of the file stays missing. [AO `todo.js:97`; F8]
- **R-EDIT-6 MUST** Field values are single lines: newlines in input become spaces (both tools do
  this). No length limit.
- **R-EDIT-7 SHOULD** Lenient reading, canonical writing: `t12`, `T-12` and `12` all mean `T-012`
  (`todo.js:241-246`); alternative heading separators are accepted (§7.3); the tool writes the
  canonical form only on lines it changes.
- **R-EDIT-8 SHOULD** Every write command accepts `--dry-run` and prints the diff it would apply.
  [lens: preview before apply]
- **R-EDIT-9 MUST** If a command doesn't change anything, the file isn't rewritten
  (`todo.js:99`).
- **R-EDIT-10 MUST** Before writing, verify the result: every byte outside the edited lines equals
  the original, and the new file parses with the same task IDs (plus any added). If either check
  fails, nothing is written and the command exits 4. This turns "byte-exact" from a hope into a
  guarantee. [lens `write.rs:119-135, 746-778`]
- **R-EDIT-11 MUST** A file containing git conflict markers (`<<<<<<<`, `=======`, `>>>>>>>` at
  the start of a line, outside code fences) is refused for writing, and `check` reports it.
  [lens `parse.rs:180-191`]

### 8.3 Locking — LOCK

- **R-LOCK-1 MUST** Every write command is one locked read-modify-write, and the file is re-read
  after the lock is taken. [AO, MD]
- **R-LOCK-2 MUST** A dead holder never blocks others: the write lock is an operating-system file
  lock on a permanent lock file, released by Windows when the process ends, so the next command
  proceeds within a second instead of waiting 30–60 s for a time-based break. Rust's standard
  library has this built in (`File::lock`), so no extra dependency is needed. [td, primum, myco,
  lens all do this; AO/MD use time-based breaking]
- **R-LOCK-3 MUST** A command waits up to 30 s (config `lock_wait`) with jittered retries, then
  exits 3 ("busy, retry") naming the holder: agent, command, process ID, since when.
  [AO `todo.js:21, 82`; MD `todo.py:42`. myco fails at once without waiting and lens waits 2 s;
  both are too short for a dozen agents.]
- **R-LOCK-4 MUST** Manual lease for hand edits: `todo lock --agent X [--minutes N]` (default 10,
  maximum 60). While it is held, other agents' writes wait; the holder's own writes pass
  (`todo.js:69-71`). `todo unlock --agent X` ends it; an expired lease is ignored; `unlock --force`
  ends someone else's lease and records that. [AO; your brief: "expire after 10 minutes"]
- **R-LOCK-5 SHOULD** `todo edit --agent X`: take a lease, open the editor (`VISUAL`, `EDITOR`, or
  Notepad), wait for it to close, run `todo check`, take a snapshot, release. [new]
- **R-LOCK-6 MUST** The lock and lease live in the state folder (R-HIST-0), keyed by the file's
  canonical path, so every worktree and every way of spelling the path uses the same lock.
  [myco, td]
- **R-LOCK-7 SHOULD** Switch-over safety: while a project still has its old script, also honour
  that script's lock (`.todo.lock/` folder protocol for `todo.js`, `TODO.md.lock` file protocol for
  `todo.py`), chosen with config `legacy_lock = "js" | "py"`. [new; F10]
- **R-LOCK-8 MUST** Read commands never take the lock. [AO; F9]
- **R-LOCK-9 MUST** The lock covers only reading, changing and writing the file. Slow work (git
  calls such as checking a `--commit`, waiting for an editor) happens before or after it. [primum
  holds its lock through network calls with no timeout, which can hang every agent]
- **R-LOCK-10 SHOULD** The README states the limits: the lock protects one machine (your brief);
  files on network shares work with Windows file locking; folders synced by OneDrive or Dropbox
  can create conflict copies and aren't supported for parallel agents.

### 8.4 Writing to disk — IO

- **R-IO-1 MUST** Write to a temporary file in the same folder, flush it to disk, then replace the
  original atomically. [AO, MD, td, primum]
- **R-IO-2 MUST** On Windows, retry the replace for up to 2 s when another program holds the file
  (antivirus, editor, indexer); then delete the temporary file and exit 4.
  [AO `todo.js:108-120`, MD `todo.py:106-115`]
- **R-IO-3 MUST** The temporary file name is unique per process, and it is removed on failure.
- **R-IO-4 MUST** UTF-8 only. A byte-order mark is kept if present. Invalid UTF-8 (for example a
  file saved as ANSI by an old editor) is refused with exit 4 and the byte position.
- **R-IO-5 MUST** Dates in notes use the local date (`YYYY-MM-DD`); `claimed` stores local date and
  time with offset, so stale-claim checks work across time zones. [F7]
- **R-IO-6 MUST** A read-only file is refused with a clear message, not a crash.
- **R-IO-7 MUST** Just before replacing the file, check that it hasn't changed since it was read
  under the lock (size, modification time and hash). If it has, someone ignored the lock (an
  editor, a script): write nothing and exit 3 so the caller retries. [lens `lock.rs:54-65`]

### 8.5 Task identity and matching — ID

- **R-ID-1 MUST** Every task has a stable ID, `<PREFIX>-<number>`: default prefix `T`, numbers
  padded to 3 digits, prefix configurable per project (for example `AO-012`). [AO]
- **R-ID-2 MUST** A new ID is the highest existing ID (archive included) plus one, computed under
  the lock. IDs are never reused. [AO `todo.js:366-367`]
- **R-ID-3 MUST** `12`, `t12`, `T-12` and `T-012` are the same ID. [AO]
- **R-ID-4 SHOULD** An argument that isn't an ID is matched against titles and aliases,
  case-insensitively, as a substring. Exactly one not-done task must match; otherwise the tool
  refuses (exit 1) and lists the candidates. [MD, td]
- **R-ID-5 SHOULD** `alias:` values (such as `GUI-P4`) resolve like IDs; an ambiguous alias is
  refused with the candidates. [F6]
- **R-ID-6 COULD** `todo renumber T-050 --to T-090` renames a task and updates every `depends`
  that points at it (for duplicate IDs reported by `check`, for example after two branches each
  added a task). [primum `sync.rs:150-181`; myco renumbers on sync]

### 8.6 Read commands — READ

- **R-READ-1 MUST** `list` with filters `--status` (default: everything not done), `--priority`,
  `--section`, `--type`, `--area`, `--owner`, `--model`, `--alias`, `--ready` (open, unowned, all
  dependencies done), `--mine` (owner = this agent), `--stale`, `--all`. One line per task: ID,
  priority, status, owner, title (cut to the terminal width unless `--wide`), then a count.
  [AO `todo.js:276-288`]
- **R-READ-2 MUST** `show <id>` prints the task block exactly as written; with `--json`, all fields,
  notes, unknown fields, section and line range. [AO]
- **R-READ-3 MUST** `next` returns the best ready task: open, unowned (or a stale claim),
  dependencies done; ordered by priority, then section order, then ID; filters `--area`, `--type`,
  `--model`, `--section`. If the agent already holds a claimed task, `next` shows that one first
  (primum shows work in progress first), which stops agents from collecting tasks.
  **`next --claim --agent X` finds and claims in one locked step.**
  [AO; myco `next.rs:131-170`; F4, F11. Neither myco's nor lens's `next` looks at dependencies.]
- **R-READ-4 SHOULD** `summary` prints a short session-start overview (under 40 lines): counts by
  status, priority and section, claimed tasks with owner and age, blocked tasks with their reason,
  open questions for the owner. [myco `status --prompt` (60 lines); lens session context
  (40 lines); F1]
- **R-READ-5 SHOULD** `search <text>` searches every field and note; `--archive` includes the
  archive file.
- **R-READ-6 COULD** `log <id>` shows a task's history from the journal (R-HIST-2).
- **R-READ-7 MUST** Every read command supports `--json`. [brief]

### 8.7 Write commands — WRITE

Every write needs an agent name (R-CLAIM-6) and prints a one-line result such as
`T-012: claim ok` (`todo.js:437`), or the changed task with `--json`.

- **R-WRITE-1 MUST** `add --title T [--section S] [--priority P] [--type] [--area] [--files]
  [--problem] [--fix] [--accept] [--depends] [--model] [--alias]` prints the new ID. The section
  must match exactly or uniquely (F17). Required fields are set per project (`todo.js` requires
  section, title, priority, type and area). A task with the same title is refused
  (`todo.py:203-206`). An `added` note is written. [AO, MD]
- **R-WRITE-2 MUST** `claim <id>`: see §8.8. [AO, MD]
- **R-WRITE-3 MUST** `release <id>`: owner, or `--force`; status back to open. [AO, MD]
- **R-WRITE-4 MUST** `done <id> [--note "what changed"] [--commit <hash>]`: owner, unowned, or
  `--force`. With git available, `--commit` is checked to exist; `--commit HEAD` stores the current
  commit. Prints the task's `accept:` line as a reminder. [AO, MD]
- **R-WRITE-5 MUST** `block <id> --note "why"`: owner or unowned, else `--force`. [AO; F15]
- **R-WRITE-6 MUST** `unblock <id> [--note]`: back to `claimed` if it has an owner, else `open`.
  [new; F14]
- **R-WRITE-7 MUST** `note <id> --text "…"`: anyone, also on done tasks. [AO, MD]
- **R-WRITE-8 MUST** `set <id> --title/--priority/--type/--area/--files/--problem/--fix/--accept/
  --depends/--model/--alias`, plus `--field key=value` for any other field: owner or `--force`.
  [AO]
- **R-WRITE-9 SHOULD** `move <id> --section S` and `reopen <id> --note "why"`. [new]
- **R-WRITE-10 SHOULD** `ask --text "question"` adds a task to the decision section (type
  `decision`); `next` never returns it; `answer <id> --text "…"` records the owner's answer and
  closes or reopens it. [MD "Needs user decision"; F5]
- **R-WRITE-11 SHOULD** Done tasks: config `done_moves_to = "## Done"` moves a finished task with
  its notes to the top of that section (`todo.py:259-274`); default leaves it in place (`todo.js`).
  Decision F4. [AO, MD]
- **R-WRITE-12 SHOULD** `add --from <file.json>` or JSON lines on standard input adds many tasks
  in one locked step and prints their IDs. [new; S8]
- **R-WRITE-13 SHOULD** Any text option accepts `-` (read standard input) or `@path` (read a file),
  which avoids PowerShell quoting problems with quotes, backticks and `$`. [primum `cli.rs:697-705`]
- **R-WRITE-14 SHOULD** A batch file of operations applied in one locked step with one snapshot
  (`todo apply ops.jsonl`): all apply or none do. [myco `batch`; lens compound writes]

### 8.8 Claims and ownership — CLAIM

- **R-CLAIM-1 MUST** `claim` sets status `claimed`, the owner, and the claim date-time. [AO, MD]
- **R-CLAIM-2 MUST** A task held by another agent's live claim is refused (exit 1, naming the owner
  and the claim's age). The same agent claiming again succeeds and changes nothing. The guard covers
  every status change, not only claiming: a non-owner can't `done`, `block` or `set` a claimed
  task without `--force`. [MD tests; myco only guards claiming, so a non-owner can complete a
  claimed task there (`itemcmd.rs:106-118`)]
- **R-CLAIM-3 MUST** Claiming a done task is refused (`todo.js:386`); a blocked task needs
  `--force`.
- **R-CLAIM-4 MUST** A claim older than `claim_ttl` (default set by decision F3; `todo.py` uses
  2 days) is stale; another agent may take it over, and the takeover is recorded as a note naming the
  previous owner. `release --stale` frees all stale claims at once. [MD `todo.py:217-226`; myco
  `reap --idle-days` and `--steal` with a "taken over from X" entry]
- **R-CLAIM-5 COULD** `touch <id>` refreshes the claim time for long tasks. [new]
- **R-CLAIM-6 MUST** `--agent NAME` is required on writes; it falls back to the `TODO_AGENT`
  environment variable. Names are checked (`A-Z a-z 0-9 . _ -`) and compared case-insensitively.
  Unlike myco, there's no silent fallback to the git user name: every agent would then share one
  identity and claims would protect nothing. [AO rule 3, MD; myco actor chain]
- **R-CLAIM-7 SHOULD** `release --all --agent X [--force]` frees every claim of one agent (end of
  session, or the coordinator cleaning up after a crashed agent). [myco `interrupt`]
- **R-CLAIM-8 COULD** A claim can carry a session ID (`--session` or `TODO_SESSION`), and
  `release --session ID` frees exactly that session's claims, so a Claude Code `SessionEnd` hook
  can clean up after an agent that stopped. Needs a check of how an agent's own commands can learn
  their session ID. [myco `interrupt --session`, continuity skill]

### 8.9 Dependencies — DEP

- **R-DEP-1 MUST** `depends: T-103, T-104` is parsed; `next` and `list --ready` skip tasks whose
  dependencies aren't done. A dependency on an unknown ID also blocks (and `check` reports it).
  "Blocked by dependencies" is worked out when reading, never written into the file. [AO; primum
  `model.rs:112-120`; F4]
- **R-DEP-2 SHOULD** `show` lists the dependencies with their status, and the tasks waiting on this
  one.
- **R-DEP-3 SHOULD** `check` reports dependencies on missing IDs and cycles.
- **R-DEP-4 COULD** `done` prints the tasks that just became ready, so the coordinator can send
  agents to them.

### 8.10 Output, errors, exit codes — OUT

- **R-OUT-1 MUST** `--json` works on every command, including writes and errors. Every JSON
  document is an envelope with a schema name and version, an `ok` flag and either `data` or
  `error`, for example
  `{"schema":"todo.claim.v1","ok":false,"error":{"code":"ALREADY_CLAIMED","message":"…","task":"T-012","owner":"agent-b"}}`.
  With `--json`, errors go to standard output too, so an agent parses one stream. The shapes are
  documented in `docs/json.md`. [brief; lens envelope `main.rs:3105-3142`; myco `{"error":…}`]
- **R-OUT-2 MUST** Exit codes: `0` ok · `1` refused (claimed by someone else, no or ambiguous
  match, not the owner; `check` found problems) · `2` usage error · `3` busy, retry (lock timeout,
  or the file changed during the write) · `4` file problem (not found, can't parse, unsupported
  content, can't write) · `5` a check couldn't run (for `align` without git history). [MD
  extended; lens's "unverified" code]
- **R-OUT-3 MUST** Non-ASCII text (`·`, `—`, Japanese titles) prints correctly in Windows
  PowerShell 5, PowerShell 7, cmd and Windows Terminal; piped output is plain UTF-8.
  [MD `todo.py:340-342`]
- **R-OUT-4 SHOULD** Colour only on a terminal; `NO_COLOR` is respected.
- **R-OUT-5 MUST** Results to standard output, warnings and errors to standard error, in plain
  English.
- **R-OUT-6 SHOULD** `todo` with no arguments prints an overview; `todo help <command>` shows
  examples. [AO]
- **R-OUT-7 COULD** `todo completions powershell` prints tab completion for PowerShell.
- **R-OUT-8 MUST** Problems in the file are reported as `TODO.md:123: message`, so editors and
  agents can jump to the line. [primum `item_file.rs:29-33`]
- **R-OUT-9 SHOULD** Commands that replace a lot (`restore`, `import`, `archive`, `hooks install`,
  `agents install`) show what they would do and change nothing unless `--yes` is given. [primum,
  lens]

### 8.11 History, recovery, audit — HIST

- **R-HIST-0 MUST** State folder: inside a git repository, `<git common dir>/todo/` (never
  committed, shared by all worktrees, no `.gitignore` change needed); otherwise `.todo/` next to
  `TODO.md`. [myco]
- **R-HIST-1 SHOULD** A numbered snapshot of the file before every write, with an index line naming
  the operation and agent; keep the last 200 (configurable) and actually prune older ones (lens
  documents 200 but never enforces it). `todo history`, `todo diff <n> [<m>]`, and
  `todo restore <n>`, which shows the diff and writes only with `--yes`, is locked, and is itself
  snapshotted so it can be undone. Snapshots are tagged with their file, so an archive snapshot can
  never be restored over `TODO.md` (a gap found in lens). [lens `history.rs`]
- **R-HIST-2 SHOULD** A journal (JSON lines): time, agent, command, arguments, task, file hash
  before and after. [lens, myco, new]
- **R-HIST-3 SHOULD** Detect outside edits: if the file's hash differs from what the tool last
  wrote, take a snapshot of it and warn ("TODO.md changed outside todo since 14:03; saved as
  snapshot 57"). The write continues, because the owner may edit by hand. [primum records outside
  edits as their own history entry, `history.rs:191-195`; S6]
- **R-HIST-4 COULD** `todo undo --agent X` reverts that agent's last change if nothing has changed
  since. [new]

### 8.12 Validation — CHECK

- **R-CHECK-1 MUST** `todo check` reports: duplicate IDs; unknown status values; a claimed task
  without owner or an open task with one; a field repeated inside a task; lines that look like a
  task heading but don't parse; dependencies on missing IDs; dependency cycles; stale claims;
  blocked tasks without a reason; leftover temporary files; a rules block that has drifted from the
  current text (R-AGENT-2). Exit 0 if clean, 1 if not; `--json`. [td, myco]
- **R-CHECK-2 SHOULD** `todo check --fix` applies only safe, mechanical fixes (normalise IDs and
  heading separators, add a missing `status: open`, remove leftover temporary files) and reports
  everything else. [myco]
- **R-CHECK-3 COULD** `check` runs automatically after `unlock` and `edit`.

### 8.13 Archive — ARCH

- **R-ARCH-1 SHOULD** `todo archive [--older-than 30d]` moves done tasks into `TODO-archive.md`
  (same format, same lock, same snapshot). IDs, `show`, `search --archive` and `depends` still find
  them. [lens `TODO.archive.md`; myco `DONE.md`; F1, F2]

### 8.14 Configuration — CFG

- **R-CFG-1 MUST** Works with no configuration; the defaults behave like `todo.js`.
- **R-CFG-2 SHOULD** Optional project file `todo.toml` next to `TODO.md` (committed): ID prefix,
  sections and their order, done and decision sections, priorities, required fields for `add`,
  allowed types and areas, `claim_ttl`, `lock_wait`, lease length, worktree mode, `done_moves_to`,
  archive file, `legacy_lock`.
- **R-CFG-3 COULD** User file `%APPDATA%\todo\config.toml` for personal defaults and the project
  list (R-XPROJ-1).
- **R-CFG-4 MUST** Precedence: command line, then environment, then project file, then user file,
  then defaults.

### 8.15 Agent integration — AGENT

- **R-AGENT-1 MUST** `todo init` creates `TODO.md` (preamble with the rules block, default sections,
  format version line) and prints the snippet for `AGENTS.md`. [td, lens]
- **R-AGENT-2 SHOULD** `todo agents install [--into AGENTS.md|CLAUDE.md]` writes or updates a block
  between `<!-- todo:begin -->` and `<!-- todo:end -->`; `todo agents check` (and `todo check`)
  reports when it is out of date. The same block can replace the hand-written rules in the
  `TODO.md` header. [myco, lens; F13]
- **R-AGENT-3 SHOULD** `todo hooks install --agent claude` adds Claude Code hooks to the project's
  `.claude/settings.json` (not the user-wide file, which lens edits):
  - a `SessionStart` hook that prints `todo summary` (at most 40 lines, within 2 s), so every
    session starts knowing the open work [lens];
  - a `PreToolUse` hook running `todo guard`, which blocks Edit/Write on the resolved `TODO.md` and
    tells the agent which command to use instead [new].

  Without `--yes` it only shows what it would change; it keeps a backup, keeps the file's
  indentation and line endings, tags its own entries, and `todo hooks uninstall` removes only those.
  Agents without hooks (Codex and others) get the rules block from R-AGENT-2. The hook command must
  work in Windows PowerShell, not only a POSIX shell (a gap found in lens).
- **R-AGENT-4 SHOULD** `todo skill install` writes `.claude/skills/todo/SKILL.md`, a Claude Code
  skill describing the commands. [td; myco ships one too]
- **R-AGENT-5 MUST** `todo help agents` prints the rules (the same text as the managed block), so an
  agent of any vendor can learn the tool in one call.
- **R-AGENT-6 LATER** `todo mcp` runs an MCP server exposing list, show, next, claim, … as tools,
  which avoids shell quoting entirely. [new; myco has a JSON-RPC `serve`]
- **R-AGENT-7 COULD** `todo handoff --agent X` writes a note summarising that agent's open claims
  for whoever continues. [myco handoffs and `resume`]
- **R-AGENT-8 COULD** `todo schema` prints a machine-readable description of every command, option,
  JSON shape and exit code, so an agent can learn the tool without reading docs. [myco `manifest`
  and `schema`]

### 8.16 Git evidence — GIT

- **R-GIT-1 COULD** `todo align` lists open and claimed tasks that have commits mentioning their ID
  (`git log --grep T-012`), and done tasks without any commit, each marked "confirmed" or
  "suspected". It only reports; it never changes a status. If git is missing or the history is
  shallow, it says which checks didn't run and exits with a distinct code, so a skipped check is
  never mistaken for a clean result. [lens `reconcile.rs:167-238`; P6]
- **R-GIT-2 COULD** A commit message convention (`T-012: …`) documented in the rules block, so
  R-GIT-1 and `done --commit HEAD` work.
- **R-GIT-3 LATER** `todo merge-driver`, registered in `.gitattributes`, merges two versions of
  `TODO.md` task by task and field by field (notes as a union), and leaves a normal conflict only
  where both sides changed the same field. Useful if tasks are ever edited on branches.
  [primum `merge.rs` rules]

### 8.17 Migration and switch-over — MIG

- **R-MIG-1 MUST** (if F1 = A) `todo import --from checkbox <file>` converts the MusicDownloader
  format: mark → status, claim suffix → owner and `claimed`, `(added: …)` → added note, `(done: …,
  commit)` → done note and `commit`, indented notes → notes, title code → `alias`, sections kept.
  It converts everything in memory before writing anything, shows the result first, writes only
  with `--yes`, and is safe to run twice. [MD; lens adopt preview; primum `migrate.rs`]
- **R-MIG-2 MUST** A written switch-over plan for each project: which lines of `AGENTS.md`,
  `CLAUDE.md` and the `TODO.md` header change, whether the old script is deleted or becomes a thin
  wrapper that calls `todo.exe`, and how to check nothing was lost (task counts and IDs before and
  after). Carried out in those repositories only with your approval.
- **R-MIG-3 SHOULD** Both old calling styles keep working during the switch-over: positional text
  (`note "title part" "text"`, `todo.py`) and options (`note T-012 --text "text"`, `todo.js`);
  section names `now/next/later/decision` for `add`.
- **R-MIG-4 COULD** Import from td's checkbox format or todo.txt for other projects.
- **R-MIG-5 LATER** `todo scan` finds untracked checkboxes in other Markdown files and `TODO:`
  comments in source code, and offers to add them as tasks (preview first). [lens `scan --import`]

### 8.18 Several projects — XPROJ

- **R-XPROJ-1 COULD** `todo projects add <path>` keeps a list of projects in the user config;
  `todo overview` shows ready, claimed and blocked counts across all of them; `--project NAME` runs
  any command against a listed project. [primum]

---

## 9. Non-functional requirements — NFR

- **NFR-1 MUST** A single `todo.exe` with no runtime: the C runtime is linked statically (done in
  the scaffold). Windows x64 first; Linux and macOS builds come cheap from CI.
- **NFR-2 MUST** `list` on a 590 KB file in under 100 ms; any write in under 200 ms plus lock wait;
  checked against a generated 5,000-task file.
- **NFR-3 MUST** No network access, no telemetry, no background process, no database.
- **NFR-4 MUST** git is optional: everything except `--commit` checks and `align` works without
  git installed; worktree detection reads the `.git` files directly.
- **NFR-5 MUST** No crashes: every failure ends with a message and an exit code. Non-test code
  avoids `unwrap`/`expect` (enforced by Clippy). (MusicDownloader's [CORE-3] task is the same
  lesson.)
- **NFR-6 SHOULD** Binary under 5 MB; few, well-known dependencies.
- **NFR-7 MUST** Paths with spaces, non-ASCII characters and more than 260 characters work.
- **NFR-8 MUST** The command line, JSON schema, exit codes and file format are versioned; breaking
  changes need a new major version and a migration.
- **NFR-9 SHOULD** The README explains the Windows SmartScreen/Defender warning for an unsigned
  `.exe`. Code signing is out of scope unless the office requires it (§12).
- **NFR-10 MUST** A closed output pipe (`todo list | Select-Object -First 5`) ends quietly. Rust's
  `println!` panics in that case by default, so output must go through a writer that handles it.
  [primum `tests/cli.rs:420-447`]

---

## 10. Testing — TEST

- **TEST-1 MUST** Parser unit tests: headings, fields, the combined priority line, notes, unknown
  fields, code fences, examples in the preamble, CRLF, byte-order mark, no newline at end, mixed
  line endings, Unicode.
- **TEST-2 MUST** Round trip: parse and write back gives identical bytes on every fixture; a
  property test applies random sequences of edits and checks that bytes outside the edited lines
  never change, and that doing and undoing a change restores the exact bytes. Fixtures include a
  deliberately messy one (CRLF with byte-order mark, code fences, tables, fake tasks, trailing
  spaces). [lens `tests/core.rs:213-259`; myco proptest round trips]
- **TEST-3 MUST** Every write command has before/after fixture pairs, including refusals (not the
  owner, done, ambiguous, missing). Every refusal is also checked to leave the file byte-identical
  ("a failed change writes nothing"). [primum `tests/store.rs:258-265`]
- **TEST-4 MUST** Locking, with separate processes: parallel adds lose nothing; parallel claims of
  one task have exactly one winner; parallel `next --claim` hands out different tasks; a killed lock
  holder blocks others for under 2 s; a live lock makes a command exit 3 without writing; a lease
  blocks other agents but not its holder; a lease expires. (The first two and the live-lock test
  mirror `test_todo_tool.py`.) [MD]
- **TEST-5 MUST** File search: from the root, from a nested folder, no `TODO.md` (stops at the
  repository root), `--file`, `TODO_FILE`, a real linked worktree created in a temporary folder
  (resolves to the main file), and `--local`.
- **TEST-6 MUST** Windows: replacing the file while another handle holds it open (retries, then
  succeeds once released); a read-only file is refused.
- **TEST-7 MUST** Command-line tests against the built `todo.exe`: exit codes and JSON output
  snapshots. Each test runs with a cleared environment (no `TODO_FILE`, `TODO_AGENT` or user
  config leaking in from the machine), and a test checks that the help text lists every command.
  [primum `tests/cli.rs:26-42, 459-476`]
- **TEST-8 SHOULD** A performance test on a generated 5,000-task file.
- **TEST-9 MUST** Fixtures are small and invented, and start with a line saying they are test
  fixtures (your rule). Tests never use this repository's own `TODO.md`: they always run in a
  temporary folder.
- **TEST-10 SHOULD** The command examples in both projects' `AGENTS.md` and `TODO.md` headers are
  replayed against fixtures, to prove the switch-over syntax works (R-MIG-3).

---

## 11. Build, release, documentation — REL

- **REL-1 MUST** GitHub Actions CI: format check, Clippy with warnings as errors, and tests on
  Windows and Linux. [td]
- **REL-2 SHOULD** Release on a version tag: build `todo.exe` (plus other platforms), publish a
  SHA-256 checksum next to it, and refuse to release if the tag doesn't match the Cargo version.
  [td]
- **REL-3 MUST** README: build, install (copy `todo.exe` into a folder on `PATH`, or
  `cargo install --path .`), use, and the `AGENTS.md` snippet. [brief]
- **REL-4 SHOULD** `docs/format.md` (exact grammar), `docs/json.md` (JSON schema), `CHANGELOG.md`.
- **REL-5 MUST** A licence for the public repository (decision F6).
- **REL-6 SHOULD** `todo --version` includes the git commit it was built from.

---

## 12. Open decisions

| # | Decision | Options | Recommendation |
|---|---|---|---|
| F1 | File layout and format | A one file, blocks · B one file, checkboxes · C one file per task · D A and B both | A (§7.2) |
| F2 | A "won't do" status | add one (primum `wontfix`, myco `cancelled`) · use `done` with a note (airline rule 5) | `done` with a note; revisit later |
| F3 | Default claim timeout | 2 days (`todo.py`) · a few hours (agent sessions are short) · none (`todo.js`) | 24 hours, configurable |
| F4 | Where done tasks go | stay in place (`todo.js`) · move to `## Done` (`todo.py`) · plus `todo archive` | per project config; default stay in place; archive available |
| F5 | Worktree behaviour | shared main file (R-FIND-5) · each worktree its own copy · tasks on their own git branch checked out once (myco) | shared main file |
| F6 | Licence (public repository) | MIT · Apache-2.0 · none | MIT, like td |
| F7 | Which agents get hooks or skills | Claude Code only · also Codex, opencode, Gemini | Claude Code first; rules block for all |
| F8 | MCP server | later · never | later (R-AGENT-6) |

**Questions about the office projects** (they change requirements, so they're worth answering
before rollout):

1. Windows only, or also Linux or macOS machines (build servers, WSL)?
2. May you run an unsigned `.exe` you built yourself, or do you need code signing or an approved
   install path?
3. Where are the repositories hosted (GitHub, GitLab, Azure DevOps, other), and are they in git at
   all?
4. Do colleagues edit `TODO.md` by hand, or only you and agents?
5. Are project folders inside OneDrive or another synced folder?
6. Which agents do you use there?

---

## 13. Out of scope for now

- Terminal UI [td], desktop app [lens], web server [primum, lens].
- Computed priority scores (primum's WSJF ranking); `P0`–`P3` plus order in the file is enough.
- Sync between machines through the tool (primum, myco); git already does this.
- An agent-session feed (lens `ai upsert`).
- Due dates and recurrence [td]: these suit personal to-do lists, not project task lists.
- A database or index next to the file [myco].
- Other record types (decision records, plans, issues) [myco].
- Sync with GitHub Issues, Jira or other trackers.
- Locking across machines (git merges cover that); real-time sync.
- Changing a task's status automatically from git history (P6).
- Network access of any kind, including self-update.
