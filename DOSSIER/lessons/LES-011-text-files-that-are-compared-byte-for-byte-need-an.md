<!-- dossier-format: 1 -->
# LES-011 · Text files that are compared byte for byte need an end-of-line rule in .gitattributes

- kind: lesson
- status: approved
- created: 2026-11-26T19:51:11+01:00
- author: reyna
- via: reyna - claude fable 5.2
- confidence: experience
- topic: testing
- code: src/agents
- related: FND-022
- approved: 2026-11-26T23:20:10+01:00 by reyna
- valid-from: 2026-11-26T23:20:10+01:00
- history: 2026-11-26T19:51:11+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related FND-022
- history: 2026-11-26T23:20:10+01:00 reyna via reyna - claude fable 5.2: approved

## Statement

A file whose bytes the program compares or embeds must have its line ending fixed in .gitattributes; otherwise git checks it out with CRLF on one machine and LF on another, and the comparison fails only there.

## Valid for

Repositories used on Windows and Linux with autocrlf; embedded texts and test fixtures.

## Evidence

FND-022; .gitattributes
