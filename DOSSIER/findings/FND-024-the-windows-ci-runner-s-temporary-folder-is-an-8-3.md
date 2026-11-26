<!-- dossier-format: 1 -->
# FND-024 · The Windows CI runner's temporary folder is an 8.3 short path

- kind: finding
- status: approved
- created: 2026-11-26T19:49:25+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:49:25+01:00 by reyna
- valid-from: 2026-11-26T19:49:25+01:00
- topic: testing
- code: tests
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: approved

## Observation

A test that compared a temporary path with its canonical form failed on the Windows runner, because TEMP there is a short 8.3 path. It can be reproduced locally by setting TEMP to a short path.

## Evidence

CI on commit 107b4d5, tests/lock_state.rs, 2026-11-15
