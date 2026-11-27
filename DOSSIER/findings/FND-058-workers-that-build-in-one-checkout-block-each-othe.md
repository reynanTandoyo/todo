<!-- dossier-format: 1 -->
# FND-058 · Workers that build in one checkout block each other on target/debug/todo.exe

- kind: finding
- status: approved
- created: 2026-11-27T20:49:27+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-27T20:49:27+01:00 by reyna
- valid-from: 2026-11-27T20:49:27+01:00
- task: T-258
- tags: workers,cargo,checkout,windows
- topic: process,verify
- history: 2026-11-27T20:49:27+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-27T20:49:27+01:00 reyna via reyna - claude fable 5.2: approved

## Observation

With four workers building in one checkout, todo done failed twice for T-258 with 'Access is denied' on target/debug/todo.exe: another worker's tests were running that program while cargo tried to replace it. The task shows 2 attempts although its code was right. It passed once the worker set CARGO_TARGET_DIR to a private folder. Also: rustfmt run on one file formats the child modules of that file and flags lines the worker did not touch, so 'format only your own file' cannot be done with plain rustfmt on a file that declares modules.

## Evidence

Worker reports for T-258 and T-254 and the notes on T-258, wave 1 of the v1.2 plan, 2026-11-27
