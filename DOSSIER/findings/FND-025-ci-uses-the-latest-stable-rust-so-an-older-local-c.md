<!-- dossier-format: 1 -->
# FND-025 · CI uses the latest stable Rust, so an older local Clippy misses lints

- kind: finding
- status: approved
- created: 2026-11-26T19:49:25+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:49:25+01:00 by reyna
- valid-from: 2026-11-26T19:49:25+01:00
- topic: testing
- related: LES-010
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:44+01:00 reyna via reyna - claude fable 5.2: changed — related LES-010

## Observation

A lint that the local Clippy did not report failed CI. CI installs the latest stable Rust; the local toolchain was updated to match (1.100 at the time). The Linux lints can be checked locally with cargo clippy --target x86_64-unknown-linux-gnu.

## Evidence

CI failures of 2026-11-15; AGENTS.md section Checks
