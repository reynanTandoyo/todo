<!-- dossier-format: 1 -->
# DEC-026 · v0.4: import from checkbox lists, session-tagged claims and tab completion

- kind: decision
- status: approved
- created: 2026-11-26T19:47:12+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:47:12+01:00 by reyna
- valid-from: 2026-11-26T19:47:12+01:00
- standing: yes
- topic: cli
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:47:12+01:00 reyna via reyna - claude fable 5.2: approved

## Decision

Three ideas became requirements: import --from checkbox converts a Markdown checkbox list (R-IMP-1); a claim carries the agent session and a Claude Code SessionEnd hook releases that session's claims (R-CLAIM-7); todo completions prints a tab-completion script (R-OUT-10). (Owner, 2026-11-16)

## Reason

They make the tool usable for an existing checkbox list, free the claims of a session that ended, and help a person typing commands.

## Rejected

Import from todo.txt: it stays an idea.
