<!-- dossier-format: 1 -->
# FND-020 · The status page for 5,000 entries takes about 110 ms and 2.9 MB

- kind: finding
- status: approved
- created: 2026-11-26T19:49:25+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:49:25+01:00 by reyna
- valid-from: 2026-11-26T19:49:25+01:00
- task: T-204
- topic: report
- code: src/report/html.rs
- related: DEC-055
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:42+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-055

## Observation

report --html on the 5,000-entry milestone tree, with a summary on every task: 110, 111 and 122 ms (limit 500 ms) and 2,897,256 bytes (limit 5 MB). The all-projects page of ten such projects went from 40.1 MB to 28.4 MB when the per-task markup was trimmed, at 579 ms (limit 5 s).

## Evidence

Tasks T-204 and T-184, release build, 2026-11-20 and 2026-11-21
