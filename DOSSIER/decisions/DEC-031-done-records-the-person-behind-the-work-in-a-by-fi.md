<!-- dossier-format: 1 -->
# DEC-031 · done records the person behind the work in a by field, through .mailmap

- kind: decision
- status: approved
- created: 2026-11-26T19:47:13+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:47:13+01:00 by reyna
- valid-from: 2026-11-26T19:47:13+01:00
- task: T-085
- standing: yes
- topic: claims
- code: src/ops/identity.rs
- history: 2026-11-26T19:47:13+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:47:13+01:00 reyna via reyna - claude fable 5.2: approved

## Decision

done writes the person who finished a task into a by field: the author of the commit, or the git user name, mapped through the repository's .mailmap. No email address is ever written to TODO.md. Without git or a name the field is left out.

## Reason

A task should show which contributor finished it, not only which agent; .mailmap gives every contributor one name however their computers are set up.
