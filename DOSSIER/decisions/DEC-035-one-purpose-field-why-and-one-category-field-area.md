<!-- dossier-format: 1 -->
# DEC-035 · One purpose field, why, and one category field, area

- kind: decision
- status: approved
- created: 2026-11-26T19:47:13+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:47:13+01:00 by reyna
- valid-from: 2026-11-26T19:47:13+01:00
- standing: yes
- topic: planning
- history: 2026-11-26T19:47:13+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:47:13+01:00 reyna via reyna - claude fable 5.2: approved

## Decision

A task says what it is for in a why field. Its category is the existing area field, for example ui, numerics or math, with one area per entry; todo.toml can fix the allowed list. No further detail fields were added in v0.6. (Owner, 2026-11-18)

## Reason

problem, fix, accept, files, area, type and sections already existed, and --field takes anything else; purpose was the gap.

## Rejected

A new category field; several categories per entry (kept as idea T-192).
