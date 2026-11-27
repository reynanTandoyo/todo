<!-- dossier-format: 1 -->
# DEC-071 · done refuses a commit that does not hold the task's files as they are on disk

- kind: decision
- status: proposed
- created: 2026-11-27T20:16:31+01:00
- author: reyna
- via: reyna - claude fable 5.2
- task: T-248
- tags: done,commit,scope,verify
- topic: verify,git
- history: 2026-11-27T20:16:31+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-27T20:16:31+01:00 reyna via reyna - claude fable 5.2: proposed

## Decision

From todo 1.2, done --commit refuses a commit that does not hold the task's files as they are on disk (R-SCOPE-3, error code UNCOMMITTED_CHANGES, exit 1); --force overrides it with a note

## Reason

The verify command runs on the files on disk. A worker that commits and then changes a file again, or forgets to add a new file, got a passing check and a recorded commit that was never checked: a false done, which R-VERIFY-1 exists to prevent. Found by reading src/cli/done.rs on 2026-11-27: nothing compared the commit with the folder. The check is limited to the task's own files, so other workers' uncommitted edits in the same checkout do not matter. It is a new refusal in a minor version: a done that was accepted with changed files is now refused until they are committed.

## Rejected

Require a clean working tree: impossible in one shared checkout, where other workers have uncommitted edits (DEC-047). An opt-in setting: a project that does not know the setting keeps the false done. Reuse OUT_OF_SCOPE: existing error codes never change meaning (DEC-054), and the fix is different (commit, not widen the files). Record the commit at claim and run verify at claim as a baseline: see the finding on the baseline idea.
