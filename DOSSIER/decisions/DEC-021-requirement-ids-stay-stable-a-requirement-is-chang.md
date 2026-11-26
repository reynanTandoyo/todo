<!-- dossier-format: 1 -->
# DEC-021 · Requirement IDs stay stable; a requirement is changed in a task, with the docs in the same commit

- kind: decision
- status: approved
- created: 2026-11-26T19:47:12+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:47:12+01:00 by reyna
- valid-from: 2026-11-26T19:47:12+01:00
- standing: yes
- topic: process
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: approved

## Decision

docs/requirements.md says what to build and docs/format.md is the reference for the file format. Requirement IDs such as R-LOCK-3 stay stable. Changing a requirement is itself a task in TODO.md; when a requirement turns out wrong or unclear while building, the documents are fixed in the same commit and a todo note says so.

## Reason

Tasks, tests, code comments and commit messages all refer to requirement IDs, so a renumbered or silently changed requirement would break those links.
