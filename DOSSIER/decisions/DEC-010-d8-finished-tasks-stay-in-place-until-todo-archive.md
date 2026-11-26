<!-- dossier-format: 1 -->
# DEC-010 · D8: Finished tasks stay in place until todo archive moves them

- kind: decision
- status: approved
- created: 2026-11-26T19:47:12+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:47:12+01:00 by reyna
- valid-from: 2026-11-26T19:47:12+01:00
- standing: yes
- topic: format
- code: src/cli/archive.rs
- related: FND-001
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:39+01:00 reyna via reyna - claude fable 5.2: changed — related FND-001

## Decision

Finished tasks stay where they are. todo archive moves done and dropped entries to TODO-archive.md on request; show, list --archive, search --archive and dependency checks still find them. From v0.6 a milestone tree is archived only as a whole.

## Reason

Context stays visible while work goes on; the file shrinks when someone decides it should. Without an archive the earlier files grew to 590 KB, mostly finished tasks.

## Rejected

Moving finished tasks automatically; archiving single tasks out of an open milestone, which would make its progress count wrong.
