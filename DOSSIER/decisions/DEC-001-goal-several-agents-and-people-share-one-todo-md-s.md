<!-- dossier-format: 1 -->
# DEC-001 · Goal: several agents and people share one TODO.md safely

- kind: decision
- status: approved
- created: 2026-11-26T19:47:11+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:47:11+01:00 by reyna
- valid-from: 2026-11-26T19:47:11+01:00
- standing: yes
- topic: process
- related: DEC-002, DEC-044
- history: 2026-11-26T19:47:11+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:47:11+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:40+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-002
- history: 2026-11-26T19:51:40+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-044

## Decision

todo is a command-line program that lets several AI agents and people share one Markdown task list, TODO.md, without corrupting it or losing each other's changes. It finds the file, takes a lock, changes only the lines it has to and replaces the file in one step. Since v0.8 the goal goes further: a planner splits the work until every task is level 1, and any number of cheap agents finish those tasks at the same time. Since v1.0 a person sees the state of the work on one read-only HTML page. (docs/requirements.md section 1)

## Reason

The owner's own task scripts in two earlier projects lost no update but had stale copies in worktrees, private lock protocols and no safe way for many agents to work at once; the owner wants one finished, self-contained tool.
