<!-- dossier-format: 1 -->
# HOW-003 · Run many workers on level-1 tasks in one checkout

- kind: howto
- status: draft
- created: 2026-11-26T19:51:12+01:00
- author: reyna
- via: reyna - claude fable 5.2
- software: Orca orchestration
- topic: planning
- related: DEC-046, DEC-060
- history: 2026-11-26T19:51:12+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:51:41+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-046
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-060

## Purpose

Has many cheap agents finish level-1 tasks of TODO.md at the same time in one checkout, with one strong agent as the planner.

## Steps

1. As the planner, run todo summary and todo plan. Split and detail tasks until todo plan is empty: every task at level 1 has why, accept, files, steps and verify.
2. Read todo waves: the size of wave 1 is how many workers are useful now, and lanes show tasks that share a file.
3. Start one worker per ready task. Each worker gets a unique agent name and runs todo next --claim --max-difficulty 1 --brief, changes only the listed files, commits only those files, and finishes with todo done <id> --commit HEAD, or gives the task back with todo stuck <id> --note.
4. When the workers of a wave have finished, run the full checks (and todo verify --wave when verify_when is "wave"), then start the next wave.
5. Take back tasks given back: read the problem in todo plan, then todo unblock or todo split.
6. Close each milestone with todo done when its last task is finished.

## Settings

Workers started with Orca orchestration use the Claude CLI on Haiku with the highest effort: worker-start with --agent claude --model claude-haiku-5-5 --effort xhigh. A stronger model is used only when the owner names it for one task.

## Traps

When todo done refuses a commit because it changed a file outside the task's files, the planner widens files with todo set; a worker does not use --force. After a merge conflict, add the resolved files before committing, and start dependent workers only after the merge commit exists, because they start from the local branch. Finished worker worktrees are removed after each batch, and the build folder is cleaned when it exceeds 3 GB.

## Tested with

todo 0.8 to 1.0 on this repository, Orca orchestration with Claude Haiku workers (2026-11-20 to 2026-11-22).
