<!-- dossier-format: 1 -->
# FND-029 · A wall-clock limit on a debug build fails at random

- kind: finding
- status: approved
- created: 2026-11-26T19:49:25+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:49:25+01:00 by reyna
- valid-from: 2026-11-26T19:49:25+01:00
- task: T-122
- topic: testing
- code: tests
- related: LES-005
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related LES-005

## Observation

A test asserted that todo status returns in under 500 ms; the debug todo.exe measured 100 to 900 ms on this PC. The test now proves the same thing without a clock limit: status returns before a 10 second lock timeout while a holder keeps the lock for 20 seconds.

## Evidence

Task T-122, tests/lock_lease.rs, 2026-11-18
