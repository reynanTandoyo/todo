<!-- dossier-format: 1 -->
# DEC-072 · The output of done says what happened to the check

- kind: decision
- status: approved
- created: 2026-11-27T20:16:42+01:00
- author: reyna
- via: reyna - claude fable 5.2
- task: T-248
- tags: done,verify,json,output
- topic: verify,cli
- approved: 2026-11-27T20:24:04+01:00 by reyna
- valid-from: 2026-11-27T20:24:04+01:00
- history: 2026-11-27T20:16:42+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-27T20:16:42+01:00 reyna via reyna - claude fable 5.2: proposed
- history: 2026-11-27T20:24:04+01:00 reyna via reyna - claude fable 5.2: accepted

## Decision

From todo 1.2 the output of done says what happened to the task's check (passed, waiting for the wave, off, skipped, none), in text and as check in JSON, and verify --wave --json carries timed_out for each failed check (R-VERIFY-3)

## Reason

In wave mode, with verify = false and with --no-verify, done printed only 'done ok', so a check that had not run read like a pass. TODO.md already keeps the states apart (the verified line, the notes, attempts) and check and plan list done tasks without a pass, so only the output had the gap. The keys are added to the JSON; nothing existing changes.

## Rejected

A new field or status in TODO.md for 'not verified': the missing verified line already says it. New error or exit codes: VERIFY_FAILED with timed_out and VERIFY_NOT_RUN already separate failed, timed out and not run.
