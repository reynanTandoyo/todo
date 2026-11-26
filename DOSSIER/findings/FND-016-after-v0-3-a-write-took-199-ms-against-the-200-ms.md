<!-- dossier-format: 1 -->
# FND-016 · After v0.3 a write took 199 ms against the 200 ms limit; the fix brought add to 73 ms

- kind: finding
- status: approved
- created: 2026-11-26T19:49:25+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-26T19:49:25+01:00 by reyna
- valid-from: 2026-11-26T19:49:25+01:00
- task: T-082
- topic: testing
- code: src/ops/pipeline.rs
- related: ASM-008
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:49:25+01:00 reyna via reyna - claude fable 5.2: approved
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related ASM-008

## Observation

After v0.3 a write on the 5,000-task file took about 150 to 199 ms where v0.1 took 59 ms, because the write path parsed the file several times; note --dry-run took 93 ms although it writes nothing. After the fix: list 41 ms, show 36 ms, note 72 ms, add 73 ms. The release todo.exe went from 4.2 MB to 3.06 MB by turning off default features of clap, toml and sha2 and setting panic=abort.

## Evidence

Task T-082, performance test on the generated 5,000-task file, 2026-11-15
