<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 1 -->
# Unfinished work under closed milestones

## Tasks

### T-001 · Done milestone
- status: done
- owner: —
- priority: P1 · type: milestone
- goal: the first piece of work is finished

### T-002 · Open task under a done milestone
- status: open
- owner: —
- priority: P1
- milestone: T-001

### T-003 · Claimed task under a done milestone
- status: claimed
- owner: t090-fixture
- claimed: 2099-01-01T10:00+01:00
- priority: P1
- milestone: T-001

### T-004 · Dropped milestone
- status: dropped
- owner: —
- priority: P1 · type: milestone
- goal: the second piece of work was abandoned

### T-005 · Blocked task under a dropped milestone
- status: blocked
- owner: —
- priority: P2
- milestone: T-004
- note (2026-11-18, planner): waiting for the other piece of work

### T-006 · Finished task under a dropped milestone
- status: done
- owner: —
- priority: P2
- milestone: T-004
