<!-- todo-format: 3 -->
<!-- todo-tool: 1.0.0 -->
# todo — task list

The work plan for building `todo`. What each task must achieve is in
[`docs/requirements.md`](docs/requirements.md); the `reqs:` line names the requirements a task
covers, and `accept:` is the check that proves it is finished. The file follows
[`docs/format.md`](docs/format.md).

The file is changed only through `todo`, like any other project that uses it. Everything finished
up to v0.8.1 is in [`TODO-archive.md`](TODO-archive.md).

Priorities say how urgent a task is: P0 = now, P1 = planned, P2 = wanted, P3 = someday.

A milestone is a piece of work with a goal, divided into tasks or further milestones (for example
"Develop the renderer" with "C shader" and "GPU compiler"); it is not a release label. The HTML
report is the one view meant for people; command output is written for agents.

## v1.0: a status page for people

The owner and a colleague see on one page what is done, what was cancelled, what is still to do
and what needs them. Problems found while using todo are fixed along the way.

### T-190 · Problems found while using todo are fixed
- status: done
- owner: claude-main
- priority: P2 · type: milestone
- goal: Every problem found while using todo on real work is recorded here and fixed, and todo's own checks (check, align, plan) stay clean on this repository
- why: the tool is only trusted when its own reports have no false alarms
- summary: Bugs we run into while using todo on real work get written down and fixed, so its own checks give no false alarms.
- by: Reynan Henry Tandoyo
- note (2026-11-21, claude-main): added
- note (2026-11-21, claude-main): Seen by t209-haiku 2026-11-21: apply only trims why (line breaks are not turned into spaces as format.md section 10 says); check and add a task if confirmed.
- note (2026-11-21, claude-main): done — The false align findings on milestones (T-191) and the line breaks refused by apply (T-215) are fixed.

### T-191 · align reports every done milestone as DONE_WITHOUT_COMMIT
- status: done
- owner: t191-haiku
- priority: P2 · difficulty: 1 · type: bug · area: git
- milestone: T-190
- why: a milestone is closed, not committed, so align exits 1 on a clean list (six false findings on this repository at v0.8.1)
- summary: The check that compares tasks with the git history no longer complains about finished milestones, which are closed rather than committed.
- files: src/cli/align.rs, tests/cmd_align.rs, docs/requirements.md
- steps:
  1. Open src/cli/align.rs and find the comment `// Questions are answered, not committed (R-GIT-1).` (near line 85). The `if` under it skips a task whose `task_type` equals "question".
  2. Change that condition so it also skips a task whose `task_type` equals "milestone", compared with `eq_ignore_ascii_case` in the same way. Change the comment to `// Questions are answered and milestones are closed, not committed (R-GIT-1).`
  3. Open tests/cmd_align.rs and find the test `answered_questions_are_ignored` (near line 112). Directly after it, add a test `done_milestones_are_ignored` that is a copy of it with one change: the task line is `task("T-001", "done", "- type: milestone\n- goal: Example goal\n")`.
  4. Open docs/requirements.md and find the sentence "`dropped` tasks and questions are ignored." inside R-GIT-1 (near line 701). Change it to "`dropped` tasks, questions and milestones are ignored."
  5. Run `cargo fmt --all`, then `cargo test --test cmd_align`. Every test must pass.
- accept: `todo align` gives no finding for an entry of type milestone, whatever its status; the new test done_milestones_are_ignored passes with the existing align tests; R-GIT-1 says milestones are ignored
- verify: cargo test --test cmd_align
- verified: 2026-11-21
- commit: 6746a8a
- reqs: R-GIT-1
- by: Reynan Henry Tandoyo
- note (2026-11-21, claude-main): added
- note (2026-11-21, t191-haiku): done — align now skips milestones like questions, and R-GIT-1 says so; cargo test --all-features passes 919 tests (0 failed), including the new done_milestones_are_ignored.

### T-195 · A status page made for people
- status: done
- owner: claude-main
- priority: P1 · type: milestone
- goal: The owner and a colleague open one page and see within a minute what is done, what was cancelled and why, what is still to do, and what needs them
- why: the v0.7 page shows what the tool knows instead of what a reader asks: too much text, too little context, and a tree that loses finished work
- summary: One page shows the owner and a colleague, within a minute, what is done, what was cancelled and why, what is still to do, and what needs them.
- by: Reynan Henry Tandoyo
- note (2026-11-21, claude-main): added
- note (2026-11-21, claude-main): done — Version 1.0.0: one page shows what is done, what was cancelled and why, what is still to do, and what needs you; the owner judged it ok on this project.

### T-196 · Requirements for the new status page
- status: done
- owner: claude-main
- priority: P0 · difficulty: 3 · type: docs · area: docs
- milestone: T-195
- why: the v0.7 page was specified in words only and came out wrong; this time the approved mockup is the reference and the requirements must describe exactly that page
- summary: The requirements describe the approved page design exactly, so everyone builds the same page.
- read: docs/report-mockup.html, docs/requirements.md, docs/format.md
- accept: docs/requirements.md describes the page shown in docs/report-mockup.html (approved by the owner on 2026-11-21): R-REPORT-1 is rewritten for the new layout (header, overall line and bar, the tree with done and cancelled entries in place, one line per entry, the second line, summary and technical details, Other work, and the closed parts Ideas, Earlier work and For the planner); R-REPORT-4 (Needs you), R-REPORT-5 (Last 7 days) and R-REPORT-6 (view buttons and light or dark mode) are new; R-WRITE-19 defines the `summary` field; the settings table gains `description`; the five words for people are fixed: to do (open), in progress (claimed), waiting (blocked), done, cancelled (dropped); the rule "no control that changes TODO.md" stays and view controls are allowed; a small inline script is allowed and the page must be readable without it; R-REPORT-3 says the planning state moves into For the planner; `report --html --all` keeps its v0.8 content and is out of scope; docs/format.md lists `summary` with its place in the field order and the format stays version 3; section 11 lists v1.0
- commit: dc6710a
- reqs: REL-4, NFR-8
- by: Reynan Henry Tandoyo
- note (2026-11-21, claude-main): added
- note (2026-11-21, claude-main): Owner 2026-11-21: R-REPORT-6 also covers the colour-blind colours button and the four colour sets (light, dark, each normal and colour-blind); R-REPORT-1 says a state is never shown by colour alone. docs/report-mockup.html was updated to show both.
- note (2026-11-21, claude-main): Review 2026-11-21: (1) R-REPORT-2 says the --all page shows each project's lists "as in R-REPORT-1"; when R-REPORT-1 is rewritten, copy the v0.8 list description into R-REPORT-2 so --all keeps its content. (2) Define how a task waits for a question (Needs you, T-199; the mockup's "waits for this answer"): a task whose depends names the question. (3) Define the page's words for a task given back with stuck (waiting, with "given back" on its second line, as in the mockup).
- note (2026-11-21, claude-main): done — R-REPORT-1 rewritten for the mockup; R-REPORT-4/5/6, R-WRITE-19, description setting, v1.0 added; R-REPORT-2 keeps v0.8 content in its own words; an open task that depends on an unanswered question shows as waiting; light-mode waiting and to-do colours darkened to reach 3:1.

### T-197 · summary field: what an entry is about, written for people
- status: done
- owner: claude-main
- priority: P1 · type: milestone · area: cli
- milestone: T-195
- goal: Every entry can carry a summary for people, written through todo and checked
- why: titles and accept lines are written by agents for agents; the page needs one or two plain sentences per entry that a colleague understands
- summary: Every task and milestone can carry a short text written for people, and todo points out unfinished work that has none.
- depends: T-196
- accept: `add`, `set`, `add --from`, `apply` and the children of `split` take `--summary` (text, `-` or `@path`) and write `- summary:` after `why` in the field order; it is allowed on tasks, milestones, questions and ideas; `set --unset summary` removes it; `show --json` and `schema` list it; `check` warns, without changing the exit code, about each task or milestone that is not done or dropped and has no summary; `brief` does not print it; a file written by todo 0.8 that has no summary anywhere works unchanged; a test covers each case
- reqs: R-WRITE-19, TEST-4
- by: Reynan Henry Tandoyo
- note (2026-11-21, claude-main): added
- note (2026-11-21, claude-main): split into T-208, T-209, T-210
- note (2026-11-21, claude-main): done — The summary field is read, written by every command and checked (T-208 to T-210).

### T-208 · summary field: read it, show it and list it in schema
- status: done
- owner: t208-haiku
- priority: P1 · difficulty: 2 · type: feature · area: core
- milestone: T-197
- why: every other part of the summary field needs the parser to know it first
- summary: todo reads the new short text for people and shows it.
- final: The owner's workers are Haiku 5.5 at xhigh effort, which finished level-3 tasks in v0.8 (T-148, T-149); this is one coherent change with full detail, and smaller pieces would not compile on their own.
- files: src/doc/parser.rs, src/doc/verify.rs, src/ops/read.rs, src/cli/schema.rs, src/cli/show.rs, tests/field_summary.rs, docs/json.md, tests/doc_parser.rs
- read: docs/requirements.md, docs/format.md
- steps:
  1. Read R-WRITE-19 in docs/requirements.md and the summary row of docs/format.md section 8 and the field order in section 10.
  2. In src/doc/parser.rs add "summary" to CANONICAL_FIELD_ORDER directly after "why", give it a slot in known_field_index (raise KNOWN_FIELD_SLOTS by one), and add `summary: Option<String>` and `summary_line: Option<usize>` to the task struct next to `why`, parsed exactly like `why` (both places where `why` is parsed). Search for every place that builds or compares a task (for example `why_line: None`, src/doc/verify.rs) and add the new fields there too.
  3. In src/ops/read.rs add `summary` to the show JSON struct next to `why`, filled from the task, and make `todo show` print it in text after `why` the same way `why` is printed (find where why is printed in show).
  4. In src/cli/schema.rs add summary to the example task and to the field list, next to why.
  5. Add tests/field_summary.rs with invented data in a temporary folder: a hand-written `- summary:` line is read (show --json has it, and it is no longer under `fields`); a file without any summary works unchanged; a repeated summary line is reported like any repeated field. Use the helpers in tests/common.
  6. Run the four checks of AGENTS.md.
- accept: a `- summary:` line is a known field placed after `why` in the field order; `show` and `show --json` carry it (not under `fields`); `schema` lists it; a repeated summary is a repeated field; a file without summary is unchanged; tests in tests/field_summary.rs cover each case
- verify: cargo test --test field_summary
- verified: 2026-11-21
- commit: 75b1603
- by: Reynan Henry Tandoyo
- note (2026-11-21, claude-main): added
- note (2026-11-21, claude-main): forced set (owner was t208-haiku)
- note (2026-11-21, t208-haiku): done — Added the summary field to the parser, show --json and schema, with 7 new tests in tests/field_summary.rs; cargo test passes 926 tests, and fmt and clippy are clean for Windows and Linux.

### T-209 · summary field: write it with add, add --from, apply, set and split
- status: done
- owner: t209-haiku
- priority: P1 · difficulty: 2 · type: feature · area: cli
- milestone: T-197
- why: planners and agents must be able to write the summary through todo, never by hand
- summary: The short text for people can be written with every command that creates or changes an entry.
- final: The owner's workers are Haiku 5.5 at xhigh effort, which finished level-3 tasks in v0.8 (T-148, T-149); this is one coherent change with full detail, and smaller pieces would not compile on their own.
- depends: T-208
- files: src/cli/add.rs, src/cli/set.rs, src/cli/apply.rs, src/cli/split.rs, src/ops/task_ops.rs, src/ops/edits.rs, tests/field_summary_write.rs
- read: docs/requirements.md
- steps:
  1. Read R-WRITE-19 in docs/requirements.md. Follow exactly how the `why` field is handled in each file below; `summary` behaves the same, written after `why`.
  2. src/cli/add.rs: a `--summary <TEXT>` option (text, `-` or `@path` like `--why`), and `summary` in the add --from JSON entry and the validated batch entry; it is allowed on every kind of entry (task, milestone, question, idea).
  3. src/cli/set.rs: `--summary <TEXT>`, and `--unset summary` removes the line. src/ops/edits.rs: set and unset for summary next to why.
  4. src/cli/apply.rs: `summary` in the add and set operations, next to why. src/ops/task_ops.rs: summary in the new-task field list after why.
  5. src/cli/split.rs: children of split take `summary` from their JSON line (they go through the same batch entry, so check that nothing drops it).
  6. Write tests/field_summary_write.rs (invented data, temporary folders, helpers from tests/common): add --summary, add --from with summary, apply add and set with summary, set --summary, set --unset summary, split child with summary, and `@path` input; each checks the line `- summary: <text>` sits directly after `- why:` when both exist, and that a refusal leaves the file byte-identical.
  7. Run the four checks of AGENTS.md.
- accept: `add`, `add --from`, `apply`, `set` and the children of `split` take `--summary` (text, `-` or `@path`) and write `- summary:` after `why`; it is allowed on tasks, milestones, questions and ideas; `set --unset summary` removes it; tests cover each command
- verify: cargo test --test field_summary_write
- attempts: 1
- verified: 2026-11-21
- commit: aed90b7
- by: Reynan Henry Tandoyo
- note (2026-11-21, claude-main): added
- note (2026-11-21, t209-haiku): verify failed — error: test failed, to rerun pass `--test field_summary_write`
- note (2026-11-21, t209-haiku): done — The summary field is written by add (--summary, --from), apply (add and set), set (--summary, --unset summary, --field) and the children of split, with -, @path and line breaks turned into spaces, after why and on every kind of entry; 15 new tests in tests/field_summary_write.rs, which pass with and without the test-hooks feature, and cargo test passes 942 tests with 0 failed, with fmt and clippy clean for Windows and Linux.

### T-210 · summary field: check warns about unfinished work without one; brief leaves it out; docs/json.md
- status: done
- owner: t210-haiku
- priority: P1 · difficulty: 2 · type: feature · area: cli
- milestone: T-197
- why: the page is only readable when every unfinished task and milestone has a summary, so check must point at the ones missing
- summary: todo warns about unfinished tasks and milestones that have no text for people yet.
- final: The owner's workers are Haiku 5.5 at xhigh effort, which finished level-3 tasks in v0.8 (T-148, T-149); this is one coherent change with full detail, and smaller pieces would not compile on their own.
- depends: T-208
- files: src/cli/check.rs, src/ops/brief.rs, docs/json.md, tests/field_summary_check.rs, tests/cmd_final.rs, tests/cmd_difficulty.rs
- read: docs/requirements.md
- steps:
  1. Read R-WRITE-19 and R-CHECK-1 (the sentence that starts `From v1.0`) in docs/requirements.md.
  2. Find where check produces the warning for an unfinished task that lacks detail: src/cli/check.rs, step 3e (search for `detail is incomplete`). Next to it, add a warning that does not change the exit code for every task or milestone (not question, not idea) whose status is open, claimed or blocked and that has no summary: `T-0xx: no summary: write one or two plain sentences for people with todo set T-0xx --summary`. Add one sentence about it to the `details` help text of check in the same file.
  3. Make sure `todo brief` does not print the summary (src/ops/brief.rs).
  4. docs/json.md: add `summary` wherever `why` is documented for show --json and for the add --from and apply input.
  5. Write tests/field_summary_check.rs (invented data, temporary folders, helpers from tests/common): check warns for an open task and an open milestone without summary and exits 0; no warning for a done task, a question or an idea without one; brief output of a task with a summary does not contain it.
  6. Run the four checks of AGENTS.md.
- accept: `check` warns, without changing the exit code, about each task or milestone that is not done or dropped and has no summary; brief does not print it; docs/json.md lists summary next to why; tests cover each case
- verify: cargo test --test field_summary_check
- verified: 2026-11-21
- commit: 980ed08
- by: Reynan Henry Tandoyo
- note (2026-11-21, claude-main): added
- note (2026-11-21, claude-main): forced set (owner was t210-haiku)
- note (2026-11-21, t210-haiku): done — check warns, without changing its exit code, about each open, claimed or blocked task or milestone without a summary; brief leaves the summary out and docs/json.md lists it next to why; cargo test --all-features passes 931 tests with 0 failed, and fmt and clippy are clean for Windows and Linux.

### T-198 · Page data: states in plain words, counts, dates and the full tree
- status: done
- owner: claude-main
- priority: P1 · type: milestone · area: report
- milestone: T-195
- goal: The data behind the new page: every entry with its state, dates and texts, in one tree with counts
- why: the page can only keep finished work in its place if the data behind it is one tree with every entry, its state and its dates
- summary: The data behind the new page: every entry with its state in plain words, its dates and texts, in one tree with counts.
- depends: T-196
- accept: the report model (src/report/model.rs) holds one tree with every entry of TODO.md, done and dropped ones included, each with one of five states: to do, in progress, waiting, done, cancelled; the overall numbers are "N of M tasks done" where M leaves out cancelled tasks, plus a count per state; each milestone has the same numbers over every task below it, a state (not started, in progress, finished) and, when finished, the date of its done note; a done entry carries the date and text of its last done note as its result, a cancelled one the date and reason of its drop note, a waiting one its latest reason, an in-progress one its owner and the age of the claim; milestones are ordered in progress, then not started, then finished, keeping file order inside each group; tasks without a milestone form the group Other work; finished trees from the archive form the group Earlier work; milestones, questions and ideas are never counted as tasks; unit tests cover each rule with an invented fixture
- reqs: R-REPORT-1, R-KIND-3, TEST-1
- by: Reynan Henry Tandoyo
- note (2026-11-21, claude-main): added
- note (2026-11-21, claude-main): Review 2026-11-21: src/report/all.rs uses model::build_page and Page for the --all page; keep what it needs (or give --all its own copy) so report --html --all keeps its v0.8 content and tests.
- note (2026-11-21, claude-main): split into T-211, T-212
- note (2026-11-21, claude-main): done — The page data: every entry with its state, dates and texts, in one tree with counts (T-211, T-212).

### T-211 · Page data: one entry with its state, date, second line and details
- status: done
- owner: t211-haiku
- priority: P1 · difficulty: 2 · type: feature · area: report
- milestone: T-198
- why: every part of the new page shows entries the same way, so one function must decide an entry's state and texts
- summary: Decides how one entry reads on the page: to do, in progress, waiting, done or cancelled, with its date and reason.
- final: The owner's workers are Haiku 5.5 at xhigh effort, which finished level-3 tasks in v0.8 (T-148, T-149); this is one coherent change with full detail, and smaller pieces would not compile on their own.
- depends: T-208
- files: src/report/mod.rs, src/report/page/mod.rs, src/report/page/entry.rs
- read: docs/requirements.md, docs/report-mockup.html, src/report/model.rs, src/doc/reason.rs, src/ops/config.rs
- steps:
  1. Read section 5.23 of docs/requirements.md (the table of five states, the exception for a task waiting for a question, and the bullets 'An entry' and 'Details' of R-REPORT-1) and look at the entries in docs/report-mockup.html.
  2. Create the module src/report/page/ for the v1.0 page: src/report/page/mod.rs (module doc comment: the v1.0 page for one project, R-REPORT-1, R-REPORT-4 to R-REPORT-6; the old src/report/model.rs and html.rs stay for the --all page) with `pub mod entry;`, and add `pub mod page;` to src/report/mod.rs. Do not change model.rs, html.rs or all.rs.
  3. In src/report/page/entry.rs define `pub enum State { ToDo, InProgress, Waiting, Done, Cancelled }` with `word()` ("to do", "in progress", "waiting", "done", "cancelled"), `mark()` (○ ● ◐ ✓ ✕) and `class()` ("todo", "prog", "wait", "done", "drop", the CSS class names of the mockup).
  4. Define `pub struct EntryView` with: id (String, formatted), title, state, date (Option<NaiveDate>: the date of the latest note whose text starts with `done` for Done, or `dropped` for Cancelled), second (Option<SecondLine>), summary (Option<String> from task.summary), why, result (Option<String>: for Done the reason of the latest `done — <text>` note, via Task::latest_reason("done")), and details: level, priority, task type, area, files, verify, verified, commit, done_by (`by` field, else the owner), depends (Vec of (id, Option<title>)), attempts, notes (date, agent, text).
  5. Define `pub enum SecondLine { InProgress { owner: String, since: Since, stale: bool }, Waiting(WaitReason), Cancelled(String) }`, `pub enum Since { Today, Yesterday, Days(i64) }` and `pub enum WaitReason { GivenBack(String), Question(String), Blocked(String) }`. Cancelled holds Task::dropped_reason() or NO_REASON; Blocked holds Task::blocked_reason() or NO_REASON; GivenBack uses Task::is_given_back() and the reason. Since is computed from OpsConfig::claim_age in whole local days (0 today, 1 yesterday, else Days(n)); stale from OpsConfig::is_claim_stale.
  6. Write `pub fn entry_view(task: &Task, ctx: &Ctx) -> EntryView` where `pub struct Ctx<'a>` holds the documents (TODO.md and the optional archive), the OpsConfig and `now: DateTime<FixedOffset>`. State: status open is ToDo, claimed InProgress, blocked Waiting, done Done, dropped Cancelled; except an open task whose depends names a question (EntryKind::Question) that is not done or dropped: then Waiting with WaitReason::Question(<that question's title>). Titles of depends are looked up in both documents.
  7. Add unit tests in entry.rs (#[cfg(test)], invented data parsed from a string, fixed `now`): each of the five states; done date and result; cancelled with and without reason; blocked reason; given back; waiting for a question, and not waiting once the question is done; in progress today, yesterday, for 3 days, and stale; summary and why; details depends with titles.
  8. Run the four checks of AGENTS.md. Items that nothing uses yet may need #[allow(dead_code)] on the module with a comment `// used by the page renderer (T-202)`; prefer making them pub.
- accept: src/report/page/entry.rs turns any task into an EntryView with one of the five states (and Waiting for an open task that depends on an unanswered question), its done or cancelled date, its second line (in progress with since and stale, waiting with given back, question or blocked reason, cancelled with reason), summary, why, result and technical details; unit tests cover each rule; model.rs, html.rs and all.rs are unchanged
- verify: cargo test --lib report::page
- verified: 2026-11-21
- commit: 910cd04
- by: Reynan Henry Tandoyo
- note (2026-11-21, claude-main): added
- note (2026-11-21, t211-haiku): done — Added report::page::entry: entry_view turns a task into an EntryView with its state in words, date, second line and details, unit-tested with 23 tests passing in report::page; fmt and clippy clean on Windows and Linux, full suite 950 passing.

### T-212 · Page data: the work tree with counts, order, Other work and Earlier work
- status: done
- owner: t212-haiku
- priority: P1 · difficulty: 2 · type: feature · area: report
- milestone: T-198
- why: the page keeps finished work in its place only if the data is one tree with every entry and the right counts
- summary: Builds the tree of all work, finished parts included, with how much of each part is done.
- final: The owner's workers are Haiku 5.5 at xhigh effort, which finished level-3 tasks in v0.8 (T-148, T-149); this is one coherent change with full detail, and smaller pieces would not compile on their own.
- depends: T-211
- files: src/report/page/mod.rs, src/report/page/tree.rs
- read: docs/requirements.md, docs/report-mockup.html, src/report/page/entry.rs, src/report/model.rs, src/doc/tree.rs
- steps:
  1. Read the bullets 'Overall line', 'All work' and 'More' of R-REPORT-1 in docs/requirements.md and the tree in docs/report-mockup.html. Read src/report/page/entry.rs (EntryView, State, Ctx) and how src/report/model.rs and src/doc/tree.rs walk milestones.
  2. Create src/report/page/tree.rs and add `pub mod tree;` to src/report/page/mod.rs.
  3. Define `pub struct Counts { done, in_progress, waiting, to_do, cancelled }` (usize) with `total_counted()` (all but cancelled) and `pub struct Overall { counts: Counts }`; only entries of kind task count (EntryKind::Task), never milestones, questions or ideas.
  4. Define `pub enum MilestoneState { NotStarted, InProgress, Finished(NaiveDate), Cancelled(NaiveDate) }` and `pub struct MilestoneView { id, title, goal, summary, state, done: usize, total: usize, milestones: Vec<MilestoneView>, tasks: Vec<EntryView> }`. done and total count the tasks below it at any depth, cancelled left out of both. State: Finished when the milestone is done (date of its latest `done` note, else the date of its latest note), Cancelled when dropped (same for `dropped`), InProgress when any task below it is done, in progress or waiting, else NotStarted.
  5. Define `pub struct Tree { milestones: Vec<MilestoneView>, other: Vec<EntryView>, counts: Counts }` and `pub fn build_tree(doc: &Document, ctx: &Ctx) -> Tree`: top-level milestones ordered InProgress, then NotStarted, then Finished and Cancelled, file order inside each group; nested milestones and tasks in file order; tasks without a milestone in `other`; questions and ideas left out. counts are over every task in the doc.
  6. Add `pub struct Earlier { tree: Tree, finished_milestones: usize, tasks: usize }` and `pub fn build_earlier(archive: &Document, ctx: &Ctx) -> Earlier`: the same tree from the archive (finished_milestones = top-level milestones, tasks = tasks of kind task).
  7. Unit tests (#[cfg(test)], invented data from a string, fixed now): overall counts leave cancelled out of the total and milestones, questions and ideas out entirely; a milestone's N of M over nested milestones; each milestone state; the order of top-level milestones; nested order is file order; Other work; an archive gives Earlier with its numbers.
  8. Run the four checks of AGENTS.md.
- accept: src/report/page/tree.rs builds one tree with every task and milestone of a document (done and cancelled included), milestone states with dates, N of M per milestone (cancelled left out), overall counts per state over tasks only, the order in progress / not started / finished at the top level, Other work, and Earlier work from the archive; unit tests cover each rule
- verify: cargo test --all-features --lib report::page
- verified: 2026-11-21
- commit: ad42973
- by: Reynan Henry Tandoyo
- note (2026-11-21, claude-main): added
- note (2026-11-21, t212-haiku): forced claim (files held by T-199)
- note (2026-11-21, claude-main): forced set (owner was t212-haiku)
- note (2026-11-21, t212-haiku): done — Added report::page::tree: build_tree places every task and milestone with its state, N of M, the overall counts, the top-level order, Other work, and build_earlier for the archive; 9 new unit tests, report::page 32 passing, full suite 959 passing, fmt and clippy clean on Windows and Linux.

### T-199 · Needs you: questions, tasks given back and work without activity
- status: done
- owner: t199-haiku
- priority: P1 · difficulty: 2 · type: feature · area: report
- milestone: T-195
- why: the first thing a person asks is whether anything waits for them, and today that is spread over three sections
- summary: The page lists everything that waits for a person: open questions, tasks a worker gave back, and work nobody has touched for too long.
- final: The owner's workers are Haiku 5.5 at xhigh effort, which finished level-3 tasks in v0.8 (T-148, T-149); this is one coherent change with full detail, and smaller pieces would not compile on their own.
- depends: T-211
- files: src/report/page/mod.rs, src/report/page/needs.rs
- read: docs/requirements.md, docs/report-mockup.html, src/report/page/entry.rs
- steps:
  1. Read R-REPORT-4 in docs/requirements.md and the Needs you box of docs/report-mockup.html. Read src/report/page/entry.rs (Ctx, EntryView, State) first.
  2. Create src/report/page/needs.rs and add `pub mod needs;` to src/report/page/mod.rs.
  3. Define `pub enum Need { Question { title, asked: NaiveDate, about: Option<String>, waiting: Vec<String> }, GivenBack { title, date: NaiveDate, problem: String }, NoActivity { title, owner: String, days: i64 } }` and `pub fn build_needs(doc: &Document, ctx: &Ctx) -> Vec<Need>`, in the order of R-REPORT-4: every open question (kind question, not done or dropped) with the date of its `added` note (else its first note), the title of its `about` entry, and the titles of the unfinished tasks whose depends name it; then every task given back (Task::is_given_back, the date and text of the latest `stuck — …` note); then every claimed task whose claim is stale (OpsConfig::is_claim_stale) with its owner and the claim age in whole days. File order inside each kind.
  4. Add `pub fn heading(needs: &[Need]) -> String` giving "Needs you (n)" and a const EMPTY = "Nothing needs you.", and `pub fn waiting_text(titles: &[String]) -> Option<String>` giving 'the task “A” waits for this answer' or 'the tasks “A” and “B” wait for this answer' (three or more: “A”, “B” and “C”).
  5. Unit tests with invented data and a fixed now: each kind, the order, a question with about and two waiting tasks, a reclaimed task that is no longer given back, an empty list.
  6. Run the four checks of AGENTS.md.
- accept: src/report/page/needs.rs lists, in the order of R-REPORT-4: every open question with its date, the entry it is about and the tasks that wait for it; every task given back with the date and problem; every task whose claim is stale with owner and days; rows carry titles, not only IDs; the heading gives the count and an empty list reads Nothing needs you.; unit tests cover each kind and the empty case
- verify: cargo test --all-features --lib report::page
- verified: 2026-11-21
- commit: be145fb
- reqs: R-REPORT-4
- by: Reynan Henry Tandoyo
- note (2026-11-21, claude-main): added
- note (2026-11-21, claude-main): forced set (owner was t199-haiku)
- note (2026-11-21, t199-haiku): done — Added report::page::needs: build_needs lists open questions, tasks given back and stale claims in R-REPORT-4 order, with heading, waiting_text and EMPTY; 13 new unit tests, 963 tests passing in the full suite, fmt and clippy clean on Windows and Linux.

### T-200 · Last 7 days: what was finished, cancelled and added
- status: done
- owner: t200-haiku
- priority: P1 · difficulty: 2 · type: feature · area: report
- milestone: T-195
- why: a person who looked last week wants to see only what changed since then
- summary: The page lists what changed in the last week: what was finished, cancelled or added.
- final: The owner's workers are Haiku 5.5 at xhigh effort, which finished level-3 tasks in v0.8 (T-148, T-149); this is one coherent change with full detail, and smaller pieces would not compile on their own.
- depends: T-212
- files: src/report/page/mod.rs, src/report/page/recent.rs
- read: docs/requirements.md, docs/report-mockup.html, src/report/page/entry.rs, src/report/page/tree.rs
- steps:
  1. Read R-REPORT-5 in docs/requirements.md and the Last 7 days list of docs/report-mockup.html. Read src/report/page/entry.rs and tree.rs first.
  2. Create src/report/page/recent.rs and add `pub mod recent;` to src/report/page/mod.rs.
  3. Define `pub enum Event { MilestoneFinished { title, done: usize, total: usize }, Done { title }, Cancelled { title, reason: String }, Added { count: usize, milestone: Option<String>, title: String } }` and `pub struct Recent { rows: Vec<(NaiveDate, Event)>, earlier: Vec<(NaiveDate, Event)> }`.
  4. Write `pub fn build_recent(doc: &Document, archive: Option<&Document>, ctx: &Ctx) -> Recent`: the seven days are the local date of ctx.now and the six days before it. Take events from note dates of TODO.md, then the archive: a task's latest `done` note gives Done, its latest `dropped` note Cancelled (reason Task::dropped_reason or NO_REASON), a milestone's latest `done` note MilestoneFinished (done and total as the tree counts them, cancelled left out), and an `added` note of a task or milestone gives Added. Added on one date below the same milestone (the `milestone` field; none means Other work) become one row with the count; title is the single entry's title when count is 1.
  5. Order: newest date first; within a date MilestoneFinished, Done, Cancelled, Added, each in file order (TODO.md before the archive). The first 15 rows go in rows, the rest in earlier.
  6. Add `pub fn row_text(event: &Event) -> String` with the texts of R-REPORT-5: Done the title; Cancelled 'title — cancelled: reason'; MilestoneFinished 'title finished — all N tasks done' or '— N of M tasks done'; Added 'N tasks added to <milestone>' / 'N tasks added to Other work' / 'Added: <title>'. And const EMPTY = "Nothing changed in the last 7 days."
  7. Unit tests with a fixed now: each event, grouping of added rows, the 7-day edge (day 7 back is out), the order within a day, archive entries included, more than 15 rows split into rows and earlier, the empty case.
  8. Run the four checks of AGENTS.md.
- accept: src/report/page/recent.rs lists the events of the last 7 days from note dates of TODO.md and the archive, newest first: task done, task cancelled with its reason, milestone finished with its task count, entries added grouped per milestone and day; the order within a day is fixed; at most 15 rows, the rest in earlier; the empty text is Nothing changed in the last 7 days.; tests use a fixed clock
- verify: cargo test --all-features --lib report::page
- verified: 2026-11-21
- commit: 3296d64
- reqs: R-REPORT-5
- by: Reynan Henry Tandoyo
- note (2026-11-21, claude-main): added
- note (2026-11-21, t200-haiku): forced claim (files held by T-213)
- note (2026-11-21, t200-haiku): done — Added report::page::recent (build_recent, row_text, EMPTY) listing the seven days' done, cancelled, finished milestones and added entries, with 9 new unit tests; report::page 54 passing, full suite 1004 passing, fmt and clippy clean on Windows and Linux.
- note (2026-11-21, t200-haiku): singular texts (commit b418352)

### T-201 · description setting: one sentence about the project for the page header
- status: done
- owner: t201-haiku
- priority: P2 · difficulty: 1 · type: feature · area: config
- milestone: T-195
- why: a colleague who opens the page needs to know what the project is before reading its tasks
- summary: A project can have a one-sentence description that the page shows under its title.
- depends: T-196
- files: src/ops/config.rs, README.md, tests/config_toml.rs
- steps:
  1. Read the description row of the settings table in docs/requirements.md (R-CFG-2).
  2. In src/ops/config.rs add `pub description: Option<String>` to OpsConfig (doc comment naming R-CFG-2 and R-REPORT-1) and read the key `description` from todo.toml the same way `idea_section` is read; replace each CR or LF with a space and trim; empty becomes None. Add it to the list of known keys so it is not reported as unknown.
  3. In README.md add the row `| `description` | `""` | one sentence about the project, shown under the title of the HTML report |` to the settings table after the last row.
  4. In tests/config_toml.rs add tests: a set value, no key, a blank value, a value with a line break.
  5. Run the four checks of AGENTS.md.
- accept: `description = "..."` in todo.toml is read into OpsConfig as `description: Option<String>` (line breaks become spaces, an empty or blank value is None, a value that is not a string is the usual todo.toml error); without the key it is None; the README settings table lists the key with its meaning (one sentence shown under the title of the HTML report); tests in tests/config_toml.rs cover set, missing, blank and multi-line values; showing it on the page is part of T-214
- verify: cargo test --test config_toml
- verified: 2026-11-21
- commit: 78bf83a
- reqs: R-CFG-2, R-REPORT-1
- by: Reynan Henry Tandoyo
- note (2026-11-21, claude-main): added
- note (2026-11-21, claude-main): Review 2026-11-21: `report --json` only returns path, bytes and opened (docs/json.md), so the description is not added there; removed from accept.
- note (2026-11-21, t201-haiku): done — Added the description setting (R-CFG-2): read from todo.toml into OpsConfig, line breaks become spaces, blank is None, a non-string is refused; README row added; cargo test --test config_toml 8 passed (1 new), full cargo test --all-features 920 passed, 0 failed.
- note (2026-11-21, t201-haiku): CRLF in description gives one space (commit 1eabde8)

### T-202 · Page layout: header, overall bar, tree, one line per entry, details on click
- status: done
- owner: claude-main
- priority: P1 · type: milestone · area: report
- milestone: T-195
- goal: The new page itself: it looks and reads like the approved mockup
- why: this is the page itself; it must look and read like the approved mockup, not like a dump of fields
- summary: The page itself, built to look and read like the approved design.
- accept: `todo report --html` writes a page with the structure, wording and order of docs/report-mockup.html: header (title, description, date); the overall line with one bar in five parts and the legend; Needs you; Last 7 days; All work as one tree of milestones (title, state, "N of M done", small bar, goal) with nested milestones and tasks; unfinished milestones open, finished ones closed. An entry is one line: state mark, title, date for done and cancelled, small ID. A second line appears only for in progress (owner, since when, "no activity" when stale), waiting (the reason) and cancelled (the reason). A click opens the summary, for a done entry its result, and a closed part Technical details with level, priority, area, files, check and whether it passed, commit, who finished it, dependencies and the notes; an entry without a summary shows its why instead. Below the tree, closed: Ideas, Earlier work, and For the planner, which holds the v0.8 planning state and the per-area counts. The v0.8 sections Done and Dropped and the raw field lists are gone. One column, at most 820 px wide, system font, readable on a narrow window and when printed; all text HTML-escaped (test: a title containing `<script>`); the snapshot test uses a new invented fixture that has every state, a nested milestone, a question, a task given back and a stale claim; `report --html --all` still passes its tests
- reqs: R-REPORT-1, R-REPORT-3, TEST-1
- by: Reynan Henry Tandoyo
- note (2026-11-21, claude-main): added
- note (2026-11-21, claude-main): Owner 2026-11-21 (red-green colour blind): a state is never shown by colour alone. Every entry has its own mark shape (done ✓, in progress ●, waiting ◐, to do ○, cancelled ✕), the second line names the state in words, and the legend under the bar pairs each mark with its word and count. All colours come from CSS variables so T-203 can switch the set.
- note (2026-11-21, claude-main): split into T-213, T-214
- note (2026-11-21, claude-main): done — The new page is written by report --html and reads like the approved mockup (T-213, T-214).

### T-213 · Page HTML: the All work tree, one line per entry, details on click
- status: done
- owner: t213-haiku
- priority: P1 · difficulty: 2 · type: feature · area: report
- milestone: T-202
- why: the tree is most of the page and must read like the approved mockup, not like a dump of fields
- summary: Draws the tree of all work: one line per entry, with details when you click it.
- final: The owner's workers are Haiku 5.5 at xhigh effort, which finished level-3 tasks in v0.8 (T-148, T-149); this is one coherent change with full detail, and smaller pieces would not compile on their own.
- depends: T-212
- files: src/report/page/mod.rs, src/report/page/tree_html.rs
- read: docs/requirements.md, docs/report-mockup.html, src/report/page/entry.rs, src/report/page/tree.rs, src/report/html.rs
- steps:
  1. Read R-REPORT-1 in docs/requirements.md (All work, An entry, Details) and the markup of the tree in docs/report-mockup.html (div id tree, details.ms, summary with ms-title, ms-state, ms-count and bar, div.ms-body, p.goal, ul.items, li.item with the state class, details, summary with span.mark, span.t, span.when, span.id, div.more, p.line2, details.tech with a dl). Read src/report/page/entry.rs and tree.rs.
  2. Create src/report/page/tree_html.rs and add `pub mod tree_html;` to src/report/page/mod.rs. Escape every text from the file with crate::report::html::escape.
  3. Write `pub fn date_short(date: NaiveDate, today: NaiveDate) -> String`: '21 Nov' in the year of today, else '21 Nov 2025' (English month abbreviations).
  4. Write `pub fn push_entry(h: &mut String, e: &EntryView, today: NaiveDate)`: li class 'item <state class>'; inside a details whose summary has the mark, the title, the date for done and cancelled (span.when) and the ID (span.id); div.more with p 'Summary' and the summary, or 'Why' and the why when there is no summary; for done 'Result' and the result; then details.tech 'Technical details' with a dl of the fields that are present (Level, Priority, Type, Area, Files, Check with ' — passed <date>' when verified, Commit, Done by, Depends as 'T-001 Title', Attempts) and the notes as a list. After the details, the second line p.line2 when the entry has one: '<b>In progress</b> · owner · since today / since yesterday / for N days' plus ' · <span class="stale">no activity</span>' when stale; '<b>Waiting</b> · given back by a worker: …' / '· for your answer: …' / '· <reason>'; '<b>Cancelled</b> · <reason>'.
  5. Write `pub fn push_milestone(h: &mut String, m: &MilestoneView, today: NaiveDate)`: details.ms, with the attribute open unless it is Finished or Cancelled; summary with ms-title, ms-state (class 'ms-state st-prog' for in progress, 'st-none' not started, 'st-done' 'finished 19 Nov', 'st-drop' 'cancelled 19 Nov'), ms-count 'N of M done' and span.bar with an i of width N/M percent (0 when M is 0); div.ms-body with p.goal 'Goal: …' when there is a goal, then the nested milestones, then ul.items with the tasks.
  6. Write `pub fn push_tree(h: &mut String, tree: &Tree, today: NaiveDate)`: div id tree with every milestone, then the Other work group as a milestone row with ms-title 'Other work', ms-state 'not part of a milestone', its count and bar, open, no goal; left out when empty. Use no inline style attribute for colours: colours come from CSS classes (the stylesheet is written in T-214), widths of bars may use style width.
  7. Unit tests: an entry of each state gives its mark, class and second line; done shows date and Result; an entry without summary shows Why; technical details leave out missing fields; a finished milestone is closed and an unfinished one open; N of M and bar width; Other work appears only when there are top-level tasks; a title with <script> is escaped.
  8. Run the four checks of AGENTS.md.
- accept: src/report/page/tree_html.rs writes the All work tree in the markup of docs/report-mockup.html: milestones with title, state, N of M done, bar and goal, open unless finished or cancelled; one line per entry with mark, title, date for done and cancelled, and ID; the second line only for in progress, waiting and cancelled, naming the state in words; details with Summary or Why, Result for done, and closed Technical details; Other work last; all text escaped; unit tests cover each rule
- verify: cargo test --all-features --lib report::page
- verified: 2026-11-21
- commit: fa9f98c
- by: Reynan Henry Tandoyo
- note (2026-11-21, claude-main): added
- note (2026-11-21, t213-haiku): done — Added report::page::tree_html: the All work tree as HTML (milestones with state, N of M, bar and goal; one line per entry with mark, title, date and ID; details with Summary or Why, Result, Technical details; second lines; Other work last; all text escaped), with 13 new unit tests; full suite 1008 passing, fmt and clippy clean on Windows and Linux.

### T-214 · Page HTML: the whole page, wired into report --html, with snapshot
- status: done
- owner: t214-haiku
- priority: P1 · difficulty: 3 · type: feature · area: report
- milestone: T-202
- why: this puts the parts together into the page people open, replacing the v0.8 page for one project
- summary: Puts all parts together into the page that todo writes for one project.
- final: The owner's workers are Haiku 5.5 at xhigh effort, which finished level-3 tasks in v0.8 (T-148, T-149); this is one coherent change with full detail, and smaller pieces would not compile on their own.
- depends: T-213, T-199, T-200, T-201, T-210, T-209
- files: src/report/page/mod.rs, src/report/page/html.rs, src/report/page/style.css, src/report/html.rs, src/cli/report.rs, tests/cmd_report.rs, tests/fixtures/report_page.md, tests/snapshots/cmd_report__report_html.snap, tests/fixtures/report_page_archive.md, tests/cmd_report_planning.rs, tests/cmd_blocked_reason.rs
- read: docs/requirements.md, docs/report-mockup.html, src/report/page/entry.rs, src/report/page/tree.rs, src/report/page/needs.rs, src/report/page/recent.rs, src/report/page/tree_html.rs, src/report/model.rs, src/report/all.rs
- steps:
  1. Read all of section 5.23 of docs/requirements.md and docs/report-mockup.html. Read the files in src/report/page/ and how src/cli/report.rs builds the single-project page today (build_page and render_page).
  2. In src/report/page/mod.rs add `pub struct PageData` (title, description, generated text, overall Counts, needs, recent, tree, earlier, ideas: Vec<String> of open idea titles, and the old crate::report::model::Page for the planner part) and `pub fn build(doc, archive, cfg, now) -> PageData` that calls build_tree, build_earlier, build_needs, build_recent and crate::report::model::build_page (for the planning state and area counts only). Title as model.rs page_title does (make that function pub(crate) if needed), description from OpsConfig.description.
  3. Create src/report/page/style.css from the style block of docs/report-mockup.html: drop the mockflag rule and every var(--background,…) style host fallback (use the plain values), use the light colours of the table in R-REPORT-6 (waiting #b07800, to do #80868e), put the dark set under @media (prefers-color-scheme: dark) for .page:not([data-theme]) and under .page[data-theme=dark], keep both colour-blind sets, the print rule and the narrow-window rule; add classes for the milestone states (st-prog, st-none, st-done, st-drop) and .stale using the state colours, replacing the inline style colours of the mockup. Buttons are hidden unless the root has the class js (.page:not(.js) .controls{display:none}); the script that adds it is T-203.
  4. Create src/report/page/html.rs with `pub fn render(page: &PageData) -> String`: doctype, lang en, charset, viewport, title '<title> — status', the style inline, then div.page with: the header (h1, p.desc when description is set, p.asof 'Status as of <d Mon yyyy>, <hh:mm> · from TODO.md'); div.overall with p.big 'N of M tasks done', div.seg role img with an aria-label giving every count, five i parts with classes seg-done, seg-prog, seg-wait, seg-todo, seg-drop and widths as shares of all tasks, ul.legend with mark, count and word for each state and '(not counted in the M)' on cancelled; h2 'Needs you (n)' with div.box.attn and one div.need per row (span.tag Question / Given back / No activity, span.what title, p.muted text as R-REPORT-4) or p 'Nothing needs you.'; h2 'Last 7 days' with ul.recent (span.d date, span.mark, text) and a details 'Earlier this week (n)' for the rest, or p with the empty text; h2 'All work' with div.controls holding the buttons of the mockup (data-f all, left, done, drop; exp; col), then push_tree; h2 'More' with details.fold Ideas (n) — not planned yet, Earlier work (n finished milestones, m tasks) — from the archive (push_tree of the archive tree, every milestone closed), and For the planner — levels, parallel waves, tasks that still need detail, whose body is the planning part and the per-area counts written by the old src/report/html.rs functions push_planning and push_areas (make them pub(crate) there and call them; change nothing else in that file). Header buttons: button id theme 'Dark mode' and button id cb 'Colour-blind colours' aria-pressed false, inside div.controls. No script yet.
  5. In src/cli/report.rs make `report --html` without --all use page::build and page::html::render; --all stays as it is.
  6. Write the invented fixture tests/fixtures/report_page.md (first line `<!-- TEST FIXTURE: invented data, not a real task list -->`, format 3) with a milestone tree two levels deep, every state (to do, in progress, a stale claim, blocked, given back with a stuck note, done with result, dropped with reason), a question with about that a task depends on, an idea, top-level tasks, summaries on some entries and not others; plus a small archive file if the test helper supports it.
  7. Update tests/cmd_report.rs for the single-project page: the snapshot test uses the new fixture with the fixed clock (accept the new snapshot with cargo insta accept or by INSTA_UPDATE; review it); replace assertions that check the v0.8 single page with checks for: the five state words in the legend, Needs you rows, Last 7 days, a done entry still in its milestone, Other work, the closed parts under More, escaping of a <script> title, no external URL (no 'http' in src or href attributes). Keep every --all test (tests/cmd_report_all.rs) passing unchanged.
  8. Run the four checks of AGENTS.md.
- accept: `todo report --html` writes the page of R-REPORT-1 with the structure, wording and order of docs/report-mockup.html: header with description; overall line, five-part bar and legend; Needs you; Last 7 days; All work as one tree; More with Ideas, Earlier work and For the planner (the v0.8 planning state and per-area counts); the v0.8 sections Done and Dropped and the raw field lists are gone from the single-project page; one column at most 820 px, system font, readable narrow and printed; all text escaped; the snapshot uses the new invented fixture; report --html --all still passes its tests
- verify: cargo test --all-features --test cmd_report --test cmd_report_all
- verified: 2026-11-21
- commit: 05327fd
- by: Reynan Henry Tandoyo
- note (2026-11-21, claude-main): added
- note (2026-11-21, claude-main): forced set (owner was t214-haiku)
- note (2026-11-21, t214-haiku): done — report --html writes the whole v1.0 page for one project (header with description, overall bar and legend, Needs you, Last 7 days, All work as one tree, More with Ideas, Earlier work and the planner part) from an invented fixture with its archive, snapshot included; the v0.8 page tests now check the new page with their count and reason assertions kept; 1026 tests passing, fmt and clippy clean on Windows and Linux.

### T-203 · View buttons, light or dark mode, and colour-blind colours
- status: done
- owner: t203-haiku
- priority: P1 · difficulty: 2 · type: feature · area: report
- milestone: T-195
- why: the owner wants to narrow the tree to what is left or what was done, and to switch between light and dark, without anything on the page being able to change TODO.md
- summary: Buttons narrow the page to what is left, done or cancelled, and switch between light and dark and to colours for red-green colour blindness.
- final: The owner's workers are Haiku 5.5 at xhigh effort, which finished level-3 tasks in v0.8 (T-148, T-149); this is one coherent change with full detail, and smaller pieces would not compile on their own.
- depends: T-202
- files: src/report/page/html.rs, src/report/page/style.css, src/report/page/script.js, tests/cmd_report.rs, tests/report_colours.rs, tests/snapshots/cmd_report__report_html.snap
- read: docs/requirements.md, docs/report-mockup.html, src/report/page/tree_html.rs
- steps:
  1. Read R-REPORT-6 in docs/requirements.md and the script at the end of docs/report-mockup.html. Read src/report/page/html.rs and style.css.
  2. Create src/report/page/script.js from the mockup's script, include it with include_str! in html.rs and write it inline in one <script> element at the end of the page. First thing the script does: add the class js to div.page, so the buttons (hidden by CSS without it) appear. Keep: theme button (follows prefers-color-scheme until used, label names the mode it switches to, choice stored under todo-report-theme), colour-blind button (aria-pressed, stored under todo-report-cb), every localStorage access in try/catch, the filter buttons Everything / Still to do / Done / Cancelled (hide non-matching li.item and every details.ms left without a visible entry, open the ones with one, aria-pressed on the pressed button), Expand all and Collapse all on the tree only. Remove the mockup's body background hack if the page background is already set by CSS on body.
  3. Check style.css has: the four colour sets of the R-REPORT-6 table as CSS variables on .page (light default, dark under prefers-color-scheme for .page:not([data-theme]) and under [data-theme=dark], colour-blind under [data-cb=on] and [data-theme=dark][data-cb=on]), the bar part colours and the striped waiting part of the colour-blind sets, the underlined state words of waiting and cancelled second lines in the colour-blind sets, and @media print hiding .controls. Make the colours exactly the values of the table.
  4. Write tests/report_colours.rs: read src/report/page/style.css as text (include_str! from the test with a relative path, or std::fs from CARGO_MANIFEST_DIR), find the five state colours of each of the four sets, and assert each has a WCAG contrast ratio of at least 3.0 against its background (#ffffff light, #17181b dark); compute relative luminance in the test.
  5. In tests/cmd_report.rs add checks: the page has exactly one script element; the script contains no 'http', no '//' URL, no fetch, no XMLHttpRequest, no import; the buttons Everything, Still to do, Done, Cancelled, Expand all, Collapse all, Dark mode and Colour-blind colours are present; the print rule hides .controls. Update the snapshot.
  6. Run the four checks of AGENTS.md.
- accept: the page has the buttons of R-REPORT-6 (Everything, Still to do, Done, Cancelled, Expand all, Collapse all, light or dark mode, Colour-blind colours) driven by one small inline script with no network access; four colour sets with exactly the colours of the R-REPORT-6 table, each state colour at least 3 to 1 against its background, computed by a test; the mode follows the system until the button is used and both choices are remembered (failures ignored); without scripts every entry shows in the system mode with the normal colours and the buttons are hidden; the buttons are left out when printing; nothing on the page writes a file or runs a command; the snapshot covers the markup and a test checks that the script names no address
- verify: cargo test --all-features --test cmd_report --test report_colours
- verified: 2026-11-21
- commit: 7b3dcee
- reqs: R-REPORT-6, NFR-4
- by: Reynan Henry Tandoyo
- note (2026-11-21, claude-main): added
- note (2026-11-21, claude-main): Owner 2026-11-21: the owner is red-green colour blind. A button switches the colour scheme only; light and dark mode each need their own colours that fit a white or a black background, in the normal and in the colour-blind scheme.
- note (2026-11-21, claude-main): Planner 2026-11-21: T-214 is merged and looks like the mockup in a browser. While you are in style.css: the date of done and cancelled entries (span.when) looked blue in dark mode in one browser; make sure it uses the muted text colour in all four sets.
- note (2026-11-21, t203-haiku): done — Added the view buttons, light or dark mode and colour-blind colours to the page, driven by one inline script that makes no network call; the four colour sets match the R-REPORT-6 table with 3 to 1 contrast (tests/report_colours.rs); cargo test --all-features passes 1036 tests.

### T-204 · Performance and size of the new page
- status: done
- owner: t204-haiku
- priority: P1 · difficulty: 1 · type: test · area: report
- milestone: T-195
- why: the new page keeps every finished entry in the tree, so it must be measured again on a large list
- summary: The new page is measured on a very large task list to make sure it stays fast and small.
- depends: T-203
- files: tests/perf.rs
- read: docs/requirements.md
- steps:
  1. Read NFR-3 in docs/requirements.md and tests/perf.rs, in particular the existing report --html timing test on the milestone-tree file.
  2. Make sure that test runs `report --html` (single project, the new page) on the generated 5,000-entry milestone-tree file and asserts both limits: under 500 ms (median of the runs the file already uses) and a page under 5 MB (5 * 1024 * 1024 bytes, read the size of the written file). Give the generated tasks a summary and a done note on some of them if that makes the page more like real use, without changing the other perf tests' files.
  3. Run `cargo test --release --test perf -- --nocapture` three times and write down the measured time and size.
  4. If the page is over a limit, do not raise the limit: report the numbers with `todo stuck`.
  5. Run the four checks of AGENTS.md, and put the numbers in the done note.
- accept: on the generated 5,000-entry milestone-tree file `report --html` finishes in under 500 ms in a release build and the page stays under 5 MB; the existing report performance test asserts both and runs in CI; the measured numbers are recorded in the done note
- verify: cargo test --release --all-features --test perf
- verified: 2026-11-21
- commit: b4c08c4
- reqs: NFR-3, TEST-9
- by: Reynan Henry Tandoyo
- note (2026-11-21, claude-main): added
- note (2026-11-21, claude-main): forced set (owner was t204-haiku)
- note (2026-11-21, t204-haiku): done — The one-page report perf test now runs on the 5,000-entry milestone tree with a summary on every task and a done note on every done task: report --html took 110, 111 and 122 ms (median of five runs per run; limit 500 ms) and the page was 2,897,256 bytes in each run (limit 5 MiB); the four AGENTS.md checks pass and cargo test --all-features passes 1036 tests (the 5 perf tests run with --ignored).

### T-205 · Planner rules for summaries, documentation and release 1.0.0
- status: done
- owner: claude-main
- priority: P1 · difficulty: 2 · type: docs · area: release
- milestone: T-195
- why: the page is only as readable as the texts the planner writes, and the release makes the new page the one people get
- summary: Version 1.0.0 is released with the new page as the page people get.
- final: Docs and release work done by the planner and one worker at the end; it depends on the owner's answer to T-206.
- depends: T-204,T-206
- accept: CHANGELOG.md has the 1.0.0 entry; Cargo.toml is 1.0.0; this repository's own unfinished and v1.0 entries have summaries; a new todo.toml has a description (and nothing else); todo check is clean; T-206 is answered
- commit: a8e491e
- reqs: R-AGENT-1, REL-2, REL-5
- by: Reynan Henry Tandoyo
- note (2026-11-21, claude-main): added
- note (2026-11-21, claude-main): Review 2026-11-21: depends on the question T-206 so 1.0.0 is not released before the owner has judged the page; this repository has no todo.toml yet.
- note (2026-11-21, claude-main): Planner: add todo.toml with description only after todo 1.0 is installed here; todo 0.8.1 refuses the unknown key.
- note (2026-11-21, claude-main): done — 1.0.0 released on dev: version and changelog (b7debb1), newer-stamp tests moved above 1.0 (e62e64e), todo.toml with the description, summaries on the v1.0 entries; todo 1.0.0 installed here.

### T-215 · apply refuses a line break in why, goal, read or verify instead of turning it into a space
- status: done
- owner: t215-haiku
- priority: P2 · difficulty: 2 · type: bug · area: cli
- milestone: T-190
- why: add and set turn line breaks in a value into single spaces (format.md section 10), but apply only trims these values and then refuses the write, so the same input works with add and fails with apply
- summary: Adding tasks in a batch now accepts text with line breaks, as adding them one by one already did.
- final: One small fix in one file plus its test; Haiku 5.5 at xhigh finishes it from these steps.
- files: src/cli/apply.rs, tests/cmd_apply_line_breaks.rs
- read: docs/format.md, src/cli/add.rs
- steps:
  1. Read the rule 'Values are single lines; line breaks in input become single spaces' in docs/format.md section 10, and how src/cli/add.rs uses sanitize_trimmed_value for every value except steps.
  2. In src/cli/apply.rs, in the add operation (near the comment about `why` may also come from `fields`, around line 547), the values goal, why, read and verify use .map(str::trim). Change each to the pattern summary_src already uses: .map(sanitize_trimmed_value).filter(|s| !s.is_empty()), then pass .as_deref() where a &str is needed. Do the same for every other free-text value of the add and set operations that is only trimmed (the helper fn trim near line 747 and its callers), except steps, which keeps its line breaks.
  3. Write tests/cmd_apply_line_breaks.rs (invented data in a temporary folder, helpers from tests/common): apply add with why, goal (on a milestone), read, verify and accept containing a line break writes each as one line with a single space; apply set with a why containing a CRLF does the same; steps given as text with line breaks still become separate steps.
  4. Run the four checks of AGENTS.md.
- accept: every free-text value of an apply add or set operation except steps turns line breaks into single spaces, as add and set do; nothing that worked before changes; tests in tests/cmd_apply_line_breaks.rs cover why, goal, read, verify, accept, a CRLF, and steps
- verify: cargo test --all-features --test cmd_apply_line_breaks
- verified: 2026-11-21
- commit: fb0c8b4
- by: Reynan Henry Tandoyo
- note (2026-11-21, claude-main): added
- note (2026-11-21, t215-haiku): done — apply add now turns line breaks in title, goal, why, read, verify, files, type, area, problem, fix, accept and extra fields into single spaces, as add does; 4 new tests in tests/cmd_apply_line_breaks.rs and 973 tests pass in the full run.

### T-216 · Needs you and the tree count the days of one claim the same way
- status: done
- owner: t216-haiku
- priority: P1 · difficulty: 1 · type: bug · area: report
- milestone: T-195
- why: for the same stale claim Needs you says 13 days and the entry's second line says 14, because one counts elapsed time and the other calendar days; a reader sees two numbers for one fact
- summary: The page shows the same number of days for a claim everywhere.
- files: src/report/page/entry.rs, src/report/page/needs.rs
- read: docs/requirements.md
- steps:
  1. In src/report/page/entry.rs, find fn since (it counts calendar days between the claim's local date and the date of ctx.now). Split the day count into a new function `pub(crate) fn claim_days(task: &Task, ctx: &Ctx<'_>) -> Option<i64>` that returns that number (None without a valid claim time), and make since() use it.
  2. In src/report/page/needs.rs, in fn no_activity_need, replace `ctx.cfg.claim_age(task, ctx.now).map(|age| age.num_days())` with `crate::report::page::entry::claim_days(task, ctx)`.
  3. Add a unit test in needs.rs: a claim made at 23:00 on one day, with now 13 days and 2 hours later (so 14 calendar days), gives days Some(14) in Needs you, the same as Since::Days(14) from entry_view for the same task.
  4. Run the four checks of AGENTS.md.
- accept: Needs you and the entry's second line give the same number of days for one claim, counted in calendar days as entry.rs does; a unit test covers a claim whose elapsed days and calendar days differ
- verify: cargo test --all-features --lib report::page
- verified: 2026-11-21
- commit: 9eadbcc
- by: Reynan Henry Tandoyo
- note (2026-11-21, claude-main): added
- note (2026-11-21, t216-haiku): done — Needs you now counts the days of a claim with the same calendar-day count as the entry's second line (new claim_days in entry.rs, used by since and by needs.rs); a unit test covers a claim 13 days 2 hours old that is 14 calendar days, and the suite passes with 1020 tests.

### T-217 · Help texts and README describe the new page and how to write for it
- status: done
- owner: t217-haiku
- priority: P1 · difficulty: 2 · type: docs · area: docs
- milestone: T-195
- why: the page is only as readable as the texts the planner writes, and people and agents learn about the page from the README and todo help
- summary: The help texts and the README explain the new page and how to write texts for it.
- by: Reynan Henry Tandoyo
- final: Docs across four files that must say the same thing; Haiku 5.5 at xhigh finishes it from these steps.
- files: src/agents/planner.md, src/cli/report.rs, README.md, docs/json.md
- read: docs/requirements.md, docs/report-mockup.html, src/report/page/html.rs
- steps:
  1. Read section 5.23 and R-WRITE-19 in docs/requirements.md.
  2. src/agents/planner.md (printed by todo help planner): add a section 'Writing for people' after 'Tasks and milestones' with these rules, each one line or two: write a summary for every task and milestone with --summary (what it is about, in plain words, at most two sentences, no IDs and no file names; the HTML report shows it); write the done note as the result in one sentence a person understands; when dropping a task or giving it back, give a reason a person understands; todo check warns about unfinished work without a summary. Keep the existing text.
  3. src/cli/report.rs: the META summary becomes 'Write one HTML page for people: what is done, cancelled, still to do, and what needs you'. In details, describe the page of R-REPORT-1 and R-REPORT-4 to R-REPORT-6 as the code now writes it, including the view buttons, light or dark mode and the colour-blind colours driven by one small inline script with no network access, that nothing on the page changes TODO.md, and that --all keeps the v0.8 page with no script; remove every sentence that says the page has no script or no buttons for the single-project page.
  4. README.md: in the part for people near the top (around the numbered steps that mention report --html --all) and in the section 'HTML report for people', describe the new page for one project: the overall line and bar, Needs you, Last 7 days, All work with done and cancelled entries in place, a click opens the summary and Technical details, More with Ideas, Earlier work and For the planner, the buttons Everything / Still to do / Done / Cancelled, Expand and Collapse all, light or dark mode and Colour-blind colours; that the page works without scripts; and that --all keeps its earlier form. Keep the PowerShell examples.
  5. docs/json.md: in the section on todo report --html --json, the paragraph that starts 'The page shows the planning state' now says it sits in the closed part For the planner.
  6. Run cargo test --all-features (help and schema snapshots may need updating with cargo insta; review every changed snapshot) and the four checks of AGENTS.md.
- accept: todo help planner tells the planner to write a summary for every task and milestone (plain words, at most two sentences, no IDs or file names), the done note as a one-sentence result, and reasons a person understands for drop and stuck; todo help report and its summary describe the new page with its buttons and script, and no longer say the single-project page has no script; the README describes the new page and its buttons for people; docs/json.md places the planning state under For the planner; all tests pass
- verify: cargo test --all-features
- verified: 2026-11-21
- commit: 9c58f0e
- note (2026-11-21, claude-main): added
- note (2026-11-21, claude-main): forced set (owner was t217-haiku)
- note (2026-11-21, t217-haiku): forced done outside files: tests/snapshots/cli_basic__cli_overview_json.snap
- note (2026-11-21, t217-haiku): done — Planner help has a Writing for people section, the report help and README describe the one-project page with its buttons, script and no-script fallback, and --all keeps its v0.8 form; cargo test passes with 1036 tests (fmt, clippy and Linux clippy clean). tests/snapshots/cli_basic__cli_overview_json.snap was changed outside files for the new report summary.

### T-218 · Last 7 days shows an entry's summary instead of its title
- status: done
- owner: t218-haiku
- priority: P1 · difficulty: 2 · type: feature · area: report
- milestone: T-195
- why: the owner reviewed the page on this project (T-206): titles are written by agents for agents, so a week's list of them is hard to read for a person
- summary: The list of what changed this week reads in plain words, using the text written for people where there is one.
- by: Reynan Henry Tandoyo
- final: One small change in one module with its requirement and snapshot; Haiku 5.5 at xhigh finishes it from these steps.
- files: src/report/page/recent.rs, docs/requirements.md, tests/snapshots/cmd_report__report_html.snap, tests/cmd_report.rs
- read: src/report/page/entry.rs, src/report/page/html.rs
- steps:
  1. In docs/requirements.md, R-REPORT-5 (section 5.23), first bullet: add the sentence 'A task done, a task cancelled and a single entry added are named by their summary (R-WRITE-19) when they have one, else by their title; a milestone finished, and the milestone in N tasks added to <milestone>, are named by their title.'
  2. In src/report/page/recent.rs, where the Done, Cancelled and Added events take the entry's title, take the summary when the task has one (task.summary, trimmed and not empty), else the title. MilestoneFinished and the milestone name of a grouped Added row keep the title. Update the doc comments of Event to say so.
  3. Add unit tests in recent.rs: a done task with a summary shows the summary; one without shows the title; a cancelled task with a summary shows the summary and the reason; a single added entry with a summary shows 'Added: <summary>'; a finished milestone with a summary still shows its title.
  4. Run cargo test --all-features; if the report snapshot changes, review the diff (only Last 7 days rows may change) and accept it with cargo insta accept.
  5. Run the four checks of AGENTS.md.
- accept: Last 7 days names a task done, a task cancelled and a single added entry by its summary when it has one, else by its title; finished milestones and grouped milestone names keep their titles; R-REPORT-5 says so; unit tests cover each case; all tests pass
- verify: cargo test --all-features --lib report::page
- verified: 2026-11-21
- commit: aed00db
- note (2026-11-21, claude-main): added
- note (2026-11-21, t218-haiku): done — Last 7 days names a task done, a task cancelled and a single added entry by its summary when it has one, else by its title; R-REPORT-5 says so; 5 new unit tests and the report snapshot updated; 1041 tests pass across 105 test binaries.

## v1.1: the status page, polished

Small fixes the owner found on this project's own page, the archive shown in the same tree, and
milestones that stand out from tasks.

### T-220 · The status page, polished
- status: open
- owner: —
- priority: P2 · type: milestone
- goal: The page reads clearly on a finished project too: no blank views, one heading style, milestones that stand out, and the archive shown in the same tree on request
- why: the owner looked at this project's own page after the 1.0.0 release (2026-11-22) and found these points
- summary: Small improvements to the status page that the owner found on this project's own page after the 1.0 release.
- note (2026-11-22, claude-main): added

### T-221 · Requirements for the v1.1 page changes
- status: done
- owner: claude-main
- priority: P1 · difficulty: 3 · type: docs · area: docs
- milestone: T-220
- why: the five changes touch R-REPORT-1 and R-REPORT-6; the requirements must say exactly what to build before workers start
- summary: Writes down exactly how the five page changes should look and behave before anyone builds them.
- accept: docs/requirements.md describes: the text a filter shows when nothing matches; the planner part's inner headings as subheadings of For the planner; text between backticks in titles, summaries and ideas shown as code (still escaped); the milestone marker and ID on every milestone row; and the archive toggle (button, what it shows, the overall line with the archive, the page without scripts); section 11 lists v1.1; docs/report-mockup.html shows the milestone marker and the toggle
- commit: 3b2c166
- by: Reynan Henry Tandoyo
- note (2026-11-22, claude-main): added
- note (2026-11-22, claude-main): done — Requirements and mockup for v1.1: milestone marker, planner subheadings, backticks as code, empty-filter line, Include archive toggle, and R-REPORT-7 Show chain.

### T-222 · A filter that matches nothing says so instead of leaving the tree blank
- status: done
- owner: claude-main
- priority: P2 · type: milestone · area: report
- milestone: T-220
- goal: A view button that matches nothing says so in words
- why: on a finished project Still to do and Cancelled leave All work completely empty, which looks broken
- summary: When a view button matches nothing, the page says so instead of showing an empty space.
- depends: T-221
- accept: when Still to do, Done or Cancelled leaves no visible entry, All work shows one line in words (Nothing still to do. / Nothing done yet. / Nothing cancelled.) and hides it again for a filter that matches; a test covers the markup and the script names the texts
- by: Reynan Henry Tandoyo
- note (2026-11-22, claude-main): added
- note (2026-11-22, claude-main): split into T-230, T-231
- note (2026-11-26, claude-main): done — T-230 and T-231 done: an empty view says so in words

### T-230 · All work holds a hidden line for a view that matches nothing
- status: done
- owner: t230-haiku
- priority: P2 · difficulty: 1 · type: feature · area: report
- milestone: T-222
- why: the script needs a place in the page to write "Nothing still to do." and the like
- summary: Prepares the line that tells the reader when a view button finds nothing.
- files: src/report/page/html.rs, tests/cmd_report.rs, tests/snapshots/cmd_report__report_html.snap
- read: docs/requirements.md
- steps:
  1. Read the bullet of R-REPORT-6 in docs/requirements.md that starts `From v1.1, when a view button leaves no visible entry`.
  2. In src/report/page/html.rs, in fn push_all_work, directly after the call `push_tree(h, &page.tree, page.today);`, add `h.push_str("<p id=\"empty-view\" class=\"muted\" hidden></p>\n");`.
  3. In tests/cmd_report.rs add a test `all_work_has_a_hidden_line_for_an_empty_view`: let tmp = TempDir::new().unwrap(); let (_work, html) = fixture_report(&tmp); assert that section(&html, "All work") contains `<p id="empty-view" class="muted" hidden></p>`.
  4. Update the page snapshot: in PowerShell run `$env:INSTA_UPDATE='always'; cargo test --all-features --test cmd_report; Remove-Item Env:INSTA_UPDATE`, then read `git diff tests/snapshots/cmd_report__report_html.snap` and check that only lines this task means to change are different.
  5. Run the four checks of AGENTS.md.
- accept: All work ends with `<p id="empty-view" class="muted" hidden></p>` after the tree; the new test passes; the snapshot is updated
- verify: cargo test --all-features --test cmd_report
- verified: 2026-11-26
- commit: e04ca24
- by: Reynan Henry Tandoyo
- note (2026-11-22, claude-main): added
- note (2026-11-26, t230-haiku): forced claim (files held by T-229)
- note (2026-11-26, t230-haiku): done — Added the hidden empty-view line after the All work tree in src/report/page/html.rs with a test and updated snapshot; full suite passes 1042 tests (1041 before plus the new one), fmt and both clippy targets clean.

### T-231 · The script writes the line for a view that matches nothing
- status: done
- owner: t231-haiku
- priority: P2 · difficulty: 1 · type: feature · area: report
- milestone: T-222
- why: on a finished project Still to do and Cancelled leave All work blank, which looks broken
- summary: When a view button finds nothing, the page now says so in words instead of showing an empty space.
- depends: T-230
- files: src/report/page/script.js, tests/cmd_report.rs, tests/snapshots/cmd_report__report_html.snap
- read: docs/requirements.md
- steps:
  1. Read the same bullet of R-REPORT-6 in docs/requirements.md, and fn applyFilter in src/report/page/script.js.
  2. In src/report/page/script.js, after the line that defines `filterButtons`, add `var emptyView = document.getElementById('empty-view');` and `var EMPTY_TEXT = { left: 'Nothing still to do.', done: 'Nothing done yet.', drop: 'Nothing cancelled.' };`.
  3. At the end of function applyFilter (after the loop that sets aria-pressed), add: `if (emptyView) { var none = filter !== 'all' && !tree.querySelector('li.item:not(.hide)'); emptyView.textContent = none ? EMPTY_TEXT[filter] : ''; emptyView.hidden = !none; }`.
  4. In tests/cmd_report.rs add a test `the_script_names_the_texts_of_an_empty_view`: build the page with fixture_report, take the text between the last `<script>` and `</script>`, and assert it contains `empty-view`, `Nothing still to do.`, `Nothing done yet.` and `Nothing cancelled.`.
  5. Run the four checks of AGENTS.md.
- accept: the script shows Nothing still to do. / Nothing done yet. / Nothing cancelled. in the empty-view line when the pressed filter leaves no visible entry, and hides the line otherwise; the new test passes
- verify: cargo test --all-features --test cmd_report
- verified: 2026-11-26
- commit: e337572
- by: Reynan Henry Tandoyo
- note (2026-11-22, claude-main): added
- note (2026-11-26, t231-haiku): Committed as e337572 on t231 (not pushed). todo done was refused by the files guard: tests/snapshots/cmd_report__report_html.snap changed but is not in the files line. The snapshot must change because the page embeds the script; it was updated by the insta candidate, minus its assertion_line metadata line, and CRLF kept; the diff is only the 8 script lines. Checks: fmt ok; clippy Windows and Linux ok; cargo test --all-features 1060 passed, 0 failed (cmd_report 25 of 25). Not forced. Needs a decision: add the snapshot to files, or finish with --force.
- note (2026-11-26, claude-main): forced set (owner was t231-haiku)
- note (2026-11-26, t231-haiku): done — The script names the empty view in words (Nothing still to do. / Nothing done yet. / Nothing cancelled.) and hides the line otherwise; the HTML snapshot changed with the script and is now listed in files; full suite passes 1060 tests, 0 failed, with fmt and both clippy targets clean.

### T-223 · For the planner uses subheadings, not main section headings
- status: done
- owner: t223-haiku
- priority: P2 · difficulty: 1 · type: bug · area: report
- milestone: T-220
- why: inside the closed part For the planner the old v0.8 code writes Planning and Unfinished tasks per area as main headings, which look like new sections of the page
- summary: The headings inside the planner part look like part of it, not like new sections of the page.
- depends: T-221,T-229
- files: src/report/page/html.rs, tests/cmd_report_planning.rs, tests/snapshots/cmd_report__report_html.snap
- read: docs/requirements.md
- steps:
  1. Read the sentence of R-REPORT-1 in docs/requirements.md that starts `From v1.1 its headings are subheadings of this part`.
  2. In src/report/page/html.rs, fn push_planner: instead of writing push_planning and push_areas straight into h, write them into a new String `part`, then lower its headings by one level before adding it to h: replace `<h3>` with `<h4>` and `</h3>` with `</h4>` first, then `<h2>` with `<h3>` and `</h2>` with `</h3>`. Add a comment that escaped text never holds these tags, as fn push_earlier says. Do not change src/report/html.rs, so the all-projects page keeps its headings.
  3. In tests/cmd_report_planning.rs, change the assertions that look for the planner's headings: `<h3>Open tasks per level</h3>` becomes `<h4>Open tasks per level</h4>`, and any `<h2>Planning` or `<h3>` heading of the planner part one level lower in the same way. Add one assertion that the planner part holds `<h3>Planning</h3>` and no `<h2>`.
  4. Update the page snapshot: in PowerShell run `$env:INSTA_UPDATE='always'; cargo test --all-features --test cmd_report; Remove-Item Env:INSTA_UPDATE`, then read `git diff tests/snapshots/cmd_report__report_html.snap` and check that only lines this task means to change are different.
  5. Run the four checks of AGENTS.md.
- accept: inside For the planner the headings are h3 (Planning, Unfinished tasks per area) and h4 below them; the page's own section headings stay h2; src/report/html.rs and the all-projects page are unchanged; the planning tests check the new levels; the snapshot is updated
- verify: cargo test --all-features --test cmd_report --test cmd_report_planning
- verified: 2026-11-26
- commit: d1ce065
- by: Reynan Henry Tandoyo
- note (2026-11-22, claude-main): added
- note (2026-11-26, t223-haiku): forced claim (files held by T-233)
- note (2026-11-26, t223-haiku): done — The planner part of the page now uses h3 for Planning and Unfinished tasks per area and h4 below them, with the all-projects page unchanged; all 1059 tests pass, and fmt and both clippy checks pass.

### T-224 · Text between backticks shows as code on the page
- status: done
- owner: claude-main
- priority: P2 · type: milestone · area: report
- milestone: T-220
- goal: Text between backticks shows as code everywhere on the page
- why: titles, summaries and ideas often name commands or fields in backticks, and the page shows the backticks as typed
- summary: Commands and field names written between backticks look like code on the page instead of showing the backticks.
- depends: T-221
- accept: in titles, summaries, results, reasons and ideas on the one-project page, a pair of backticks around text becomes a code element with the text inside it, still HTML-escaped; an unpaired backtick stays as typed; tests cover a pair, two pairs, an unpaired backtick and a <script> inside backticks
- by: Reynan Henry Tandoyo
- note (2026-11-22, claude-main): added
- note (2026-11-22, claude-main): split into T-232, T-233, T-234
- note (2026-11-27, claude-main): done — T-232, T-233, T-234 done: text between backticks shows as code

### T-232 · A text helper that shows text between backticks as code
- status: done
- owner: t232-haiku
- priority: P2 · difficulty: 1 · type: feature · area: report
- milestone: T-224
- why: titles, summaries and ideas often name commands in backticks; the page needs one function that escapes text and turns those pairs into code
- summary: Adds the small function that turns text written between backticks into code on the page.
- files: src/report/page/text.rs, src/report/page/mod.rs
- read: docs/requirements.md
- steps:
  1. Read the bullet of R-REPORT-1 in docs/requirements.md that starts `From v1.1 text between a pair of backticks`.
  2. Create src/report/page/text.rs with a module doc comment (`//! Text from the task list as HTML (T-224; R-REPORT-1).`) and `pub fn inline_code(text: &str) -> String`. Use crate::report::html::escape for every piece of text. Walk the text: find a backtick; find the next backtick after it; if both exist and the text between them is not empty, write escape(text before), then `<code>`, escape(text between), `</code>`, and continue after the second backtick; otherwise write the rest escaped as it is (an unpaired or empty pair of backticks stays as typed, escaped).
  3. Add `pub mod text;` to src/report/page/mod.rs next to the other `pub mod` lines.
  4. In text.rs add #[cfg(test)] mod tests with: plain text is only escaped; `a `b` c` gives `a <code>b</code> c`; two pairs give two code elements; an unpaired backtick stays; two backticks with nothing between stay as typed; `<script>` between backticks is escaped inside the code element.
  5. Run the four checks of AGENTS.md.
- accept: src/report/page/text.rs has inline_code, which escapes text and turns each pair of backticks around non-empty text into a code element; an unpaired or empty pair stays as typed; unit tests cover each case
- verify: cargo test --all-features --lib report::page
- verified: 2026-11-26
- commit: de4df6b
- by: Reynan Henry Tandoyo
- note (2026-11-22, claude-main): added
- note (2026-11-26, t232-haiku): forced claim (files held by T-235, T-239)
- note (2026-11-26, t232-haiku): Worker t232-haiku, commit de4df6b: a reading of step 2 that the spec leaves open is used. An empty pair of backticks stays as typed and the scan goes on after it, so '`` `c`' still shows c as code; the 'write the rest as it is' wording is read as the unpaired case only. Pairs are taken from the left, as CommonMark does. The claim needed --force: src/report/page/mod.rs is also in T-239's files (both add one pub mod line; git merged them without conflict here, but a later merge may need a look). The first write of text.rs was refused by the dossier guard with entries in force (DEC-055, DEC-059, LES-002, DEC-038); none conflicts with the change, and the same write was then made. Only the Windows and Linux clippy checks and the test run were executed here; no dossier command was run, and nothing was pushed.
- note (2026-11-26, t232-haiku): done — Added inline_code in src/report/page/text.rs, which escapes text and turns pairs of backticks around non-empty text into code; 10 new tests, 1051 tests passing in cargo test --all-features.

### T-233 · The tree shows text between backticks as code
- status: done
- owner: t233-haiku
- priority: P2 · difficulty: 1 · type: feature · area: report
- milestone: T-224
- why: the tree is where most text from the task list appears
- summary: Titles, goals, summaries, results, reasons and notes in the tree show commands as code.
- depends: T-232
- files: src/report/page/tree_html.rs, tests/snapshots/cmd_report__report_html.snap
- read: docs/requirements.md
- steps:
  1. Read src/report/page/text.rs (fn inline_code) and src/report/page/tree_html.rs.
  2. In src/report/page/tree_html.rs import `super::text::inline_code`. Replace `escape(...)` by `inline_code(...)` where these texts are written: the entry title in push_entry, the milestone title in push_summary (not the words Other work, which never hold backticks, but it does no harm), the goal, the text in push_labelled (summary, why, result), the reasons in second_line_html and waiting_html, and the note text in notes_html. Keep `escape` for IDs, owners, files, the check command, commit and every attribute value.
  3. Add a unit test in tree_html.rs that builds an entry from invented text whose title is `Add the `--summary` option` and checks that the HTML holds `<code>--summary</code>`.
  4. Update the page snapshot: in PowerShell run `$env:INSTA_UPDATE='always'; cargo test --all-features --test cmd_report; Remove-Item Env:INSTA_UPDATE`, then read `git diff tests/snapshots/cmd_report__report_html.snap` and check that only lines this task means to change are different.
  5. Run the four checks of AGENTS.md.
- accept: every title, goal, summary, why, result, reason and note in the tree goes through inline_code; IDs, files and the check stay escaped only; a unit test covers a title with backticks
- verify: cargo test --all-features --lib report::page
- verified: 2026-11-27
- commit: c4ad609
- by: Reynan Henry Tandoyo
- note (2026-11-22, claude-main): added
- note (2026-11-27, t233-haiku): done — Titles, goals, summaries, why, results, reasons, notes and dependency titles in the tree now go through inline_code, so text between backticks shows as code; the page snapshot is unchanged, and cargo test --all-features passes 1060 tests (106 binaries, 5 ignored).

### T-234 · Needs you, Last 7 days, ideas and the description show text between backticks as code
- status: done
- owner: t234-haiku
- priority: P2 · difficulty: 1 · type: feature · area: report
- milestone: T-224
- why: the same text appears in Needs you, Last 7 days and Ideas
- summary: The other parts of the page show commands written between backticks as code too.
- depends: T-232
- files: src/report/page/html.rs, tests/cmd_report.rs, tests/snapshots/cmd_report__report_html.snap
- read: docs/requirements.md
- steps:
  1. Read src/report/page/text.rs (fn inline_code) and src/report/page/html.rs.
  2. In src/report/page/html.rs import `super::text::inline_code`. Replace `escape(...)` by `inline_code(...)` for: the description in push_header, the titles and texts of the rows in push_need, the row texts in push_recent_rows and event_html, and each idea in push_more. Keep `escape` for the page title in <title>, attribute values and IDs.
  3. In tests/cmd_report.rs add a test `ideas_show_text_between_backticks_as_code`: write a small invented TODO.md (start it with the line `<!-- TEST FIXTURE: invented data, not a real task list -->`, then `<!-- todo-format: 3 -->`, a `# Title`, a `## Ideas` section and one idea `### T-001 · Try `todo add --template bug`` with `- status: open`, `- owner: —`, `- type: idea`), run the report with the helper `report`, and assert the page holds `<code>todo add --template bug</code>`. Look at the existing tests for how they set up a project (setup helpers in tests/common).
  4. Update the page snapshot: in PowerShell run `$env:INSTA_UPDATE='always'; cargo test --all-features --test cmd_report; Remove-Item Env:INSTA_UPDATE`, then read `git diff tests/snapshots/cmd_report__report_html.snap` and check that only lines this task means to change are different.
  5. Run the four checks of AGENTS.md.
- accept: the description, Needs you rows, Last 7 days rows and ideas go through inline_code; the new test passes; the snapshot is updated
- verify: cargo test --all-features --test cmd_report
- verified: 2026-11-27
- commit: bf73f93
- by: Reynan Henry Tandoyo
- note (2026-11-22, claude-main): added
- note (2026-11-26, t234-haiku): forced claim (files held by T-233)
- note (2026-11-27, t234-haiku): done — Needs you rows, Last 7 days rows, the description and ideas now go through inline_code, and a new test ideas_show_text_between_backticks_as_code passes; cargo test --all-features passes 1060 tests with 0 failed, fmt and both clippy targets pass, and the page snapshot is unchanged because its fixtures hold no backticks.

### T-225 · Milestones stand out: a marker and their ID on every milestone row
- status: done
- owner: t225-haiku
- priority: P2 · difficulty: 1 · type: feature · area: report
- milestone: T-220
- why: in the tree a milestone and a task differ only by a small triangle and bold text, so the structure is hard to see
- summary: Milestones get a clear marker and their number, so the parts of the work are easy to tell from single tasks.
- depends: T-221,T-229
- files: src/report/page/tree_html.rs, tests/cmd_report.rs, tests/snapshots/cmd_report__report_html.snap
- read: docs/requirements.md, docs/report-mockup.html
- steps:
  1. Read the bullet of R-REPORT-1 in docs/requirements.md that starts `From v1.1 every milestone row`, and a milestone row of docs/report-mockup.html (span.ms-mark and span.ms-id before span.ms-title).
  2. In src/report/page/tree_html.rs, give fn push_summary one more parameter `id: Option<&str>` after `h`. When it is Some(id), write `<span class=\"ms-mark\" role=\"img\" aria-label=\"Milestone\">M</span><span class=\"ms-id\">{escaped id}</span>` directly after `<summary>` and before the ms-title span. push_milestone passes `Some(&milestone.id)`; push_other_work passes None.
  3. Add unit tests in the tests module of tree_html.rs: a milestone row holds the marker and its ID before its title; a nested milestone has them too; the Other work row has neither.
  4. In tests/cmd_report.rs, run the tests and fix any assertion that expects `<summary><span class=\"ms-title\">` directly for a milestone, so it allows the marker and ID in front; do not weaken other checks.
  5. Update the page snapshot: in PowerShell run `$env:INSTA_UPDATE='always'; cargo test --all-features --test cmd_report; Remove-Item Env:INSTA_UPDATE`, then read `git diff tests/snapshots/cmd_report__report_html.snap` and check that only lines this task means to change are different.
  6. Run the four checks of AGENTS.md.
- accept: every milestone row in All work and Earlier work, nested ones too, starts with the marker M (aria-label Milestone) and the milestone's ID before its title; Other work has neither; unit tests cover it; the snapshot is updated
- verify: cargo test --all-features --test cmd_report --lib report::page
- verified: 2026-11-27
- commit: 3205ba9
- by: Reynan Henry Tandoyo
- note (2026-11-22, claude-main): added
- note (2026-11-22, claude-main): Owner 2026-11-22: make the milestone (the tree's knot) more visible, for example with 'M - <ID>' in front of the title.
- note (2026-11-27, t225-haiku): forced claim (files held by T-234)
- note (2026-11-27, t225-haiku): done — Milestone rows now start with the M marker and their ID before the title, with three new unit tests, the two cmd_report assertions and the report snapshot updated; cargo test --all-features passes 1075 tests (5 ignored), and fmt and both clippy checks pass.
- note (2026-11-27, t225-haiku): Claim needed --force: T-234 (t234-haiku) also lists src/report/page/tree_html.rs, so both tasks edit that file and the merge of T-234 will need care. Snapshot update wrote LF line endings; git warns that LF will become CRLF on the next touch, which was already the case for this file and was not changed. Dossier entries for tree_html.rs, tests/cmd_report.rs were refused twice on first edit (re-made unchanged); none of them conflicted with the change.
- note (2026-11-27, t225-haiku): Correction to the note above: the committed snapshot is LF before and after this commit (git ls-files --eol shows i/lf). The CRLF warning only appears because insta wrote LF into the snapshot in a working copy whose other files are CRLF; nothing in the commit changed line endings.
- note (2026-11-27, t225-haiku): Correction: the file overlap with T-234 was tests/cmd_report.rs and tests/snapshots/cmd_report__report_html.snap, not tree_html.rs; T-234 (bf73f93) is already on dev. A trial merge of this branch with dev (git merge-tree) is clean, but the merged result has not been tested.

### T-226 · A button shows the archived work in the All work tree
- status: done
- owner: claude-main
- priority: P2 · type: milestone · area: report
- milestone: T-220
- goal: One button adds the archive to All work and to the overall line
- why: the owner wants to see the whole history in one tree, not only the work still in TODO.md
- summary: One button adds everything that was finished earlier to the tree, so the whole history of the project can be seen in one place.
- depends: T-221
- accept: All work has a toggle button (Include archive, aria-pressed) that adds the archived milestones and tasks to the tree in the usual order (finished, closed) and makes the overall line count them too; the filters work on the archived entries as well; the choice is remembered like the colour choices; without scripts the page is as in v1.0 (the archive under Earlier work); tests cover the markup and the script
- by: Reynan Henry Tandoyo
- note (2026-11-22, claude-main): added
- note (2026-11-22, claude-main): Owner 2026-11-22: a toggle button that also builds and shows the tree for the archived items.
- note (2026-11-22, claude-main): split into T-235, T-236, T-237, T-238
- note (2026-11-27, claude-main): done — T-235 to T-238 done: Include archive adds the archive's tree to All work

### T-235 · Page data counts the tasks of TODO.md and the archive together
- status: done
- owner: t235-haiku
- priority: P2 · difficulty: 1 · type: feature · area: report
- milestone: T-226
- why: the overall line with the archive included needs counts over both files
- summary: Prepares the numbers the overall line shows when the archive is included.
- files: src/report/page/tree.rs, src/report/page/mod.rs
- read: docs/requirements.md
- steps:
  1. Read the bullet of R-REPORT-6 in docs/requirements.md that starts `From v1.1, when the project has an archive`.
  2. In src/report/page/tree.rs, in `impl Counts`, add `#[must_use] pub const fn plus(self, other: Self) -> Self` that adds each of the five fields. Add a unit test for it in the tests module of tree.rs.
  3. In src/report/page/mod.rs add to PageData the field `pub counts_with_archive: Option<tree::Counts>` with the doc comment `The overall counts over TODO.md and the archive together (R-REPORT-6, Include archive); None without an archive.` In fn build, set it to `earlier.as_ref().map(|e| tree.counts.plus(e.tree.counts))`: build `earlier` into a local variable before the PageData literal so both fields can use it.
  4. Run the four checks of AGENTS.md.
- accept: Counts has plus; PageData has counts_with_archive, the sum of the counts of TODO.md and the archive, None without an archive; a unit test covers plus
- verify: cargo test --all-features --lib report::page
- verified: 2026-11-26
- commit: 3a66a0b
- by: Reynan Henry Tandoyo
- note (2026-11-22, claude-main): added
- note (2026-11-26, t235-haiku): forced claim (files held by T-239)
- note (2026-11-26, t235-haiku): done — Added Counts::plus and PageData.counts_with_archive (TODO.md plus archive counts, None without an archive) with a unit test for plus; 1042 tests pass, 76 in report::page; the claim needed --force because T-239 lists the same files.

### T-236 · The page writes a second, hidden overall line with the archive included
- status: done
- owner: t236-haiku
- priority: P2 · difficulty: 1 · type: feature · area: report
- milestone: T-226
- why: the script only switches between two overall lines; the page must already hold both
- summary: Adds the overall numbers for the whole history, shown when the archive is included.
- depends: T-235
- files: src/report/page/html.rs, tests/cmd_report.rs, tests/snapshots/cmd_report__report_html.snap
- read: docs/requirements.md
- steps:
  1. Read the same bullet of R-REPORT-6 and fn push_overall in src/report/page/html.rs.
  2. Give push_overall two more parameters: `id: &str` and `with_archive: bool`. It opens the block with `<div class="overall" id="{id}"` followed by ` hidden` when with_archive is true, then `>`. The first line reads `{done} of {total} tasks done` as now, and with with_archive true `{done} of {total} tasks done, archive included`.
  3. In fn render, call `push_overall(h, &page.counts, "overall-now", false)`, and directly after it, when `page.counts_with_archive` is Some(counts), `push_overall(h, &counts, "overall-all", true)`.
  4. In tests/cmd_report.rs add a test `the_overall_line_with_the_archive_is_in_the_page_and_hidden`: with fixture_report (which has an archive) assert the page holds `id="overall-now"` and `<div class="overall" id="overall-all" hidden>` and `tasks done, archive included`; and with a project without an archive (see the existing test that leaves the archive out) assert it holds no `overall-all`.
  5. Update the page snapshot: in PowerShell run `$env:INSTA_UPDATE='always'; cargo test --all-features --test cmd_report; Remove-Item Env:INSTA_UPDATE`, then read `git diff tests/snapshots/cmd_report__report_html.snap` and check that only lines this task means to change are different.
  6. Run the four checks of AGENTS.md.
- accept: the page has the overall line with id overall-now and, with an archive, a hidden second one with id overall-all that counts both files and says archive included; tests cover both cases
- verify: cargo test --all-features --test cmd_report
- verified: 2026-11-27
- commit: be9fc26
- by: Reynan Henry Tandoyo
- note (2026-11-22, claude-main): added
- note (2026-11-27, t236-haiku): forced claim (files held by T-225)
- note (2026-11-27, t236-haiku): Claimed with --force: the claim was refused only because claimed T-225 (t225-haiku) lists the same files, so T-225 may touch src/report/page/html.rs too and conflict on merge. The dossier guard refused my first edit of html.rs and tests/cmd_report.rs; I read the entries listed (none conflicts with this change) and made the same edits again. The new test's expected numbers (6 of 13, 3 cancelled) were counted by hand from the fixtures and match the regenerated snapshot.
- note (2026-11-27, t236-haiku): done — push_overall takes an id and a with-archive flag, so the page writes the hidden overall-all line (6 of 13 tasks done, archive included) beside overall-now when the project has an archive, with a test for both cases; cargo test passes 1074 tests with 0 failures.

### T-237 · The Include archive button and a movable archive tree
- status: done
- owner: t237-haiku
- priority: P2 · difficulty: 1 · type: feature · area: report
- milestone: T-226
- why: the script moves the archive's tree into All work; the markup must give it IDs and a label
- summary: Adds the button and gives the archived tree what it needs to move into All work.
- depends: T-236
- files: src/report/page/html.rs, tests/cmd_report.rs, tests/snapshots/cmd_report__report_html.snap
- read: docs/requirements.md
- steps:
  1. Read the same bullet of R-REPORT-6, fn push_all_work and fn push_earlier in src/report/page/html.rs.
  2. In push_all_work, when `page.earlier.is_some()`, write `<button id="arch" type="button" aria-pressed="false">Include archive</button>\n` directly after the Cancelled button (before `<span class="sp"></span>`). Split the one string literal into two pushes to do that.
  3. In push_earlier: open the fold as `<details class="fold" id="earlier">` instead of `<details class="fold">`; change the replacement of `<div id="tree">` so it becomes `<div class="tree" id="arch-tree">\n<p class="arch-label">From the archive</p>`.
  4. In tests/cmd_report.rs add a test `the_archive_has_a_toggle_and_a_movable_tree`: with fixture_report assert the controls hold `<button id="arch"` and the text `Include archive`, the page holds `id="earlier"`, `id="arch-tree"` and `From the archive`; with a project without an archive assert there is no `id="arch"`.
  5. Update the page snapshot: in PowerShell run `$env:INSTA_UPDATE='always'; cargo test --all-features --test cmd_report; Remove-Item Env:INSTA_UPDATE`, then read `git diff tests/snapshots/cmd_report__report_html.snap` and check that only lines this task means to change are different.
  6. Run the four checks of AGENTS.md.
- accept: with an archive All work has the button Include archive (id arch, aria-pressed false), Earlier work has id earlier and its tree id arch-tree with the label From the archive; without an archive there is no button; tests cover both
- verify: cargo test --all-features --test cmd_report
- verified: 2026-11-27
- commit: 2d1b77f
- by: Reynan Henry Tandoyo
- note (2026-11-22, claude-main): added
- note (2026-11-27, t237-haiku): Worker notes: the dossier guard refused the first edit of src/report/page/html.rs and tests/cmd_report.rs with entries in force (LES-002, FND-034, DEC-038, DEC-055, DEC-059 and timing/version lessons); none conflicts with this task, so the same edits were made again. The script that moves arch-tree into All work and hides the earlier fold is not in files (script.js is not listed), so it is left for the task that owns the script; the markup carries the ids it needs. Each push is a separate literal: the button is pushed only when page.earlier is Some. git warns that LF will become CRLF for tests/snapshots/cmd_report__report_html.snap; the diff shows only the intended five lines.
- note (2026-11-27, t237-haiku): done — The page adds the Include archive button to All work when the project has an archive, gives Earlier work the id earlier and its tree the id arch-tree with the label From the archive, with a test for both cases; cargo test passes 1079 tests with 0 failures.

### T-238 · The script adds the archive to All work when Include archive is pressed
- status: done
- owner: t238-haiku
- priority: P2 · difficulty: 1 · type: feature · area: report
- milestone: T-226
- why: the owner wants to see the whole history in one tree
- summary: One button now shows everything finished earlier in the same tree, with the overall numbers for the whole history.
- depends: T-237
- files: src/report/page/script.js, tests/cmd_report.rs, tests/snapshots/cmd_report__report_html.snap
- read: docs/requirements.md
- steps:
  1. Read the same bullet of R-REPORT-6 and the whole of src/report/page/script.js.
  2. After the line that defines `tree`, add: `var archButton = document.getElementById('arch'); var archTree = document.getElementById('arch-tree'); var earlierFold = document.getElementById('earlier'); var overallNow = document.getElementById('overall-now'); var overallAll = document.getElementById('overall-all'); var ARCH_KEY = 'todo-report-archive'; var archiveIncluded = false; var currentFilter = 'all';`.
  3. Add `function workItems(selector)`: it returns an array of the elements matching selector inside `tree`, followed by those inside `archTree` when `archiveIncluded` is true (use Array.prototype.slice.call on each querySelectorAll result and concat).
  4. In applyFilter, set `currentFilter = filter;` first; replace `tree.querySelectorAll('li.item')` and `tree.querySelectorAll('details.ms')` by `workItems('li.item')` and `workItems('details.ms')`; in the empty-view check use `!workItems('li.item:not(.hide)').length` instead of `!tree.querySelector('li.item:not(.hide)')`. In the Expand all and Collapse all handlers use `workItems('details.ms')` and `workItems('details')` instead of the tree queries.
  5. Add `function setArchive(on)`: if `!archButton || !archTree` return; archiveIncluded = on; when on, insert archTree directly after tree (`tree.parentNode.insertBefore(archTree, tree.nextSibling)`), else append it back to earlierFold (`earlierFold.appendChild(archTree)`); `earlierFold.hidden = on`; when both overall lines exist, `overallNow.hidden = on` and `overallAll.hidden = !on`; set aria-pressed on archButton; then `applyFilter(currentFilter)`.
  6. After the filter buttons get their click handlers, add: `if (archButton) { setArchive(load(ARCH_KEY) === 'on'); archButton.addEventListener('click', function () { var on = !archiveIncluded; setArchive(on); save(ARCH_KEY, on ? 'on' : 'off'); }); }`.
  7. In tests/cmd_report.rs add a test `the_script_moves_the_archive_into_all_work`: the script text holds `todo-report-archive`, `arch-tree`, `overall-all` and `function setArchive`.
  8. Run the four checks of AGENTS.md.
- accept: pressing Include archive moves the archive's tree into All work below the tree of TODO.md, hides Earlier work, switches the overall line to the one with the archive, applies the current filter to both trees and remembers the choice under todo-report-archive; pressing it again puts everything back; Expand all, Collapse all and the empty-view line cover the archive too; the test passes
- verify: cargo test --all-features --test cmd_report
- verified: 2026-11-27
- commit: dda6e73
- by: Reynan Henry Tandoyo
- note (2026-11-22, claude-main): added
- note (2026-11-26, claude-main): files widened: the page embeds script.js, so the report snapshot changes too (as on T-231)
- note (2026-11-27, t238-haiku): done — The script moves the archive tree into All work when Include archive is pressed, and the view filters, Expand all, Collapse all, the empty-view line, the overall line and the stored choice cover it; four absence assertions in cmd_report.rs were narrowed because the script now names those ids, and the snapshot changed only in the script. cargo test --all-features passes 1095 tests with 0 failures (5 ignored), fmt and both clippy targets are clean, and a jsdom run of the page's script passed 33 checks outside the repository.
- note (2026-11-27, t238-haiku): Assumed: when the archive leaves All work its entries lose the view filter's hide, so Earlier work shows them all as before the button; milestones the filter opened stay open. Not run in a real browser: the behaviour was checked with jsdom on the snapshot page only.

### T-227 · Release 1.1.0
- status: open
- owner: —
- priority: P2 · type: milestone · area: release
- milestone: T-220
- goal: Version 1.1.0 is released with the polished page
- why: the owner gets the polished page through a release
- summary: Version 1.1.0 is released with the polished page.
- depends: T-222, T-223, T-224, T-225, T-226, T-228
- accept: README and todo help report describe the archive toggle and the milestone marker; CHANGELOG.md has the 1.1.0 entry; Cargo.toml is 1.1.0; all checks pass on Windows and Linux; the owner has looked at this project's page
- note (2026-11-22, claude-main): added
- note (2026-11-22, claude-main): split into T-245, T-246

### T-245 · Help text and README describe the v1.1 page
- status: done
- owner: t245-haiku
- priority: P2 · difficulty: 1 · type: docs · area: docs
- milestone: T-227
- why: people and agents learn about the page from the README and todo help report
- summary: The help and the README explain the milestone marker, the archive button and the chain.
- files: README.md, src/cli/report.rs, tests/snapshots/cli_basic__cli_overview_json.snap
- read: docs/requirements.md
- steps:
  1. Read the v1.1 bullets of R-REPORT-1, R-REPORT-6 and R-REPORT-7 in docs/requirements.md.
  2. In src/cli/report.rs, in the `details` text of META, add one sentence each for: the milestone marker M with the ID; Include archive, which adds the archive's tree to All work and counts it in the overall line; Show chain in a task's details; the line shown when a view button matches nothing. Keep the summary line unless a test requires otherwise.
  3. In README.md, in the section 'HTML report for people' (and in the part for people near the top if it lists the buttons), add the same four points in plain words.
  4. Run cargo test --all-features; if a help snapshot changes, update it the same way (INSTA_UPDATE=always for that test) and read the diff.
  5. Run the four checks of AGENTS.md.
- accept: todo help report and the README describe the milestone marker, Include archive, Show chain and the empty-view line; all tests pass
- verify: cargo test --all-features
- verified: 2026-11-27
- commit: 326af71
- by: Reynan Henry Tandoyo
- note (2026-11-22, claude-main): added
- note (2026-11-27, t245-haiku): done — Help text (report META details) and README HTML report section now describe the milestone marker, Include archive, Show chain and the empty-view line; fmt, clippy (Windows and Linux) and cargo test pass: 1096 tests passed, 0 failed, 5 ignored.

### T-246 · Version 1.1.0 and changelog
- status: claimed
- owner: claude-main
- claimed: 2026-11-27T00:50+01:00
- session: f7957659-295e-4e1b-b8f8-96b2d860c5e5
- priority: P2 · difficulty: 2 · type: docs · area: release
- milestone: T-227
- why: the owner gets the polished page through a release
- summary: Version 1.1.0 is released with the polished page.
- final: The release is done by the planner: version, changelog, checks on both systems and the owner's look before tagging.
- depends: T-245
- files: Cargo.toml, CHANGELOG.md, docs/json.md
- read: docs/requirements.md
- steps:
  1. The planner does this task: version 1.1.0 in Cargo.toml, Cargo.lock and the examples of docs/json.md, the changelog entry, the overview snapshot, all checks on Windows and Linux, and the owner's look at this project's page before tagging.
- accept: Cargo.toml is 1.1.0; CHANGELOG.md has the 1.1.0 entry; all checks pass on Windows and Linux; the owner has looked at this project's page
- verify: cargo test --all-features
- note (2026-11-22, claude-main): added

### T-228 · Show chain: each task can show the chain of tasks it waits for and that wait for it
- status: done
- owner: claude-main
- priority: P2 · type: milestone · area: report
- milestone: T-220
- goal: Each task shows, on request, the chain of tasks before and after it
- why: the owner wants to see why a task is where it is: which tasks must finish before it and which wait for it, without cluttering the main view
- summary: Each task can show, on request, the chain of tasks that come before it and after it, with the state of each one.
- depends: T-221
- accept: inside an opened entry (next to Technical details, closed by default, so the main view does not change) a part Show chain lists the tasks it waits for, through every level of depends, and the tasks that wait for it, as chains in order such as T-001 ✓ → T-005 ● → this task → T-112 ○, each step with its mark, ID, title and state in words; milestones in the way count as R-MILE-6 says; dependencies on archived entries show their state and are marked archived; a task without dependencies in either direction shows no Show chain part; branches (a task that waits for two) are shown as separate lines, at most 10 lines, with the rest counted; works without scripts; unit tests cover a straight chain, a branch, an archived step, a missing ID and a cycle
- by: Reynan Henry Tandoyo
- note (2026-11-22, claude-main): added
- note (2026-11-22, claude-main): Owner 2026-11-22: emphasise the chain of a todo item without cluttering the main view; a 'show chain' button per task showing e.g. T001 -> T005 -> T112 with the status of each item. T-221 decides the exact form and draws it in the mockup.
- note (2026-11-22, claude-main): split into T-239, T-240, T-241, T-242, T-243, T-244
- note (2026-11-27, claude-main): T-242 readings accepted: a loop's repeated step is named only by the back-to text; a cut after-line ends with the ellipsis (requirements updated)
- note (2026-11-27, claude-main): done — T-239 to T-244 and T-247 done: a task's details show its chain; page 137-147 ms and 3.4 MB on the 5,000-entry file

### T-239 · Chain data: the steps and an index of every entry
- status: done
- owner: t239-haiku
- priority: P2 · difficulty: 1 · type: feature · area: report
- milestone: T-228
- why: the chain of each task is found through depends in both files; one index built once keeps the page fast
- summary: Prepares what the page needs to know about every entry to draw the chain of a task.
- files: src/report/page/chain.rs, src/report/page/mod.rs
- read: docs/requirements.md, src/report/page/entry.rs, src/report/page/tree.rs
- steps:
  1. Read R-REPORT-7 in docs/requirements.md, and in src/report/page/entry.rs fn entry_view, fn dependencies and the Ctx struct (to see how depends are read and IDs are written).
  2. Create src/report/page/chain.rs with the module doc comment `//! The chain of a task (T-228; R-REPORT-7): the tasks it waits for and the tasks that wait for it.` and add `pub mod chain;` to src/report/page/mod.rs.
  3. Define in src/report/page/chain.rs, each with `#[derive(Debug, Clone, PartialEq, Eq)]` and a doc comment: `pub enum StepState { Task(State), Milestone(MilestoneState), NotFound }`; `pub struct Step { pub id: String, pub title: String, pub state: StepState, pub archived: bool }` (title is empty when not found); `pub enum PathEnd { Open, Loop(String) }` (Loop names the ID the path came back to); `pub struct Path { pub steps: Vec<Step>, pub cut: bool, pub end: PathEnd }` (cut is true when steps far from the task were left out); `pub struct Chain { pub before: Vec<Path>, pub more_before: usize, pub after: Vec<Path>, pub more_after: usize }`. State is super::entry::State and MilestoneState is super::tree::MilestoneState.
  4. Add `pub struct ChainIndex` (derive Debug, Default) with private fields: `nodes: HashMap<String, Node>` and `dependents: HashMap<String, Vec<String>>`, where `struct Node { title: String, archived: bool, milestone: bool, state: State, milestone_state: Option<MilestoneState>, depends: Vec<String> }` (derive Debug, Clone).
  5. Add `pub fn new(doc: &Document, archive: Option<&Document>, ctx: &Ctx<'_>) -> Self`: for every task of doc (archived false) and then of the archive (archived true), the key is the canonical ID (ctx.cfg.id_config.format_id), the state is `entry_view(task, ctx).state`, milestone is `task.kind() == EntryKind::Milestone`, and depends are the canonical IDs of its depends, read the same way fn dependencies in entry.rs reads them. An ID already in the map is not replaced (TODO.md wins). For every depends ID d of an entry e, push e's ID to `dependents[d]` in file order.
  6. Add `pub fn set_milestone_states(&mut self, tree: &Tree)`: walk tree.milestones and their nested milestones recursively and set `milestone_state = Some(view.state)` on the node with the same ID.
  7. Add `pub fn step(&self, id: &str) -> Step`: a known node gives its title, archived flag and StepState::Milestone(state) when it is a milestone with a milestone_state, else StepState::Task(state); an unknown ID gives an empty title and StepState::NotFound.
  8. Add unit tests (invented text parsed into a Document, as the other tests of this module folder do): the index finds a task of TODO.md and one of the archive (archived true); dependents lists the entries that name an ID; step of an unknown ID is NotFound; a milestone step after set_milestone_states carries the milestone state.
  9. Run the four checks of AGENTS.md.
- accept: src/report/page/chain.rs has StepState, Step, PathEnd, Path, Chain and ChainIndex with new, set_milestone_states and step; the index covers both files, TODO.md first; unit tests cover each point
- verify: cargo test --all-features --lib report::page
- verified: 2026-11-26
- commit: 28dc0c8
- by: Reynan Henry Tandoyo
- note (2026-11-22, claude-main): added
- note (2026-11-26, t239-haiku): done — Added src/report/page/chain.rs (StepState, Step, PathEnd, Path, Chain, ChainIndex with new, set_milestone_states and step) and registered it in page/mod.rs; cargo test --all-features passes 1047 tests (6 new chain tests), fmt and clippy pass for Windows and Linux. Read as: the index is in memory per page build, which D15 allows, and holds every entry of both files; dependents list each ID once; an archived milestone shows its tree state only when the archive tree is also passed to set_milestone_states.

### T-240 · Chain data: the paths before and after a task, with their limits
- status: done
- owner: t240-haiku
- priority: P2 · difficulty: 1 · type: feature · area: report
- milestone: T-228
- why: the page shows at most 10 paths of at most 12 steps each way, and counts the rest
- summary: Works out, for one task, every path of tasks that leads to it and away from it, within fixed limits.
- depends: T-239
- files: src/report/page/chain.rs
- read: docs/requirements.md
- steps:
  1. Read R-REPORT-7 in docs/requirements.md (the bullets about the two groups, a line, and the limits) and src/report/page/chain.rs.
  2. Add constants `MAX_LINES: usize = 10`, `MAX_STEPS: usize = 12`, `MAX_COUNTED: usize = 101` with doc comments naming R-REPORT-7.
  3. Add `pub fn chain_of(&self, id: &str) -> Option<Chain>`: return None when the node has no depends and no dependents. Otherwise fill `before` and `after` as below.
  4. Before: a depth-first walk from the task over depends. Keep the current path as a Vec<String> of IDs, starting empty. For the node at the end of the path (the task itself at the start), look at its depends: when it has none, or the ID is unknown (not found), the path is complete. For each dependency d in order: if d is the task itself or already in the path, record the path plus d with PathEnd::Loop(d); else push d and continue from it. When the path reaches MAX_STEPS IDs, record it with cut = true instead of going deeper. A recorded path's steps are the IDs reversed (the start first, the direct dependency last), each turned into a Step with self.step. A not-found ID is a step of its own and ends its path.
  5. After: the same walk over `dependents` instead of depends, from the task outwards; the steps are not reversed (the direct dependent first).
  6. Counting: keep the first MAX_LINES recorded paths in the Vec; count every recorded path; stop the walk as soon as the count reaches MAX_LINES + MAX_COUNTED. more_before (and more_after) is the count minus the kept lines, so a value of MAX_COUNTED or more means more than 100.
  7. Unit tests: a straight chain A <- B <- C gives for C one before path [A, B] and for A one after path [B, C]; a branch (C depends on A and B) gives two before paths; a step in the archive is archived; a milestone in depends is a milestone step; an unknown ID gives a NotFound step that ends its path; A depends on B and B on A gives a Loop end; 15 tasks that all wait for one task give 10 after paths and more_after 5; a 20-long chain gives a cut path of 12 steps holding the 12 nearest IDs; a task without depends and dependents gives None.
  8. Run the four checks of AGENTS.md.
- accept: chain_of returns None for a task with no depends and no dependents, and otherwise the before paths (start first) and after paths (nearest first), each at most 12 steps (cut keeps the nearest), loops ended with Loop, unknown IDs as NotFound steps, at most 10 paths per group with the rest counted up to 101; unit tests cover each rule
- verify: cargo test --all-features --lib report::page
- verified: 2026-11-26
- commit: 6927441
- by: Reynan Henry Tandoyo
- note (2026-11-22, claude-main): added
- note (2026-11-26, t240-haiku): done — Added chain_of with the walk, limits and counting in src/report/page/chain.rs plus 11 unit tests (17 in the chain module, 1070 passing in cargo test); read as: a path that reaches 12 steps is cut only when something lies beyond it, a loop step includes the repeated ID, an unknown task gives None, and milestones are not excluded here so the page must skip them.

### T-241 · Every task on the page carries its chain
- status: done
- owner: t241-haiku
- priority: P2 · difficulty: 1 · type: feature · area: report
- milestone: T-228
- why: the HTML of an entry is written from its EntryView, so the chain must be part of it
- summary: Connects the chain to every task in the tree and in the archive.
- depends: T-240
- files: src/report/page/entry.rs, src/report/page/mod.rs, src/report/page/chain.rs
- read: docs/requirements.md
- steps:
  1. Read src/report/page/chain.rs, the struct EntryView in src/report/page/entry.rs and fn build in src/report/page/mod.rs.
  2. In entry.rs add to EntryView the field `pub chain: Option<super::chain::Chain>` with the doc comment `The chain of R-REPORT-7, filled in by chain::fill_chains; None in entry_view.`, and set `chain: None` in the EntryView literal of fn entry_view.
  3. In chain.rs add `pub fn fill_chains(tree: &mut Tree, index: &ChainIndex)`: for every EntryView in tree.other and in the tasks of every milestone at any depth, set `entry.chain = index.chain_of(&entry.id)`.
  4. In mod.rs fn build: build tree and earlier into local variables first (earlier may already be one); then `let mut index = chain::ChainIndex::new(doc, archive, &ctx); index.set_milestone_states(&tree);` and the same for the earlier tree when there is one; then `chain::fill_chains(&mut tree, &index)` and the same for `&mut earlier.tree`; then build the PageData literal with them. The overall counts are read before or after; they do not change.
  5. Add a unit test in chain.rs: after fill_chains on a tree of invented text, a task that depends on another has Some chain and a task without depends or dependents has None.
  6. Run the four checks of AGENTS.md.
- accept: EntryView has chain; fill_chains sets it for every task of the tree and of the archive's tree; build fills both with one index; a unit test covers it
- verify: cargo test --all-features --lib report::page
- verified: 2026-11-27
- commit: ed26074
- by: Reynan Henry Tandoyo
- note (2026-11-22, claude-main): added
- note (2026-11-27, t241-haiku): done — EntryView has a chain field that build fills for every task of the tree and the archive's tree from one ChainIndex, milestones keep none, and a unit test covers both trees; cargo test passes 1072 tests, fmt and clippy pass on Windows and Linux; the dossier guard refused my first edit of entry.rs with a list of entries that did not conflict, and the same edit went through on retry.

### T-242 · Chain HTML: the closed part Show chain
- status: done
- owner: t242-haiku
- priority: P2 · difficulty: 1 · type: feature · area: report
- milestone: T-228
- why: the chain must not clutter the main view, so it is a closed part inside a task's details
- summary: Draws the chain of a task as short lines of marked steps, hidden until the reader opens it.
- depends: T-240
- files: src/report/page/chain_html.rs, src/report/page/mod.rs
- read: docs/requirements.md, docs/report-mockup.html
- steps:
  1. Read R-REPORT-7 in docs/requirements.md and the opened chain at the end of the T-211 entry in docs/report-mockup.html (details.chain, p.lbl, p.chain-line, span.sr), and src/report/page/chain.rs.
  2. Create src/report/page/chain_html.rs with a module doc comment naming T-228 and R-REPORT-7, and add `pub mod chain_html;` to src/report/page/mod.rs. Escape every text with crate::report::html::escape.
  3. Write `pub fn push_chain(h: &mut String, chain: &Chain)`: `<details class="chain"><summary>Show chain</summary>`; when before is not empty, `<p class="lbl">Before this task</p>` and one `<p class="chain-line">` per path, its steps joined by ` → `, then ` → <b>this task</b>`; when after is not empty, `<p class="lbl">After this task</p>` and per path `<b>this task</b> → ` then the steps joined by ` → `; after each group with more paths, `<p class="muted">and N more paths</p>` (N = more), or `and more than 100 more paths` when more is 101 or more, or `and 1 more path` for one; then `</details>`.
  4. A step is: for a task `<span class="mark {class}">{mark}</span> <span class="cid">{id}</span> {title} <span class="sr">({word})</span>` with State::class, mark and word; for a milestone `<span class="ms-mark" role="img" aria-label="Milestone">M</span> <span class="cid">{id}</span> {title} <span class="muted">({state words})</span>` with not started / in progress / finished / cancelled; for NotFound `<span class="cid">{id}</span> <span class="muted">(not found)</span>`. An archived step adds ` <span class="muted">(archived)</span>`. A cut path starts with `… → `. A path with PathEnd::Loop(id) ends with ` → … back to <span class="cid">{id}</span>` (after the steps, before `→ <b>this task</b>` in a before line; at the end of an after line).
  5. Unit tests with hand-built Chain values: one before and one after line in order; a milestone step; a not-found step; an archived step; a cut path; a loop; the more texts for 1, 5 and 101; a title with <script> is escaped. Never use the class `id` for an ID in a chain (other tests count `class="id"` per entry).
  6. Run the four checks of AGENTS.md.
- accept: src/report/page/chain_html.rs writes the closed part Show chain with the groups Before this task and After this task, each line of marked steps joined by arrows with this task in bold, milestone, archived, not-found, cut and loop steps as R-REPORT-7 says, and the more texts; unit tests cover each case
- verify: cargo test --all-features --lib report::page
- verified: 2026-11-27
- commit: 99bb210
- by: Reynan Henry Tandoyo
- note (2026-11-22, claude-main): added
- note (2026-11-27, t242-haiku): Readings of R-REPORT-7 that the spec left open, decided in chain_html.rs: (1) a loop path repeats the ID it comes back to as its walk-end step; that step is not written as a step, the line ends with '… back to <id>' (before lines: after the steps, before this task; after lines: at the end). (2) A cut path puts its '…' at the far end: first on a before line as the spec says, but last on an after line, since the spec's 'starts with' would put it before this task. (3) Step titles go through inline_code (backticks as code, as tree titles do), so the escape guarantee holds. R-REPORT-7 and step 4 should say which end the cut marker sits at; the owner should confirm (2). The .lbl and .cid rules for the chain are not in src/report/page/style.css (T-214 / a later task); I did not touch that file.
- note (2026-11-27, t242-haiku): Correction to the previous note: style.css has .chain-line .cid, and .chain-line and details.chain rules, so only the group labels are unstyled: the rule .more .lbl is the only one for p.lbl, so 'Before this task' and 'After this task' show as plain text. A rule such as details.chain .lbl belongs in src/report/page/style.css, which this task's files do not list; I did not change it.
- note (2026-11-27, t242-haiku): done — Added src/report/page/chain_html.rs with push_chain for the closed part Show chain (both groups, marked steps, cut, loop, archived and not-found steps, and the more counts), registered in page/mod.rs; 15 new tests, and cargo test passes 1089 tests (223 in the library).

### T-243 · Opened tasks show their chain
- status: done
- owner: t243-haiku
- priority: P2 · difficulty: 1 · type: feature · area: report
- milestone: T-228
- why: this puts the chain on the page
- summary: Each task's details end with Show chain when the task has one.
- depends: T-241, T-242
- files: src/report/page/tree_html.rs, tests/cmd_report.rs, tests/snapshots/cmd_report__report_html.snap
- read: docs/requirements.md
- steps:
  1. Read fn push_details in src/report/page/tree_html.rs and fn push_chain in src/report/page/chain_html.rs.
  2. In push_details, after the technical details are added to the details body, add: when `entry.chain` is Some(chain), call `super::chain_html::push_chain(&mut body, chain)` (use the same String the technical details go into, so the chain is the last part of the opened details). If push_details writes nothing when body is empty, keep that rule.
  3. In tests/cmd_report.rs add a test `a_task_with_dependencies_shows_its_chain`: build the page with fixture_report; assert it holds `<details class="chain"><summary>Show chain</summary>` at least once and that a task of the fixture that has `depends` (look in tests/fixtures/report_page.md) shows `this task` in its row.
  4. Update the page snapshot: in PowerShell run `$env:INSTA_UPDATE='always'; cargo test --all-features --test cmd_report; Remove-Item Env:INSTA_UPDATE`, then read `git diff tests/snapshots/cmd_report__report_html.snap` and check that only lines this task means to change are different.
  5. Run the four checks of AGENTS.md.
- accept: a task with a chain has the closed part Show chain at the end of its details; a task without one has none; the test passes and the snapshot is updated
- verify: cargo test --all-features --test cmd_report
- verified: 2026-11-27
- commit: 982daab
- by: Reynan Henry Tandoyo
- note (2026-11-22, claude-main): added
- note (2026-11-27, claude-main): added after T-242: in src/report/page/style.css change the selector '.more .lbl{' to '.more .lbl, details.chain .lbl{' so the Before this task / After this task labels are styled; the snapshot then changes too
- note (2026-11-27, claude-main): the label style moved to its own task T-247; ignore the earlier note about style.css
- note (2026-11-27, t243-haiku): forced claim (files held by T-237)
- note (2026-11-27, t243-haiku): done — Added the closed Show chain part to push_details after Technical details, with test a_task_with_dependencies_shows_its_chain and the updated snapshot (three chains: T-004, T-005, T-014); cargo test passes 1094 tests with 0 failed, and fmt and clippy pass on Windows and Linux; I claimed with --force because T-237 (t237-haiku) lists the same tests/cmd_report.rs and snapshot, and the dossier guard refused my first edits of tree_html.rs and cmd_report.rs with entries that did not conflict, so I retried each edit as the task says; questions such as T-015 are not opened entries, so they show no chain.

### T-244 · The page with chains stays within the speed and size limits
- status: done
- owner: t244-haiku
- priority: P2 · difficulty: 1 · type: test · area: report
- milestone: T-228
- why: chains add work and HTML for every task; NFR-3 still holds
- summary: Checks that adding chains keeps the page fast and small on a very large task list.
- depends: T-243
- files: tests/perf.rs
- read: docs/requirements.md
- steps:
  1. Read NFR-3 in docs/requirements.md and the report test in tests/perf.rs.
  2. Run `cargo test --release --all-features --test perf -- --ignored --nocapture` three times and write down the time and the size of report --html.
  3. If the page is under 500 ms and under 5 MB, change nothing in tests/perf.rs and put the numbers in the done note. If it is over a limit, do not raise the limit: run `todo stuck` with the numbers.
  4. Run the four checks of AGENTS.md.
- accept: report --html on the 5,000-entry milestone tree is still under 500 ms and 5 MB with chains, measured three times in a release build; the numbers are in the done note
- verify: cargo test --release --all-features --test perf -- --ignored
- verified: 2026-11-27
- commit: d36f0ac
- by: Reynan Henry Tandoyo
- note (2026-11-22, claude-main): added
- note (2026-11-27, t244-haiku): done — No file changed: report --html on the 5,000-entry milestone tree with chains took 147.0, 136.0 and 142.7 ms over three release runs (budget 500 ms) and wrote a 3,443,562-byte page (budget 5 MB), and cargo test passes 1095 tests with 0 failed, with fmt and clippy passing on Windows and Linux.
- note (2026-11-27, t244-haiku): Worker note: the brief's Finish step (git add tests/perf.rs, then git commit) cannot commit here because the budgets held and the file stayed unchanged; the task commit is an empty commit (d36f0ac) instead. The chain part of the page is in the timed file: push_details writes Show chain for every entry that has a chain, and every fifth task has a depends line. Also: cargo test counted 1095 passed and 5 ignored, where the T-243 note counted 1094.

### T-229 · Stylesheet rules for the v1.1 page
- status: done
- owner: t229-haiku
- priority: P2 · difficulty: 1 · type: feature · area: report
- milestone: T-220
- why: every v1.1 page part needs its CSS; writing it in one place first keeps the other tasks to their markup
- summary: Adds the look of the new page parts: the milestone badge, code text, the chain and the archive line.
- depends: T-221
- files: src/report/page/style.css, tests/snapshots/cmd_report__report_html.snap
- read: docs/requirements.md, docs/report-mockup.html
- steps:
  1. Read the v1.1 bullets of R-REPORT-1, R-REPORT-6 and R-REPORT-7 in docs/requirements.md, and the rules at the end of the style block of docs/report-mockup.html (.ms-mark, .ms-id, details.chain, .chain-line, .sr).
  2. Open src/report/page/style.css. Directly before the line that starts with `@media (max-width:560px)`, add these rules, each on its own line: `.page h4{font-size:13px; margin:12px 0 4px}` ; `.page code{font-family:ui-monospace,Consolas,monospace; font-size:.92em; background:var(--p-mut); padding:0 3px; border-radius:3px}` ; `.ms-mark{display:inline-block; flex:none; min-width:1.5em; padding:0 4px; border:1.5px solid var(--p-fg); border-radius:4px; font-size:11px; font-weight:700; line-height:1.5; text-align:center}` ; `.ms-id{flex:none; font-size:12px; color:var(--p-mfg); font-family:ui-monospace,Consolas,monospace}` ; `details.chain{margin-top:6px}` ; `details.chain>summary{cursor:pointer; font-size:13px; color:var(--p-mfg)}` ; `.chain-line{margin:2px 0 6px; font-size:13px; line-height:1.6}` ; `.chain-line .cid{font-family:ui-monospace,Consolas,monospace; font-size:12px}` ; `.sr{position:absolute; width:1px; height:1px; overflow:hidden; clip:rect(0 0 0 0); white-space:nowrap}` ; `.arch-label{margin:14px 0 8px; font-size:13px; font-weight:600; color:var(--p-mfg)}`.
  3. Change the line `@media (max-width:560px){ .bar{display:none} .id{display:none} }` to `@media (max-width:560px){ .bar{display:none} .id{display:none} .ms-id{display:none} }`.
  4. Change the print rule `@media print{ .controls{display:none} details.ms{break-inside:avoid} }` so it also contains `.sr{position:static; width:auto; height:auto; clip:auto}` inside the braces.
  5. Check that `--p-mut`, `--p-fg` and `--p-mfg` are defined in the file (search for `--p-mut:`); if a name differs, use the name the file defines for the muted background, the text colour and the muted text colour.
  6. Update the page snapshot: in PowerShell run `$env:INSTA_UPDATE='always'; cargo test --all-features --test cmd_report; Remove-Item Env:INSTA_UPDATE`, then read `git diff tests/snapshots/cmd_report__report_html.snap` and check that only lines this task means to change are different.
  7. Run the four checks of AGENTS.md.
- accept: src/report/page/style.css has the rules for h4, code, .ms-mark, .ms-id, details.chain, .chain-line, .sr and .arch-label, the narrow-window rule hides .ms-id, the print rule shows .sr; the colour test and all report tests pass; the snapshot is updated
- verify: cargo test --all-features --test cmd_report --test report_colours
- verified: 2026-11-26
- commit: 07ce428
- by: Reynan Henry Tandoyo
- note (2026-11-22, claude-main): added
- note (2026-11-26, t229-haiku): done — Added the v1.1 page rules to style.css (milestone marker and ID, code, h4 subheadings, chain part, archive label, screen-reader class), hid the milestone ID under 560 px, showed the screen-reader class in print and updated the report snapshot; all 1041 tests pass, and fmt and both clippy checks pass.

### T-247 · The labels inside Show chain use the small label style
- status: done
- owner: t247-haiku
- priority: P2 · difficulty: 1 · type: bug · area: report
- milestone: T-228
- why: the Before this task and After this task labels of Show chain show as plain text, because the label rule in style.css only covers .more
- summary: The two group names inside Show chain look like the other small labels on the page.
- depends: T-243
- files: src/report/page/style.css, tests/snapshots/cmd_report__report_html.snap
- read: src/report/page/chain_html.rs
- steps:
  1. Read fn push_chain in src/report/page/chain_html.rs: the labels are <p class="lbl">.
  2. In src/report/page/style.css change the selector `.more .lbl{` to `.more .lbl, details.chain .lbl{` and change nothing else.
  3. Run cargo test --all-features --test cmd_report; the snapshot test shows the one changed CSS line; accept it (PowerShell: $env:INSTA_UPDATE='always'; cargo test --all-features --test cmd_report) and check that git diff of the snapshot shows only that line.
  4. Run the four checks of AGENTS.md.
- accept: style.css styles p.lbl inside details.chain like the labels under More; the snapshot differs only in that CSS line
- verify: cargo test --all-features --test cmd_report
- verified: 2026-11-27
- commit: 55ba68d
- by: Reynan Henry Tandoyo
- note (2026-11-27, claude-main): added
- note (2026-11-27, t247-haiku): forced claim (files held by T-238)
- note (2026-11-27, t247-haiku): done — Extended the .more .lbl rule to details.chain .lbl so the Show chain labels use the small label style, and the snapshot differs only in that CSS line; cargo test --all-features passes 1095 tests with 0 failed, and fmt and clippy pass on Windows and Linux; I claimed with --force because T-238 (t238-haiku) lists the same snapshot file, and the dossier guard refused my first style.css edit with entries that did not conflict, so I made the same edit again as the task says.

## Questions

### T-206 · When the new page is built from a real project: can you and your colleague each say within a minute what is done, what was cancelled, what is left and what needs you? Answer after T-203, before the 1.0.0 release (T-205).
- status: done
- owner: claude-main
- type: question
- about: T-195
- note (2026-11-21, claude-main): added
- note (2026-11-21, claude-main): answer — Owner 2026-11-21: yes, the page built from this project looks ok for now. Change: Last 7 days shows an entry's summary instead of its title when it has one (T-218).

## Ideas

### T-192 · Several categories per entry: let `area` hold more than one value (for example ui and math), with list, next and tree matching any of them
- status: open
- owner: —
- type: idea
- note (2026-11-21, claude-main): added

### T-193 · Review flag: work finished by a low-level worker is marked for a stronger agent to review before it counts as done
- status: open
- owner: —
- type: idea
- note (2026-11-21, claude-main): added

### T-194 · Task templates: `todo add --template bug` asks for the fields that kind of task needs
- status: open
- owner: —
- type: idea
- note (2026-11-21, claude-main): added

### T-207 · All-projects page in the new design: an index with one row per project (name, description, bar, needs you, last activity) that links to each project's own page
- status: open
- owner: —
- type: idea
- about: T-195
- note (2026-11-21, claude-main): added

### T-219 · Flaky test: cmd_verify a_check_that_outlives_verify_timeout_seconds_counts_as_failed hung once under full-suite load on Windows (seen by t218-haiku 2026-11-21) and left todo.exe and pwsh running; passed alone and on rerun
- status: open
- owner: —
- type: idea
- note (2026-11-21, claude-main): added
- note (2026-11-22, claude-main): Also seen in CI on main 2026-11-21 (run 44908893803, windows-latest): cmd_verify a_check_that_ends_normally_leaves_no_child_process_running failed at tests/cmd_verify.rs:1341; the same commit passed on dev. Both flakes are process timing in cmd_verify on Windows.
- note (2026-11-27, claude-main): 2026-11-26: t233 worker saw two more cmd_verify timing failures in full runs under parallel load; both passed on rerun
