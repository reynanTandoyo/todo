<!-- todo-format: 3 -->
<!-- todo-tool: 0.8.0 -->
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
- status: done
- owner: claude-main
- priority: P1 · type: milestone
- goal: A planner splits the work until every task is level 1; any number of cheap agents then finish those tasks at the same time, without touching each other's files and without a false done
- why: the owner wants work finished faster by spawning many cheap agents (Haiku, DeepSeek flash) instead of a few strong ones
- by: Reynan Henry Tandoyo
- note (2026-11-19, claude-main): added
- note (2026-11-20, claude-main): done — v0.8 released as 0.8.0: levels 1 to 5, detailed level-1 tasks, split, verify, stuck, commit scope, file scopes, waves, plan and brief.

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

## Questions

### T-176 · T-172: a verify timeout should stop the whole process tree. Which option: (1) libc and windows-sys as direct dependencies (already in Cargo.lock, NFR-9 list changes), (2) hand-written FFI with no dependency change, or (3) keep the limit and document it? Details are in the note on T-172.
- status: done
- owner: claude-main
- type: question
- about: T-172
- note (2026-11-20, t158-haiku): added
- note (2026-11-20, claude-main): answer — Option 1: make libc (Unix) and windows-sys (Windows) direct dependencies, target-specific, with only the features needed; both are already in Cargo.lock. Add them to the NFR-9 list with the reason (stop a verify command's whole process tree on timeout, NFR-5 forbids running kill/taskkill). Keep the unsafe calls few, small and commented.
