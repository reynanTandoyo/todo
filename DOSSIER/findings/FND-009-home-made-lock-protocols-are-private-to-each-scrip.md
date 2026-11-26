<!-- dossier-format: 1 -->
# FND-009 · Home-made lock protocols are private to each script

- kind: finding
- status: approved
- created: 2026-11-26T19:49:25+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:49:25+01:00 by reyna
- valid-from: 2026-11-26T19:49:25+01:00
- topic: locking
- related: DEC-008
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:39+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-008

## Observation

One script used a lock folder with an owner file, broken after 30 seconds; the other a lock file, broken after 60 seconds. Each only saw its own kind of lock. (Research finding F10)

## Evidence

docs/research.md section 1.2, study of 2026-11-15
