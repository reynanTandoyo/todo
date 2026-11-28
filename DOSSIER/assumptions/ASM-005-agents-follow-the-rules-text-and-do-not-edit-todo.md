<!-- dossier-format: 1 -->
# ASM-005 · Agents follow the rules text and do not edit TODO.md directly

- kind: assumption
- status: approved
- created: 2026-11-26T19:49:26+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:49:26+01:00 by reyna
- valid-from: 2026-11-26T19:49:26+01:00
- state: confirmed
- topic: agents
- confirmed: 2026-11-28T08:18:56+01:00 by reyna
- history: 2026-11-26T19:49:26+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:49:26+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-28T08:18:56+01:00 reyna via reyna - claude opus 5.5: confirmed — reyna, 2026-11-28: confirmed for Claude Haiku 5.5 at the highest effort, from the first rules trial (FND-059): the worker kept all four rules and refused the three tempting steps by name

## Assumption

An agent that has read AGENTS.md uses todo for every change and never edits TODO.md or the archive itself.

## Reason

Only Claude Code has a hook (todo guard) that stops a direct edit; other agents are bound by the rules text alone. A direct edit is detected afterwards and saved as a snapshot, not prevented.

## Affects

rules.common-lines, outside-edit detection

## Confirmation

reyna, 2026-11-28: confirmed for Claude Haiku 5.5 at the highest effort, from the first rules trial (FND-059): the worker kept all four rules and refused the three tempting steps by name
