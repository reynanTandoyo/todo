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

1. Run `todo summary`, then take work with `todo next --claim --agent <your name>` (or
   `todo claim T-0xx --agent <your name>`). Every agent uses its own unique name.
2. `TODO.md` is changed only through `todo` (the rules are in the block at the end of this file).
   Never edit it directly; several agents may be working at once.
3. Build what the task's `reqs` and `accept` lines say, nothing more. If a requirement turns out to
   be wrong or unclear, fix `docs/requirements.md` (and `docs/format.md`) in the same commit, keep
   existing IDs stable, and say so with `todo note`.
4. When every check below passes, commit: one commit per task, starting with its ID
   (`T-002: line model with verified writes`).
5. Then `todo done T-0xx --agent <your name> --commit HEAD --note "<what changed>"`.
6. Never push. The owner decides when to push.

## Checks

All must pass before a task counts as done:

```
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

CI also runs clippy for Linux. To check that locally:
`cargo clippy --target x86_64-unknown-linux-gnu --all-targets --all-features -- -D warnings`
(once: `rustup target add x86_64-unknown-linux-gnu`).

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

<!-- todo:begin -->
# Working with TODO.md

- Never edit `TODO.md` or `TODO-archive.md` directly; always use `todo`.
- Use one unique agent name per agent: `--agent <name>` or `TODO_AGENT`.
- Exit code `1`: someone else has the task, or a rule refused the change; pick another task.
- Exit code `3`: the lock or lease is busy; run the same command again.
- Before you start a task, read the goals of the milestones above it (`todo show <id>` prints them).
- Start every commit message with the task ID, such as `T-012: short summary`.
- Workers: run `todo help worker`.
- Planner: run `todo help planner`.
<!-- todo:end -->
