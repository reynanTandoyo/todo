<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 3 -->
<!-- todo-tool: 0.7.0 -->
# Harbour lights — tasks

## Now

### T-001 · Develop the renderer
- status: open
- owner: —
- priority: P1 · type: milestone
- goal: the test scene renders with lighting at 60 frames per second

### T-002 · Lighting
- status: open
- owner: —
- priority: P1 · type: milestone
- milestone: T-001
- goal: point and directional lights match the reference images

### T-003 · C shader
- status: claimed
- owner: alice
- priority: P1 · difficulty: 3 · area: ui
- claimed: 2026-11-19T08:00+01:00
- milestone: T-002
- why: the scene needs a shader before anything else

### T-004 · GPU compiler
- status: blocked
- owner: —
- priority: P2 · difficulty: 4 · area: gpu
- milestone: T-001
- note (2026-11-17, bob): waiting for the driver update
- note (2026-11-18, bob): blocked — the vendor driver crashes on the test scene

### T-005 · Write the docs
- status: claimed
- owner: carol
- priority: P2 · area: docs
- claimed: 2026-11-10T09:30+01:00

### T-006 · Shadow maps
- status: open
- owner: —
- priority: P2 · difficulty: 4 · area: ui
- milestone: T-002
- why: the reference images show soft shadows

### T-007 · Which lamp model to use
- status: open
- owner: —
- priority: P2 · type: question
- about: T-002

### T-008 · Keep the old renderer?
- status: open
- owner: —
- priority: P2 · type: question
- about: T-001

### T-009 · Is the old exporter still needed?
- status: open
- owner: —
- priority: P3 · type: question
- about: T-099

### T-010 · Hot-reload shaders while the scene runs
- status: open
- owner: —
- type: idea

### T-011 · Set up the test scene
- status: done
- owner: —
- priority: P1 · difficulty: easy · area: ui
- milestone: T-001
- note (2026-11-12, alice): done

### T-012 · Pick the colour palette
- status: done
- owner: —
- priority: P3
- note (2026-11-13, carol): done — the warm palette wins

### T-013 · Old shader
- status: dropped
- owner: —
- priority: P2
- milestone: T-002
- note (2026-11-14, carol): dropped — replaced by the C shader

### T-014 · Legacy light path
- status: dropped
- owner: —
- priority: P2
- milestone: T-001
- note (2026-11-16, carol): dropped with T-001 — the scene no longer needs it

### T-015 · Ship the demo
- status: open
- owner: —
- priority: P1 · difficulty: 3 · area: docs
