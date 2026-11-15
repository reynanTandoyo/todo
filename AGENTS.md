# Working on this repository

`todo` is a Rust command-line tool (`todo.exe`) that lets several agents and people share one
`TODO.md` task list safely. This repository stands alone.

## Read first

- [`docs/requirements.md`](docs/requirements.md): what to build. Requirement IDs such as `R-LOCK-3`
  are referenced everywhere. Decisions D1–D18 are final.
- [`docs/format.md`](docs/format.md): the `TODO.md` format. The parser must follow it exactly.
- [`TODO.md`](TODO.md): the work plan. Each task lists the requirements it covers (`reqs:`), the
  tasks that must be finished first (`depends:`) and the check that proves it is done (`accept:`).

## How to work

1. Take the first open task in priority order (P0 first) whose `depends` tasks are all done.
2. Until v0.1 is released, `TODO.md` is edited by hand, so **only one agent works on this repository
   at a time**. Set `status: claimed` and your name as `owner` before you start. From v0.1 on,
   `TODO.md` is changed only through `todo` itself.
3. Build what the task's `reqs` and `accept` lines say, nothing more. If a requirement turns out to
   be wrong or unclear, fix `docs/requirements.md` (and `docs/format.md`) in the same commit, keep
   existing IDs stable, and say so in a note on the task.
4. When every check below passes, set `status: done` and add a note:
   `- note (YYYY-MM-DD, <your name>): done — <what changed>`.
5. One commit per task, starting with its ID: `T-002: line model with verified writes`.
6. Never push. The owner decides when to push.

## Checks

All must pass before a task counts as done:

```
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Code rules

- Rust 1.89 or newer, edition 2024. Only the dependencies approved in NFR-9; anything else needs a
  reason in the commit message.
- No `unwrap`, `expect` or `panic!` outside tests. Every failure becomes an error with a code and an
  exit code (R-OUT-2, R-OUT-3).
- Don't write command output with `println!`, which crashes when a pipe closes (R-OUT-7).
- Never rebuild `TODO.md` from parsed data. Change whole lines, keep every other byte (R-EDIT-1).
- Windows first: paths with spaces and non-ASCII characters must work. Show examples in PowerShell.

## Test rules

- Fixtures in `tests/fixtures/` are invented, and each starts with
  `<!-- TEST FIXTURE: invented data, not a real task list -->`. Never copy a real task list into
  this repository.
- Tests run in temporary folders and never read or write this repository's own `TODO.md`.
- `.gitattributes` keeps fixture line endings exactly as written; don't change that.

## Building

`cargo build --release` produces `target\release\todo.exe`. The C runtime is linked in
(`.cargo/config.toml`), so the program runs on a PC without Visual C++ installed.
