# Working with TODO.md

- Never edit `TODO.md` or `TODO-archive.md` directly; always use `todo`.
- Use one unique agent name per agent: `--agent <name>` or `TODO_AGENT`.
- Exit code `1`: someone else has the task, or a rule refused the change; pick another task.
- Exit code `3`: the lock or lease is busy; run the same command again.
- Before you start a task, read the goals of the milestones above it (`todo show <id>` prints them).
- Start every commit message with the task ID, such as `T-012: short summary`.
- Workers: run `todo help worker`.
- Planner: run `todo help planner`.
