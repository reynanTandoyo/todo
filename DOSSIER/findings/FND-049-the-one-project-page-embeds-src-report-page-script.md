<!-- dossier-format: 1 -->
# FND-049 · The one-project page embeds src/report/page/script.js and style.css, so a change

- kind: finding
- status: approved
- created: 2026-11-26T23:59:07+01:00
- author: reyna
- via: reyna - claude opus 5.5
- approved: 2026-11-26T23:59:07+01:00 by reyna
- valid-from: 2026-11-26T23:59:07+01:00
- task: T-231
- topic: report
- code: src/report/page/script.js
- history: 2026-11-26T23:59:07+01:00 reyna via reyna - claude opus 5.5: created
- history: 2026-11-26T23:59:07+01:00 reyna via reyna - claude opus 5.5: approved

## Observation

The one-project page embeds src/report/page/script.js and style.css, so a change to either also changes tests/snapshots/cmd_report__report_html.snap; T-231 listed only script.js and todo done refused it until the snapshot was added to files.
