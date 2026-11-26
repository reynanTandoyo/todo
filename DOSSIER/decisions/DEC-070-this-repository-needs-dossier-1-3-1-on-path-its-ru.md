<!-- dossier-format: 1 -->
# DEC-070 · This repository needs dossier 1.3.1 on PATH; its rules block is installed and workers run with DOSSIER_ROLE=worker

- kind: decision
- status: approved
- created: 2026-11-26T23:17:07+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T23:17:07+01:00 by reyna
- valid-from: 2026-11-26T23:17:07+01:00
- standing: yes
- topic: process
- supersedes: DEC-069
- history: 2026-11-26T23:17:07+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T23:17:07+01:00 reyna via reyna - claude fable 5.2: approved

## Decision

Working on this repository needs dossier 1.3.1 or newer on PATH, next to the released todo; AGENTS.md names both under "Tools this repository needs". The todo program itself has no dependency on dossier. Dossier's rules block is installed in AGENTS.md with dossier agents install: 7 common lines, with dossier help worker and dossier help planner for each role. Every worker is started with DOSSIER_ROLE=worker in its environment, so it gets one line instead of the project brief and only the entries for its task; workers write nothing to the dossier, and the planner runs dossier review-task when a worker's task is done or given back. (Owner, 2026-11-26)

## Reason

dossier 1.3.1 splits its rules by role and keeps the session start of a worker quiet, so the short block fits weak workers; the four hand-written lines of the stopgap are no longer needed.
