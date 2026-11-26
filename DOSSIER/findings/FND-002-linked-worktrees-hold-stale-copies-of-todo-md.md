<!-- dossier-format: 1 -->
# FND-002 · Linked worktrees hold stale copies of TODO.md

- kind: finding
- status: approved
- created: 2026-11-26T19:49:24+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:49:24+01:00 by reyna
- valid-from: 2026-11-26T19:49:24+01:00
- topic: git
- code: src/find.rs
- related: DEC-007, DEC-009
- history: 2026-11-26T19:49:24+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:49:24+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:39+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-007
- history: 2026-11-26T19:51:39+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-009

## Observation

One earlier project had 7 linked git worktrees, each with its own TODO.md, 34 to 993 lines behind the main checkout. A script started inside a worktree walks up from its folder and finds the stale copy first, and a lock on that copy protects nothing. (Research finding F3)

## Evidence

docs/research.md section 1.2, study of 2026-11-15
