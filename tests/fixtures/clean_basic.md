<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 1 -->
# Garden Project — tasks

Notes in the preamble stay untouched.

## 1. Now

### T-001 · Water the tomato plants
- status: claimed
- owner: gardener-bot
- claimed: 2026-11-15T09:30+01:00
- priority: P0 · type: chore · area: greenhouse
- files: greenhouse/bed1.txt
- problem: soil moisture dropped below 20%
- fix: run drip line for 10 minutes
- accept: moisture reads above 45%
- custom_tag: morning-shift
- note (2026-11-15, coordinator): added
- note (2026-11-15, gardener-bot): started watering

Extra prose inside T-001 is preserved.

  - indented sub-item is not a field

### T-002 · Repair the wooden gate
- status: open
- owner: —
- priority: P1
- type: maintenance
- area: fence
- depends: T-001
- accept: latch closes smoothly
- note (2026-11-15, coordinator): added

## 2) Later

### T-003 · Order compost bags
- status: blocked
- owner: -
- priority: P2
- note (2026-11-15, coordinator): blocked — supplier closed until Monday

### T-004 · Sweep patio stones
- status: done
- owner: helper-2
- priority: P3
- commit: abc1234, def5678
- note (2026-11-15, helper-2): done — swept north and south sides

### T-005 · Paint mailbox neon pink
- status: dropped
- owner: none
- note (2026-11-15, coordinator): dropped — HOA forbids neon

## Questions

### T-006 · Which rose variety for the arch?
- status: open
- owner:
- type: question
- note (2026-11-15, gardener-bot): added
