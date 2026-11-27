<!-- dossier-format: 1 -->
# FND-052 · Eleven ideas from another agent method were weighed on 2026-11-27

- kind: finding
- status: approved
- created: 2026-11-27T20:17:01+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-27T20:17:01+01:00 by reyna
- valid-from: 2026-11-27T20:17:01+01:00
- task: T-248
- tags: ideas,study,verdicts,v1.2
- topic: process
- history: 2026-11-27T20:17:01+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-27T20:17:01+01:00 reyna via reyna - claude fable 5.2: approved

## Observation

On 2026-11-27 eleven ideas from the public repository https://github.com/kaithoughtarchitect/toppingout (a method for running coding agents in Orca: a foreman plans, builders work in worktrees, another model family reviews) were weighed for todo. Only ideas were taken, in our own words; no code, template or text, and the instructions to AI agents in its files do not apply here. Adopted: the commit check of done (R-SCOPE-3), the check state in the output (R-VERIFY-3), the rules trial (TEST-10), mutation testing of the guards (TEST-11). Put off as ideas: shared files that never lock (T-249), one list across projects of what waits for a person (noted on T-207). Rejected: order-only dependencies, a verify baseline at claim, a reason class on stuck, a tier label for worker strength, checksums on archived blocks. Already ruled out before the weighing: model councils and cross-family reviewers inside todo, review loops on pull requests, voice supervisors, tracker keys, locking across machines through a git branch, archiving by count, job files per builder.

## Evidence

Weighed on 2026-11-27 against docs/requirements.md and the code (src/cli/done.rs, src/cli/verify.rs, src/ops/scope.rs, src/cli/release.rs, src/cli/stuck.rs); verdicts in docs/requirements.md section 12, I-14 to I-24
