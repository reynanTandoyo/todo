<!-- dossier-format: 1 -->
# DEC-013 · D11: Settings are optional: one todo.toml next to TODO.md, no per-user settings

- kind: decision
- status: superseded
- created: 2026-11-26T19:47:12+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:47:12+01:00 by reyna
- valid-from: 2026-11-26T19:47:12+01:00
- standing: yes
- topic: cli
- code: src/ops/config.rs
- valid-until: 2026-11-26T19:47:13+01:00
- superseded-by: DEC-037
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:47:13+01:00 reyna via reyna - claude fable 5.2: superseded — by DEC-037

## Decision

Settings are optional. They live in one todo.toml next to TODO.md, and there are no per-user settings. Everything works without a settings file. An unknown key or a bad value is an error.

## Reason

New projects need no setup; projects that want rules can have them.

## Rejected

A per-user settings file.
