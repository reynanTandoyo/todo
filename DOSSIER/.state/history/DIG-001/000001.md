<!-- dossier-format: 1 -->
# DIG-001 · Digest of T-248: A done that holds what was checked, and guards that are tested

- kind: digest
- status: draft
- created: 2026-11-28T06:13:24+01:00
- author: reyna
- via: reyna - claude fable 5.2
- milestone: T-248
- tasks: T-248, T-250, T-251, T-252, T-253, T-254, T-255, T-256, T-257, T-258, T-259, T-260, T-261, T-262, T-263, T-264, T-265, T-266, T-267, T-268, T-269, T-270, T-271, T-272, T-273, T-275
- history: 2026-11-28T06:13:24+01:00 reyna via reyna - claude fable 5.2: created

## Summary

Built on 2026-11-27 and 2026-11-28 from eleven ideas of another agent method: four adopted, two put off, five rejected (FND-052).
done --commit and the done operations of apply refuse a commit that does not hold the task's files as they are on disk: error code UNCOMMITTED_CHANGES, exit 1, --force leaves a note (R-SCOPE-3, DEC-071).
done prints what happened to the check and carries it as check in JSON; verify --wave --json carries timed_out (R-VERIFY-3, DEC-072).
The file format stays version 3. The one tightening: a done that was accepted with uncommitted files inside the task's files is now refused.
Rules trial (TEST-10, DEC-073): scripts/rules-trial.ps1 and docs/rules-trial.md. First run: one Haiku worker kept all four rules and refused the three tempting steps by name (FND-059). One sample.
Mutation testing (TEST-11, DEC-074): 532 mutants in six guard modules; the tests noticed 80 percent, and about 90 percent after eight test tasks (FND-060, FND-063). 46 mutants stay unnoticed, each with its reason in docs/mutation-testing.md.
The new tests found one fault in a guard: the verified write refused a correct append after an empty insert at the end of a file without a final newline (FND-061, fixed by T-272).
Also fixed: done --commit no longer depends on git's English error text (T-273).
Run: 25 tasks, 21 of them by Haiku workers in one shared checkout; two tasks came back because the plan had missed something, and both were finished after the planner corrected the task.
Found on the way: workers that build in one checkout block each other on target/debug/todo.exe, so each needs its own build folder (FND-058); --commit HEAD can name another worker's commit (FND-062, idea T-274).
Not done: no version bump, no release, no push. ASM-005 waits for the owner's word.
