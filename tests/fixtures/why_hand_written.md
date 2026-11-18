<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 1 -->
## Tasks

### T-001 · Render the first frame
- status: open
- owner: —
- priority: P1 · type: feature · area: renderer
- why: the renderer must show a frame before shading is added
- problem: nothing is drawn yet
- accept: a frame is written to disk

### T-002 · Add the shading stage
- status: open
- owner: —
- priority: P2 · type: feature · area: renderer
- depends: T-001
- why: shading makes the frame match the reference images
- accept: the reference scene matches
