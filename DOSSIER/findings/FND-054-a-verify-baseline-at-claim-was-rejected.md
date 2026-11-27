<!-- dossier-format: 1 -->
# FND-054 · A verify baseline at claim was rejected

- kind: finding
- status: approved
- created: 2026-11-27T20:17:01+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-27T20:17:01+01:00 by reyna
- valid-from: 2026-11-27T20:17:01+01:00
- task: T-248
- tags: verify,claim,baseline,rejected
- topic: verify
- history: 2026-11-27T20:17:01+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-27T20:17:01+01:00 reyna via reyna - claude fable 5.2: approved

## Observation

Rejected: claim runs the task's verify command once as a baseline, so a later failure reads as 'already failing before' or 'introduced by this task'. The check of a level-1 task is usually a test the task itself adds, so the baseline fails by design. In one shared checkout a baseline measures other workers' half-finished edits (the reason wave mode exists). It runs every check twice. What is needed exists: todo verify <task> runs a check and changes nothing, and the checked wave before is the baseline of the next wave. The part worth having, that done proves the commit it records, became R-SCOPE-3.

## Evidence

Weighed on 2026-11-27 against docs/requirements.md and the code (src/cli/done.rs, src/cli/verify.rs, src/ops/scope.rs, src/cli/release.rs, src/cli/stuck.rs); verdicts in docs/requirements.md section 12, I-14 to I-24
