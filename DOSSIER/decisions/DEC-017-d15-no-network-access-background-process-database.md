<!-- dossier-format: 1 -->
# DEC-017 · D15: No network access, background process, database or index

- kind: decision
- status: approved
- created: 2026-11-26T19:47:12+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:47:12+01:00 by reyna
- valid-from: 2026-11-26T19:47:12+01:00
- standing: yes
- topic: cli
- related: DEC-042, DEC-038, DEC-065
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:40+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-042
- history: 2026-11-26T19:51:40+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-038
- history: 2026-11-26T19:52:40+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-065

## Decision

todo has no network access, no telemetry, no background process, no database and no index. git is optional and used only when it is on PATH.

## Reason

Fewer ways to fail; it works offline and in any folder.

## Rejected

A database or index next to the file; a daemon; git as the transaction log.
