<!-- dossier-format: 1 -->
# LES-006 · Tests must clear the tool's environment variables and pin the clock

- kind: lesson
- status: approved
- created: 2026-11-26T19:51:11+01:00
- author: reyna
- via: reyna - claude fable 5.2
- confidence: experience
- topic: testing
- code: tests/common
- related: DEC-023, FND-027
- approved: 2026-11-26T23:20:10+01:00 by reyna
- valid-from: 2026-11-26T23:20:10+01:00
- history: 2026-11-26T19:51:11+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-023
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related FND-027
- history: 2026-11-26T23:20:10+01:00 reyna via reyna - claude fable 5.2: approved

## Statement

Every test that runs the program removes the tool's own environment variables and, where a result depends on age, sets a fixed clock. Otherwise the tests pass on one machine and read the developer's real data, or fail a day later.

## Valid for

Command-line tools that read settings from environment variables and compare times, tested from inside an agent session.

## Evidence

FND-027, FND-028; tasks T-084, T-120, T-131
