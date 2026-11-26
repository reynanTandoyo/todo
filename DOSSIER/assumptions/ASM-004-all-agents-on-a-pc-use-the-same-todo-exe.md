<!-- dossier-format: 1 -->
# ASM-004 · All agents on a PC use the same todo.exe

- kind: assumption
- status: approved
- created: 2026-11-26T19:49:26+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:49:26+01:00 by reyna
- valid-from: 2026-11-26T19:49:26+01:00
- state: open
- topic: versions
- related: DEC-039
- history: 2026-11-26T19:49:26+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:49:26+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:41+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-039

## Assumption

There is one todo.exe on PATH, so every agent on the PC runs the same version.

## Reason

Versions are compared only through the stamp in the file. Two versions on one PC with the same format could behave differently without any refusal.

## Affects

format.version, release.version, todo migrate
