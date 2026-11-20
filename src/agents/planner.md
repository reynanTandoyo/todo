# Planner rules (todo help planner)

Start with `todo summary`, then `todo plan`: plan until `todo plan` is empty (exit 0).

Tasks and milestones
- Put each new task under a milestone with `--milestone <id> --why "<why>"`. A milestone is a piece of
  work with a goal, never a release label. Read a milestone's goal before you plan its tasks.
- Close a milestone with `todo done <milestone-id>` when its last task is finished.
- Record an idea with `todo idea "<text>"`. Ask a question that only a person can answer with
  `todo ask "<text>"`; `--about <id>` names the entry it is about.
- Record a problem you find with `todo add ...` instead of fixing it silently.

Levels
- Give every task a level with `--difficulty 1` to `5`; `todo help add` prints what each level means.
  Level 1 is the lowest: everything is written down, so the weakest worker finishes it.
- Split a task above the target level with `todo split <id> --from <file|->`, until `todo plan` is
  empty. Keep a higher level only with `todo set <id> --final "<reason>"`.
- A task at the target level needs `files` (what it may change), `steps`, `verify`, `why` and
  `accept`; `read` names what the worker should look at. Write them with `todo set`. `todo plan`
  lists what is still missing.
- Tasks that share a file never run at the same time: keep `files` exact, and see the lanes in
  `todo waves`.

Waves and chains
- Read `todo waves` before you release workers. Break a long chain by splitting a task, or by removing
  a dependency that is not needed.
- In wave mode (`verify_when = "wave"`), run `todo verify --wave` when every worker of a wave has
  finished, before the next wave starts. Otherwise run the full test suite when a wave is finished.
- Take back tasks given back with `todo stuck`: `todo unblock <id>` makes one open again, and
  `todo split <id>` breaks it up.

Keeping the list healthy
- Run `todo migrate --yes` after todo is updated, in each project.
- When todo says a file needs a newer todo (`TOOL_TOO_OLD`), follow `todo help update`.
- Run `todo check` before you end a session: it reports problems, and warnings that do not change
  its exit code.
