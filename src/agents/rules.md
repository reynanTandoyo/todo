# Working with TODO.md

- Never edit `TODO.md` (or `TODO-archive.md`) directly; always use `todo`.
- Use one unique agent name per agent via `--agent <name>` or `TODO_AGENT`.
- Start by running `todo summary` or `todo next`.
- Claim a task before working on it (`todo claim <id>`, or `todo next --claim` to pick and claim in one step).
- Before you start a task, read the goals of the milestones above it: `todo claim` and `todo show <id>` print them.
- Exit code `1` means someone else claimed it or the rule refused the change — pick another task.
- Exit code `3` means the lock or lease is busy — try the command again.
- Add notes as you learn things (`todo note <id> "..."`).
- Finish a task after committing: `todo done <id> --note "..." --commit HEAD`.
- Close a milestone with `todo done <milestone-id>` once its last task is finished; `todo done` names a milestone when it is ready to close.
- Put each new task under a milestone, with a `why`: `todo add --title "..." --milestone <id> --why "..." --priority P2 --agent <name>`. A milestone is a piece of work with a goal, such as "Develop the renderer", never a release label.
- Use `todo add --title "..." --milestone <id> --why "..."` to record problems you find instead of fixing them silently.
- Record ideas with `todo idea "..."`, and questions about a task or milestone with `todo ask "..." --about <id>`.
- Use `todo ask "..."` for questions only a person can decide.
- Start every git commit message with the task ID (for example `T-012: short summary`).
- After todo is updated, run `todo migrate --yes` in each project.
- When todo says a file needs a newer todo (`TOOL_TOO_OLD`, or a warning that it is behind), update the todo executable first: follow `todo help update`.
