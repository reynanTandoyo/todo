---
name: todo
description: Share a TODO.md task list safely across agents and people using the `todo` CLI.
---

# Working with TODO.md

- Never edit `TODO.md` (or `TODO-archive.md`) directly; always use `todo`.
- Use one unique agent name per agent via `--agent <name>` or `TODO_AGENT`.
- Start by running `todo summary` or `todo next`.
- Claim a task before working on it (`todo claim <id>`, or `todo next --claim` to pick and claim in one step).
- Exit code `1` means someone else claimed it or the rule refused the change — pick another task.
- Exit code `3` means the lock or lease is busy — try the command again.
- Add notes as you learn things (`todo note <id> "..."`).
- Finish a task after committing: `todo done <id> --note "..." --commit HEAD`.
- Use `todo add --title "..." --priority P2` to record problems you find instead of fixing them silently.
- Use `todo ask "..."` for questions only a person can decide.
- Start every git commit message with the task ID (for example `T-012: short summary`).
- When todo says a file needs a newer todo (`TOOL_TOO_OLD`, or a warning that it is behind), update the todo executable first: follow `todo help update`.
