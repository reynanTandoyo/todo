<!-- dossier-format: 1 -->
# DEC-033 · A milestone is an entry of type milestone and is never closed automatically

- kind: decision
- status: approved
- created: 2026-11-26T19:47:13+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:47:13+01:00 by reyna
- valid-from: 2026-11-26T19:47:13+01:00
- standing: yes
- topic: milestones
- code: src/ops/milestone.rs
- related: DEC-032
- history: 2026-11-26T19:47:13+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:47:13+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:40+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-032

## Decision

A milestone is an entry in TODO.md with type milestone and a goal line, like a question is an entry of type question. next never hands it out and claim refuses it. done on a milestone is refused while anything below it is unfinished. A milestone is never closed automatically: done on its last task names it as ready to close, and summary keeps listing it. A task may depend on a milestone, and dependencies are inherited down the tree. Priority says how urgent a task is, not which release it belongs to.

## Reason

Reusing the entry format keeps IDs, notes, the archive and the merge driver working unchanged, and the format needed no new record type. Closing automatically would close a milestone too early, because more tasks may still be added to it.

## Rejected

A separate record type for milestones; closing a milestone when its last task is done.
