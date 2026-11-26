<!-- dossier-format: 1 -->
# DEC-016 · D14: Every agent learns the tool from one rules text; hooks and a skill are extras; no MCP server

- kind: decision
- status: superseded
- created: 2026-11-26T19:47:12+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:47:12+01:00 by reyna
- valid-from: 2026-11-26T19:47:12+01:00
- standing: yes
- topic: agents
- code: src/agents
- valid-until: 2026-11-26T19:47:13+01:00
- superseded-by: DEC-051
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:47:13+01:00 reyna via reyna - claude fable 5.2: superseded — by DEC-051

## Decision

Every agent learns the tool from one rules text, printed by todo help agents and installed into AGENTS.md between two markers. Claude Code hooks and a skill are extras. There is no MCP server.

## Reason

Every agent reads AGENTS.md; hooks only exist in Claude Code; an MCP server is another protocol to maintain. Hand-written rules in several files drifted apart in the earlier projects.

## Rejected

An MCP server exposing the commands as tools.
