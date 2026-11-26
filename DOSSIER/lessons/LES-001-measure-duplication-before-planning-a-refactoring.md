<!-- dossier-format: 1 -->
# LES-001 · Measure duplication before planning a refactoring

- kind: lesson
- status: approved
- created: 2026-11-26T19:51:11+01:00
- author: reyna
- via: reyna - claude fable 5.2
- confidence: measured
- topic: process
- related: FND-021, FND-042
- approved: 2026-11-26T23:20:10+01:00 by reyna
- valid-from: 2026-11-26T23:20:10+01:00
- history: 2026-11-26T19:51:11+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:51:40+01:00 reyna via reyna - claude fable 5.2: changed — related FND-021
- history: 2026-11-26T19:51:40+01:00 reyna via reyna - claude fable 5.2: changed — related FND-042
- history: 2026-11-26T23:20:10+01:00 reyna via reyna - claude fable 5.2: approved

## Statement

Measure duplicated code with a tool before estimating or starting a refactoring to remove it. An audit by reading estimated about 4,100 duplicated lines; the measurement found 575.

## Valid for

Code bases where the size is suspected to come from copy and paste; any language a clone detector such as jscpd reads.

## Evidence

FND-021, FND-042; docs/refactoring.md
