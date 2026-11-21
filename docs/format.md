# TODO.md format, version 3

The reference for the file `todo` reads and writes. The parser is built from this document and
tested against every rule in it. Anything this document doesn't describe is kept exactly as it is
and otherwise ignored. Version 3 is written from todo 0.8 on; §18 says what changed in each version.

## 1. Example

An invented example, not real data:

```markdown
<!-- todo-format: 3 -->
<!-- todo-tool: 0.8.0 -->
# Example project — tasks

Anything up here is kept exactly as written.

## Now

### T-001 · Develop the renderer
- status: open
- owner: —
- priority: P1 · type: milestone · area: renderer
- goal: the test scene renders with lighting at 60 frames per second
- note (2026-11-15, planner): added

### T-002 · Diffuse term in the C shader
- status: claimed
- owner: worker-1
- claimed: 2026-11-15T14:03+01:00
- priority: P1 · difficulty: 1 · type: feature · area: renderer
- milestone: T-001
- why: the renderer has no shading stage yet
- files: src/shader.c, tests/shader_test.c
- read: src/light.h
- steps:
  1. In src/shader.c, add the function diffuse() that returns max(dot(n, l), 0) times the light colour.
  2. Call diffuse() from shade() for every light.
  3. In tests/shader_test.c, add a test: a light straight above a flat surface gives full brightness.
- accept: the test scene shows lit surfaces
- verify: make test
- note (2026-11-15, planner): added
- note (2026-11-15, worker-1): the light direction is flipped in view space

### T-003 · GPU compiler
- status: open
- owner: —
- priority: P2 · difficulty: 4 · type: feature · area: renderer
- milestone: T-001
- depends: T-002
- files: src/gpu/
- note (2026-11-15, planner): added

### T-004 · Shadow map pass
- status: blocked
- owner: —
- priority: P2 · difficulty: 1 · type: feature · area: renderer
- milestone: T-001
- files: src/shadow.c
- attempts: 1
- note (2026-11-15, planner): added
- note (2026-11-16, worker-2): stuck — src/shadow.c does not exist yet; the steps assume it does

## Questions

### T-005 · Should the C shader support shadows in its first version?
- status: open
- owner: —
- type: question
- about: T-002
- note (2026-11-15, worker-1): added

## Ideas

### T-006 · Hot-reload shaders while the scene runs
- status: open
- owner: —
- type: idea
- about: T-001
- note (2026-11-15, worker-1): added
```

## 2. Text and lines

- The file is UTF-8. A byte-order mark at the start is allowed and kept.
- Each line ends with LF or CRLF and keeps its own ending. Lines the tool adds use the ending most
  lines use; LF on a tie or when no line has an ending.
- The last line may have no ending, and keeps not having one. When the tool adds lines after such a
  line, that line gets an ending and the new last line has none.
- Matching ignores the line ending. Values and titles are trimmed of surrounding spaces.

## 3. Ignored regions

Lines inside these regions never start a section, a task, a field or a note:

- **Fenced code blocks.** A line that starts, after up to 3 spaces, with 3 or more backticks or
  tildes opens a fence. The fence closes at a line made only of the same character, at least as
  many of them, after up to 3 spaces, plus optional trailing spaces. A fence still open at the end
  of the file makes the file unsafe to write (§13).
- **HTML comments.** A line that starts, after up to 3 spaces, with `<!--` opens a comment, which
  ends on the first line containing `-->` (possibly the same line). The format line, the tool line
  and the managed-block markers are comments too; they are recognised before this rule applies.

## 4. Format line and tool line

**Format line.** `<!-- todo-format: 3 -->` on its own line, before the first section or task
heading, names the format version. A file without it is version 1 (in version 1 the line was
optional), so every file from version 2 on has it. `todo init` writes it, and a migration raises it
(§18).

**Tool line.** `<!-- todo-tool: 0.8.0 -->` on its own line, directly after the format line, records
the highest todo version that has written the file. When the file has no format line, it is the
first line of the file.

- `init` writes it. A command that writes the file adds it when it is missing, or raises it in the
  same write when its own version is higher; it is never lowered. Read commands never write it.
- Versions are `MAJOR.MINOR.PATCH`, compared number by number: `0.10.0` is above `0.9.0`.
- A file without the line counts as written by todo 0.5.0 or older.
- A value that is not `MAJOR.MINOR.PATCH` is kept as it is, and reported as a warning.
- A tool older than the stamp, on a format it knows, works normally and warns on standard error
  that it is behind, naming `todo help update`.

**Which versions a tool accepts.**

- An **older** format is read normally. The first write migrates it to the current format, in the
  same write (§18).
- A format the tool **doesn't know** (a higher version) stops every command that interprets or
  changes task content: exit 4, error `TOOL_TOO_OLD`, with a message naming the todo version stamped
  in the file and the running version, and saying to update the todo executable first
  (`todo help update`). Nothing is written. These still work: `where`, which shows the format
  version, the tool line and the running version but counts no tasks; `help`, `schema`,
  `completions` and `--version`; and the commands that never read task content: `status`,
  `history`, `diff`, `lock`, `unlock`, `agents`, `skill`, `hooks`, `guard`, `session-end`, and
  `merge-driver` (which falls back to a plain text merge, R-GIT-2). This rule starts with todo 0.6;
  todo 0.5 and older read a newer format with a warning but don't write it.

## 5. Structure

- **Preamble**: the lines before the first section heading or task heading. Only `init`,
  `agents install` and `migrate` (inside the managed block, §12) ever change it, apart from the
  format and tool lines (§4), which write commands keep up to date.
- **Section heading**: a line starting with `## ` (exactly two `#` and a space). The section runs to
  the next section heading or the end of the file. Its name is the rest of the line, trimmed. When
  a command names a section, a leading number such as `12.` or `3)` is ignored, so `Phase 2` matches
  `## 12. Phase 2`.
- **Task heading**: a line starting with `### ` that matches §6. The task belongs to the section
  above it, or to no section if it comes before the first one.
- **Other headings** (`# `, `#### ` and deeper, and `### ` headings that aren't task headings) have
  no special meaning, except that any `### ` heading ends the task above it.

## 6. Task heading

```
### <ID><separator><title>
```

- **ID**: the configured prefix (default `T`, matched ignoring case), a hyphen, and one or more
  digits. Leading zeros don't matter: `T-7`, `t-007` and `T-007` are the same task. The tool writes
  the prefix in upper case and pads the number to `id_width` (default 3) digits; larger numbers
  simply get longer (`T-1000`).
- **Separator**: canonical ` · ` (space, middle dot U+00B7, space). Also accepted when reading:
  ` - `, ` — ` (em dash), ` – ` (en dash), and `: ` directly after the ID.
- **Title**: the rest of the line, trimmed; must not be empty.
- A `### ` heading that starts with the prefix and digits but doesn't fit these rules
  (`### T12 Fix`, `### T-12Fix`, `### T-12 · ` with no title) is an ordinary heading, and `todo check`
  reports it as a probable mistake. A `### ` heading with a different prefix (`### ADR-001 · …`) is
  an ordinary heading.

## 7. Task block

- A task block runs from its heading to just before the next line that starts with `## ` or `### `
  outside ignored regions, or to the end of the file. Blank lines at its end don't belong to it.
- Inside a block, a line that starts at column 0 with `- key: value` is a **field**, and a line
  starting with `- note (` that fits §9 is a **note**. The numbered lines directly under a
  `- steps:` field belong to that field (§8). Everything else (prose, other indented lists, tables,
  `####` headings, code blocks) is kept and shown by `show`, but has no meaning to the tool.
- A field key starts with a lower-case letter and contains only lower-case letters, digits, `_` and
  `-`. The value is the rest of the line after `: `, trimmed; it may be empty.

## 8. Fields

| Field | Value | When missing |
|---|---|---|
| `status` | `open`, `claimed`, `blocked`, `done` or `dropped` | `open` |
| `owner` | an agent name (§11); `—` means nobody (`-`, `none` and empty are read the same way) | nobody |
| `claimed` | local date and time with UTC offset, `2026-11-15T14:03+01:00` | no claim time; a claimed task without one counts as stale |
| `session` | the agent session that made the claim (an opaque ID such as Claude Code's `session_id`); written by `claim`, `next --claim` and `unblock` when a session is known, removed whenever `claimed` is removed | no session |
| `priority` | one of the configured priorities (default `P0`, `P1`, `P2`, `P3`, most urgent first); may carry other parts, see below | no priority; sorted after all priorities |
| `difficulty` | the task's level: one of the configured `difficulties` (default `1` to `5`, easiest first; `easy`, `medium`, `hard` before format 3); how much is left to find out, guess or decide, not how urgent | none; `add` requires it of a task, `next` never hands out a task without it, and `todo plan` lists it |
| `type` | free text, or one of the configured `types`; `milestone`, `question` and `idea` are always allowed and set the kind (§16) | none (a task) |
| `area` | free text, or one of the configured `areas` | none |
| `milestone` | the ID of the parent milestone (§17) | top-level |
| `about` | on a question or idea: the ID of the milestone or task it is about | about nothing in particular |
| `goal` | on a milestone: what the piece of work is meant to achieve | none; `check` reports a milestone without a goal |
| `why` | free text: what the task is for | none |
| `summary` | free text for people: what the entry is about, in one or two plain sentences, without IDs or file names; allowed on every kind of entry and shown by the HTML report (from todo 1.0) | none; `check` warns about an unfinished task or milestone without one |
| `final` | on a task above the target level: why it stays at that level on purpose | none |
| `depends` | task IDs separated by commas; may name a milestone, which counts as finished when it is done or dropped | none |
| `files` | the paths, folders and patterns the task may change, separated by commas (§19) | none; the task may change any file and holds none |
| `read` | the paths the worker should look at but not change, separated by commas (§19) | none |
| `problem`, `fix`, `accept` | free text | none |
| `steps` | numbered steps on the indented lines under `- steps:`; the field line itself has no value (see below) | none |
| `verify` | one command line that checks the task; run by todo through `verify_shell` | none; nothing is run |
| `attempts` | a whole number: how often the task failed, counted by `stuck` and failed checks | `0` |
| `verified` | local date when the task's `verify` command last passed after it was done | not verified |
| `commit` | commit hashes separated by commas | none |
| `by` | the person who finished the task, written by `done` (git name after `.mailmap`) | not recorded |
| any other key | kept as written; shown in JSON under `fields` | — |

- Each of these fields appears at most once per task. A task with a repeated field can be read, but
  commands that change it are refused until it is fixed (§13).
- **Steps list.** The `steps` field is the line `- steps:` followed directly by its step lines. A
  step line starts with two spaces, a number, `.` and a space: `  1. Open src/shader.c.` The list
  ends at the first line that isn't a step line. The tool numbers the steps `1.`, `2.`, … and writes
  each on its own line with the file's usual line ending; when reading, the numbers don't matter
  (step lines with 2 to 4 leading spaces are read) and `show --json` returns the steps without them.
  Setting `steps` replaces the field line and all its step lines; removing it removes them all. Any
  other indented line under a field, or a numbered line not directly under `- steps:`, stays plain
  text. A value written by hand after `- steps:` on the same line is read as a step before the list,
  and `todo check` reports it. A `steps` part of the combined priority line is read the same way;
  setting or removing the steps takes that part off the line and writes the list below `- steps:`.
- **Combined priority line**: `- priority: P1 · difficulty: 3 · type: bug · area: ui`. The value
  is split at ` · `. The first part is the priority; every other part is `key: value` and counts as
  that field (so `difficulty`, `type` and `area` may live here or on their own lines, but not both).
  The tool edits a part where it is. A new `difficulty`, `type` or `area` goes into the combined
  line when the task has a priority line, after the nearest existing part that comes before it in
  the order of §10, and onto its own line otherwise. A value that contains ` · ` cannot stay on the
  combined line: the tool writes it on a line of its own, for every part but `priority`, which is
  refused (`BAD_VALUE`).
  A part that is not `key: value` (for example `· see the design notes`) is never dropped: it is kept
  as written, in its place, and `todo check` reports it as a warning. If the priority is removed, such
  parts stay on one line of their own where the priority line was.

## 9. Notes

```
- note (2026-11-15, agent-ui-1): the click handler runs twice
```

The date is the local date of writing, followed by the agent name and the text. Notes are
append-only: the tool adds a note after the last line of the block and never changes or removes one.
Notes the tool writes itself:

| Event | Note text |
|---|---|
| `add`, `ask`, `idea` | `added` |
| `done` | `done` or `done — <note>` |
| `block` | `blocked — <reason>` |
| `unblock` | `unblocked` or `unblocked — <note>` |
| `drop` | `dropped — <reason>` |
| `drop --tree`, on each entry below the milestone | `dropped with <milestone ID> — <reason>` |
| `promote` | `promoted from idea` |
| `done --force` on a milestone with unfinished entries below | `forced done (<N> entries below unfinished)` |
| `reopen` | `reopened — <reason>` |
| `answer` | `answer — <text>` |
| `release --session` | `released — session <id> ended` |
| claim of a stale claim | `took over from <previous owner> (claim was <N> h old)` |
| any change made with `--force` | `forced <command> (owner was <owner>)` |
| `stuck` | `stuck — <problem>` |
| `done` whose `verify` command failed | `verify failed — <last line of the output>` |
| `done` whose `verify` command failed for the `verify_attempts`-th time | `stuck — verify failed <N> times: <last line of the output>` |
| `verify --wave`, on a task whose command failed | `stuck — verify failed after done: <last line of the output>` |
| `done --no-verify --force` | `forced done without verify` |
| `done --force` with a commit outside the task's files | `forced done outside files: <files>` |
| `claim --force` of a task whose files another agent holds | `forced claim (files held by <ID>)` |
| `split`, on the task that became a milestone | `split into <ID>, <ID>, …` |

A task is *given back* when it is blocked and the latest of its `blocked — …` and `stuck — …` notes
is a `stuck — …` note; that note's text is its reason.

## 10. Where the tool writes

- **Order of fields.** When a field is missing and must be added, it goes after the nearest existing
  field that comes before it in this order, or directly under the heading if there is none:
  `status`, `owner`, `claimed`, `session`, `priority`, `difficulty`, `type`, `area`, `milestone`,
  `about`, `goal`, `why`, `summary`, `final`, `depends`, `files`, `read`, `problem`, `fix`, `steps`,
  `accept`, `verify`, `attempts`, `verified`, `commit`, then other fields. Removing a field removes its line
  only, and for `steps` its step lines too.
- **New tasks** go at the end of their section: after the section's last non-blank line, with one
  blank line before the heading. Blank lines that followed the section stay after the new task. A
  new task is written as:

  ```
  ### T-014 · <title>
  - status: open
  - owner: —
  - priority: <priority> · difficulty: <difficulty> · type: <type> · area: <area>    (only the parts given)
  - milestone: … / - about: … / - goal: … / - why: … / - summary: …    (each only if given)
  - depends: … / - files: … / - read: … / - problem: … / - fix: …    (each only if given)
  - steps:    (only if given, followed by its numbered step lines)
  - accept: … / - verify: …    (each only if given)
  - <other fields given with --field>
  - note (<date>, <agent>): added
  ```

- **Values** are single lines; line breaks in input become single spaces.

## 11. Agent names

1 to 64 characters from `A–Z a–z 0–9 . _ -`, compared ignoring case.

## 12. Managed block

`<!-- todo:begin -->` and `<!-- todo:end -->`, each on its own line, enclose text the tool owns: the
agent rules. Between them is ordinary Markdown. `agents install` replaces only the lines between the
markers, and `todo check` reports when they differ from the current rules text. The same markers are
used in `AGENTS.md`, `CLAUDE.md` or any other file.

## 13. When the tool refuses to write

Write commands are refused (exit 4, error `UNSAFE_FILE`, message `TODO.md:<line>: <reason>`) when:

- the file isn't valid UTF-8;
- it contains git conflict markers (`<<<<<<< `, `=======` or `>>>>>>> ` at the start of a line,
  outside ignored regions);
- a code fence or HTML comment is never closed.

A command that changes one task is also refused when that task:

- shares its ID with another task (in this file or the archive);
- repeats a field;
- has a `status` value that isn't one of the five.

Read commands still work in all these cases and print the same messages as warnings.

A format line naming a version the tool doesn't know is not in this list: it stops reads as well as
writes, with exit 4 and `TOOL_TOO_OLD` (§4).

## 14. Archive file

`TODO-archive.md` (setting `archive_file`) sits next to `TODO.md` and uses this same format. When
`archive` moves a task there, the whole block goes to the end of the section with the same name in
the archive; missing sections are added at the end. A new archive file starts with the format line,
the tool line and `# Archive`. The archive is changed only under the lock of `TODO.md`, and its IDs
count when the next ID is chosen. If `TODO.md` cannot be replaced after the archive was written, the
archive is put back as it was (a new archive is removed), so a task is never in both files. It carries its own format and tool lines and is migrated in the
same step as `TODO.md` (§18). Entries that belong to a milestone are archived only with their whole
tree (§17).

## 15. Lenient reading, canonical writing

| Read | Written by the tool |
|---|---|
| `T-7`, `t-007`, `T-007` | `T-007` |
| ` · `, ` - `, ` — `, ` – `, `: ` after the ID | ` · ` |
| owner `—`, `-`, `none`, empty | `—` |
| `type` and `area` on their own lines | left where they are |
| step numbers out of order, or 3 to 4 leading spaces | renumbered `1.`, `2.`, … with two spaces, only when `steps` is set again |
| `\` in `files` and `read` | left as written; compared as `/` (§19) |

The tool writes canonical forms only on lines it changes; it never tidies other lines on its own.
`todo check --fix` can tidy them on request.

## 16. Kinds of entry

Every task block is an entry of exactly one kind, taken from its `type`:

| `type` | Kind | Handed out by `next` | Notes |
|---|---|---|---|
| `milestone` | milestone | never | a piece of work with a `goal`, divided into tasks; never claimed |
| `question` | question | never | may name the entry it is about with `about` |
| `idea` | idea | never | needs no priority; may name an entry with `about`; `promote` turns it into a task |
| anything else, or none | task | yes | the ordinary unit of work |

Milestones and tasks are work; questions and ideas are not. Done and dropped entries of every kind
are shown apart from open work. The kind is never written as a field of its own. From format 3 only
tasks need a `priority` and a `difficulty`; the fields `final`, `steps`, `read`, `verify`,
`attempts` and `verified` belong to tasks, and `split` removes them when a task becomes a milestone.

## 17. Milestone tree

A milestone is a piece of work with a goal, divided into tasks: for example the milestone
`Develop the renderer` with the tasks `C shader` and `GPU compiler`. It is never a release or
version label.

- An entry's `milestone` field names its parent, which must be a milestone. A milestone may itself
  have a parent, so the tree can be any number of levels deep. An entry without `milestone` is
  top-level.
- The tree is logical: entries stay where they are in the file, in any section and in any order, and
  only the `milestone` field links them. Moving an entry to another parent changes that one line.
- A milestone may not be its own ancestor; `todo check` reports such a cycle.
- Progress of a milestone is `done/total` over every task below it at any depth: `done` counts done
  tasks, `total` every task that isn't dropped.

```markdown
### T-010 · Develop the renderer
- status: open
- owner: —
- priority: P1 · type: milestone
- goal: the test scene renders with lighting at 60 frames per second
- note (2026-11-18, planner): added

### T-011 · Lighting
- status: open
- owner: —
- priority: P1 · type: milestone
- milestone: T-010
- goal: point and directional lights match the reference images
- note (2026-11-18, planner): added

### T-012 · C shader
- status: open
- owner: —
- priority: P1 · difficulty: 4 · type: feature
- milestone: T-011
- why: the renderer has no shading stage yet
- note (2026-11-18, planner): added
```

## 18. Format versions and migration

| Version | Written by | Differences |
|---|---|---|
| 1 | todo 0.1 to 0.5 | the format line is optional |
| 2 | todo 0.6 and 0.7 | the format line is required; the tool line (§4); the kinds `milestone` and `idea` (§16); the fields `difficulty`, `milestone`, `about`, `goal` and `why` (§8) |
| 3 | todo 0.8 and later (todo 1.0 adds the field `summary`, §8, without a new version: an older todo keeps it as an unknown field) | the levels `1` to `5` as the default `difficulties`; the fields `steps` (a numbered list on indented lines), `read`, `verify`, `final`, `attempts` and `verified` (§8); `files` as the list of paths and patterns a task may change (§19); the notes of `stuck`, `split` and the verify command (§9) |

A tool migrates a file in an older format on its first write, and with `todo migrate`, one version
at a time, under the lock and in one write after a snapshot. Each step changes whole lines only.

- **1 to 2**: the format line is raised to `2`, or added as the first line (after a byte-order mark)
  when it is missing, and the tool line is added directly after it. Task lines stay as they are.
  Fields such as `why` or `milestone` written by hand in a version-1 file were unknown fields and
  are now read as the fields of §8; `todo check` reports any that don't fit.
- **2 to 3**: the format line is raised to `3`. When `todo.toml` sets no `difficulties` of its own,
  the difficulty of every entry that is not done or dropped is rewritten: `easy` to `2`, `medium`
  to `3` and `hard` to `4` (compared ignoring case), in place, whether it is a part of the combined
  priority line or a line of its own; only that part of the line changes. Done and dropped entries,
  and so the archive file, keep their old values as a record, and `todo check` doesn't report them.
  Any other difficulty is left as it is, and `todo check` reports it. Open tasks without a
  difficulty stay as they are; `todo plan` lists them for the planner. Fields such as `steps`,
  `read`, `verify`, `final`, `attempts` or `verified` written by hand in a version-2 file were
  unknown fields and are now read as the fields of §8 (a hand-written `- steps:` now owns the
  numbered lines directly under it); `files` values are now read as lists (§19), and nothing in them
  is rewritten.

The archive file is migrated in the same step. Read commands read every older version and never
write.

## 19. Paths in `files` and `read`

`files` lists what a task may change; `read` lists what its worker should look at but not change.
Both are lists separated by commas, each entry trimmed. An entry is one of:

- a **path** to a file, such as `src/shader.c`;
- a **folder**, ending in `/`, such as `src/gpu/`, which covers everything below it;
- a **pattern** with `*`, such as `src/*.c` or `tests/**/fixture_*.md`: `*` matches any characters
  except `/`, and `**` as a whole path part matches any number of folders.

Paths are relative to the folder that holds `TODO.md`. `/` and `\` are the same separator, and case
is ignored on Windows. A trailing `:<line>` (`src/main.rs:42`, as `todo scan` writes it) names a
line and is ignored when paths are compared. Paths may contain spaces; a comma can't be part of a
path. `todo check` reports an entry that is absolute or leaves the folder of `TODO.md` with `..`.

Two entries **overlap** when they are equal, when one lies under a folder named by the other, or
when one is a pattern that can match the other. Two patterns overlap unless their fixed parts (the
text before the first `*`) differ before either of them ends; this errs towards overlap. Two tasks
overlap when any entry of one overlaps any entry of the other. A task at or below the target level
names at most `level1_max_files` files (default 3) and no folder or pattern.
