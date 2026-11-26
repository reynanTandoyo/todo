<!-- dossier-format: 1 -->
# DEC-038 · The HTML page is the one view for people; it is read-only and built only on request

- kind: decision
- status: approved
- created: 2026-11-26T19:47:13+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:47:13+01:00 by reyna
- valid-from: 2026-11-26T19:47:13+01:00
- standing: yes
- topic: report
- code: src/report
- related: DEC-017, DEC-055
- history: 2026-11-26T19:47:13+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:47:13+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:40+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-017
- history: 2026-11-26T19:51:42+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-055

## Decision

Command output, plain text and JSON alike, is written for agents. The one view meant for people is the HTML page from todo report --html: one self-contained file, with nothing loaded from the network, that never changes anything. It is built only when someone asks for it. Nothing on the page can change TODO.md: no form, no script that writes or sends, no copy-ready command. A person who wants a change asks an agent. Buttons that only change what the page shows are allowed. (Owner, 2026-11-18; view buttons allowed 2026-11-21)

## Reason

People need to see the state of the work without running commands, and fewer controls mean fewer human errors. A static file needs no server, so the rule of no background process and no network stays.

## Rejected

A local web server or a terminal UI; buttons that answer a question or add an idea; rewriting the page after every write with a page that reloads itself (idea I-13).
