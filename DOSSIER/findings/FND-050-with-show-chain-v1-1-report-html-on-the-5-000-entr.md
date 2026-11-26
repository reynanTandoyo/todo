<!-- dossier-format: 1 -->
# FND-050 · With Show chain (v1.1), report --html on the 5,000-entry perf file takes 136-147

- kind: finding
- status: approved
- created: 2026-11-27T00:45:41+01:00
- author: reyna
- via: reyna - claude opus 5.5
- approved: 2026-11-27T00:45:41+01:00 by reyna
- valid-from: 2026-11-27T00:45:41+01:00
- task: T-244
- topic: report
- code: tests/perf.rs
- history: 2026-11-27T00:45:41+01:00 reyna via reyna - claude opus 5.5: created
- history: 2026-11-27T00:45:41+01:00 reyna via reyna - claude opus 5.5: approved

## Observation

With Show chain (v1.1), report --html on the 5,000-entry perf file takes 136-147 ms in release (v1.0: about 110 ms) and writes 3.4 MB (v1.0: 2.9 MB); NFR-3 allows 500 ms and 5 MB.
