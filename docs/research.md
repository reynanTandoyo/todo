# Research behind the requirements

A record of what was studied on 2026-11-15 and what it taught. It adds no requirements; those are
in [requirements.md](requirements.md). This project doesn't depend on any of the sources below.

**Sources**

- Two task scripts the owner wrote for other projects, read-only: a JavaScript script
  (`scripts/todo.js`, in a game project) and a Python module (`todo.py`, in a music project), plus
  the real `TODO.md` files they manage.
- Four public Rust projects, read at source level: [arkan/td](https://github.com/arkan/td) (MIT),
  [aegea-dev/primum](https://github.com/aegea-dev/primum) (Apache-2.0),
  [9aia/myco](https://github.com/9aia/myco) (MIT) and
  [lens077/ToDo](https://github.com/lens077/ToDo) (no licence: its `NOTICE.md` reserves all rights,
  so it was used for ideas only). td was also built and run on copies of both `TODO.md` files.

---

## 1. The two home-grown scripts

### 1.1 Side by side

| | JavaScript script | Python module |
|---|---|---|
| A task is | `### T-012 · Title` heading plus `- key: value` lines | one line: `- [ ] **Title** — text (added: claude, 2026-11-12)` |
| ID | `T-012` | none: matched by part of the title |
| Status | `- status: open/claimed/blocked/done` | `[ ]` open · `[~]` claimed · `[!]` blocked · `[x]` done |
| Owner | `- owner: name` (`—` = nobody) | `(claimed: agent, date)` appended to the line |
| Priority | `P0`–`P3`, with type and area: `- priority: P1 · type: bug · area: ui` | the section (Now / Next / Later) |
| Description | `files`, `problem`, `fix`, `accept` fields | text after the em dash |
| Dependencies | `- depends: T-103` (51 tasks), plus prose | prose only |
| Notes | `- note (2026-11-13, agent): text` | indented `  - 2026-11-13 agent: text` |
| Finishing | status → `done`, note `done — …`, stays in place | `[x]`, `(done: agent, date, commit)`, moved to top of `## Done` |
| Questions for a person | none (two tasks say `model: owner decision`) | `## Needs user decision` section |
| Lock | `.todo.lock/` folder + `owner.json`; one-shot locks broken after 30 s, manual `lock` after 10 min, waits up to 60 s | `TODO.md.lock` file; broken after 60 s; waits up to 30 s |
| Finding the file | fixed: the repository the script lives in | environment variable, then walk up from the current folder |
| Exit codes | 1 for every error | 0 ok · 1 refused · 2 usage or lock timeout |
| Commands | list, show, next, claim, release, done, block, note, set, add, lock, unlock, status | list, add, claim, release, note, block, done |
| Size of the real file | 375 tasks, 34 sections, 4,475 lines, 590 KB | 251 items, 7 sections, 513 lines, 253 KB |

### 1.2 Findings

- **F1 — Files grow big.** 590 KB and 253 KB. An agent reading the whole file at session start
  spends roughly 150k and 60k tokens. A short summary command and an archive are needed.
- **F2 — Finished tasks dominate.** 289 of 375 tasks, and 111 of 251 items, are done.
- **F3 — Worktrees hold stale copies.** One project has 7 linked git worktrees (agent sandboxes in
  `.claude/worktrees/`), each with its own `TODO.md`, 34 to 993 lines behind the main checkout. No
  update was lost, but a script started inside a worktree walks up from its folder and finds the
  stale copy first. A lock on that copy protects nothing.
- **F4 — Dependencies are used but ignored.** `depends:` appears on 51 tasks and a `model:` field
  (which model should do the task) on 103; `next` ignores both, so it can hand out a task whose
  prerequisite is still open.
- **F5 — Both projects need "a question for a person".** One has a section for it, the other
  misuses a field.
- **F6 — Title codes are not unique.** 168 of 251 items start with a code like `[GUI-P4]`, but some
  codes appear on several items. Matching must not rely on them.
- **F7 — Dates disagree.** One script writes the UTC date, the other the local date.
- **F8 — One script rewrites line endings.** The Python module always writes LF and normalises the
  end of the file, so a CRLF file changes completely on every write. The JavaScript script keeps
  the file's line ending.
- **F9 — Reading doesn't need the lock.** One script locks for reading; atomic replacement makes
  that unnecessary.
- **F10 — Lock protocols are private.** Each script only sees its own kind of lock.
- **F11 — `next` then `claim` is a race.** Two agents calling `next` get the same task; one claim
  fails and has to start over.
- **F12 — Meaningful exit codes help agents.** 0/1/2 lets an agent tell "someone else has it" from
  "try again"; a single error code doesn't.
- **F13 — Rules drift.** Usage rules are written by hand in the `TODO.md` header, `AGENTS.md` and
  `CLAUDE.md`, and the copies differ.
- **F14 — Nothing unblocks.** Neither script has `unblock`; one file has 25 blocked tasks.
- **F15 — Ownership rules differ.** One script lets only the owner block a task, the other lets
  anyone.
- **F16 — Ambiguous matches.** The Python module refuses a title fragment that matches several
  items and lists them, and never matches the example in its rules header. The JavaScript script
  would treat an example `### T-…` heading in the header as a real task.
- **F17 — Section matching is loose.** `add --section "Phase 3"` takes the first heading that
  contains the text, even when several do.
- **F18 — Hand-written preambles matter.** Both files start with a title, how-to and rules that must
  survive every write.

---

## 2. The four GitHub projects

None of them can read either real `TODO.md`. td, run on copies, listed 0 tasks for both and refused
every write.

### 2.1 At a glance

| Concern | td | primum | myco | lens077/ToDo |
|---|---|---|---|---|
| Storage | one file, checkbox lines | one file per item, front matter | `TODO.md` lines plus one file per record, on an orphan git branch | one file, checkbox lines |
| IDs | random 8 hex, hidden comment | number = file name, highest + 1 | `T-001`, counter per type | random base36, hidden comment |
| Claims | none | none (`start` checks nothing) | owner + status, `--steal`, sessions, `reap` | none (every agent gets the same `next`) |
| Lock | OS lock, path hash in a config folder | OS lock, no timeout, held during network calls | OS lock in the git common folder, no waiting | OS lock on a sibling file, 2 s wait |
| Unknown text | refuses to write | parse error | parse error; rewrites the whole file | byte-exact splice, verified |
| History | none | git commit per change | git commit per change, `undo` | numbered snapshots |
| Worktrees | n/a | n/a | lock shared, but the file isn't found from a worktree | `./TODO.md` only |
| Agent setup | `SKILL.md` | one line in its docs | `AGENTS.md` block, skill, `manifest` | `AGENTS.md` block, SessionStart hook |
| Windows | CI builds it | Linux and macOS only | not checked | POSIX-only hook command |

### 2.2 arkan/td — MIT, Rust, ~6.5k lines, 32 commits, v0.1.1

A personal task manager for one Obsidian `TODO.md` with `## Tasks`, `## Backlog` and `## Archive`,
plus a terminal UI and an agent skill.

- Worth taking: an OS file lock held across load and save, named by a hash of the canonical path
  (`file.rs:100-127, 326-334`); refusing to write a file it can't round-trip (`file.rs:181-199`);
  matching by ID prefix or text with a candidate list when ambiguous (`task_match.rs`); a skill file
  shipped with the tool; CI with format, Clippy and tests on three systems; releases that check the
  tag against the Cargo version and publish SHA-256 checksums.
- Avoid: re-rendering the whole file, automatic "migrations" that move tasks and rename sections,
  IDs hidden in HTML comments, due dates and recurrence (personal to-dos, not project work).

### 2.3 aegea-dev/primum — Apache-2.0, Rust, ~4.8k lines, 100 commits, v0.2.1

A backlog as one Markdown file per item (`items/87.md` with front matter), ranked by a computed
score, a generated `BOARD.md`, a web board, history in a separate bare git repository, and
field-by-field sync between machines.

- Worth taking: lock, re-read, change only the fields named, write (`store.rs:147-159, 324-356`);
  recording edits made outside the tool as their own history entry before the next change
  (`history.rs:191-195`); text through `--file` or standard input (`cli.rs:697-705`); `--yes` for
  risky commands without a terminal; `next` shows work in progress first and works out "blocked"
  when reading; IDs never reused, duplicates renumbered with references rewritten
  (`sync.rs:150-181`); errors as `file:line: message`; migrations that convert everything before
  writing; tests that run the real binary with a cleared environment, non-ASCII data, "a failed
  change writes nothing" and "a closed pipe ends quietly".
- Avoid: one file per item for this purpose (harder to read, needs a generated index); rejecting
  unknown fields (one bad file stops every command); a lock without timeout held during network
  calls; `start` without any check; Unix-only assumptions and no Windows CI.

### 2.4 9aia/myco — MIT, Rust, ~57k lines, 148 commits, v0.2.0

A "records engine" for agent work: todos, issues, plans, decision records and handoffs in a
`project/` folder that is a worktree of a separate orphan branch, with a git commit per write.

- Worth taking: the lock file in the git common folder, shared by every worktree and never committed,
  using Rust's standard `File::try_lock` (`myco-ops/ops.rs:18-90`); `next --claim` choosing and
  claiming under one lock (`cli/next.rs:131-170`); a claim guard with `--steal` that leaves an audit
  entry, and sweeps for abandoned claims; IDs from the highest seen across open, done and archived
  records; a capped session-start summary (`status --prompt`); a transition table whose errors list
  the allowed next statuses; `check --fix`; an `AGENTS.md` block with a drift check; property-based
  round-trip tests and lock race tests.
- Avoid: strict parsing with a canonical whole-file rewrite (drops unknown keys, turns CRLF into
  LF); git as the transaction log (fetch and push on every write); a lock that fails at once
  instead of waiting; seven statuses; a folder walk that stops at the worktree root, so its ledger
  isn't found from `.claude/worktrees/*` without a registry; a guard that lets a non-owner finish a
  claimed task.

### 2.5 lens077/ToDo — no licence, Rust + Tauri, 238 commits, experimental

A "local task reconciler for AI-assisted development": `TODO.md` with YAML front matter, checkbox
tasks with hidden random IDs and emoji metadata, a CLI, a desktop app, an HTTP server and an agent
session feed. Ideas only; no code may be copied.

- Worth taking (as ideas): byte-range edits followed by a check that every byte outside them is
  unchanged and the result still parses, before writing (`write.rs:119-135, 746-778`); a hash and
  time check just before writing, to catch editors that ignore the lock (`lock.rs:54-65`); refusing
  files with git conflict markers; numbered snapshots with restore that previews and can be undone;
  a versioned JSON envelope with `ok`/`data`/`error` and a distinct exit code for "a check couldn't
  run"; a managed `AGENTS.md` block and a SessionStart hook printing at most 40 lines within
  2 seconds; reporting tasks against git evidence without ever changing their status; property
  tests over random edits and a 10,000-task benchmark.
- Avoid: reading only `./TODO.md` (no parent search, no `--file`); no claims at all; one replaceable
  note per task; `init --adopt` rewriting the whole file without a restore point; heavy
  dependencies (SQLite, an HTTP client) even for `list`; hooks that assume a POSIX shell.

---

## 3. File layouts considered

| Criterion | A. One `TODO.md`, task blocks | B. One `TODO.md`, checkbox lines | C. One file per task | D. Support A and B |
|---|---|---|---|---|
| Readable in an editor or on GitHub | good: headings give an outline | best: a checklist | poor: needs a list command or generated index | mixed |
| Stable IDs | yes | would need adding | yes (file name) | mixed |
| Room for fields (priority, depends, accept) | yes, one per line | awkward, inline | yes (front matter) | mixed |
| Git diffs | one line per field | the whole long line changes | one file per change | mixed |
| Merge conflicts between branches | possible; avoided when every worktree uses one file | possible | rare, except two branches picking the same ID | possible |
| File growth | needs an archive | needs an archive | none | needs an archive |
| Build and test cost | one parser | one parser | a store, an index, ID allocation across branches | two of everything |

**Chosen: A** (requirements.md, decision D1). It keeps one readable file, has room for the fields
agents need, and diffs cleanly; the main advantage of C, fewer merge conflicts, is mostly achieved
by having every worktree use the main checkout's file (D5), at a fraction of the cost.

---

## 4. Where each decision came from

| Decision (requirements.md §3) | Main evidence |
|---|---|
| D1 one file, task blocks | §3 above; the block format managed 375 tasks in practice |
| D2 five statuses with `dropped` | primum `wontfix`, myco `cancelled`; F14 |
| D3 sequential IDs, never reused | JavaScript script, myco, primum; F6, F16 |
| D4 claims expire, re-claim refreshes | Python module (2-day expiry), myco `reap` |
| D5 worktrees share the main file | F3; myco's lock placement; the gap in myco and lens |
| D6 OS lock plus 10-minute lease | td, primum, myco, lens use OS locks; the JavaScript script's manual lock |
| D7 state in the git common folder | myco |
| D8 archive on request | F1, F2; lens and myco archive files |
| D9 agent name required | the JavaScript script's rule 3; myco's fallback shows the risk |
| D10 JSON envelope and exit codes | F12; lens, myco |
| D18 bytes and line endings preserved | F8, F18; lens |
