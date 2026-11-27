<!-- dossier-format: 1 -->
# FND-051 · On a busy Windows PC a new powershell.exe started by a test check can take over

- kind: finding
- status: approved
- created: 2026-11-27T18:41:32+01:00
- author: reyna
- via: reyna - claude opus 5.5
- approved: 2026-11-27T18:41:32+01:00 by reyna
- valid-from: 2026-11-27T18:41:32+01:00
- task: T-219
- topic: testing
- code: tests/cmd_verify.rs
- history: 2026-11-27T18:41:32+01:00 reyna via reyna - claude opus 5.5: created
- history: 2026-11-27T18:41:32+01:00 reyna via reyna - claude opus 5.5: approved

## Observation

On a busy Windows PC a new powershell.exe started by a test check can take over ten seconds to start; cmd.exe starts in milliseconds. Tests that need a child process to be alive should use cmd.exe and wait for its first sign of life instead of a fixed pause (T-219: 9 of 16 runs failed under load before, 16 of 16 passed after).
