<!-- dossier-format: 1 -->
# FND-057 · Checksums on archived blocks were rejected

- kind: finding
- status: approved
- created: 2026-11-27T20:17:01+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-27T20:17:01+01:00 by reyna
- valid-from: 2026-11-27T20:17:01+01:00
- task: T-248
- tags: archive,checksum,format,rejected
- topic: format
- history: 2026-11-27T20:17:01+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-27T20:17:01+01:00 reyna via reyna - claude fable 5.2: approved

## Observation

Rejected: archive records a checksum for each block it moves to TODO-archive.md, so check notices a hand edit. The archive is committed, so git shows every hand edit with its author; people may edit the files by hand under a lease on purpose (R-LOCK-6); and a checksum line per block would be a new format version with a migration, for a question git already answers.

## Evidence

Weighed on 2026-11-27 against docs/requirements.md and the code (src/cli/done.rs, src/cli/verify.rs, src/ops/scope.rs, src/cli/release.rs, src/cli/stuck.rs); verdicts in docs/requirements.md section 12, I-14 to I-24
