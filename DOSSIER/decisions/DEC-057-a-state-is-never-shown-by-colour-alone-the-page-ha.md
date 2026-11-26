<!-- dossier-format: 1 -->
# DEC-057 · A state is never shown by colour alone; the page has light, dark and colour-blind colours

- kind: decision
- status: approved
- created: 2026-11-26T19:47:14+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:47:14+01:00 by reyna
- valid-from: 2026-11-26T19:47:14+01:00
- task: T-203
- standing: yes
- topic: report
- code: src/report/style.css
- related: DEC-055, ASM-009
- history: 2026-11-26T19:47:14+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:47:14+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:42+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-055
- history: 2026-11-26T19:51:42+01:00 reyna via reyna - claude fable 5.2: changed — related ASM-009

## Decision

Every state has its own mark shape and is named in words, so colour is never the only signal. The page has one button for light or dark mode and one for colour-blind colours, which changes the colour scheme only. There are four colour sets: light and dark each have their own colours that fit a white or a black background, in the normal and in the colour-blind scheme. The colour-blind sets use only blue, orange and greys, and waiting has a stripe pattern in the bar. Every state colour has a contrast of at least 3 to 1, checked by a test. (Owner, 2026-11-21)

## Reason

The owner is red-green colour blind.
