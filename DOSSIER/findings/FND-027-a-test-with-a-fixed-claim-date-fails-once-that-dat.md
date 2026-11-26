<!-- dossier-format: 1 -->
# FND-027 · A test with a fixed claim date fails once that date is over 24 hours old

- kind: finding
- status: approved
- created: 2026-11-26T19:49:25+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:49:25+01:00 by reyna
- valid-from: 2026-11-26T19:49:25+01:00
- task: T-084
- topic: testing
- code: tests
- related: LES-006
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related LES-006

## Observation

tests/cmd_apply.rs used a fixed claim time; a day later the claim counted as stale and the test failed. Tests that depend on claim age now pin the clock with TODO_TEST_NOW.

## Evidence

Task T-084, 2026-11-17
