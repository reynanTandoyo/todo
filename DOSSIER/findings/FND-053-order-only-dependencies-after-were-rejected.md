<!-- dossier-format: 1 -->
# FND-053 · Order-only dependencies (after) were rejected

- kind: finding
- status: approved
- created: 2026-11-27T20:17:01+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-27T20:17:01+01:00 by reyna
- valid-from: 2026-11-27T20:17:01+01:00
- task: T-248
- tags: depends,waves,after,rejected
- topic: planning
- history: 2026-11-27T20:17:01+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-27T20:17:01+01:00 reyna via reyna - claude fable 5.2: approved

## Observation

Rejected: a second relation 'after' that next and claim ignore and done enforces (order-only dependencies). In one shared checkout finishing a task is its commit, so a worker whose task may be built but not yet finished would hold finished work it cannot close, and would have to wait or poll, which a weak worker does badly. The waves would not honestly get fewer, because the done order still runs one after another. The planner already has the two right tools: remove a dependency that is not needed, or keep it (R-WAVE-1).

## Evidence

Weighed on 2026-11-27 against docs/requirements.md and the code (src/cli/done.rs, src/cli/verify.rs, src/ops/scope.rs, src/cli/release.rs, src/cli/stuck.rs); verdicts in docs/requirements.md section 12, I-14 to I-24
