<!-- dossier-format: 1 -->
# DEC-074 · The guard modules are mutation-tested by hand

- kind: decision
- status: approved
- created: 2026-11-27T20:16:42+01:00
- author: reyna
- via: reyna - claude fable 5.2
- task: T-248
- tags: mutation,guards,tests,cargo-mutants
- topic: testing
- approved: 2026-11-27T20:24:04+01:00 by reyna
- valid-from: 2026-11-27T20:24:04+01:00
- history: 2026-11-27T20:16:42+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-27T20:16:42+01:00 reyna via reyna - claude fable 5.2: proposed
- history: 2026-11-27T20:24:04+01:00 reyna via reyna - claude fable 5.2: accepted

## Decision

The guard modules (lock, write to disk, line model, status rules, file scopes, commit checks) are mutation-tested by hand with cargo-mutants before a release that changes one of them; every missed mutant gets a test or a recorded reason (TEST-11)

## Reason

todo's promise is safety, and a guard that can be broken without a test going red is not protected against the next change. cargo-mutants is a tool installed apart, so it is no dependency of the crate (NFR-9).

## Rejected

Running it in CI on every push: a full run takes far longer than the test suite. Mutating the whole crate: the report and the help texts are not guards, and the run time grows with every module.
