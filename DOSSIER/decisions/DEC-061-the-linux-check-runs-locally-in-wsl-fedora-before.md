<!-- dossier-format: 1 -->
# DEC-061 · The Linux check runs locally in WSL Fedora before every push; Linux CI stays

- kind: decision
- status: approved
- created: 2026-11-26T19:47:14+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:47:14+01:00 by reyna
- valid-from: 2026-11-26T19:47:14+01:00
- standing: yes
- topic: testing
- related: DEC-014, HOW-001, FND-045, ASM-011, LES-010
- history: 2026-11-26T19:47:14+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:47:14+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:39+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-014
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related HOW-001
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related FND-045
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related ASM-011
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related LES-010

## Decision

After each merge and before pushing, cargo fmt, Clippy and the full test suite run on Linux locally, in the WSL distribution FedoraLinux-44 as user reyna, on a clone in the Linux file system. CI on Linux stays as well. Disk use must not explode: cargo clean when a target folder exceeds 3 GB, and finished worker worktrees are removed after each batch. (Owner, 2026-11-21)

## Reason

Two Linux-only failures were found only in CI after a push; there is no Linux PC, and the office uses Fedora. The WSL disk image never shrinks.

## Rejected

Building through /mnt/c: slow and with the wrong permissions; running as root: the read-only-file tests fail.
