<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 1 -->
# Three levels of milestones

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
- status: open
- owner: —
- priority: P1 · difficulty: hard · type: feature
- milestone: T-002
- why: the renderer has no shading stage yet

### T-004 · Shadow maps
- status: done
- owner: —
- priority: P2
- milestone: T-002

### T-005 · Finished piece of work
- status: done
- owner: —
- priority: P2 · type: milestone
- milestone: T-001
- goal: the earlier piece of work is finished

### T-006 · Finished task under a finished milestone
- status: done
- owner: —
- priority: P2
- milestone: T-005

## Questions

### T-007 · Should the shader support shadows?
- status: open
- owner: —
- priority: P2 · type: question
- about: T-003
