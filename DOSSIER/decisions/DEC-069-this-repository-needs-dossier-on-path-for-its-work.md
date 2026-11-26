<!-- dossier-format: 1 -->
# DEC-069 · This repository needs dossier on PATH for its work; todo itself does not depend on it

- kind: decision
- status: superseded
- created: 2026-11-26T20:00:13+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T20:00:13+01:00 by reyna
- valid-from: 2026-11-26T20:00:13+01:00
- standing: yes
- topic: process
- valid-until: 2026-11-26T23:17:07+01:00
- superseded-by: DEC-070
- history: 2026-11-26T20:00:13+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T20:00:13+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T23:17:07+01:00 reyna via reyna - claude fable 5.2: superseded — by DEC-070

## Decision

Working on this repository needs dossier 1.2 or newer on PATH, next to the released todo. AGENTS.md names both under "Tools this repository needs". The todo program itself gains no dependency: it neither calls dossier nor needs it to run. Dossier's own 37-line rules block is not installed in AGENTS.md; four hand-written lines say what a worker and a planner must know, until dossier has rules by role. (Owner, 2026-11-26)

## Reason

The hooks in .claude/settings.json call dossier, so a session without it gets hook errors. Dossier's rules are about 1,080 tokens against 150 for todo's common rules, too much for a level-1 worker.

## Rejected

Installing dossier's full rules block in AGENTS.md for every agent; making todo call dossier.
