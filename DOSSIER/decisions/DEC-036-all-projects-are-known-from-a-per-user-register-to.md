<!-- dossier-format: 1 -->
# DEC-036 · All projects are known from a per-user register; todo never searches the PC

- kind: decision
- status: approved
- created: 2026-11-26T19:47:13+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:47:13+01:00 by reyna
- valid-from: 2026-11-26T19:47:13+01:00
- task: T-093
- standing: yes
- topic: projects
- supersedes: DEC-028
- code: src/register
- related: DEC-037, DEC-058
- history: 2026-11-26T19:47:13+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:47:13+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:40+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-037
- history: 2026-11-26T19:51:40+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-058

## Decision

The overview of all projects is back, with one change: todo never scans the PC for TODO.md files. A project is registered with todo project add and noted in one per-user file, projects.json in the user data folder (%LOCALAPPDATA%\todo, or the XDG data folder on Linux). overview and report --html --all read only that register. init may print a hint but does not register a project by itself. (Owner, 2026-11-18)

## Reason

The owner wants one place to see all projects, without a tool that walks through private folders.

## Rejected

Scanning a root folder for TODO.md files.
