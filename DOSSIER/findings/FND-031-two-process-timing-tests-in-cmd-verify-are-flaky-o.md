<!-- dossier-format: 1 -->
# FND-031 · Two process-timing tests in cmd_verify are flaky on Windows

- kind: finding
- status: approved
- created: 2026-11-26T19:49:25+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:49:25+01:00 by reyna
- valid-from: 2026-11-26T19:49:25+01:00
- task: T-219
- topic: testing
- code: tests/cmd_verify.rs
- related: DEC-048
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:42+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-048

## Observation

a_check_that_outlives_verify_timeout_seconds_counts_as_failed hung once under full-suite load on this PC and left todo.exe and pwsh running; a_check_that_ends_normally_leaves_no_child_process_running failed once in CI on main (windows-latest) while the same commit passed on dev. Both passed alone and on a rerun. Still open as idea T-219.

## Evidence

Idea T-219; CI run 44908893803 of 2026-11-21
