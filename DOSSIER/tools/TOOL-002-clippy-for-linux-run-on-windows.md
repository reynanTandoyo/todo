<!-- dossier-format: 1 -->
# TOOL-002 · Clippy for Linux, run on Windows

- kind: tool
- status: approved
- created: 2026-11-26T19:51:12+01:00
- author: reyna
- via: reyna - claude fable 5.2
- software: Rust 1.89 or newer
- type: command
- location: cargo
- related: HOW-001
- approved: 2026-11-26T23:20:53+01:00 by reyna
- valid-from: 2026-11-26T23:20:53+01:00
- history: 2026-11-26T19:51:12+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:51:44+01:00 reyna via reyna - claude fable 5.2: changed — related HOW-001
- history: 2026-11-26T23:20:53+01:00 reyna via reyna - claude fable 5.2: approved

## Purpose

Finds lints that only show in the Linux build, without a Linux machine.

## Run

Once: rustup target add x86_64-unknown-linux-gnu. Then: cargo clippy --target x86_64-unknown-linux-gnu --all-targets --all-features -- -D warnings
