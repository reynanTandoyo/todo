<!-- dossier-format: 1 -->
# FND-007 · One earlier script rewrote every line ending on each write

- kind: finding
- status: approved
- created: 2026-11-26T19:49:25+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:49:25+01:00 by reyna
- valid-from: 2026-11-26T19:49:25+01:00
- topic: format
- code: src/doc/lines.rs
- related: DEC-020
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:40+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-020

## Observation

It always wrote LF and normalised the end of the file, so a CRLF file changed completely on every write. Both files also start with a hand-written title, how-to and rules that must survive every write. (Research findings F8, F18)

## Evidence

docs/research.md section 1.2, study of 2026-11-15
