<!-- dossier-format: 1 -->
# FND-055 · A reason class on stuck was rejected: release and stuck are the two classes

- kind: finding
- status: approved
- created: 2026-11-27T20:17:01+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-27T20:17:01+01:00 by reyna
- valid-from: 2026-11-27T20:17:01+01:00
- task: T-248
- tags: stuck,release,attempts,rejected
- topic: claims
- history: 2026-11-27T20:17:01+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-27T20:17:01+01:00 reyna via reyna - claude fable 5.2: approved

## Observation

Rejected: a reason class on stuck or release (environment or task) with an attempts count. The two classes are already two commands: release gives a task back as open, for anyone to take (the task is fine), and stuck gives it to the planner as blocked with the problem (the task is wrong or too big). attempts exists and plan lists it (R-STUCK-1). A worker whose session died or hit a rate limit runs no command at all; its claim is freed by the session-end hook or goes stale after 24 hours (R-CLAIM-7, D4). A missing tool on the machine stops every worker, so it is the planner's to fix, which is stuck.

## Evidence

Weighed on 2026-11-27 against docs/requirements.md and the code (src/cli/done.rs, src/cli/verify.rs, src/ops/scope.rs, src/cli/release.rs, src/cli/stuck.rs); verdicts in docs/requirements.md section 12, I-14 to I-24
