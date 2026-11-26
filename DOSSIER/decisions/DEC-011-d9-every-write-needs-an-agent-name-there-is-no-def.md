<!-- dossier-format: 1 -->
# DEC-011 · D9: Every write needs an agent name; there is no default

- kind: decision
- status: approved
- created: 2026-11-26T19:47:12+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:47:12+01:00 by reyna
- valid-from: 2026-11-26T19:47:12+01:00
- standing: yes
- topic: claims
- code: src/cli/mod.rs
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: approved

## Decision

Every write command needs an agent name, from --agent or TODO_AGENT. Without one it exits 2 with NO_AGENT. Each agent uses its own unique name.

## Reason

A shared default name would make claims meaningless.

## Rejected

A fallback name such as the user name.
