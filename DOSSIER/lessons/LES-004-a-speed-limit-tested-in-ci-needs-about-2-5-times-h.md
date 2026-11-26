<!-- dossier-format: 1 -->
# LES-004 · A speed limit tested in CI needs about 2.5 times headroom over the local number

- kind: lesson
- status: approved
- created: 2026-11-26T19:51:11+01:00
- author: reyna
- via: reyna - claude fable 5.2
- confidence: measured
- topic: testing
- code: tests
- related: FND-019
- approved: 2026-11-26T23:20:10+01:00 by reyna
- valid-from: 2026-11-26T23:20:10+01:00
- history: 2026-11-26T19:51:11+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related FND-019
- history: 2026-11-26T23:20:10+01:00 reyna via reyna - claude fable 5.2: approved

## Statement

Keep a command at 40 percent of its limit or below on the development PC when the limit is asserted in CI. The GitHub runners were about 2.5 times slower than this PC, and a limit met locally with little room failed there.

## Valid for

Speed tests on shared CI runners (windows-latest, ubuntu-latest), measured as medians of release builds.

## Evidence

FND-019; tasks T-185 to T-188
