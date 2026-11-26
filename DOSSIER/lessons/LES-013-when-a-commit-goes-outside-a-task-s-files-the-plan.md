<!-- dossier-format: 1 -->
# LES-013 · When a commit goes outside a task's files, the planner widens the files

- kind: lesson
- status: draft
- created: 2026-11-26T19:51:12+01:00
- author: reyna
- via: reyna - claude fable 5.2
- confidence: experience
- topic: verify
- related: DEC-047
- history: 2026-11-26T19:51:12+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:51:42+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-047

## Statement

When todo done refuses a commit because it changed a file outside the task's files, the planner decides: it widens files with todo set, or sends the task back. A worker that uses --force hides a planning mistake and may break another worker's task.

## Valid for

Projects that use file scopes with several workers in one checkout.

## Evidence

v1.0 release run of 2026-11-22
