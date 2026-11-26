<!-- dossier-format: 1 -->
# FND-014 · Loose section matching picks the wrong section

- kind: finding
- status: approved
- created: 2026-11-26T19:49:25+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:49:25+01:00 by reyna
- valid-from: 2026-11-26T19:49:25+01:00
- topic: cli
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: approved

## Observation

add --section "Phase 3" took the first heading that contained the text, even when several did. todo refuses an unknown or ambiguous section and lists the sections. (Research finding F17)

## Evidence

docs/research.md section 1.2, study of 2026-11-15
