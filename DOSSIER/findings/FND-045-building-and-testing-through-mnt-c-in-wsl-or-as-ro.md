<!-- dossier-format: 1 -->
# FND-045 · Building and testing through /mnt/c in WSL, or as root, does not work

- kind: finding
- status: approved
- created: 2026-11-26T19:49:26+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:49:26+01:00 by reyna
- valid-from: 2026-11-26T19:49:26+01:00
- outcome: dead end
- topic: testing
- related: DEC-061
- history: 2026-11-26T19:49:26+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:49:26+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-061

## Observation

Tried: running the Linux checks on the Windows checkout through /mnt/c, which is slow and has the wrong file permissions; and running as root, where the read-only-file tests fail because root may write anyway. The checks run on a clone in the Linux file system as a normal user.

## Evidence

Setup of the WSL Fedora check, 2026-11-21
