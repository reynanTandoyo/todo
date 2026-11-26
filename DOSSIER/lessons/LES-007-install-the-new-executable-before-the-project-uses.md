<!-- dossier-format: 1 -->
# LES-007 · Install the new executable before the project uses a new setting or file format

- kind: lesson
- status: approved
- created: 2026-11-26T19:51:11+01:00
- author: reyna
- via: reyna - claude fable 5.2
- confidence: experience
- topic: versions
- related: DEC-062, FND-032
- approved: 2026-11-26T23:20:10+01:00 by reyna
- valid-from: 2026-11-26T23:20:10+01:00
- history: 2026-11-26T19:51:11+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-062
- history: 2026-11-26T19:51:44+01:00 reyna via reyna - claude fable 5.2: changed — related FND-032
- history: 2026-11-26T23:20:10+01:00 reyna via reyna - claude fable 5.2: approved

## Statement

When a release adds a setting or raises the file format, install the new executable on every PC before the project's files use it. An older executable refuses an unknown setting or format, and so does every worker that runs it.

## Valid for

Tools that refuse settings and formats they do not know, used by several agents from one PATH.

## Evidence

FND-032; v1.0 release run of 2026-11-22
