<!-- dossier-format: 1 -->
# FND-022 · On the Windows CI runner the rules text is checked out with CRLF

- kind: finding
- status: approved
- created: 2026-11-26T19:49:25+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:49:25+01:00 by reyna
- valid-from: 2026-11-26T19:49:25+01:00
- topic: testing
- code: src/agents/mod.rs
- related: DEC-014, LES-011
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:39+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-014
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related LES-011

## Observation

agents check failed right after agents install on Windows CI: src/agents/rules.md was checked out with CRLF (autocrlf) while the built-in text was compared raw. Fixed by normalising line endings of the rules text and by the line "src/agents/*.md text eol=lf" in .gitattributes.

## Evidence

CI run 43760785784 and commit c9b0846, 2026-11-15
