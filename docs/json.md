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
  "schema": "todo.show.v1",
  "ok": false,
  "error": {
    "code": "NOT_FOUND",
    "message": "task T-999 not found",
    "task": "T-999"
  },
  "warnings": []
}
```

The details of an error sit in the `error` object next to `code` and `message`, not under a
`details` key. They are left out when empty, and may be `task`, `owner`, `candidates`, `file`,
`line`, or command-specific fields such as `count` and `problems` (`check`). The exit code of each
error is in the table under "Error Codes".

---

## Shared `TaskJson` Shape

Returned by `show`, `add`, `claim`, `release`, `done`, `drop`, `block`, `unblock`, `stuck`, `note`, `set`, `reopen`, `move`, `ask`, `answer`, `idea`, `promote`, and as `next.task`. The example is the `show` output under "`todo show <task> --json`" below, which lists every key. Every key is always present: a value that is not set is `null`, and a list or an object with nothing in it is `[]` or `{}`.

- `type` is the `type` field as written; `section` is the `##` section the entry is under, or `null`.
- `archived` is `true` for an entry read from `TODO-archive.md`.
- `stale` is `true` when the claim is past its timeout (`R-CLAIM-4`); `ready` is `true` when `next` could hand the entry out now, files and detail included.
- `missing` lists what the detail rule lacks (`R-DETAIL-2`) for a task at or below the target level, in a fixed order: `why`, `accept`, `steps`, `verify`, `files`, `files: more than <n>`, `files: folder or pattern`, `read: <path> does not exist`, then `open question <ID>` for each open question about it. It is `[]` when complete, and always for a task above the target level, a milestone, a question or an idea. `ready` is `false` while it lists anything.
- `files_busy` lists the tasks of other agents that hold a file this entry names (`R-SCOPE-1`), in file order, each with `id`, `title`, `status` (`claimed` or `blocked`) and `owner`. It is `[]` when none holds one, and `ready` is `false` while it lists a task. A task owned by the same agent is not listed.
- `milestones` lists the milestones above the entry, from the top down, with `goal` and `status`; `milestone` is the parent as written.
- `children` lists the entries whose `milestone` is this entry, with `kind` (`task` or `milestone`).
- `depends` holds the dependency tokens as written. `dependencies` has one entry per token: `raw`, `id`, `exists`, `archived`, `status`, `title` and `satisfied` (`true` when the target is done or dropped; `null` and `false` when it does not exist).
- `dependents` lists the tasks whose `depends` names this entry; `questions` the questions whose `about` names it (`R-KIND-2`).
- `fields` holds the unknown `key: value` fields as written.
- `notes` has `date`, `agent`, `text` and `line`, the 1-based line of the note in the file. `line_start` and `line_end` are the 1-based first and last lines of the block.

---

## Command Shapes

### `todo --json` (`todo.overview.v1`)

```json
{
  "schema": "todo.overview.v1",
  "ok": true,
  "data": {
    "name": "todo",
    "version": "0.7.0",
    "about": "Concurrency-safe editor for TODO.md task lists",
    "commands": [
      { "kind": "read", "name": "list", "summary": "List and filter tasks" }
    ]
  },
  "warnings": []
}
```

The list of the projects in the project register is `todo overview --json` (`todo.projects.v1`,
below), not this command.

### `todo --version --json` (`todo.version.v1`)

```json
{
  "schema": "todo.version.v1",
  "ok": true,
  "data": { "version": "0.7.0", "commit": "abc1234" },
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

### `todo help update --json` (`todo.help.v1`)

For `todo help update --json`, `data` has `topic` (always `update`), `version`, `executable` (the full path of the running executable, or `null` when it cannot be read), `repository`, `latest_release`, `assets` (each with `platform`, `file` and `checksum`), `network`, and `windows` and `linux`. Each of those two is an object with `shell` and `steps`, and a step has `text` and `commands`. todo runs none of the commands; the agent does. Example, with the steps shortened:

```json
{
  "schema": "todo.help.v1",
  "ok": true,
  "data": {
    "topic": "update",
    "version": "0.7.0",
    "executable": "C:\\Users\\alice\\AppData\\Local\\Programs\\todo\\todo.exe",
    "repository": "https://github.com/reynanTandoyo/todo",
    "latest_release": "https://github.com/reynanTandoyo/todo/releases/latest",
    "assets": [
      { "platform": "windows-x86_64", "file": "todo.exe", "checksum": "todo.exe.sha256" },
      { "platform": "linux-x86_64", "file": "todo-linux-x86_64", "checksum": "todo-linux-x86_64.sha256" }
    ],
    "network": "todo never connects to the network. The agent runs these steps itself, one after another, and stops at the first failure.",
    "windows": {
      "shell": "PowerShell",
      "steps": [
        { "text": "Read the latest version. ...", "commands": ["gh release view --repo reynanTandoyo/todo --json tagName --jq .tagName"] }
      ]
    },
    "linux": { "shell": "POSIX sh", "steps": [ { "text": "...", "commands": ["..."] } ] }
  },
  "warnings": []
}
```

### `todo list --json` (`todo.list.v1`)

```json
{
  "schema": "todo.list.v1",
  "ok": true,
  "data": {
    "archive": false,
    "count": 1,
    "tasks": [
      {
        "id": "T-001",
        "title": "First task",
        "section": "Tasks",
        "status": "open",
        "owner": null,
        "claimed": null,
        "stale": false,
        "priority": "P1",
        "difficulty": null,
        "type": null,
        "area": null,
        "depends": [],
        "ready": true,
        "files_busy": [],
        "fields": {},
        "line_start": 7,
        "line_end": 11
      }
    ]
  },
  "warnings": []
}
```

### `todo show <task> --json` (`todo.show.v1`)

`data` is the Shared `TaskJson` shape, for the milestone `T-011`. The fixture has a parent milestone (`T-010`), a dependency (`T-003`), a dependent (`T-020`), a question about it (`T-005`) and a task below it (`T-012`):

```json
{
  "schema": "todo.show.v1",
  "ok": true,
  "data": {
    "id": "T-011",
    "title": "Lighting",
    "section": "Tasks",
    "archived": false,
    "status": "open",
    "owner": null,
    "claimed": null,
    "stale": false,
    "priority": "P1",
    "difficulty": null,
    "type": "milestone",
    "area": null,
    "goal": "point and directional lights match the reference images",
    "milestone": "T-010",
    "milestones": [
      { "id": "T-010", "title": "Develop the renderer", "goal": "the test scene renders with lighting at 60 frames per second", "status": "open" }
    ],
    "children": [
      { "id": "T-012", "title": "C shader", "kind": "task", "status": "open" }
    ],
    "why": null,
    "depends": ["T-003"],
    "dependencies": [
      { "raw": "T-003", "id": "T-003", "exists": true, "archived": false, "status": "done", "title": "Setup", "satisfied": true }
    ],
    "dependents": [
      { "id": "T-020", "title": "Ship lighting", "status": "open", "archived": false }
    ],
    "questions": [
      { "id": "T-005", "title": "Are shadows in scope?", "status": "open", "archived": false }
    ],
    "ready": false,
    "missing": [],
    "files_busy": [],
    "files": null,
    "read": [],
    "problem": null,
    "fix": null,
    "steps": [],
    "accept": null,
    "verify": null,
    "commit": [],
    "fields": {},
    "notes": [
      { "date": "2026-11-15", "agent": "planner", "text": "added", "line": 27 }
    ],
    "line_start": 20,
    "line_end": 27
  },
  "warnings": []
}
```

`warnings` holds the warnings that the text output prints on standard error.

### `todo next --json` (`todo.next.v1`)

`task` is the Shared `TaskJson` shape (see above) of the task that `next` hands out. The example is
the output for a fixture with one open task:

```json
{
  "schema": "todo.next.v1",
  "ok": true,
  "data": {
    "task": {
      "id": "T-001",
      "title": "Write the parser",
      "section": "Tasks",
      "archived": false,
      "status": "open",
      "owner": null,
      "claimed": null,
      "stale": false,
      "priority": "P1",
      "difficulty": "easy",
      "type": "feature",
      "area": "cli",
      "goal": null,
      "milestone": null,
      "milestones": [],
      "children": [],
      "why": "the reader needs it first",
      "depends": [],
      "dependencies": [],
      "dependents": [],
      "questions": [],
      "ready": true,
      "missing": [],
      "files_busy": [],
      "files": null,
      "read": [],
      "problem": null,
      "fix": null,
      "steps": [],
      "accept": null,
      "verify": null,
      "commit": [],
      "fields": {},
      "notes": [
        { "date": "2026-11-15", "agent": "planner", "text": "added", "line": 12 }
      ],
      "line_start": 7,
      "line_end": 12
    },
    "claimed": false,
    "already_claimed": false
  },
  "warnings": []
}
```

- `claimed` is `true` when `--claim` wrote the claim in this run; then `task` shows the claim
  (`owner` and `claimed`, and a `session` entry in `fields` when a session is known).
- `already_claimed` is `true` when the task was already the agent's own claim. `next` returns that
  task first unless `--new` is given.
- When no ready task exists, `task` is `null` and `claimed` and `already_claimed` are `false`.

### `todo summary --json` (`todo.summary.v1`)

```json
{
  "schema": "todo.summary.v1",
  "ok": true,
  "data": {
    "file": "TODO.md",
    "counts": { "open": 2, "claimed": 1, "blocked": 1, "done": 3, "dropped": 0, "total": 7, "questions": 1, "ideas": 1 },
    "open_by_priority": [{ "priority": "P0", "count": 1 }],
    "milestones": [{ "id": "T-010", "title": "Develop the renderer", "priority": "P1", "status": "open", "progress": { "done": 1, "total": 3 } }],
    "claimed": [
      { "id": "T-002", "title": "Work", "owner": "alice", "claimed": "2026-11-15T18:00+01:00", "age_hours": 2, "stale": false }
    ],
    "blocked": [{ "id": "T-004", "title": "Wait", "reason": "waiting on key" }],
    "next_ready": [{ "id": "T-001", "title": "First", "priority": "P0", "section": "Tasks" }],
    "open_questions": [{ "id": "T-007", "title": "Ship today?", "status": "open" }]
  },
  "warnings": []
}
```

`next_ready` lists up to five tasks that `next` could hand out now, in its order. A task whose files another agent holds is left out, as its `ready` is `false` in `show --json` (`R-SCOPE-1`).

`ready_to_close` (`R-MILE-6`) lists the milestones that are ready to close, as `{ "id", "title" }`; the key is left out when there are none.

`given_back` (`R-STUCK-1`) lists the blocked tasks that `todo stuck` gave back to the planner, each as a `blocked` entry whose `reason` is the problem; the key is left out when there are none.

`counts` covers tasks only: `open`, `claimed`, `blocked`, `done`, `dropped` and `total` (their sum) leave out milestones, which are listed under `milestones`. `questions` and `ideas` count the open ones (not done or dropped) apart from the rest (`R-KIND-3`, `R-KIND-1`); the open questions are listed in `open_questions`.

### `todo tree --json` (`todo.tree.v1`)

`nodes` is nested: each milestone lists its `children` (milestones and tasks, in file order), and
top-level tasks come after the top-level milestones. `progress` is `{done, total}` for a milestone
and `null` for a task; `goal` is set only for a milestone. `questions` and `ideas` list the open
ones only for the whole tree (no milestone argument and no `--area`); otherwise they are `[]`.
`archived` is `true` for an entry from `TODO-archive.md`, which is read and shown only with `--all`
(done and dropped entries are shown only with `--all` too).

```json
{
  "schema": "todo.tree.v1",
  "ok": true,
  "data": {
    "file": "TODO.md",
    "all": false,
    "milestone": null,
    "area": null,
    "milestones": 1,
    "tasks": 1,
    "nodes": [
      {
        "id": "T-010",
        "title": "Develop the renderer",
        "kind": "milestone",
        "status": "open",
        "priority": "P1",
        "difficulty": null,
        "area": null,
        "goal": "the test scene renders with lighting at 60 frames per second",
        "progress": { "done": 1, "total": 2 },
        "archived": false,
        "children": [
          { "id": "T-012", "title": "C shader", "kind": "task", "status": "open", "priority": "P1", "difficulty": "hard", "area": "cli", "goal": null, "progress": null, "archived": false, "children": [] }
        ]
      }
    ],
    "questions": [
      { "id": "T-004", "title": "Should the C shader support shadows?", "status": "open", "about": "T-012", "archived": false }
    ],
    "ideas": [
      { "id": "T-005", "title": "Hot-reload shaders while the scene runs", "status": "open", "about": "T-010", "archived": false }
    ]
  },
  "warnings": []
}
```

### `todo waves --json` (`todo.waves.v1`, `R-WAVE-1`)

```json
{
  "schema": "todo.waves.v1",
  "ok": true,
  "data": {
    "file": "TODO.md",
    "milestone": null,
    "tasks": 3,
    "wave_count": 2,
    "max_workers": 1,
    "longest_chain": [
      { "id": "T-001", "title": "Parse the config", "status": "open", "owner": null, "level": "1", "missing": [] },
      { "id": "T-002", "title": "Load the config", "status": "open", "owner": null, "level": "2", "missing": [] }
    ],
    "waves": [
      {
        "wave": 1,
        "size": { "tasks": 1, "lanes": 1 },
        "lanes": [[{ "id": "T-001", "title": "Parse the config", "status": "open", "owner": null, "level": "1", "missing": [] }]]
      },
      {
        "wave": 2,
        "size": { "tasks": 1, "lanes": 1 },
        "lanes": [[{ "id": "T-002", "title": "Load the config", "status": "open", "owner": null, "level": "2", "missing": [] }]]
      }
    ],
    "apart": [
      { "id": "T-003", "title": "Blocked for now", "status": "blocked", "reason": "blocked: waiting for the key" }
    ]
  },
  "warnings": []
}
```

`waves` (`R-WAVE-1`) groups the unfinished tasks (open and claimed) into waves: wave 1 holds every
task that is ready now, and wave `n` the tasks that become ready when the waves before it are done.
Each wave lists its `lanes`, and the tasks of one lane share files, so they run one after another.
`size` gives the `tasks` and `lanes` of the wave. `wave_count` is the number of waves, which is the
length of `longest_chain`, the longest dependency chain task by task. `max_workers` is the most lanes
in any wave: the most workers useful at once.

`apart` lists the unfinished tasks that are in no wave, in file order, each with its `reason`:
`blocked: …` or `given back: …` (`todo stuck`), `below blocked milestone T-…`, `below done milestone
T-…` or `below dropped milestone T-…` (the milestone above the task that holds it back, `R-MILE-6`),
`depends on T-…, which does not exist`, `in a dependency cycle: T-… -> T-… -> T-…`, `no level: give it one with
todo set T-… --difficulty <level>` (`R-LEVEL-1`, the rule of `next`) or `waits on T-…`.

`level` is the task's difficulty as written; a task without one is in `apart`, not in a wave. `missing`
lists what the task lacks for the detail rule at the lowest level (`R-DETAIL-2`): `why`, `accept`,
`steps`, `verify`, `files`, `files: more than 3` or `files: folder or pattern`. It is `[]` when the
task has all of it, and when it is at another level. A task with a missing item is still placed in
its wave.

`todo waves <milestone>` limits `tasks`, the waves and `apart` to the tasks below the milestone;
`milestone` then holds its ID, and `null` otherwise. The wave numbers are those of the whole file.

### `todo where --json` (`todo.where.v1`)

```json
{
  "schema": "todo.where.v1",
  "ok": true,
  "data": {
    "file": "C:\\Project\\todo\\TODO.md",
    "found_by": "worktree-main",
    "repo_root": "C:\\Project\\todo",
    "state_dir": "C:\\Project\\todo\\.git\\todo\\todo.md",
    "write_lock": { "held": false },
    "format_version": 2,
    "tool_stamp": "0.6.0",
    "running_version": "0.7.0",
    "counts": { "open": 10, "claimed": 4, "blocked": 0, "done": 27, "dropped": 3, "total": 44, "milestones": 2, "questions": 1, "ideas": 3 }
  },
  "warnings": []
}
```

- `file` is the `TODO.md` in use and `found_by` says how it was found (`flag`, `env`, `walk`,
  `worktree-main`, ...). `repo_root` is the git repository root, or `null` outside a repository.
- `state_dir` is the folder that holds the lock and the state files. `write_lock.held` is `true`
  while a command holds the write lock; then `write_lock` also has `holder` with `agent`,
  `command`, `pid` and `started_at`.
- `lease` is present only while a manual lease (`todo lock`) is active: `agent`, `started_at`,
  `expires_at` and `remaining_seconds`.
- `format_version` is the version in the format line (`format.md` §4), or `null` when the file has
  none. `tool_stamp` is the version in the tool line (`R-VER-1`), or `null` when there is none, and
  `running_version` is the version of the todo that ran. When the stamp is newer than
  `running_version`, a warning says to run `todo help update`.
- `counts` holds the number of tasks per status (`open`, `claimed`, `blocked`, `done`, `dropped`) and
  `total`, their sum. `milestones`, `questions` and `ideas` count the entries of those kinds that are
  not done or dropped, apart from the tasks (`R-KIND-3`). When the format version is newer than this
  todo knows, `where` still succeeds but leaves out `counts`, because the tasks are not read
  (R-VER-2).

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

Problems found (`exit 1`): `ok: false`, `error.code: "BAD_VALUE"`, `error.count`, and `error.problems` (`[{"file":"TODO.md","line":10,"message":"..."}]`), as in:

```json
{
  "schema": "todo.check.v1",
  "ok": false,
  "error": {
    "code": "BAD_VALUE",
    "message": "1 problem(s) found",
    "count": 1,
    "problems": [
      { "file": "TODO.md", "line": 17, "message": "T-002: stale claim by agent-ui-1" }
    ]
  },
  "warnings": []
}
```

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

### Single-task write commands (`add`, `claim`, `release`, `done`, `drop`, `block`, `unblock`, `stuck`, `note`, `set`, `reopen`, `move`, `ask`, `answer`, `idea`, `promote`)

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
    ],
    "kept_back": [
      { "id": "T-002", "title": "Lighting", "status": "done", "section": "Tasks", "reason": "milestone T-000 is still open" }
    ]
  },
  "warnings": []
}
```

`kept_back` lists the finished entries that stayed in `TODO.md` because of the milestone rules
(R-ARCH-2), in file order, each with the `reason` in words; `[]` when there are none. `count` and
`tasks` list only the entries that moved, a whole milestone tree included.

### `todo migrate --json` (`todo.migrate.v1`)

```json
{
  "schema": "todo.migrate.v1",
  "ok": true,
  "data": {
    "file": "TODO.md",
    "current_format": 2,
    "migrated": [
      { "file": "TODO.md", "from": 1, "to": 2 },
      { "file": "TODO-archive.md", "from": 1, "to": 2 }
    ],
    "changed": true,
    "project": [
      { "file": "TODO.md", "kind": "rules_block", "updated": true },
      { "file": "AGENTS.md", "kind": "rules_block", "updated": true },
      { "file": ".claude/skills/todo/SKILL.md", "kind": "skill", "updated": true },
      { "file": ".claude/settings.json", "kind": "hooks", "events": ["SessionStart"], "updated": true }
    ]
  },
  "warnings": []
}
```

`migrated` is empty when nothing was in an older format; `changed` is `false` when the file was not written (including `--dry-run`, which also sets `dry_run` and `diff`). `project` lists each installed project file that is out of date (R-MIG-2): `kind` is `rules_block`, `skill` or `hooks`; `events` names the hook events whose entries are out of date (hooks only); `updated` is `true` when `--yes` changed the file. Without `--yes` every row has `updated: false`.

### `todo lock --json`, `todo unlock --json`, `todo status --json`

- `lock`: `{"agent":"alice","started_at":"...","expires_at":"...","minutes":10}`
- `unlock`: `{"unlocked":true,"forced":false,"previous_holder":"alice"}`
- `status`: `{"file":"TODO.md","state_dir":".git/todo/todo.md","write_lock":{"held":false}}`; while a manual lease is active it also has `lease` with `agent`, `started_at`, `expires_at` and `remaining_seconds`.

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
    "text": "<!-- todo-format: 2 -->\n<!-- todo-tool: 0.6.0 -->\n..."
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

An entry written by a write that migrated the file (R-MIG-1) also has `"migrated": "TODO.md: format 1 to 2"`; the field is absent otherwise.

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

A task whose files another agent's live claim holds is left out, as `next` would not hand it out (`R-SCOPE-1`); the text output's `now ready` lines come from the same list.

When it finishes the last unfinished entry below a milestone, the milestone appears in `"ready_to_close"` (`R-MILE-6`). Nothing closes it; `todo done <id>` does. The key is left out when it is empty.

```json
"ready_to_close": [
  { "id": "T-010", "title": "Renderer" }
]
```

### `todo verify <task> --json` and `todo verify --wave --json` (`todo.verify.v1`, `R-VERIFY-1`, `R-VERIFY-2`)

A passing `verify <task>` prints `data` with `task`, `passed: true` and `log` (the path of
`verify/<ID>.log`). A failing one is the `VERIFY_FAILED` error above. The output of the command never
reaches the JSON; it is in the log file.

`verify --wave` prints `data` with `checked` (the number of done tasks that were checked), `passed`
(the IDs that passed), `failed` (one object per failed task: `task`, `exit_code`, `last_line` and
`log`) and `given_back` (the IDs given back as `stuck`). It is `ok: true` with exit 1 when anything
failed, as `align` is.

```json
{
  "schema": "todo.verify.v1",
  "ok": true,
  "data": {
    "checked": 2,
    "passed": ["T-001"],
    "failed": [
      { "task": "T-002", "exit_code": 1, "last_line": "2 tests failed", "log": "…/verify/T-002.log" }
    ],
    "given_back": ["T-002"]
  },
  "warnings": []
}
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

### `todo report --html --json` (`todo.report.v1`, `R-REPORT-1`)

`data` has `path` (the file written, as a full path), `bytes` (its size) and `opened` (`true` only
when `--open` started the program). The page itself is HTML, not part of the JSON. The `--all` page
(R-REPORT-2), written with `todo report --html --all --json`, has the same `data`.

```json
{
  "schema": "todo.report.v1",
  "ok": true,
  "data": {
    "path": "C:\\Project\\todo\\.todo\\report.html",
    "bytes": 48213,
    "opened": false
  },
  "warnings": []
}
```

### `by` field (`R-WRITE-16`)

`done` may add `{"fields":{"by":"reyna-gh"}}` to the task: the person who finished it, after
`.mailmap`. Never an email address.

### `todo project add --json` (`todo.project.v1`, `R-PROJ-1`)

```json
{
  "schema": "todo.project.v1",
  "ok": true,
  "data": {
    "project": { "name": "renderer", "path": "C:\\Projekte\\Grafik Engine\\TODO.md", "added": "2026-11-19T10:00+01:00" },
    "register": "C:\\Users\\Example\\AppData\\Local\\todo\\projects.json"
  },
  "warnings": []
}
```

`register` is the file that holds the projects: `%LOCALAPPDATA%\todo\projects.json`, or the file
named by `TODO_REGISTER`. The same file, or the same name, as a project already registered is refused
with `DUPLICATE_PROJECT`.

### `todo project remove <name|path> --json` (`todo.project.v1`, `R-PROJ-1`)

```json
{
  "schema": "todo.project.v1",
  "ok": true,
  "data": {
    "removed": { "name": "renderer", "path": "C:\\Projekte\\Grafik Engine\\TODO.md", "added": "2026-11-19T10:00+01:00" },
    "register": "C:\\Users\\Example\\AppData\\Local\\todo\\projects.json"
  },
  "warnings": []
}
```

### `todo project list --json` (`todo.project.v1`, `R-PROJ-1`)

```json
{
  "schema": "todo.project.v1",
  "ok": true,
  "data": {
    "projects": [
      { "name": "renderer", "path": "C:\\Projekte\\Grafik Engine\\TODO.md", "added": "2026-11-19T10:00+01:00" }
    ],
    "register": "C:\\Users\\Example\\AppData\\Local\\todo\\projects.json"
  },
  "warnings": []
}
```

`projects` is empty when the register has no project yet, or does not exist yet.

### `todo overview --json` (`todo.projects.v1`, `R-PROJ-2`)

The schema is `todo.projects.v1`, not `todo.overview.v1`: that name is the one of `todo --json`
with no command, which lists the commands above. The overview reads only the register and each
project's `TODO.md`, never searches folders and writes nothing.

```json
{
  "schema": "todo.projects.v1",
  "ok": true,
  "data": {
    "projects": [
      {
        "name": "renderer",
        "path": "C:\\Projekte\\Grafik Engine\\TODO.md",
        "added": "2026-11-19T10:00+01:00",
        "problem": null,
        "format_version": 2,
        "tool_stamp": "0.6.0",
        "needs_migration": false,
        "tasks": { "open": 2, "claimed": 1, "blocked": 0, "done": 3, "dropped": 0, "total": 6 },
        "milestones": { "done": 0, "total": 1 },
        "stale_claims": 0,
        "open_questions": 1
      },
      {
        "name": "archiv",
        "path": "D:\\Archiv\\TODO.md",
        "added": "2026-11-10T09:00+01:00",
        "problem": { "code": "NO_FILE", "message": "TODO.md not found" },
        "format_version": null,
        "tool_stamp": null,
        "needs_migration": null,
        "tasks": null,
        "milestones": null,
        "stale_claims": null,
        "open_questions": null
      }
    ],
    "register": "C:\\Users\\Example\\AppData\\Local\\todo\\projects.json"
  },
  "warnings": []
}
```

- Each figure comes from the project's `TODO.md` (not its archive). `tasks` counts tasks per status;
  `milestones` counts the top-level milestones that are done, out of those not dropped; `stale_claims`
  and `open_questions` are as in `summary`.
- `problem` is set when the file is gone, cannot be read, has a problem that blocks writes, or is in a
  format this todo does not know (`TOOL_TOO_OLD`); the figures are then `null`, and the other
  projects are still shown. The exit code is 0.
- `needs_migration` is `true` when the format is older than this todo's, or the stamp is missing or
  older than the running todo: `todo migrate --yes` brings the file up to date.
- `format_version` is 1 when the file has no format line.

---

## Error Codes (`R-OUT-2`, `R-OUT-3`)

| Code | Exit | Meaning | Example |
|---|---|---|---|
| `NO_FILE` | 4 | `TODO.md` or `@path` file not found; for `project add`, a folder that is not there or has no `TODO.md`; no folder for the project register | `{"code":"NO_FILE","message":"no TODO.md found"}` |
| `NO_AGENT` | 2 | Write command without `--agent` or `TODO_AGENT` | `{"code":"NO_AGENT","message":"missing agent name: pass --agent <name> or set TODO_AGENT"}` |
| `USAGE` | 2 | Invalid CLI flags, arguments, or agent name | `{"code":"USAGE","message":"invalid agent name \"bad name\""}` |
| `NOT_FOUND` | 1 | Task, snapshot, target file, or registered project not found | `{"code":"NOT_FOUND","message":"task T-999 not found","task":"T-999"}` |
| `AMBIGUOUS` | 1 | Title substring matched multiple tasks | `{"code":"AMBIGUOUS","message":"\"retry\" matched 2 tasks","candidates":["T-001","T-002"]}` |
| `ALREADY_CLAIMED` | 1 | Task is already claimed by another agent | `{"code":"ALREADY_CLAIMED","message":"T-012 is claimed by bob","owner":"bob","task":"T-012"}` |
| `NOT_OWNER` | 1 | Task or lease belongs to another agent without `--force` | `{"code":"NOT_OWNER","message":"T-012 is owned by bob; pass --force","owner":"bob","task":"T-012"}` |
| `BAD_TRANSITION` | 1 | Disallowed status transition (R-WRITE-9; e.g. `unblock` on an open task) | `{"code":"BAD_TRANSITION","message":"cannot unblock task T-012 while status is open; allowed from open: claim, block, done, drop","task":"T-012"}` |
| `DUPLICATE_TITLE` | 1 | `add` title matches an existing task | `{"code":"DUPLICATE_TITLE","message":"task with title \"Setup\" already exists"}` |
| `UNKNOWN_SECTION` | 1 | Named `--section` does not exist, or `add` has no `--section` while the file has several sections and no `default_section` (R-WRITE-1) | `{"code":"UNKNOWN_SECTION","message":"multiple sections exist; pass --section <name>. Available sections: Now, Later"}` |
| `AMBIGUOUS_SECTION` | 1 | A `--section` name matches more than one section | `{"code":"AMBIGUOUS_SECTION","message":"section 'Next' is ambiguous (matches: Next steps, Next release); available sections: Next steps, Next release"}` |
| `MISSING_FIELD` | 1 | Required field (e.g. `priority`) missing on `add` | `{"code":"MISSING_FIELD","message":"missing required field: priority"}` |
| `BAD_VALUE` | 1 | Invalid field, config, snapshot number, or `check` failure (refused by a rule: R-WRITE-18, R-MILE-1) | `{"code":"BAD_VALUE","message":"invalid priority \"P9\""}` |
| `BUSY` | 3 | Write lock held by another process past wait timeout | `{"code":"BUSY","message":"write lock busy: held by agent bob","owner":"bob"}` |
| `LEASE_HELD` | 3 | Manual editing lease held by another agent | `{"code":"LEASE_HELD","message":"manual lease held by alice until ...","owner":"alice"}` |
| `CHANGED_DURING_WRITE` | 3 | Target file changed on disk between read and replace | `{"code":"CHANGED_DURING_WRITE","message":"TODO.md changed on disk during write"}` |
| `UNSAFE_FILE` | 4 | Invalid UTF-8, conflict markers, unclosed fence/comment, duplicate ID, `init` target exists, or a project register that is not valid JSON of this format | `{"code":"UNSAFE_FILE","message":"TODO.md:4: git conflict marker"}` |
| `READ_ONLY` | 4 | Target file is read-only | `{"code":"READ_ONLY","message":"TODO.md is read-only"}` |
| `WRITE_FAILED` | 4 | Atomic write or rename failed | `{"code":"WRITE_FAILED","message":"failed writing temporary file"}` |
| `NO_GIT_HISTORY` | 5 | `align` without git, outside a repository, in a shallow clone or with no commits | `{"code":"NO_GIT_HISTORY","message":"not inside a git repository"}` |
| `TOOL_TOO_OLD` | 4 | `TODO.md`, the archive or the project register names a format version this todo does not know; update todo first (R-VER-2) | `{"code":"TOOL_TOO_OLD","message":"format version 3 is newer than this todo 0.7.0 supports; ...","line":1,"file":"TODO.md"}` |
| `FILES_BUSY` | 1 | `claim` of a task whose `files` overlap a task another agent holds (R-SCOPE-1, R-CLAIM-2); `--force` overrides it | `{"code":"FILES_BUSY","message":"task T-002 shares files with T-001 (claimed by alice); pass --force to claim it anyway","task":"T-002","files_busy":["T-001"]}` |
| `DUPLICATE_PROJECT` | 1 | `project add` for a file, or a name, that the project register already has (R-PROJ-1) | `{"code":"DUPLICATE_PROJECT","message":"C:\\Projekte\\TODO.md is already registered as renderer"}` |
| `VERIFY_FAILED` | 1 | A task's `verify` command failed or timed out: `done` is refused (the task stays claimed, or is given back as `stuck`), or `todo verify <task>` failed (R-VERIFY-1) | `{"code":"VERIFY_FAILED","message":"verify of T-012 failed (exit code 1), so T-012 is not done and stays claimed: tests failed","task":"T-012","exit_code":1,"last_line":"tests failed","timed_out":false,"log":"C:\\Projekte\\.git\\todo\\TODO.md\\verify\\T-012.log"}` |
| `VERIFY_NOT_RUN` | 5 | A `verify` command could not run: no shell could start, or `verify = false` in `todo.toml`; nothing is written (R-VERIFY-1) | `{"code":"VERIFY_NOT_RUN","message":"verify commands are off: verify = false in todo.toml"}` |

### `TOOL_TOO_OLD` (`R-VER-2`)

A `TODO.md` or archive whose format line names a version this todo does not know (for example
`<!-- todo-format: 3 -->` for todo 0.6) stops every command that reads or changes task content, with
exit 4 and the error below. The message names the version stamped in the file's tool line and the
running version. Nothing is written. Run `todo help update --json` for the steps that update todo.

```json
{
  "schema": "todo.list.v1",
  "ok": false,
  "error": {
    "code": "TOOL_TOO_OLD",
    "message": "format version 3 is newer than this todo 0.7.0 supports; the file was last written by todo 0.9.0. Update the todo executable first (`todo help update`).",
    "line": 1,
    "file": "TODO.md"
  },
  "warnings": []
}
```

`where --json` still succeeds for such a file. It shows `format_version` (3), `tool_stamp` (`0.9.0`)
and `running_version` (`0.7.0`), leaves out `counts` because the tasks are not read, and warns on the
format line and the tool line.
