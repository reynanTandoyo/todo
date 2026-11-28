<!-- dossier-format: 1 -->
# FND-062 · In a shared checkout --commit HEAD can name another worker's commit

- kind: finding
- status: approved
- created: 2026-11-28T03:21:29+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-28T03:21:29+01:00 by reyna
- valid-from: 2026-11-28T03:21:29+01:00
- task: T-269
- tags: workers,commit,checkout,head
- topic: process,git
- history: 2026-11-28T03:21:29+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-28T03:21:29+01:00 reyna via reyna - claude fable 5.2: approved

## Observation

In one shared checkout with several workers, 'todo done <id> --commit HEAD' can name another worker's commit: HEAD moves whenever any worker commits, and the worker rules and the work order both tell a worker to pass HEAD. Seen twice on 2026-11-28 (workers of T-268 and T-269): each noticed that HEAD had become another worker's commit and passed its own hash instead. With file scopes the wrong commit is refused (OUT_OF_SCOPE), so nothing false is recorded, but the worker loses an attempt; for a task without files the wrong commit would be recorded. Also seen four times: a worker that tried a hand-made mutant in a scratch copy used the same CARGO_TARGET_DIR as its checkout build, left a broken todo.exe there, and its next verify failed until it ran cargo clean -p todo.

## Evidence

Worker reports for T-268, T-269, T-264 and T-270, and the notes on those tasks, 2026-11-28
