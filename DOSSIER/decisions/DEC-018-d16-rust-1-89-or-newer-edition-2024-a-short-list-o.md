<!-- dossier-format: 1 -->
# DEC-018 · D16: Rust 1.89 or newer, edition 2024, a short list of approved crates

- kind: decision
- status: approved
- created: 2026-11-26T19:47:12+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:47:12+01:00 by reyna
- valid-from: 2026-11-26T19:47:12+01:00
- standing: yes
- topic: release
- related: FND-026, DEC-052, LES-009
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:40+01:00 reyna via reyna - claude fable 5.2: changed — related FND-026
- history: 2026-11-26T19:51:40+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-052
- history: 2026-11-26T19:51:40+01:00 reyna via reyna - claude fable 5.2: changed — related LES-009

## Decision

The program is written in Rust 1.89 or newer, edition 2024. Only the crates approved in NFR-9 are used: clap, serde, serde_json, toml, sha2, similar, chrono, clap_complete, and libc and windows-sys for stopping a verify command; for tests assert_cmd, predicates, tempfile, proptest and insta. Anything else needs a reason in its commit message. unwrap, expect and panic are not used outside tests.

## Reason

File::try_lock is in the standard library from Rust 1.89. Few, well-known dependencies keep the single executable small and the build simple.

## Rejected

Heavy dependencies such as SQLite or an HTTP client.
