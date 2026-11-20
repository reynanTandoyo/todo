# Worker rules (todo help worker)

- Take work: `todo next --claim --max-difficulty 1 --brief --agent <name>`.
- Read the files in `read`; change only the files in `files`, and follow the numbered steps.
- Commit only the listed files, with the task ID first in the message.
- Finish: `todo done <id> --commit HEAD --agent <name>`; it runs the `verify` line first.
- Cannot finish: `todo stuck <id> --note "<problem>" --agent <name>`, then stop work on it.
- `todo brief <id> --agent <name>` repeats the last three lines with the task's ID filled in.
