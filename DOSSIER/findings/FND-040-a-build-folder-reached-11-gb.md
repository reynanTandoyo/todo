<!-- dossier-format: 1 -->
# FND-040 · A build folder reached 11 GB

- kind: finding
- status: approved
- created: 2026-11-26T19:49:26+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:49:26+01:00 by reyna
- valid-from: 2026-11-26T19:49:26+01:00
- topic: process
- related: DEC-047
- history: 2026-11-26T19:49:26+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:49:26+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:41+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-047

## Observation

The cargo target folder of this repository grew to 11 GB and was cleared; the debug profile now uses line-tables-only debug information. Each worktree has its own build folder of that order, which is the main disk cost of a worktree per agent.

## Evidence

Refactoring run of 2026-11-15; Cargo.toml profile.dev
