<!-- dossier-format: 1 -->
# DEC-073 · A rules trial measures whether an agent follows the worker rules

- kind: decision
- status: approved
- created: 2026-11-27T20:16:42+01:00
- author: reyna
- via: reyna - claude fable 5.2
- task: T-248
- tags: rules,worker,trial,script
- topic: testing,agents
- approved: 2026-11-27T20:24:04+01:00 by reyna
- valid-from: 2026-11-27T20:24:04+01:00
- history: 2026-11-27T20:16:42+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-27T20:16:42+01:00 reyna via reyna - claude fable 5.2: proposed
- history: 2026-11-27T20:24:04+01:00 reyna via reyna - claude fable 5.2: accepted

## Decision

A rules trial measures by hand whether an agent follows the worker rules: a script sets up an invented project whose task asks for three rule breaks, and grades afterwards from todo's history, git and a local bare repository (TEST-10)

## Reason

ASM-005 (agents follow the rules text and do not edit TODO.md directly) is open and nothing measures it, while the worker rules are limited to ten lines. The trial is cheap: todo already records outside edits, git records commits, and a local bare repository shows a push.

## Rejected

A test in CI: it needs an agent session, so a network and a model. Grading from the agent's own report or transcript: what an agent says it did is the one thing that cannot be trusted; the records can. Building it into the todo program: todo never starts agents (DEC-045).
