<!-- dossier-format: 1 -->
# FND-063 · Mutation re-run: 39 of 51 gaps caught, 12 misjudged, about 90 percent noticed

- kind: finding
- status: approved
- created: 2026-11-28T06:13:03+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-28T06:13:03+01:00 by reyna
- valid-from: 2026-11-28T06:13:03+01:00
- task: T-271
- tags: mutation,guards,tests,cargo-mutants
- topic: testing
- history: 2026-11-28T06:13:03+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-28T06:13:03+01:00 reyna via reyna - claude fable 5.2: approved

## Observation

Mutation re-run (TEST-11) on 2026-11-28, commit 9b92fe8, after the tests of T-264 to T-270 and T-275 and the fixes T-272 and T-273: 535 mutants, 46 missed, 5 timeouts, 484 caught or unviable (78 unviable in the first run); about 90 percent of the mutants that compile are noticed, 80 percent before. Of the 51 mutants judged as real gaps in the first run, 39 are caught now (34 by the first seven test tasks, 5 after T-275 sharpened three tests) and 12 were misjudged: a second look at the code showed they are no gap. Two of the three weak tests came from the planner's own steps: a fixture whose two line endings were the same, and a test on another code path than the mutated one. A new test only counts once its mutant has been run again.

## Evidence

target/mutants/pilot/mutants.out of the runs 4 to 6 (missed.txt 46, timeout.txt 5, previously_caught.txt); docs/mutation-testing.md
