<!-- dossier-format: 1 -->
# FND-039 · The test suite at v1.0: 1041 tests in 105 test binaries

- kind: finding
- status: approved
- created: 2026-11-26T19:49:25+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:49:25+01:00 by reyna
- valid-from: 2026-11-26T19:49:25+01:00
- task: T-218
- topic: testing
- code: tests
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: approved

## Observation

cargo test --all-features passes 1041 tests across 105 test binaries on Windows; the five performance tests run with --ignored. Linux runs 3 fewer tests, which are Windows-only.

## Evidence

Task T-218 closing note, 2026-11-21; Linux check in WSL
