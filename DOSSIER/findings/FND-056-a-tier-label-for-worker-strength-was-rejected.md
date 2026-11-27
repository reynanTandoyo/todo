<!-- dossier-format: 1 -->
# FND-056 · A tier label for worker strength was rejected

- kind: finding
- status: approved
- created: 2026-11-27T20:17:01+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-27T20:17:01+01:00 by reyna
- valid-from: 2026-11-27T20:17:01+01:00
- task: T-248
- tags: levels,tier,workers,rejected
- topic: planning
- history: 2026-11-27T20:17:01+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-27T20:17:01+01:00 reyna via reyna - claude fable 5.2: approved

## Observation

Rejected: an optional label for how strong a worker must be (tier: standard or strong) on a fully detailed task. It contradicts the level model: level 1 means the weakest agent finishes the task, and the planner drives every task there (DEC-044, DEC-046); a task that is not that stays above the target level with final and a reason (R-LEVEL-3), and workers take only next --max-difficulty 1. A level-1 task that a weak worker fails shows up through stuck and attempts, and the planner splits it. R-LEVEL-1 names no agent, model or product; DEC-060 allows another model only by the owner's exception for one task.

## Evidence

Weighed on 2026-11-27 against docs/requirements.md and the code (src/cli/done.rs, src/cli/verify.rs, src/ops/scope.rs, src/cli/release.rs, src/cli/stuck.rs); verdicts in docs/requirements.md section 12, I-14 to I-24
