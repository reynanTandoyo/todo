<!-- dossier-format: 1 -->
# TOOL-001 · The three checks before a task counts as done

- kind: tool
- status: approved
- created: 2026-11-26T19:51:12+01:00
- author: reyna
- via: reyna - claude fable 5.2
- software: Rust 1.89 or newer
- type: command
- location: cargo
- related: DEC-022
- approved: 2026-11-26T23:20:53+01:00 by reyna
- valid-from: 2026-11-26T23:20:53+01:00
- history: 2026-11-26T19:51:12+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-022
- history: 2026-11-26T23:20:53+01:00 reyna via reyna - claude fable 5.2: approved

## Purpose

Checks the format, the lints and every test; all three must pass before a commit.

## Run

cargo fmt --all -- --check ; cargo clippy --all-targets --all-features -- -D warnings ; cargo test --all-features
