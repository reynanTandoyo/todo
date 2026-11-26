<!-- dossier-format: 1 -->
# LES-009 · A new dependency is added in two places: the requirements and the CI allow-list

- kind: lesson
- status: draft
- created: 2026-11-26T19:51:11+01:00
- author: reyna
- via: reyna - claude fable 5.2
- confidence: experience
- topic: release
- code: .github/workflows
- related: DEC-018, FND-026
- history: 2026-11-26T19:51:11+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:51:40+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-018
- history: 2026-11-26T19:51:44+01:00 reyna via reyna - claude fable 5.2: changed — related FND-026

## Statement

When a crate is added, update NFR-9 in docs/requirements.md and the allow-list in .github/workflows/ci.yml in the same commit, and give the reason in the commit message.

## Valid for

This repository, as long as CI checks the dependency list.

## Evidence

FND-026; commit a1b1d9d
