<!-- dossier-format: 1 -->
# FND-060 · First mutation run: the tests notice 80 percent of the changes to the guard modules

- kind: finding
- status: approved
- created: 2026-11-28T02:54:52+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-28T02:54:52+01:00 by reyna
- valid-from: 2026-11-28T02:54:52+01:00
- task: T-262
- tags: mutation,guards,tests,cargo-mutants
- topic: testing
- history: 2026-11-28T02:54:52+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-28T02:54:52+01:00 reyna via reyna - claude fable 5.2: approved

## Observation

First mutation run of the guard modules (TEST-11), 2026-11-27 to 2026-11-28, cargo-mutants 27.1.0 on Windows with 16 logical processors, commit 1e2f833: 532 mutants in src/lock.rs, src/atomic.rs, src/doc/lines.rs, src/ops/rules.rs, src/ops/scope.rs and src/ops/commit_files.rs. Caught 361 (17 of them only by the whole suite), timeout 4, unviable 78, missed 89; of the 454 that compile the tests noticed 365, which is 80 percent. Of the 89 missed, 51 are real test gaps (tasks T-264 to T-270) and 38 are no gap, each with its reason in docs/mutation-testing.md. The most serious gap: no test notices when a second agent's todo lock takes over another agent's live lease (src/lock.rs acquire_lease). Run time: about 3.5 hours for the first pass against 33 guard test programs with 4 to 6 jobs, 2 hours for the 110 missed mutants against the whole suite with 5 jobs; the whole suite for every mutant would have taken most of a day.

## Evidence

target/mutants/pilot/mutants.out (caught.txt, missed.txt, timeout.txt, unviable.txt, previously_caught.txt) of the three runs; docs/mutation-testing.md
