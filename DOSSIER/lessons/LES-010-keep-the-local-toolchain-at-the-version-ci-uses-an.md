<!-- dossier-format: 1 -->
# LES-010 · Keep the local toolchain at the version CI uses, and run the Linux checks before pushing

- kind: lesson
- status: draft
- created: 2026-11-26T19:51:11+01:00
- author: reyna
- via: reyna - claude fable 5.2
- confidence: experience
- topic: testing
- related: DEC-061, FND-025
- history: 2026-11-26T19:51:11+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-061
- history: 2026-11-26T19:51:44+01:00 reyna via reyna - claude fable 5.2: changed — related FND-025

## Statement

CI installs the latest stable Rust and also builds for Linux. Keep the local Rust current, and run Clippy for the Linux target and the Linux test run before a push; otherwise the first report of a lint or a Linux-only bug is a red CI run.

## Valid for

Projects developed on Windows whose CI also runs on Linux with the latest stable toolchain.

## Evidence

FND-025, FND-023, FND-022
