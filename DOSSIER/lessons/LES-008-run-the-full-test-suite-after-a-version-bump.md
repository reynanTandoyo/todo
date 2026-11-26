<!-- dossier-format: 1 -->
# LES-008 · Run the full test suite after a version bump

- kind: lesson
- status: approved
- created: 2026-11-26T19:51:11+01:00
- author: reyna
- via: reyna - claude fable 5.2
- confidence: experience
- topic: testing
- code: tests
- related: DEC-062, FND-033
- approved: 2026-11-26T23:20:10+01:00 by reyna
- valid-from: 2026-11-26T23:20:10+01:00
- history: 2026-11-26T19:51:11+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-062
- history: 2026-11-26T19:51:44+01:00 reyna via reyna - claude fable 5.2: changed — related FND-033
- history: 2026-11-26T23:20:10+01:00 reyna via reyna - claude fable 5.2: approved

## Statement

A version bump is a code change: run every test after it. Tests that use a fixed version number as "newer than the program" break when the program passes that number.

## Valid for

Programs that compare version numbers, such as a tool stamp or a format check.

## Evidence

FND-033; v1.0 release run of 2026-11-22
