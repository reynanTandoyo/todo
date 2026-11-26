<!-- dossier-format: 1 -->
# DEC-062 · A version is tagged only after CI on main is green; the tag builds the release

- kind: decision
- status: approved
- created: 2026-11-26T19:47:14+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:47:14+01:00 by reyna
- valid-from: 2026-11-26T19:47:14+01:00
- standing: yes
- topic: release
- code: .github/workflows/release.yml
- related: HOW-002, LES-007, LES-008, FND-033
- history: 2026-11-26T19:47:14+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:47:14+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related HOW-002
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related LES-007
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related LES-008
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related FND-033

## Decision

A release is made by setting the version in Cargo.toml and CHANGELOG.md, merging to main, waiting for CI on main to be green on Windows and Linux, and then pushing the v* tag. The release workflow checks the tag against Cargo.toml, runs the tests, builds todo.exe and the Linux binary and publishes both with SHA-256 files. The new todo.exe is installed on this PC before the repository uses any new setting.

## Reason

A tag on a red commit publishes a broken executable, and an older installed todo refuses unknown todo.toml keys.
