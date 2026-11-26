<!-- dossier-format: 1 -->
# DEC-039 · TODO.md records the highest todo version that has written it

- kind: decision
- status: approved
- created: 2026-11-26T19:47:13+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:47:13+01:00 by reyna
- valid-from: 2026-11-26T19:47:13+01:00
- standing: yes
- topic: versions
- code: src/doc/stamp.rs, src/version.rs
- related: DEC-040, ASM-004
- history: 2026-11-26T19:47:13+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:47:13+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:41+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-040
- history: 2026-11-26T19:51:41+01:00 reyna via reyna - claude fable 5.2: changed — related ASM-004

## Decision

TODO.md carries a line todo-tool: x.y.z under the format line: the highest todo version that has written the file. A write adds it or raises it and never lowers it; read commands never write it. An older tool on a file stamped newer, in a format it knows, still works and warns that it is behind. The merge driver takes the higher stamp. (Owner, 2026-11-18)

## Reason

The file must say which todo last shaped it, so a newer tool knows what to migrate and an older one knows it is behind. Raising it only on an upgrade avoids diff noise and merge conflicts.

## Rejected

Writing the running version on every write.
