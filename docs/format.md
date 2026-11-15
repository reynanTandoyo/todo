# TODO.md format, version 1

The reference for the file `todo` reads and writes. The parser is built from this document and
tested against every rule in it. Anything this document doesn't describe is kept exactly as it is
and otherwise ignored.

## 1. Example

An invented example, not real data:

```markdown
<!-- todo-format: 1 -->
# Example project — tasks

Anything up here is kept exactly as written.

## Now

### T-001 · Fix the save button
- status: claimed
- owner: agent-ui-1
- claimed: 2026-11-15T14:03+01:00
- priority: P1 · type: bug · area: ui
- files: src/save.rs
- accept: saving twice leaves one file
- note (2026-11-15, coordinator): added
- note (2026-11-15, agent-ui-1): the click handler runs twice

### T-002 · Add an export command
- status: open
- owner: —
- priority: P2 · type: feature · area: cli
- depends: T-001
- note (2026-11-15, coordinator): added

## Questions

### T-003 · Should exports include archived tasks?
- status: open
- owner: —
- type: question
- note (2026-11-15, agent-ui-1): added
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
  ends on the first line containing `-->` (possibly the same line). The format line and the
  managed-block markers are comments too; they are recognised before this rule applies.

## 4. Format line

`<!-- todo-format: 1 -->` on its own line, before the first section or task heading. It is
optional; a file without it is version 1. `todo init` writes it. A file with a higher version than
the tool knows can be read, with a warning, but not written.

## 5. Structure

- **Preamble**: the lines before the first section heading or task heading. Only `init` and
  `agents install` (inside the managed block, §12) ever change it.
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
  starting with `- note (` that fits §9 is a **note**. Everything else (prose, indented lists, tables,
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
| `type` | free text, or one of the configured `types`; `question` is always allowed | none |
| `area` | free text, or one of the configured `areas` | none |
| `depends` | task IDs separated by commas | none |
| `files` | free text | none |
| `problem`, `fix`, `accept` | free text | none |
| `commit` | commit hashes separated by commas | none |
| any other key | kept as written; shown in JSON under `fields` | — |

- Each of these fields appears at most once per task. A task with a repeated field can be read, but
  commands that change it are refused until it is fixed (§13).
- **Combined priority line**: `- priority: P1 · type: bug · area: ui`. The value is split at ` · `.
  The first part is the priority; every other part is `key: value` and counts as that field (so
  `type` and `area` may live here or on their own lines, but not both). The tool edits a part where
  it is. A new `type` or `area` goes into the combined line when the task has a priority line, and
  onto its own line otherwise.

## 9. Notes

```
- note (2026-11-15, agent-ui-1): the click handler runs twice
```

The date is the local date of writing, followed by the agent name and the text. Notes are
append-only: the tool adds a note after the last line of the block and never changes or removes one.
Notes the tool writes itself:

| Event | Note text |
|---|---|
| `add` | `added` |
| `done` | `done` or `done — <note>` |
| `block` | `blocked — <reason>` |
| `unblock` | `unblocked` or `unblocked — <note>` |
| `drop` | `dropped — <reason>` |
| `reopen` | `reopened — <reason>` |
| `answer` | `answer — <text>` |
| `release --session` | `released — session <id> ended` |
| claim of a stale claim | `took over from <previous owner> (claim was <N> h old)` |
| any change made with `--force` | `forced <command> (owner was <owner>)` |

## 10. Where the tool writes

- **Order of fields.** When a field is missing and must be added, it goes after the nearest existing
  field that comes before it in this order, or directly under the heading if there is none:
  `status`, `owner`, `claimed`, `session`, `priority`, `type`, `area`, `depends`, `files`, `problem`, `fix`,
  `accept`, `commit`, then other fields. Removing a field removes its line only.
- **New tasks** go at the end of their section: after the section's last non-blank line, with one
  blank line before the heading. Blank lines that followed the section stay after the new task. A
  new task is written as:

  ```
  ### T-014 · <title>
  - status: open
  - owner: —
  - priority: <priority> · type: <type> · area: <area>    (only the parts given)
  - depends: … / - files: … / - problem: … / - fix: … / - accept: …    (each only if given)
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
- a code fence or HTML comment is never closed;
- its format line names a version the tool doesn't know.

A command that changes one task is also refused when that task:

- shares its ID with another task (in this file or the archive);
- repeats a field;
- has a `status` value that isn't one of the five.

Read commands still work in all these cases and print the same messages as warnings.

## 14. Archive file

`TODO-archive.md` (setting `archive_file`) sits next to `TODO.md` and uses this same format. When
`archive` moves a task there, the whole block goes to the end of the section with the same name in
the archive; missing sections are added at the end. A new archive file starts with the format line
and `# Archive`. The archive is changed only under the lock of `TODO.md`, and its IDs count when the
next ID is chosen.

## 15. Lenient reading, canonical writing

| Read | Written by the tool |
|---|---|
| `T-7`, `t-007`, `T-007` | `T-007` |
| ` · `, ` - `, ` — `, ` – `, `: ` after the ID | ` · ` |
| owner `—`, `-`, `none`, empty | `—` |
| `type` and `area` on their own lines | left where they are |

The tool writes canonical forms only on lines it changes; it never tidies other lines on its own.
`todo check --fix` can tidy them on request.
