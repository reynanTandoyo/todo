<!-- dossier-format: 1 -->
# FND-026 · The CI workflow has its own list of approved dependencies

- kind: finding
- status: approved
- created: 2026-11-26T19:49:25+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:49:25+01:00 by reyna
- valid-from: 2026-11-26T19:49:25+01:00
- topic: release
- code: .github/workflows
- related: DEC-018, LES-009
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:40+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-018
- history: 2026-11-26T19:51:44+01:00 reyna via reyna - claude fable 5.2: changed — related LES-009

## Observation

.github/workflows/ci.yml holds its own allow-list of crates besides NFR-9 in docs/requirements.md. Adding clap_complete to the requirements only made CI fail until the workflow list was updated too.

## Evidence

CI failure fixed in commit a1b1d9d, 2026-11-16
