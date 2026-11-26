<!-- dossier-format: 1 -->
# FND-008 · Reading does not need the lock

- kind: finding
- status: approved
- created: 2026-11-26T19:49:25+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:49:25+01:00 by reyna
- valid-from: 2026-11-26T19:49:25+01:00
- topic: locking
- code: src/atomic.rs
- related: DEC-008
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:39+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-008

## Observation

One earlier script locked for reading; replacing the file in one step makes that unnecessary. (Research finding F9)

## Evidence

docs/research.md section 1.2, study of 2026-11-15
