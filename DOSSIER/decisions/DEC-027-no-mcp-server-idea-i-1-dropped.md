<!-- dossier-format: 1 -->
# DEC-027 · No MCP server (idea I-1 dropped)

- kind: decision
- status: approved
- created: 2026-11-26T19:47:12+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:47:12+01:00 by reyna
- valid-from: 2026-11-26T19:47:12+01:00
- task: T-070
- standing: yes
- topic: agents
- related: DEC-051, FND-036
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:40+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-051
- history: 2026-11-26T19:51:40+01:00 reyna via reyna - claude fable 5.2: changed — related FND-036

## Decision

todo gets no MCP server. (Owner, 2026-11-17)

## Reason

Agents already use the command line with --json (D10, D14); an MCP server adds dependencies and a second interface. A command costs no context until its help is read, while an MCP server would load the description of every command, about 51,000 characters, into every session.

## Rejected

An MCP server exposing the commands as tools.
