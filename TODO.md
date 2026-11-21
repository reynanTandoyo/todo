<!-- todo-format: 3 -->
<!-- todo-tool: 0.8.1 -->
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
- status: open
- owner: —
- priority: P2 · type: milestone
- goal: Every problem found while using todo on real work is recorded here and fixed, and todo's own checks (check, align, plan) stay clean on this repository
- why: the tool is only trusted when its own reports have no false alarms
- note (2026-11-21, claude-main): added

### T-191 · align reports every done milestone as DONE_WITHOUT_COMMIT
- status: done
- owner: t191-haiku
- priority: P2 · difficulty: 1 · type: bug · area: git
- milestone: T-190
- why: a milestone is closed, not committed, so align exits 1 on a clean list (six false findings on this repository at v0.8.1)
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
- status: open
- owner: —
- priority: P1 · type: milestone
- goal: The owner and a colleague open one page and see within a minute what is done, what was cancelled and why, what is still to do, and what needs them
- why: the v0.7 page shows what the tool knows instead of what a reader asks: too much text, too little context, and a tree that loses finished work
- note (2026-11-21, claude-main): added

### T-196 · Requirements for the new status page
- status: done
- owner: claude-main
- priority: P0 · difficulty: 3 · type: docs · area: docs
- milestone: T-195
- why: the v0.7 page was specified in words only and came out wrong; this time the approved mockup is the reference and the requirements must describe exactly that page
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
- status: open
- owner: —
- priority: P1 · type: milestone · area: cli
- milestone: T-195
- goal: Every entry can carry a summary for people, written through todo and checked
- why: titles and accept lines are written by agents for agents; the page needs one or two plain sentences per entry that a colleague understands
- depends: T-196
- accept: `add`, `set`, `add --from`, `apply` and the children of `split` take `--summary` (text, `-` or `@path`) and write `- summary:` after `why` in the field order; it is allowed on tasks, milestones, questions and ideas; `set --unset summary` removes it; `show --json` and `schema` list it; `check` warns, without changing the exit code, about each task or milestone that is not done or dropped and has no summary; `brief` does not print it; a file written by todo 0.8 that has no summary anywhere works unchanged; a test covers each case
- reqs: R-WRITE-19, TEST-4
- note (2026-11-21, claude-main): added
- note (2026-11-21, claude-main): split into T-208, T-209, T-210

### T-208 · summary field: read it, show it and list it in schema
- status: done
- owner: t208-haiku
- priority: P1 · difficulty: 2 · type: feature · area: core
- milestone: T-197
- why: every other part of the summary field needs the parser to know it first
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
- status: open
- owner: —
- priority: P1 · difficulty: 2 · type: feature · area: cli
- milestone: T-197
- why: planners and agents must be able to write the summary through todo, never by hand
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
- note (2026-11-21, claude-main): added

### T-210 · summary field: check warns about unfinished work without one; brief leaves it out; docs/json.md
- status: open
- owner: —
- priority: P1 · difficulty: 2 · type: feature · area: cli
- milestone: T-197
- why: the page is only readable when every unfinished task and milestone has a summary, so check must point at the ones missing
- final: The owner's workers are Haiku 5.5 at xhigh effort, which finished level-3 tasks in v0.8 (T-148, T-149); this is one coherent change with full detail, and smaller pieces would not compile on their own.
- depends: T-208
- files: src/cli/check.rs, src/ops/brief.rs, docs/json.md, tests/field_summary_check.rs
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
- note (2026-11-21, claude-main): added

### T-198 · Page data: states in plain words, counts, dates and the full tree
- status: open
- owner: —
- priority: P1 · type: milestone · area: report
- milestone: T-195
- goal: The data behind the new page: every entry with its state, dates and texts, in one tree with counts
- why: the page can only keep finished work in its place if the data behind it is one tree with every entry, its state and its dates
- depends: T-196
- accept: the report model (src/report/model.rs) holds one tree with every entry of TODO.md, done and dropped ones included, each with one of five states: to do, in progress, waiting, done, cancelled; the overall numbers are "N of M tasks done" where M leaves out cancelled tasks, plus a count per state; each milestone has the same numbers over every task below it, a state (not started, in progress, finished) and, when finished, the date of its done note; a done entry carries the date and text of its last done note as its result, a cancelled one the date and reason of its drop note, a waiting one its latest reason, an in-progress one its owner and the age of the claim; milestones are ordered in progress, then not started, then finished, keeping file order inside each group; tasks without a milestone form the group Other work; finished trees from the archive form the group Earlier work; milestones, questions and ideas are never counted as tasks; unit tests cover each rule with an invented fixture
- reqs: R-REPORT-1, R-KIND-3, TEST-1
- note (2026-11-21, claude-main): added
- note (2026-11-21, claude-main): Review 2026-11-21: src/report/all.rs uses model::build_page and Page for the --all page; keep what it needs (or give --all its own copy) so report --html --all keeps its v0.8 content and tests.
- note (2026-11-21, claude-main): split into T-211, T-212

### T-211 · Page data: one entry with its state, date, second line and details
- status: open
- owner: —
- priority: P1 · difficulty: 2 · type: feature · area: report
- milestone: T-198
- why: every part of the new page shows entries the same way, so one function must decide an entry's state and texts
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
- note (2026-11-21, claude-main): added

### T-212 · Page data: the work tree with counts, order, Other work and Earlier work
- status: open
- owner: —
- priority: P1 · difficulty: 2 · type: feature · area: report
- milestone: T-198
- why: the page keeps finished work in its place only if the data is one tree with every entry and the right counts
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
- verify: cargo test --lib report::page
- note (2026-11-21, claude-main): added

### T-199 · Needs you: questions, tasks given back and work without activity
- status: open
- owner: —
- priority: P1 · difficulty: 2 · type: feature · area: report
- milestone: T-195
- why: the first thing a person asks is whether anything waits for them, and today that is spread over three sections
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
- verify: cargo test --lib report::page
- reqs: R-REPORT-4
- note (2026-11-21, claude-main): added

### T-200 · Last 7 days: what was finished, cancelled and added
- status: open
- owner: —
- priority: P1 · difficulty: 2 · type: feature · area: report
- milestone: T-195
- why: a person who looked last week wants to see only what changed since then
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
- verify: cargo test --lib report::page
- reqs: R-REPORT-5
- note (2026-11-21, claude-main): added

### T-201 · description setting: one sentence about the project for the page header
- status: done
- owner: t201-haiku
- priority: P2 · difficulty: 1 · type: feature · area: config
- milestone: T-195
- why: a colleague who opens the page needs to know what the project is before reading its tasks
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
- status: open
- owner: —
- priority: P1 · type: milestone · area: report
- milestone: T-195
- goal: The new page itself: it looks and reads like the approved mockup
- why: this is the page itself; it must look and read like the approved mockup, not like a dump of fields
- accept: `todo report --html` writes a page with the structure, wording and order of docs/report-mockup.html: header (title, description, date); the overall line with one bar in five parts and the legend; Needs you; Last 7 days; All work as one tree of milestones (title, state, "N of M done", small bar, goal) with nested milestones and tasks; unfinished milestones open, finished ones closed. An entry is one line: state mark, title, date for done and cancelled, small ID. A second line appears only for in progress (owner, since when, "no activity" when stale), waiting (the reason) and cancelled (the reason). A click opens the summary, for a done entry its result, and a closed part Technical details with level, priority, area, files, check and whether it passed, commit, who finished it, dependencies and the notes; an entry without a summary shows its why instead. Below the tree, closed: Ideas, Earlier work, and For the planner, which holds the v0.8 planning state and the per-area counts. The v0.8 sections Done and Dropped and the raw field lists are gone. One column, at most 820 px wide, system font, readable on a narrow window and when printed; all text HTML-escaped (test: a title containing `<script>`); the snapshot test uses a new invented fixture that has every state, a nested milestone, a question, a task given back and a stale claim; `report --html --all` still passes its tests
- reqs: R-REPORT-1, R-REPORT-3, TEST-1
- note (2026-11-21, claude-main): added
- note (2026-11-21, claude-main): Owner 2026-11-21 (red-green colour blind): a state is never shown by colour alone. Every entry has its own mark shape (done ✓, in progress ●, waiting ◐, to do ○, cancelled ✕), the second line names the state in words, and the legend under the bar pairs each mark with its word and count. All colours come from CSS variables so T-203 can switch the set.
- note (2026-11-21, claude-main): split into T-213, T-214

### T-213 · Page HTML: the All work tree, one line per entry, details on click
- status: open
- owner: —
- priority: P1 · difficulty: 2 · type: feature · area: report
- milestone: T-202
- why: the tree is most of the page and must read like the approved mockup, not like a dump of fields
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
- verify: cargo test --lib report::page
- note (2026-11-21, claude-main): added

### T-214 · Page HTML: the whole page, wired into report --html, with snapshot
- status: open
- owner: —
- priority: P1 · difficulty: 3 · type: feature · area: report
- milestone: T-202
- why: this puts the parts together into the page people open, replacing the v0.8 page for one project
- final: The owner's workers are Haiku 5.5 at xhigh effort, which finished level-3 tasks in v0.8 (T-148, T-149); this is one coherent change with full detail, and smaller pieces would not compile on their own.
- depends: T-213, T-199, T-200, T-201, T-210, T-209
- files: src/report/page/mod.rs, src/report/page/html.rs, src/report/page/style.css, src/report/html.rs, src/cli/report.rs, tests/cmd_report.rs, tests/fixtures/report_page.md, tests/snapshots/cmd_report__report_html.snap
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
- verify: cargo test --test cmd_report --test cmd_report_all
- note (2026-11-21, claude-main): added

### T-203 · View buttons, light or dark mode, and colour-blind colours
- status: open
- owner: —
- priority: P1 · difficulty: 2 · type: feature · area: report
- milestone: T-195
- why: the owner wants to narrow the tree to what is left or what was done, and to switch between light and dark, without anything on the page being able to change TODO.md
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
- verify: cargo test --test cmd_report --test report_colours
- reqs: R-REPORT-6, NFR-4
- note (2026-11-21, claude-main): added
- note (2026-11-21, claude-main): Owner 2026-11-21: the owner is red-green colour blind. A button switches the colour scheme only; light and dark mode each need their own colours that fit a white or a black background, in the normal and in the colour-blind scheme.

### T-204 · Performance and size of the new page
- status: open
- owner: —
- priority: P1 · difficulty: 1 · type: test · area: report
- milestone: T-195
- why: the new page keeps every finished entry in the tree, so it must be measured again on a large list
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
- verify: cargo test --release --test perf
- reqs: NFR-3, TEST-9
- note (2026-11-21, claude-main): added

### T-205 · Planner rules for summaries, documentation and release 1.0.0
- status: open
- owner: —
- priority: P1 · difficulty: 2 · type: docs · area: release
- milestone: T-195
- why: the page is only as readable as the texts the planner writes, and the release makes the new page the one people get
- final: Docs and release work done by the planner and one worker at the end; it depends on the owner's answer to T-206.
- depends: T-204,T-206
- accept: `todo help planner` tells the planner to write a `summary` in plain words for every task and milestone (what it is about, no IDs, no file names, at most two sentences), to write the done note as the result in one sentence, and to give a reason a person understands when dropping or giving back a task; the common rules block stays within 15 lines and the worker text within 10; `todo help report` describes the new page (it still says "no script, no buttons"); the README part for people describes the new page with its sections and buttons; docs/json.md documents the `summary` field in `show --json` and `schema` and drops the R-REPORT-3 remark from the `report --json` part; CHANGELOG.md has the 1.0.0 entry; Cargo.toml is 1.0.0; this repository's own entries have summaries, a new `todo.toml` has a description (and nothing else), and `todo check` is clean; T-206 is answered
- reqs: R-AGENT-1, REL-2, REL-5
- note (2026-11-21, claude-main): added
- note (2026-11-21, claude-main): Review 2026-11-21: depends on the question T-206 so 1.0.0 is not released before the owner has judged the page; this repository has no todo.toml yet.

## Questions

### T-206 · When the new page is built from a real project: can you and your colleague each say within a minute what is done, what was cancelled, what is left and what needs you? Answer after T-203, before the 1.0.0 release (T-205).
- status: open
- owner: —
- type: question
- about: T-195
- note (2026-11-21, claude-main): added

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
