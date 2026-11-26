<!-- dossier-format: 1 -->
# DEC-054 · Existing error codes never change meaning; the requirements win over the JSON documentation

- kind: decision
- status: approved
- created: 2026-11-26T19:47:13+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:47:13+01:00 by reyna
- valid-from: 2026-11-26T19:47:13+01:00
- task: T-143
- standing: yes
- topic: cli
- code: src/error.rs
- related: DEC-012, DEC-066
- history: 2026-11-26T19:47:13+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:47:13+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:39+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-012
- history: 2026-11-26T19:52:40+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-066

## Decision

Two cases decided with the owner on 2026-11-19: add without --section in a file with several sections keeps returning UNKNOWN_SECTION, as it has since v0.1, and AMBIGUOUS_SECTION is for an ambiguous name; BAD_VALUE exits 1, as the requirements say, and docs/json.md was corrected. A test compares the documented exit codes with the program.

## Reason

Command names, JSON shapes, error codes and exit codes are versioned interfaces that agents branch on (NFR-8).

## Rejected

Changing the code the program returns to match the documentation.
