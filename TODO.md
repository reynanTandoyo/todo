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
- status: open
- owner: —
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
- reqs: R-GIT-1
- note (2026-11-21, claude-main): added

### T-195 · A status page made for people
- status: open
- owner: —
- priority: P1 · type: milestone
- goal: The owner and a colleague open one page and see within a minute what is done, what was cancelled and why, what is still to do, and what needs them
- why: the v0.7 page shows what the tool knows instead of what a reader asks: too much text, too little context, and a tree that loses finished work
- note (2026-11-21, claude-main): added

### T-196 · Requirements for the new status page
- status: open
- owner: —
- priority: P0 · difficulty: 3 · type: docs · area: docs
- milestone: T-195
- why: the v0.7 page was specified in words only and came out wrong; this time the approved mockup is the reference and the requirements must describe exactly that page
- read: docs/report-mockup.html, docs/requirements.md, docs/format.md
- accept: docs/requirements.md describes the page shown in docs/report-mockup.html (approved by the owner on 2026-11-21): R-REPORT-1 is rewritten for the new layout (header, overall line and bar, the tree with done and cancelled entries in place, one line per entry, the second line, summary and technical details, Other work, and the closed parts Ideas, Earlier work and For the planner); R-REPORT-4 (Needs you), R-REPORT-5 (Last 7 days) and R-REPORT-6 (view buttons and light or dark mode) are new; R-WRITE-19 defines the `summary` field; the settings table gains `description`; the five words for people are fixed: to do (open), in progress (claimed), waiting (blocked), done, cancelled (dropped); the rule "no control that changes TODO.md" stays and view controls are allowed; a small inline script is allowed and the page must be readable without it; R-REPORT-3 says the planning state moves into For the planner; `report --html --all` keeps its v0.8 content and is out of scope; docs/format.md lists `summary` with its place in the field order and the format stays version 3; section 11 lists v1.0
- reqs: REL-4, NFR-8
- note (2026-11-21, claude-main): added
- note (2026-11-21, claude-main): Owner 2026-11-21: R-REPORT-6 also covers the colour-blind colours button and the four colour sets (light, dark, each normal and colour-blind); R-REPORT-1 says a state is never shown by colour alone. docs/report-mockup.html was updated to show both.
- note (2026-11-21, claude-main): Review 2026-11-21: (1) R-REPORT-2 says the --all page shows each project's lists "as in R-REPORT-1"; when R-REPORT-1 is rewritten, copy the v0.8 list description into R-REPORT-2 so --all keeps its content. (2) Define how a task waits for a question (Needs you, T-199; the mockup's "waits for this answer"): a task whose depends names the question. (3) Define the page's words for a task given back with stuck (waiting, with "given back" on its second line, as in the mockup).

### T-197 · summary field: what an entry is about, written for people
- status: open
- owner: —
- priority: P1 · difficulty: 2 · type: feature · area: cli
- milestone: T-195
- why: titles and accept lines are written by agents for agents; the page needs one or two plain sentences per entry that a colleague understands
- depends: T-196
- accept: `add`, `set`, `add --from`, `apply` and the children of `split` take `--summary` (text, `-` or `@path`) and write `- summary:` after `why` in the field order; it is allowed on tasks, milestones, questions and ideas; `set --unset summary` removes it; `show --json` and `schema` list it; `check` warns, without changing the exit code, about each task or milestone that is not done or dropped and has no summary; `brief` does not print it; a file written by todo 0.8 that has no summary anywhere works unchanged; a test covers each case
- reqs: R-WRITE-19, TEST-4
- note (2026-11-21, claude-main): added

### T-198 · Page data: states in plain words, counts, dates and the full tree
- status: open
- owner: —
- priority: P1 · difficulty: 3 · type: feature · area: report
- milestone: T-195
- why: the page can only keep finished work in its place if the data behind it is one tree with every entry, its state and its dates
- depends: T-196
- read: docs/report-mockup.html, src/report/model.rs, tests/fixtures/report_basic.md
- accept: the report model (src/report/model.rs) holds one tree with every entry of TODO.md, done and dropped ones included, each with one of five states: to do, in progress, waiting, done, cancelled; the overall numbers are "N of M tasks done" where M leaves out cancelled tasks, plus a count per state; each milestone has the same numbers over every task below it, a state (not started, in progress, finished) and, when finished, the date of its done note; a done entry carries the date and text of its last done note as its result, a cancelled one the date and reason of its drop note, a waiting one its latest reason, an in-progress one its owner and the age of the claim; milestones are ordered in progress, then not started, then finished, keeping file order inside each group; tasks without a milestone form the group Other work; finished trees from the archive form the group Earlier work; milestones, questions and ideas are never counted as tasks; unit tests cover each rule with an invented fixture
- reqs: R-REPORT-1, R-KIND-3, TEST-1
- note (2026-11-21, claude-main): added
- note (2026-11-21, claude-main): Review 2026-11-21: src/report/all.rs uses model::build_page and Page for the --all page; keep what it needs (or give --all its own copy) so report --html --all keeps its v0.8 content and tests.

### T-199 · Needs you: questions, tasks given back and work without activity
- status: open
- owner: —
- priority: P1 · difficulty: 2 · type: feature · area: report
- milestone: T-195
- why: the first thing a person asks is whether anything waits for them, and today that is spread over three sections
- depends: T-198
- read: docs/report-mockup.html, src/report/model.rs
- accept: the model has a list Needs you with, in this order: every open question, with the entry it is about and the tasks that wait for it; every task given back with `stuck`, with the problem and the date; every in-progress task whose claim is stale (older than `claim_ttl_hours`), with the owner and the age; the heading shows the count; an empty list reads "Nothing needs you"; each row names the entry by title, not by ID alone; tests cover each kind and the empty case
- reqs: R-REPORT-4
- note (2026-11-21, claude-main): added

### T-200 · Last 7 days: what was finished, cancelled and added
- status: open
- owner: —
- priority: P1 · difficulty: 2 · type: feature · area: report
- milestone: T-195
- why: a person who looked last week wants to see only what changed since then
- depends: T-198
- read: docs/report-mockup.html, src/report/model.rs
- accept: the model has a list of the events of the last 7 days, newest first, taken from note dates: task done, task cancelled with its reason, milestone finished with its task count, and entries added; several entries added to one milestone on one day are one row ("5 tasks added to Scene loading"); archived entries are included; at most 15 rows are shown and the rest sit behind one closed "earlier this week" part; no event in 7 days reads "Nothing changed in the last 7 days"; tests use a fixed clock
- reqs: R-REPORT-5
- note (2026-11-21, claude-main): added

### T-201 · description setting: one sentence about the project for the page header
- status: open
- owner: —
- priority: P2 · difficulty: 2 · type: feature · area: config
- milestone: T-195
- why: a colleague who opens the page needs to know what the project is before reading its tasks
- depends: T-196
- accept: `description = "..."` in todo.toml is shown under the project title on the page; without it the line is left out; line breaks become spaces; the README settings table lists the key; a test covers both cases
- reqs: R-CFG-2, R-REPORT-1
- note (2026-11-21, claude-main): added
- note (2026-11-21, claude-main): Review 2026-11-21: `report --json` only returns path, bytes and opened (docs/json.md), so the description is not added there; removed from accept.

### T-202 · Page layout: header, overall bar, tree, one line per entry, details on click
- status: open
- owner: —
- priority: P1 · difficulty: 3 · type: feature · area: report
- milestone: T-195
- why: this is the page itself; it must look and read like the approved mockup, not like a dump of fields
- depends: T-198, T-197, T-199, T-200, T-201
- read: docs/report-mockup.html, src/report/html.rs, src/report/style.css, tests/cmd_report.rs
- accept: `todo report --html` writes a page with the structure, wording and order of docs/report-mockup.html: header (title, description, date); the overall line with one bar in five parts and the legend; Needs you; Last 7 days; All work as one tree of milestones (title, state, "N of M done", small bar, goal) with nested milestones and tasks; unfinished milestones open, finished ones closed. An entry is one line: state mark, title, date for done and cancelled, small ID. A second line appears only for in progress (owner, since when, "no activity" when stale), waiting (the reason) and cancelled (the reason). A click opens the summary, for a done entry its result, and a closed part Technical details with level, priority, area, files, check and whether it passed, commit, who finished it, dependencies and the notes; an entry without a summary shows its why instead. Below the tree, closed: Ideas, Earlier work, and For the planner, which holds the v0.8 planning state and the per-area counts. The v0.8 sections Done and Dropped and the raw field lists are gone. One column, at most 820 px wide, system font, readable on a narrow window and when printed; all text HTML-escaped (test: a title containing `<script>`); the snapshot test uses a new invented fixture that has every state, a nested milestone, a question, a task given back and a stale claim; `report --html --all` still passes its tests
- reqs: R-REPORT-1, R-REPORT-3, TEST-1
- note (2026-11-21, claude-main): added
- note (2026-11-21, claude-main): Owner 2026-11-21 (red-green colour blind): a state is never shown by colour alone. Every entry has its own mark shape (done ✓, in progress ●, waiting ◐, to do ○, cancelled ✕), the second line names the state in words, and the legend under the bar pairs each mark with its word and count. All colours come from CSS variables so T-203 can switch the set.

### T-203 · View buttons, light or dark mode, and colour-blind colours
- status: open
- owner: —
- priority: P1 · difficulty: 2 · type: feature · area: report
- milestone: T-195
- why: the owner wants to narrow the tree to what is left or what was done, and to switch between light and dark, without anything on the page being able to change TODO.md
- depends: T-202
- read: docs/report-mockup.html
- accept: the page has the buttons Everything, Still to do, Done and Cancelled, which hide the entries that do not match and every milestone left without a visible entry, and open the milestones that have one; Expand all and Collapse all; one button that switches between light and dark mode; and one button "Colour-blind colours". There are four colour sets, one for each pair of mode and colour-blind setting, as in docs/report-mockup.html: the normal sets use green, blue, amber, grey and red, tuned separately for the white and the dark background; the colour-blind sets (red-green) use only blue for done, orange for in progress, orange with dark stripes in the bar for waiting, light grey for to do and dark grey (near white on the dark background) for cancelled. In every set each state colour has a contrast of at least 3 to 1 against its background, and a test computes that for all four sets. The mode follows the system setting until the button is used; both choices are remembered in the browser (a failure to store them is ignored). All of it is one small inline script with no network access; without scripts the page shows everything in the system mode with the normal colours and the buttons are hidden; the buttons are left out when printing; nothing on the page writes a file or runs a command; the snapshot covers the markup and a test checks that the script names no address
- reqs: R-REPORT-6, NFR-4
- note (2026-11-21, claude-main): added
- note (2026-11-21, claude-main): Owner 2026-11-21: the owner is red-green colour blind. A button switches the colour scheme only; light and dark mode each need their own colours that fit a white or a black background, in the normal and in the colour-blind scheme.

### T-204 · Performance and size of the new page
- status: open
- owner: —
- priority: P1 · difficulty: 2 · type: test · area: report
- milestone: T-195
- why: the new page keeps every finished entry in the tree, so it must be measured again on a large list
- depends: T-203
- accept: on the generated 5,000-entry milestone-tree file `report --html` finishes in under 500 ms in a release build and the page stays under 5 MB; the existing report performance test asserts both and runs in CI; the measured numbers are recorded in a note
- reqs: NFR-3, TEST-9
- note (2026-11-21, claude-main): added

### T-205 · Planner rules for summaries, documentation and release 1.0.0
- status: open
- owner: —
- priority: P1 · difficulty: 2 · type: docs · area: release
- milestone: T-195
- why: the page is only as readable as the texts the planner writes, and the release makes the new page the one people get
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
