<!-- dossier-format: 1 -->
# LES-005 · Do not assert a wall-clock limit on a debug build; prove the property another way

- kind: lesson
- status: approved
- created: 2026-11-26T19:51:11+01:00
- author: reyna
- via: reyna - claude fable 5.2
- confidence: measured
- topic: testing
- code: tests
- related: FND-029
- approved: 2026-11-26T23:20:10+01:00 by reyna
- valid-from: 2026-11-26T23:20:10+01:00
- history: 2026-11-26T19:51:11+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related FND-029
- history: 2026-11-26T23:20:10+01:00 reyna via reyna - claude fable 5.2: approved

## Statement

A test that asserts "faster than N ms" on a debug build fails at random. Prove the property itself instead, for example that a command returns before a much longer timeout it would otherwise wait for.

## Valid for

Tests that run the built program on a loaded development PC or a CI runner.

## Evidence

FND-029; task T-122
