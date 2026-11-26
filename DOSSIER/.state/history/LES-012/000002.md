<!-- dossier-format: 1 -->
# LES-012 · Keep the rules every agent reads short; put the rest behind a help command

- kind: lesson
- status: draft
- created: 2026-11-26T19:51:12+01:00
- author: reyna
- via: reyna - claude fable 5.2
- confidence: experience
- topic: agents
- code: src/agents
- related: DEC-051
- history: 2026-11-26T19:51:12+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:51:40+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-051

## Statement

What every agent must read costs context in every session, and a weak model follows long rules badly. Keep the common rules to a few lines, limit them with a test, and move the rules of one role into a help text that only that role reads.

## Valid for

Tools that agents learn from a rules text in AGENTS.md; any number of commands.

## Evidence

FND-036; rules by role in v0.8
