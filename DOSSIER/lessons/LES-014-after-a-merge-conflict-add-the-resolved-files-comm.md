<!-- dossier-format: 1 -->
# LES-014 · After a merge conflict, add the resolved files, commit, and only then start dependent workers

- kind: lesson
- status: draft
- created: 2026-11-26T19:51:12+01:00
- author: reyna
- via: reyna - claude fable 5.2
- confidence: experience
- topic: process
- related: DEC-022
- history: 2026-11-26T19:51:12+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-022

## Statement

After resolving a merge conflict, run git add before git commit --no-edit, and start workers whose tasks depend on the merge only after the merge commit exists, because they start from the local branch.

## Valid for

Orchestration runs in which workers branch from the local development branch.

## Evidence

v1.0 release run of 2026-11-22
