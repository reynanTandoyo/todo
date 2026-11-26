<!-- dossier-format: 1 -->
# TOOL-004 · Console check: non-ASCII text in each Windows console

- kind: tool
- status: draft
- created: 2026-11-26T19:51:12+01:00
- author: reyna
- via: reyna - claude fable 5.2
- type: script
- location: scripts/console-check.ps1
- language: PowerShell
- checksum: sha256:b5b53477a30fb35db4fe60c2a189609b0994587d19313364af78496083979137
- history: 2026-11-26T19:51:12+01:00 reyna via reyna - claude fable 5.2: created

## Purpose

Prints sample todo output with a middle dot, an em dash and Japanese text, for a person to look at in PowerShell 5, PowerShell 7, cmd and Windows Terminal (R-OUT-6).

## Run

Build the release program first, then run scripts\console-check.ps1; it takes the path of todo.exe as an optional argument.
