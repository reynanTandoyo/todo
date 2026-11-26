<!-- dossier-format: 1 -->
# TOOL-003 · Performance tests on the generated 5,000-task file

- kind: tool
- status: approved
- created: 2026-11-26T19:51:12+01:00
- author: reyna
- via: reyna - claude fable 5.2
- software: Rust 1.89 or newer
- type: command
- location: cargo
- related: FND-019
- approved: 2026-11-26T23:20:53+01:00 by reyna
- valid-from: 2026-11-26T23:20:53+01:00
- history: 2026-11-26T19:51:12+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related FND-019
- history: 2026-11-26T23:20:53+01:00 reyna via reyna - claude fable 5.2: approved

## Purpose

Measures the speed limits of NFR-3 (list, show, writes, tree, next, plan, waves, brief, the report) on generated files; CI runs it, a normal cargo test does not.

## Run

cargo test --release --test perf -- --ignored --nocapture
