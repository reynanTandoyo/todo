<!-- dossier-format: 1 -->
# DEC-022 · One commit per task that starts with its ID; three checks must pass; nobody pushes but the owner

- kind: decision
- status: approved
- created: 2026-11-26T19:47:12+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:47:12+01:00 by reyna
- valid-from: 2026-11-26T19:47:12+01:00
- standing: yes
- topic: process
- related: TOOL-001, LES-014, DEC-068
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related TOOL-001
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related LES-014
- history: 2026-11-26T19:52:40+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-068

## Decision

Work is taken with todo and finished with one commit per task whose message starts with the task ID. Before a task counts as done, cargo fmt --check, cargo clippy with warnings as errors and cargo test --all-features must pass. Agents never push; the owner decides when to push, and every push needs the owner's word again.

## Reason

The task ID in the commit links the history to the plan (todo align reads it). The owner keeps control of what becomes public.

## Rejected

Agents pushing on their own after a green test run.
