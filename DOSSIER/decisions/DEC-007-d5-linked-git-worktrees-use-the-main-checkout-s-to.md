<!-- dossier-format: 1 -->
# DEC-007 · D5: Linked git worktrees use the main checkout's TODO.md and lock

- kind: decision
- status: approved
- created: 2026-11-26T19:47:12+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:47:12+01:00 by reyna
- valid-from: 2026-11-26T19:47:12+01:00
- standing: yes
- topic: git
- code: src/find.rs
- related: FND-002, FND-023
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:39+01:00 reyna via reyna - claude fable 5.2: changed — related FND-002
- history: 2026-11-26T19:51:39+01:00 reyna via reyna - claude fable 5.2: changed — related FND-023

## Decision

In a linked git worktree todo uses the TODO.md and the lock of the main checkout. It finds the main checkout from the worktree's .git file without needing git. The setting worktree = "local" turns this off.

## Reason

Otherwise agents in worktrees work on stale copies and their locks protect nothing: one earlier project had 7 worktrees, each 34 to 993 lines behind.

## Rejected

Each worktree with its own file, merged later.
