<!-- todo-format: 2 -->
<!-- todo-tool: 0.7.0 -->
# todo — task list

The work plan for building `todo`. What each task must achieve is in
[`docs/requirements.md`](docs/requirements.md); the `reqs:` line names the requirements a task
covers, and `accept:` is the check that proves it is finished. The file follows
[`docs/format.md`](docs/format.md).

The file is changed only through `todo`, like any other project that uses it. Everything finished
up to v0.7 is in [`TODO-archive.md`](TODO-archive.md).

Priorities say how urgent a task is: P0 = now, P1 = planned, P2 = wanted, P3 = someday.

A milestone is a piece of work with a goal, divided into tasks or further milestones (for example
"Develop the renderer" with "C shader" and "GPU compiler"); it is not a release label. The HTML
report is the one view meant for people; command output is written for agents.

## v0.8: level-1 tasks and parallel work

A planner splits the work until every task is level 1, the level the weakest model finishes
without a mistake. Workers then take those tasks at the same time, in one checkout.

### T-145 · Weak models finish the work in parallel
- status: open
- owner: —
- priority: P1 · type: milestone
- goal: A planner splits the work until every task is level 1; any number of cheap agents then finish those tasks at the same time, without touching each other's files and without a false done
- why: the owner wants work finished faster by spawning many cheap agents (Haiku, DeepSeek flash) instead of a few strong ones
- note (2026-11-19, claude-main): added

### T-146 · v0.8 requirements: levels, detailed tasks, split, verify, stuck, file scopes, waves
- status: open
- owner: —
- priority: P0 · difficulty: hard · type: docs · area: docs
- milestone: T-145
- why: every other v0.8 task builds what this one writes down; the owner's decisions of 2026-11-19 must be in the requirements first
- accept: docs/requirements.md defines R-LEVEL-1 to R-LEVEL-3, R-DETAIL-1 to R-DETAIL-3, R-SPLIT-1, R-VERIFY-1, R-VERIFY-2, R-STUCK-1, R-SCOPE-1, R-SCOPE-2, R-WAVE-1 and R-REPORT-3 as the v0.8 tasks describe them; section 2 describes the two roles, planner and worker; NFR-5 and section 13 say that todo runs one more kind of command, a task's `verify` line, and nothing else; the settings table gains `target_difficulty`, `level1_max_files`, `verify`, `verify_timeout_seconds`, `verify_attempts` and `verify_shell`, and the defaults of `difficulties` (`1` to `5`) and `required_fields` (`priority`, `difficulty`) change; no model name appears anywhere, only what each level means; section 11 lists v0.8; docs/format.md becomes format version 3: the fields `steps` (a numbered list on indented lines under `- steps:`), `read`, `verify`, `final` and `attempts`, `files` as the list of paths and patterns a task may change, and the migration step from format 2
- reqs: REL-4, NFR-8
- note (2026-11-19, claude-main): added
- note (2026-11-19, claude-main): Also specify `verify_when` (`done` or `wave`, T-158), the `verified` field and `todo verify --wave`.
- note (2026-11-19, claude-main): R-AGENT-1 changes: the one rules text becomes a short common block (at most 15 lines) plus `todo help worker` and `todo help planner` (T-166).

### T-147 · Difficulty levels 1 to 5 and the planner's list
- status: open
- owner: —
- priority: P1 · type: milestone
- milestone: T-145
- goal: Every task has a level from 1 (the weakest model finishes it without a mistake) to 5, and the planner sees at a glance what still has to be split or detailed
- why: the planner can only drive everything to level 1 if nothing is unrated and the remaining hard tasks are listed for it
- note (2026-11-19, claude-main): added

### T-148 · Levels 1 to 5: new default difficulties, difficulty required, old values migrated
- status: open
- owner: —
- priority: P1 · difficulty: medium · type: feature · area: core
- milestone: T-147
- why: a task without a level cannot be given to the right agent, so an unrated task must not exist
- depends: T-146
- accept: the default `difficulties` is `1`, `2`, `3`, `4`, `5` and `todo help add` says what each level means without naming a model; `required_fields` defaults to `priority` and `difficulty`, applied to tasks only (milestones, ideas and questions need neither); `add` and `add --from` without a difficulty are refused with `MISSING_FIELD`; the format 2 to 3 migration rewrites `easy` to `2`, `medium` to `3` and `hard` to `4` on entries that are not done or dropped, only when todo.toml sets no `difficulties` of its own, and leaves open tasks without a difficulty for the planner's list; `next` never hands out a task without a difficulty; every refusal leaves the file byte-identical
- reqs: R-LEVEL-1, R-MIG-1, R-CFG-2, TEST-4
- note (2026-11-19, claude-main): added

### T-149 · todo plan: what the planner still has to split, detail or fix
- status: open
- owner: —
- priority: P1 · difficulty: medium · type: feature · area: cli
- milestone: T-147
- why: todo does not split tasks itself; it shows the planner exactly which ones need its attention
- depends: T-148, T-153, T-159
- accept: `todo plan [<milestone>]` lists, in this order: tasks a worker gave back (stuck), each with the problem; tasks above `target_difficulty` (default: the lowest level), hardest first; tasks at the target level that are not ready, each with what is missing; tasks without a difficulty; a last group, counted apart, holds tasks kept above the target on purpose (`final`); done, dropped and claimed entries, milestones, ideas and questions are left out; `summary` shows one line with the three counts within its 40 lines; `plan --json` matches its snapshot; exit 0 when the first four groups are empty, 1 otherwise
- reqs: R-LEVEL-2, R-READ-4
- note (2026-11-19, claude-main): added

### T-150 · final: keep a task above level 1 on purpose
- status: open
- owner: —
- priority: P2 · difficulty: easy · type: feature · area: cli
- milestone: T-147
- why: splitting is the intended way, but the planner may decide as a last resort that a task stays hard; that decision needs a reason and must stop the reminders
- depends: T-148
- accept: `set <task> --final <reason>` writes `- final: <reason>` and is refused on a task at the lowest level or without a reason; `set --unset final` removes it; `plan` and `summary` count such tasks apart (see the plan task); `check` reports a `final` on a milestone, idea or question; `show --json` and `schema` list the field
- reqs: R-LEVEL-3, TEST-4
- note (2026-11-19, claude-main): added

### T-151 · Level-1 tasks carry everything a weak model needs
- status: open
- owner: —
- priority: P1 · type: milestone
- milestone: T-145
- goal: A level-1 task names its files, its steps and its check, todo refuses to hand it out until it does, and the worker gets all of it in one work order
- why: a weak model fails when it has to explore, guess or decide; the task must leave nothing open
- note (2026-11-19, claude-main): added

### T-152 · Detail fields: steps, read and verify
- status: open
- owner: —
- priority: P1 · difficulty: hard · type: feature · area: core
- milestone: T-151
- why: the steps, the files to look at and the check command are the detail a weak model needs, and they do not fit the existing one-line fields
- depends: T-146
- accept: `add`, `set`, `add --from` and `apply` take `--steps` (text, `-` or `@path`; one step per input line, written as numbered indented lines under `- steps:`), `--read` (paths the worker should look at but not change) and `--verify` (one command line); `set --steps` replaces the whole list and `--unset steps` removes it with its lines; the parser reads the indented lines as the value of `steps` and still keeps every other indented line as text; LF, CRLF and byte-order-mark fixtures round-trip byte-identical; the merge driver treats the list as part of the task block; `show --json` returns `steps` as a list; `schema` lists the three fields; the field order in docs/format.md section 10 is followed
- reqs: R-DETAIL-1, R-EDIT-1, R-EDIT-7, TEST-2, TEST-3, TEST-4
- note (2026-11-19, claude-main): added

### T-153 · Ready rule: a level-1 task is handed out only when it is complete
- status: open
- owner: —
- priority: P1 · difficulty: medium · type: feature · area: core
- milestone: T-151
- why: calling a task level 1 is only a claim; the tool can at least check that the detail a weak model needs is there
- depends: T-152, T-148
- accept: a task at the lowest level is ready only when it has `why`, `accept`, `steps`, `verify` and `files`; `files` has at most `level1_max_files` entries (default 3) and none is a pattern or a folder; every path in `read` exists; no open question is about it; `next` never hands out a lowest-level task that fails the rule; `check` reports each such task with what is missing, as a warning that does not change the exit code; `show --json` carries `ready` and `missing`; tasks at other levels are not affected
- reqs: R-DETAIL-2, R-READ-3, R-CHECK-1
- note (2026-11-19, claude-main): added

### T-154 · todo brief: one work order for the worker
- status: open
- owner: —
- priority: P1 · difficulty: medium · type: feature · area: cli
- milestone: T-151
- why: a weak model should read one text and start, without running several commands to collect the context
- depends: T-152, T-158, T-159
- accept: `todo brief <task>` prints, in this order: the goals of the milestones above the task, top down; the title and the why; the files to read; the files it may change, with the sentence that no other file may be changed; the numbered steps; the verify command; for each finished dependency its done note and commit; the exact commands to finish (commit only the listed files, `todo done <id> --commit HEAD`) and to give up (`todo stuck <id> --note`), with the agent name filled in; `next --claim --brief` and `claim --brief` print the same after claiming; `brief --json` matches its snapshot; it reads only and takes no lock
- reqs: R-DETAIL-3, R-MILE-4, R-LOCK-5
- note (2026-11-19, claude-main): added

### T-155 · Splitting a task into smaller tasks
- status: open
- owner: —
- priority: P1 · type: milestone
- milestone: T-145
- goal: The planner replaces one hard task with several smaller ones in a single step, and nothing that depended on the old task breaks
- why: dividing is the planner's main move, so it must be one safe command and not a dozen
- note (2026-11-19, claude-main): added

### T-156 · todo split: turn a task into a milestone with child tasks
- status: open
- owner: —
- priority: P1 · difficulty: medium · type: feature · area: cli
- milestone: T-155
- why: splitting by hand takes one command per child plus a change of the parent, and a crash in between leaves half a split
- depends: T-152, T-148
- accept: `todo split <task> --from <file|->` takes child tasks as JSON lines in the shape of `add --from`; in one locked write with one snapshot the task becomes `type: milestone` with `goal` taken from `--goal` or else from its `accept`, loses `difficulty`, `files`, `steps`, `verify`, `final` and its claim, and every child is added after it with `milestone` set to it; children may depend on each other with `@ref`; tasks that depended on the split task still do and now wait for the whole milestone; all or nothing: one bad child changes nothing; refused on a done, dropped or milestone entry and on a task claimed by another agent without --force; a note on the parent names the children; `--dry-run` prints the diff
- reqs: R-SPLIT-1, R-WRITE-13, R-MILE-1, TEST-4
- note (2026-11-19, claude-main): added

### T-157 · A done that can be trusted
- status: open
- owner: —
- priority: P1 · type: milestone
- milestone: T-145
- goal: A task counts as done only after its own check passed and its commit stayed inside its files; a worker that fails gives the task back with the problem
- why: weak models report success when the work is broken, and with many of them nobody can re-check every task by hand
- note (2026-11-19, claude-main): added

### T-158 · Verify command: checked at done, or for the whole wave
- status: open
- owner: —
- priority: P1 · difficulty: hard · type: feature · area: core
- milestone: T-157
- why: asking the worker whether its test passed relies on the one thing that cannot be trusted
- depends: T-152
- accept: a task's `verify` command is run by todo in the folder of TODO.md, outside the write lock, through `verify_shell` (default PowerShell on Windows, `sh` elsewhere), stopped after `verify_timeout_seconds` (default 300). `verify_when` in todo.toml picks the mode. Mode `done` (default): `done` runs the command first; exit 0 marks the task done and writes `- verified: <date>`; anything else refuses with exit 1 and the new code `VERIFY_FAILED`, leaves the task claimed, raises `attempts` by one, saves the full output in the state folder and puts its last line in a note; after `verify_attempts` failures (default 3) the task is given back as `stuck` does. Mode `wave`: `done` does not run the command and the task is done without `verified`; `todo verify --wave` then runs the command of every done task that has `verify` and no `verified`, one after another; a pass writes `verified`; a failure reopens the task and gives it back as `stuck` does, with the last line of the output as the problem; it prints passed, failed and the tasks given back, exit 0 only when all passed, and refuses to start while any task is claimed unless --force. In both modes: `todo verify <task>` runs one command without changing anything; `done --no-verify` needs --force and leaves a note; `verify = false` turns running off; a task without `verify` behaves as today; `check` and `plan` list done tasks that are still unverified; the command's output never breaks `--json`; tests use a command that passes, one that fails and one that outlives a short timeout, in each mode
- reqs: R-VERIFY-1, R-VERIFY-2, R-LOCK-4, R-OUT-3, NFR-5, NFR-6
- note (2026-11-19, claude-main): added
- note (2026-11-19, claude-main): Owner 2026-11-19: both modes wanted. Wave mode exists because in a compiled project one worker's half-finished edit breaks the build for every other worker's check; there the planner checks all tasks when the wave is finished.

### T-159 · todo stuck: give a task back to the planner with the problem
- status: open
- owner: —
- priority: P1 · difficulty: easy · type: feature · area: cli
- milestone: T-157
- why: a weak model that cannot finish must have a clean way out, and the planner must learn what went wrong
- depends: T-146
- accept: `todo stuck <task> --note <problem>` sets the task to blocked, removes the owner and the claim, adds the note `stuck — <problem>` and raises `attempts` by one, in one write; the note is required; only the owner may do it, others need --force; the level is not changed; such a task is never handed out by `next` until the planner runs `unblock` or `split` on it; `summary` counts tasks given back; a test covers each case and each refusal
- reqs: R-STUCK-1, R-WRITE-9, TEST-4
- note (2026-11-19, claude-main): added

### T-160 · done checks that the commit stayed inside the task's files
- status: open
- owner: —
- priority: P1 · difficulty: medium · type: feature · area: core
- milestone: T-157
- why: in one shared checkout a worker that edits a file outside its task breaks another worker's task without anyone noticing
- depends: T-162
- accept: `done --commit` on a task with `files` compares the files that commit changed (from git, before the lock is taken) with the task's paths and patterns; a file outside them refuses with exit 1 and the new code `OUT_OF_SCOPE`, naming the files; `TODO.md` and the archive file are always allowed; --force overrides with a note; a lowest-level task with `files` cannot be finished without `--commit` while git is available; a task without `files`, or a folder without git, behaves as today; tests use a temporary repository with one commit inside and one outside the scope
- reqs: R-SCOPE-2, R-WRITE-4, NFR-5
- note (2026-11-19, claude-main): added

### T-161 · Many workers at once without collisions
- status: open
- owner: —
- priority: P1 · type: milestone
- milestone: T-145
- goal: Workers in one checkout never get tasks that touch the same file, and the planner sees how many workers are useful now and which chain of tasks limits the speed
- why: worktrees per agent cost disk space and a serial merge; file scopes let every agent work in the same folder
- note (2026-11-19, claude-main): added

### T-162 · File scopes: next never hands out tasks that share a file
- status: open
- owner: —
- priority: P1 · difficulty: hard · type: feature · area: core
- milestone: T-161
- why: two tasks with no dependency between them still collide when both edit the same file
- depends: T-146
- accept: `files` is a comma-separated list of paths, folders (ending in `/`) and `*` patterns, relative to the folder of TODO.md, compared with `/` and `\` as the same and ignoring case on Windows; two tasks overlap when a path of one equals, lies under or can match a path of the other; `next` and `next --claim` skip a ready task that overlaps a task claimed by someone else whose claim is not stale; `claim <task>` on such a task is refused with exit 1 and the new code `FILES_BUSY`, naming the task in the way, unless --force; tasks without `files` are handed out as today; 10 parallel `next --claim` on tasks that pairwise share files hand out tasks that never overlap; `list --ready` marks tasks held back for this reason; `show --json` lists the overlapping claimed tasks
- reqs: R-SCOPE-1, R-READ-3, R-CLAIM-2, TEST-5
- note (2026-11-19, claude-main): added

### T-163 · todo waves: how much can run in parallel, and the longest chain
- status: open
- owner: —
- priority: P1 · difficulty: medium · type: feature · area: cli
- milestone: T-161
- why: the number of waves is the fewest rounds the work needs however many agents are spawned, so it is the number the planner should shrink
- depends: T-162
- accept: `todo waves [<milestone>]` groups the unfinished tasks into waves: wave 1 is everything ready now, wave n is what becomes ready when the waves before it are done, using each task's dependencies and those inherited from milestones; inside a wave, tasks that share files are shown as one lane because they must run one after another; it prints the number of waves, the size of each, the largest number of workers useful at once, and the longest dependency chain task by task; tasks waiting on a missing ID or in a cycle are listed apart; `waves --json` matches its snapshot; it reads only
- reqs: R-WAVE-1, R-DEP-1, R-MILE-6
- note (2026-11-19, claude-main): added

### T-164 · HTML report shows the planning state
- status: open
- owner: —
- priority: P2 · difficulty: medium · type: feature · area: report
- milestone: T-145
- why: the page is the owner's only view, so it must show how far the planner is and where workers failed
- depends: T-149, T-163
- accept: each project page shows: open tasks per level; how many are not ready; tasks given back with the problem and their `attempts`; tasks kept above level 1 with the reason; the number of waves and the size of wave 1; each task shows its level, files and verify command when opened; the all-projects page shows open tasks per level on each card; snapshot tests are updated; still no button, form or command on the page
- reqs: R-REPORT-3, R-REPORT-1
- note (2026-11-19, claude-main): added

### T-165 · Performance test for plan, waves and file scopes
- status: open
- owner: —
- priority: P1 · difficulty: medium · type: test · area: core
- milestone: T-145
- why: with thousands of level-1 tasks, overlap and wave calculations must not make every `next` slow
- depends: T-149, T-163
- accept: on the generated 5,000-task milestone-tree file, extended so every task has files and a third of them share one, `next`, `plan`, `waves` and `brief` each take under 100 ms and `next --claim` under 200 ms in a release build; the test runs in CI with the other performance tests
- reqs: NFR-3, TEST-9
- note (2026-11-19, claude-main): added

### T-166 · v0.8 agent rules by role, documentation and release 0.8.0
- status: open
- owner: —
- priority: P1 · difficulty: medium · type: docs · area: release
- milestone: T-145
- why: the new commands only help when every agent knows which role it has and what to run
- depends: T-150, T-154, T-156, T-160, T-164, T-165
- accept: the rules are split by role so that no agent carries rules it does not need. The common rules block (AGENTS.md, TODO.md, the skill) is at most 15 lines and ends with one line per role naming its help text. `todo help worker` is at most 10 lines: take work with `todo next --claim --max-difficulty 1 --brief`, change only the listed files, commit only those files, `todo done <id> --commit HEAD`, `todo stuck <id> --note` when it cannot finish; `todo brief` repeats the last three with the task's ID filled in, so a worker needs no other text. `todo help planner` holds the planner's rules: give every task a level; split with `todo split` until `todo plan` is empty; write files, steps and verify for level-1 tasks; keep a higher level only with `--final`; read `todo waves` and break long chains; in wave mode run `todo verify --wave` when a wave is finished, otherwise the full test suite. A test fails when the common block grows past 15 lines or the worker text past 10. The README explains the roles, the levels, file scopes in one checkout and what the verify command is trusted to do; docs/json.md documents every new JSON shape and the codes `VERIFY_FAILED`, `OUT_OF_SCOPE` and `FILES_BUSY`; CHANGELOG.md has the 0.8.0 entry including the format change and the changed defaults; Cargo.toml is 0.8.0; `todo migrate --yes` was run in this repository and `todo check` is clean
- reqs: R-AGENT-1, REL-2, REL-5
- note (2026-11-19, claude-main): added
- note (2026-11-19, claude-main): Rules text for the planner: in wave mode run `todo verify --wave` when every worker of the wave has finished, before starting the next wave.
- note (2026-11-19, claude-main): Owner 2026-11-19: commands cost no context (nothing is loaded until `todo help <command>` is run), but the rules text does and it grew from 11 to 20 lines by v0.7. Keep the common rules short and move role rules into `todo help worker` and `todo help planner`. Lines moved out of the common block today (milestones, ideas, questions, migrate, update) go to the planner text or stay only where a refusal message already names the command.

