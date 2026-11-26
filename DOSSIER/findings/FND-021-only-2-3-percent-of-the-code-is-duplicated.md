<!-- dossier-format: 1 -->
# FND-021 · Only 2.3 percent of the code is duplicated

- kind: finding
- status: approved
- created: 2026-11-26T19:49:25+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:49:25+01:00 by reyna
- valid-from: 2026-11-26T19:49:25+01:00
- task: T-083
- topic: process
- related: DEC-025, LES-001
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:40+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-025
- history: 2026-11-26T19:51:40+01:00 reyna via reyna - claude fable 5.2: changed — related LES-001

## Observation

jscpd (minimum 6 lines) found 48 exact clones, 575 duplicated lines, 2.3 percent of src/ and tests/, measured after three refactoring chunks that had removed about 520 lines. The size of the code comes from the feature set (35 commands at about 180 lines each then), the safety machinery and the tests, not from copy and paste. An audit before measuring had estimated about 4,100 duplicated lines.

## Evidence

docs/refactoring.md, section Outcome, 2026-11-15
