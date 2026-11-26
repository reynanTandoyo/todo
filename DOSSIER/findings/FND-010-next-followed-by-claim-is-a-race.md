<!-- dossier-format: 1 -->
# FND-010 · next followed by claim is a race

- kind: finding
- status: approved
- created: 2026-11-26T19:49:25+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:49:25+01:00 by reyna
- valid-from: 2026-11-26T19:49:25+01:00
- topic: claims
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: approved

## Observation

Two agents calling next get the same task; one claim fails and that agent has to start over. Choosing and claiming must happen under one lock (next --claim). (Research finding F11)

## Evidence

docs/research.md section 1.2, study of 2026-11-15
