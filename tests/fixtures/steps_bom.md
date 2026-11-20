<!-- TEST FIXTURE: invented data, not a real task list -->
# Invented steps project

Prose above the sections is kept exactly as written.

## Now

### T-001 · Invented task with a steps list
- status: open
- owner: —
- priority: P1 · difficulty: 2 · type: feature · area: demo
- why: the fixture needs a list of steps and the fields around it
- files: src/demo.rs
- read: docs/demo.md, src/lib.rs
- steps:
  1. Open the invented file src/demo.rs.
  2. Add a function that returns 42, with `code`, "quotes" and $dollars.
  3. Add a test that calls it.
- accept: the invented check passes
- verify: cargo test --all-features
- note (2026-11-19, fixture): added

### T-002 · Invented task without steps
- status: open
- owner: —
- priority: P2 · difficulty: 1 · type: chore · area: demo
- accept: nothing to do
- note (2026-11-19, fixture): added
