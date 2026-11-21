<!-- TEST FIXTURE: invented data, not a real task list -->
<!-- todo-format: 3 -->
<!-- todo-tool: 0.8.1 -->
# Lantern tool

## Now

### T-001 · Lantern tool
- status: open
- owner: —
- priority: P1 · type: milestone
- goal: a sky map becomes the list of lanterns for the club hall
- summary: Turns a sky map into the lanterns to hang in the club hall.

### T-002 · Sky map reader
- status: open
- owner: —
- priority: P1 · type: milestone
- milestone: T-001
- goal: every map format the club uses can be read

### T-003 · Read the star catalogue
- status: done
- owner: —
- priority: P1 · difficulty: 2 · area: parser
- milestone: T-002
- summary: The star catalogue is read from its text file, with the position and brightness of each star.
- note (2026-11-13, ines): added
- note (2026-11-17, ines): done — the reader also takes the old file layout

### T-004 · Parse the map header
- status: claimed
- owner: ines
- priority: P1 · difficulty: 2 · area: parser
- claimed: 2026-11-18T20:00+01:00
- milestone: T-002
- why: the header says which map format a file uses
- note (2026-11-16, ines): added

### T-005 · Read the grid format
- status: open
- owner: —
- priority: P2 · difficulty: 3 · area: parser
- milestone: T-002
- depends: T-004
- why: the club's older maps use a grid, not a list of stars
- note (2026-11-16, ines): added

### T-006 · Write the sky map test set
- status: claimed
- owner: sam
- priority: P2 · difficulty: 2 · area: tests
- claimed: 2026-11-05T14:00+01:00
- milestone: T-002
- summary: A small set of maps with known answers, checked on every change.

### T-010 · Convert the old map format
- status: dropped
- owner: —
- priority: P3 · difficulty: 3 · area: parser
- milestone: T-002
- note (2026-11-10, sam): dropped — the club has no maps left in that format

### T-011 · Lamp plan
- status: open
- owner: —
- priority: P1 · type: milestone
- milestone: T-001
- goal: each room has a plan of its lanterns

### T-012 · Print the lantern plan
- status: blocked
- owner: —
- priority: P2 · difficulty: 3 · area: output
- milestone: T-011
- note (2026-11-15, sam): blocked — waiting for the printer driver

### T-013 · Place the lanterns on the east wall
- status: blocked
- owner: —
- priority: P1 · difficulty: 4 · area: plan
- milestone: T-011
- note (2026-11-16, sam): stuck — the plan puts one lantern too many on the east wall

### T-014 · Count the lanterns per room
- status: open
- owner: —
- priority: P1 · difficulty: 3 · area: plan
- milestone: T-011
- depends: T-015
- note (2026-11-15, sam): added

### T-015 · How many lanterns per room?
- status: open
- owner: —
- priority: P1 · type: question
- about: T-011
- note (2026-11-15, sam): added

### T-016 · Agree the lamp colour
- status: done
- owner: —
- priority: P1 · area: plan
- milestone: T-001
- note (2026-11-15, ines): done — the warm white is agreed

### T-020 · Club hall
- status: done
- owner: —
- priority: P1 · type: milestone
- milestone: T-001
- goal: the hall is measured and marked before the plan is drawn
- note (2026-11-18, ines): done — every point is measured

### T-021 · Measure the hall
- status: done
- owner: —
- priority: P1 · difficulty: 1 · area: hall
- milestone: T-020
- summary: The hall is measured wall to wall, with the height of the ceiling.
- note (2026-11-17, ines): added
- note (2026-11-17, ines): done — 12 m by 8 m

### T-022 · Mark the ceiling points
- status: done
- owner: —
- priority: P2 · difficulty: 1 · area: hall
- milestone: T-020
- note (2026-11-18, ines): done — 24 points marked

### T-030 · Add a licence file
- status: open
- owner: —
- priority: P3 · difficulty: 1 · area: docs
- summary: The project has no licence file yet.

### T-031 · Show the lamp heat map
- status: open
- owner: —
- type: idea

### T-032 · Pick the font for the plan
- status: done
- owner: —
- priority: P3 · area: output
- note (2026-11-16, ines): done — a plain sans-serif font

### T-033 · Print a poster
- status: dropped
- owner: —
- priority: P3 · area: output
- note (2026-11-14, sam): dropped — the plan on the screen is enough
