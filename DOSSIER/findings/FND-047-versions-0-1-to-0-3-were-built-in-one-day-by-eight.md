<!-- dossier-format: 1 -->
# FND-047 · Versions 0.1 to 0.3 were built in one day by eight workers and a coordinator

- kind: finding
- status: approved
- created: 2026-11-26T19:52:40+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:52:40+01:00 by reyna
- valid-from: 2026-11-26T19:52:40+01:00
- topic: process
- related: DEC-060
- history: 2026-11-26T19:52:40+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:52:40+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:52:40+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-060

## Observation

Versions 0.1 to 0.3 were built on 2026-11-15 by eight workers (W1 to W8) in an Orca orchestration run, mostly Gemini Hydrogen through Antigravity, with one main agent session as coordinator that verified each batch (format, Clippy, tests) and recorded the tasks. All 55 planned tasks plus the speed fix T-082 were done that day; 150 tests passed. From v0.6 on the workers were Claude Haiku.

## Evidence

Task notes in TODO-archive.md (owners w1-hydrogen to w8-hydrogen, coordinator); CHANGELOG.md
