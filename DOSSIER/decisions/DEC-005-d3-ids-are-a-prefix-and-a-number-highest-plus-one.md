<!-- dossier-format: 1 -->
# DEC-005 · D3: IDs are a prefix and a number, highest plus one, never reused

- kind: decision
- status: approved
- created: 2026-11-26T19:47:12+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:47:12+01:00 by reyna
- valid-from: 2026-11-26T19:47:12+01:00
- standing: yes
- topic: format
- code: src/doc/id.rs
- related: FND-005, DEC-029
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:39+01:00 reyna via reyna - claude fable 5.2: changed — related FND-005
- history: 2026-11-26T19:51:39+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-029

## Decision

IDs are a prefix and a number (T-001), chosen as the highest so far plus one, under the lock, and never reused, also not after a task was archived or deleted by hand. 12, t12, T-12 and T-012 all name task 12.

## Reason

Short, sortable and easy to say; commit messages start with the ID, so an ID must keep its meaning for ever.

## Rejected

Random IDs hidden in comments: they only help when tasks are added on separate branches, which D5 avoids. Title codes: they were not unique in practice.
