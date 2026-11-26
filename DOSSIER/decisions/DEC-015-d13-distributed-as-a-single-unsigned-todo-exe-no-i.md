<!-- dossier-format: 1 -->
# DEC-015 · D13: Distributed as a single unsigned todo.exe, no installer

- kind: decision
- status: approved
- created: 2026-11-26T19:47:12+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:47:12+01:00 by reyna
- valid-from: 2026-11-26T19:47:12+01:00
- standing: yes
- topic: release
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: approved

## Decision

todo is distributed as a single unsigned todo.exe with the C runtime linked in, from GitHub Releases or built from source, and copied into a folder on PATH (%LOCALAPPDATA%\Programs\todo). There is no installer and no code signing.

## Reason

It needs no administrator rights and runs on a PC without Visual C++ installed.

## Rejected

An installer; code signing.
