# `todo` JSON Output & Error Reference

When `--json` is passed, every `todo` command—whether read, write, or failed—prints **one** JSON object on standard output (`R-OUT-1`).

## Envelopes

### Success envelope

```json
{
  "schema": "todo.<command>.v1",
  "ok": true,
  "data": { ... },
  "warnings": []
}
```

When `--dry-run` is active on a write command, `"dry_run": true` and `"diff": "..."` (a unified diff) are included on the envelope.

### Error envelope

```json
{
  "schema": "todo.<command>.v1",
  "ok": false,
  "error": {
    "code": "NOT_FOUND",
    "message": "task T-999 not found",
    "details": {
      "task": "T-999"
    }
  },
  "warnings": []
}
```

`error.details` is omitted when empty and may contain `task`, `owner`, `candidates`, `file`, `line`, or command-specific fields such as `problems` (`check`).

---

## Shared `TaskJson` Shape

Returned by `show` (plus `block_lines`), `add`, `claim`, `release`, `done`, `drop`, `block`, `unblock`, `note`, `set`, `reopen`, `move`, `ask`, `answer`, and `next.task`:

```json
{
  "id": "T-012",
  "title": "Add retry logic",
  "status": "claimed",
  "owner": "agent-1",
  "priority": "P1",
  "difficulty": "hard",
  "type": "feature",
  "area": "cli",
  "depends": ["T-003"],
  "claimed": "2026-11-15T18:00+01:00",
  "stale": false,
  "ready": false,
  "section": "Tasks",
  "line_start": 42,
  "line_end": 49,
  "notes": [
    {
      "date": "2026-11-15",
      "author": "agent-1",
      "text": "started work"
    }
  ]
}
```

---

## Command Shapes

### `todo --json` (`todo.overview.v1`)

```json
{
  "schema": "todo.overview.v1",
  "ok": true,
  "data": {
    "name": "todo",
    "version": "0.5.0",
    "about": "Concurrency-safe editor for TODO.md task lists",
    "commands": [
      { "kind": "read", "name": "list", "summary": "List and filter tasks" }
    ]
  },
  "warnings": []
}
```

### `todo --version --json` (`todo.version.v1`)

```json
{
  "schema": "todo.version.v1",
  "ok": true,
  "data": { "version": "0.5.0", "commit": "abc1234" },
  "warnings": []
}
```

### `todo help [<command>|agents] --json` (`todo.help.v1`)

```json
{
  "schema": "todo.help.v1",
  "ok": true,
  "data": {
    "name": "claim",
    "summary": "Claim a task for an agent",
    "kind": "write",
    "details": "...",
    "examples": ["todo claim T-012 --agent agent-1"]
  },
  "warnings": []
}
```

For `todo help agents --json`, `data` is `{"topic":"agents","rules":"..."}`.

### `todo list --json` (`todo.list.v1`)

```json
{
  "schema": "todo.list.v1",
  "ok": true,
  "data": {
    "file": "TODO.md",
    "count": 1,
    "tasks": [
      {
        "id": "T-001",
        "title": "First task",
        "status": "open",
        "owner": null,
        "priority": "P1",
        "difficulty": null,
        "type": null,
        "area": null,
        "depends": [],
        "claimed": null,
        "stale": false,
        "ready": true,
        "section": "Tasks",
        "line_start": 7,
        "line_end": 11,
        "archived": false
      }
    ]
  },
  "warnings": []
}
```

### `todo show <task> --json` (`todo.show.v1`)

```json
{
  "schema": "todo.show.v1",
  "ok": true,
  "data": {
    "id": "T-001",
    "title": "First task",
    "status": "open",
    "owner": null,
    "priority": "P1",
    "difficulty": null,
    "type": null,
    "area": null,
    "depends": [],
    "claimed": null,
    "stale": false,
    "ready": true,
    "section": "Tasks",
    "line_start": 7,
    "line_end": 11,
    "archived": false,
    "block_lines": ["### T-001 · First task", "- status: open"],
    "depends_on": [
      { "id": "T-000", "status": "done", "title": "Setup", "found": true, "archived": false }
    ],
    "waiting_on_this": []
  },
  "warnings": []
}
```

### `todo next --json` (`todo.next.v1`)

```json
{
  "schema": "todo.next.v1",
  "ok": true,
  "data": {
    "found": true,
    "claimed": false,
    "already_theirs": false,
    "task": { "id": "T-001", "title": "First task", "status": "open" }
  },
  "warnings": []
}
```

When no ready task exists, `"found": false, "task": null`.

### `todo summary --json` (`todo.summary.v1`)

```json
{
  "schema": "todo.summary.v1",
  "ok": true,
  "data": {
    "file": "TODO.md",
    "counts": { "open": 2, "claimed": 1, "blocked": 1, "done": 3, "dropped": 0, "total": 7 },
    "open_by_priority": [{ "priority": "P0", "count": 1 }],
    "claimed": [
      { "id": "T-002", "title": "Work", "owner": "alice", "claimed": "2026-11-15T18:00+01:00", "age_hours": 2, "stale": false }
    ],
    "blocked": [{ "id": "T-004", "title": "Wait", "reason": "blocked — waiting on key" }],
    "next_ready": [{ "id": "T-001", "title": "First", "priority": "P0", "section": "Tasks" }],
    "open_questions": [{ "id": "T-007", "title": "Ship today?", "status": "open" }]
  },
  "warnings": []
}
```

`ready_to_close` (`R-MILE-6`) lists the milestones that are ready to close, as `{ "id", "title" }`; the key is left out when there are none.

### `todo where --json` (`todo.where.v1`)

```json
{
  "schema": "todo.where.v1",
  "ok": true,
  "data": {
    "todo_file": "C:\\Project\\todo\\TODO.md",
    "found_by": "git_worktree_main",
    "state_dir": "C:\\Project\\todo\\.git\\todo\\TODO.md",
    "archive_file": "C:\\Project\\todo\\TODO-archive.md",
    "archive_exists": false,
    "config_file": "C:\\Project\\todo\\todo.toml",
    "config_exists": false
  },
  "warnings": []
}
```

### `todo check --json` (`todo.check.v1`)

Clean (`exit 0`):

```json
{
  "schema": "todo.check.v1",
  "ok": true,
  "data": { "clean": true, "count": 0, "problems": [] },
  "warnings": []
}
```

Problems found (`exit 1`): `ok: false`, `error.code: "BAD_VALUE"`, `error.details.count`, and `error.details.problems` (`[{"file":"TODO.md","line":10,"message":"..."}]`).

### `todo history --json` (`todo.history.v1`)

```json
{
  "schema": "todo.history.v1",
  "ok": true,
  "data": {
    "count": 1,
    "snapshots": [
      {
        "number": 1,
        "time": "2026-11-15T20:00:00+01:00",
        "agent": "agent-1",
        "command": "add",
        "sha256": "e3b0c442...",
        "file": "000001.md"
      }
    ]
  },
  "warnings": []
}
```

### `todo diff <n> [<m>] --json` (`todo.diff.v1`)

```json
{
  "schema": "todo.diff.v1",
  "ok": true,
  "data": {
    "from": 1,
    "to": 2,
    "changed": true,
    "diff": "--- a/TODO.md#1\n+++ b/TODO.md#2\n..."
  },
  "warnings": []
}
```

### Single-task write commands (`add`, `claim`, `release`, `done`, `drop`, `block`, `unblock`, `note`, `set`, `reopen`, `move`, `ask`, `answer`)

Each emits `"schema": "todo.<command>.v1"` with `data` equal to the post-edit `TaskJson`.

- `answer --json` (`todo.answer.v1`): the question's `TaskJson`, plus `about` with the `TaskJson` of the entry the question names in its `about` field. `about` is left out when the question has none.
- `show --json` and every other `TaskJson` carry `questions`: the questions whose `about` field names the entry, each `{"id", "title", "status", "archived"}`, in file order (`[]` when there are none).
- `add --from <file|-> --json` (`todo.add.v1`): `{"added": [<TaskJson>, ...], "count": N}`.
- `release --stale --json` / `release --all --json` (`todo.release.v1`): `{"released": [<TaskJson>, ...], "count": N}`.

### `todo restore <n> --yes --json` (`todo.restore.v1`)

```json
{
  "schema": "todo.restore.v1",
  "ok": true,
  "data": {
    "restored_from": 2,
    "applied": true,
    "changed": true
  },
  "warnings": []
}
```

### `todo archive --yes --json` (`todo.archive.v1`)

```json
{
  "schema": "todo.archive.v1",
  "ok": true,
  "data": {
    "archive_file": "TODO-archive.md",
    "applied": true,
    "count": 2,
    "tasks": [
      { "id": "T-001", "title": "Old task", "status": "done", "section": "Tasks" }
    ]
  },
  "warnings": []
}
```

### `todo lock --json`, `todo unlock --json`, `todo status --json`

- `lock`: `{"agent":"alice","started_at":"...","expires_at":"...","minutes":10}`
- `unlock`: `{"unlocked":true,"forced":false,"previous_holder":"alice"}`
- `status`: `{"write_lock_held":false,"lease":null}`

### `todo init --json` (`todo.init.v1`)

```json
{
  "schema": "todo.init.v1",
  "ok": true,
  "data": {
    "file": "TODO.md",
    "created": true,
    "agents_snippet": "<!-- todo:begin -->\n...\n<!-- todo:end -->"
  },
  "warnings": []
}
```

### `todo import --json` (`todo.import.v1`)

```json
{
  "schema": "todo.import.v1",
  "ok": true,
  "data": {
    "source": "notes.md",
    "target": "TODO.md",
    "written": false,
    "backup": null,
    "checkbox_lines": 7,
    "tasks": 7,
    "problems": 0,
    "converted": [
      { "id": "T-001", "title": "Add export", "status": "open", "source_line": 11 }
    ],
    "text": "<!-- todo-format: 1 -->\n..."
  },
  "warnings": []
}
```

`written` is `true` with `--yes` (and no `--dry-run`); then `backup` is the path of the
`<name>.pre-import` copy when the target already existed, otherwise `null`. Without `--yes`, `text`
holds the converted file when the target does not exist, and `diff` (unified diff) when it does;
neither is present after a write. `checkbox_lines` and `tasks` are always equal; a result that fails
the `todo check` rules or the count is an error (`BAD_VALUE`, exit 1, `details.problems`).

### `todo completions <shell> --json` (`todo.completions.v1`)

`{"shell":"powershell","script":"<the completion script>"}`. Without `--json` the script itself is
printed, ready to pipe into `Invoke-Expression`.

### `todo agents install --json` and `todo agents check --json` (`todo.agents.v1`)

- `install`: `{"file":"AGENTS.md","applied":true,"changed":true,"created":false}`
- `check` (clean): `{"file":"AGENTS.md","status":"current","line":3}`

### `todo search --json` (`todo.search.v1`)

```json
{
  "schema": "todo.search.v1",
  "ok": true,
  "data": {
    "archive": false,
    "query": "parser",
    "count": 1,
    "matches": [
      {
        "id": "T-001",
        "status": "open",
        "title": "Fix parser",
        "field": "title",
        "line": 7,
        "text": "Fix parser"
      }
    ]
  },
  "warnings": []
}
```

### `todo log --json` (`todo.log.v1`)

```json
{
  "schema": "todo.log.v1",
  "ok": true,
  "data": {
    "task": "T-001",
    "count": 1,
    "entries": [
      {
        "time": "2026-11-15T20:00:00+01:00",
        "agent": "alice",
        "command": "claim",
        "arguments": ["claim", "T-001", "--agent", "alice"],
        "tasks": ["T-001"],
        "hash_before": "...",
        "hash_after": "..."
      }
    ]
  },
  "warnings": []
}
```

### `todo apply --json` (`todo.apply.v1`)

```json
{
  "schema": "todo.apply.v1",
  "ok": true,
  "data": {
    "count": 2,
    "applied": [
      { "index": 1, "op": "claim", "task": "T-001" },
      { "index": 2, "op": "done", "task": "T-001" }
    ]
  },
  "warnings": []
}
```

### `todo done --json` and `todo drop --json` (`newly_ready`, `R-DEP-4`)

When finishing or dropping a task makes other tasks ready, their summaries appear in `"newly_ready"`:

```json
"newly_ready": [
  { "id": "T-003", "priority": "P1", "title": "Follow-up task" }
]
```

When it finishes the last unfinished entry below a milestone, the milestone appears in `"ready_to_close"` (`R-MILE-6`). Nothing closes it; `todo done <id>` does. The key is left out when it is empty.

```json
"ready_to_close": [
  { "id": "T-010", "title": "Renderer" }
]
```

### Sessions: `fields.session`, `todo release --session --json`, `todo session-end --json` (`R-CLAIM-7`)

- A claim made with a session shows it in the task's `fields`: `{"fields":{"session":"6f0c1c2e-…"}}`.
- `release --session <id> --json` has the same shape as the other bulk releases
  (`todo.release.v1`): `{"released":[<TaskJson>…],"count":2}`.
- The hidden `session-end` command (the `SessionEnd` hook; `--json` is for tests and scripts) prints
  `todo.session-end.v1` with `{"session":"6f0c1c2e-…","released":[<TaskJson>…],"count":1}`;
  `session` is `null` when neither the hook input nor the environment names one.

### `todo edit --json`, `todo hooks --json`, `todo guard --json`, `todo skill --json`

- `edit`: `{"file":"TODO.md","editor":"notepad","clean":true,"problems":0}`
- `hooks`: `{"file":".claude/settings.json","action":"install","applied":true,"changed":true,"backup":".claude/settings.json.bak"}`
- `guard`: `{"allowed":true,"target":"src/main.rs"}`
- `skill`: `{"file":".claude/skills/todo/SKILL.md","applied":true,"changed":true,"created":true}`

### `todo align --json` (`todo.align.v1`, `R-GIT-1`)

```json
{"clean":false,"count":1,"findings":[
  {"task":"T-002","line":28,"kind":"COMMITTED_NOT_DONE","commits":["7d23e38"]}
]}
```

`kind` is `COMMITTED_NOT_DONE`, `DONE_WITHOUT_COMMIT` or `COMMIT_MISSING`. With findings the
envelope is still `"ok":true` and the exit code is 1. Without usable git history the result is an
error envelope with `NO_GIT_HISTORY` and exit 5.

### `todo merge-driver install|uninstall --json` (`todo.merge-driver.v1`, `R-GIT-3`)

```json
{"action":"install","changed":true,"attributes_file":"C:\work\shop\.gitattributes","path":"TODO.md"}
```

`merge-driver run` is called by git and prints nothing on success.

### `todo scan --json` (`todo.scan.v1`, `R-DISC-1`)

```json
{"count":1,"found":[{"path":"src/a.rs","line":1,"text":"TODO: tidy this"}],"added":["T-002"]}
```

`text` is the comment from the marker on; `added` (the new task IDs) is present only with `--add`.

### `todo schema --json` (`todo.schema.v1`, `R-SCHEMA-1`)

`data` has `tool`, `version`, `commit`, `envelope`, `exit_codes` (`{code, meaning}`),
`error_codes`, `task` (the `TaskJson` fields), `global_options` and `commands`. Each command is
`{"name","schema","summary","subcommands":[…],"arguments":[…],"options":[…]}`, and each option
`{"name","value_name","required","repeatable","help"}`. `todo schema <command>` returns just that
command.

### `by` field (`R-WRITE-16`)

`done` may add `{"fields":{"by":"reyna-gh"}}` to the task: the person who finished it, after
`.mailmap`. Never an email address.

---

## Error Codes (`R-OUT-2`, `R-OUT-3`)

| Code | Exit | Meaning | Example |
|---|---|---|---|
| `NO_FILE` | 4 | `TODO.md` or `@path` file not found | `{"code":"NO_FILE","message":"no TODO.md found"}` |
| `NO_AGENT` | 2 | Write command without `--agent` or `TODO_AGENT` | `{"code":"NO_AGENT","message":"missing agent name: pass --agent <name> or set TODO_AGENT"}` |
| `USAGE` | 2 | Invalid CLI flags, arguments, or agent name | `{"code":"USAGE","message":"invalid agent name \"bad name\""}` |
| `NOT_FOUND` | 1 | Task, snapshot, or target file not found | `{"code":"NOT_FOUND","message":"task T-999 not found","details":{"task":"T-999"}}` |
| `AMBIGUOUS` | 1 | Title substring matched multiple tasks | `{"code":"AMBIGUOUS","message":"\"retry\" matched 2 tasks","details":{"candidates":["T-001","T-002"]}}` |
| `ALREADY_CLAIMED` | 1 | Task is already claimed by another agent | `{"code":"ALREADY_CLAIMED","message":"T-012 is claimed by bob","details":{"owner":"bob","task":"T-012"}}` |
| `NOT_OWNER` | 1 | Task or lease belongs to another agent without `--force` | `{"code":"NOT_OWNER","message":"T-012 is owned by bob; pass --force","details":{"owner":"bob","task":"T-012"}}` |
| `BAD_TRANSITION` | 1 | Disallowed status transition (e.g. `done` on `open`) | `{"code":"BAD_TRANSITION","message":"cannot mark open task T-012 done without claiming"}` |
| `DUPLICATE_TITLE` | 1 | `add` title matches an existing task | `{"code":"DUPLICATE_TITLE","message":"task with title \"Setup\" already exists"}` |
| `UNKNOWN_SECTION` | 1 | Named `--section` does not exist | `{"code":"UNKNOWN_SECTION","message":"section \"Foo\" not found"}` |
| `AMBIGUOUS_SECTION` | 1 | Multiple sections exist and `--section` was omitted | `{"code":"AMBIGUOUS_SECTION","message":"multiple sections exist; pass --section"}` |
| `MISSING_FIELD` | 1 | Required field (e.g. `priority`) missing on `add` | `{"code":"MISSING_FIELD","message":"missing required field: priority"}` |
| `BAD_VALUE` | 2 (1 for `check`) | Invalid field, config, snapshot number, or `check` failure | `{"code":"BAD_VALUE","message":"invalid priority \"P9\""}` |
| `BUSY` | 3 | Write lock held by another process past wait timeout | `{"code":"BUSY","message":"write lock busy: held by agent bob","details":{"owner":"bob"}}` |
| `LEASE_HELD` | 3 | Manual editing lease held by another agent | `{"code":"LEASE_HELD","message":"manual lease held by alice until ...","details":{"owner":"alice"}}` |
| `CHANGED_DURING_WRITE` | 3 | Target file changed on disk between read and replace | `{"code":"CHANGED_DURING_WRITE","message":"TODO.md changed on disk during write"}` |
| `UNSAFE_FILE` | 4 | Invalid UTF-8, conflict markers, unclosed fence/comment, duplicate ID, or `init` target exists | `{"code":"UNSAFE_FILE","message":"TODO.md:4: git conflict marker"}` |
| `READ_ONLY` | 4 | Target file is read-only | `{"code":"READ_ONLY","message":"TODO.md is read-only"}` |
| `WRITE_FAILED` | 4 | Atomic write or rename failed | `{"code":"WRITE_FAILED","message":"failed writing temporary file"}` |
| `NO_GIT_HISTORY` | 5 | `align` without git, outside a repository, in a shallow clone or with no commits | `{"code":"NO_GIT_HISTORY","message":"not inside a git repository"}` |
