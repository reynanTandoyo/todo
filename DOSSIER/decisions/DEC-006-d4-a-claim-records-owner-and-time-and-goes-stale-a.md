<!-- dossier-format: 1 -->
# DEC-006 · D4: A claim records owner and time and goes stale after 24 hours

- kind: decision
- status: approved
- created: 2026-11-26T19:47:12+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:47:12+01:00 by reyna
- valid-from: 2026-11-26T19:47:12+01:00
- standing: yes
- topic: claims
- code: src/ops/task_ops.rs
- related: ASM-006
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:39+01:00 reyna via reyna - claude fable 5.2: changed — related ASM-006

## Decision

A claim records the owner and the time. It goes stale after 24 hours (claim_ttl_hours); claiming again refreshes it. Another agent may take a stale claim over, and a note names the previous owner.

## Reason

Agent sessions last hours, so abandoned work frees up the next day without taking active work away. No separate heartbeat command is needed.

## Rejected

A heartbeat command; a 2-day expiry as in one earlier script.
