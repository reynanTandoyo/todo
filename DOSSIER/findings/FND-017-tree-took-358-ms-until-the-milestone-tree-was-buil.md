<!-- dossier-format: 1 -->
# FND-017 · tree took 358 ms until the milestone tree was built once

- kind: finding
- status: approved
- created: 2026-11-26T19:49:25+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:49:25+01:00 by reyna
- valid-from: 2026-11-26T19:49:25+01:00
- task: T-125
- topic: testing
- code: src/doc/tree.rs
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: approved

## Observation

todo tree scanned every entry for every milestone. Building the tree once (MilestoneIndex) brought it from 358 ms to 34 ms on the 5,000-entry milestone tree; check and summary use the same index.

## Evidence

Task T-125, release build, 2026-11-18
