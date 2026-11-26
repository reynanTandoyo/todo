<!-- dossier-format: 1 -->
# DEC-050 · todo waves shows how much can run at once; the number of waves is what the planner shrinks

- kind: decision
- status: approved
- created: 2026-11-26T19:47:13+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:47:13+01:00 by reyna
- valid-from: 2026-11-26T19:47:13+01:00
- standing: yes
- topic: planning
- code: src/ops/waves.rs
- related: DEC-047, FND-018
- history: 2026-11-26T19:47:13+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:47:13+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:42+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-047
- history: 2026-11-26T19:51:42+01:00 reyna via reyna - claude fable 5.2: changed — related FND-018

## Decision

todo waves groups the unfinished tasks into waves by their dependencies, shows tasks of one wave that share a file as one lane, and prints the number of waves, the most workers useful at once and the longest chain. Tasks should be as independent of each other as possible. (Owner, 2026-11-19)

## Reason

The owner wants the planner to spawn as many agents as are useful so the job gets done faster. The number of waves is the fewest rounds the work needs however many agents are spawned.
