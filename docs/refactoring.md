# Refactoring audit (T-083)

Line counts are non-blank lines. Baseline: `src/` 16,346 in 61 files, `tests/` 7,852 in 45 files.
Items are ordered by estimated lines saved.

## 1. Test suite scaffolding (~1,500 lines, tests/)
- Every `tests/cmd_*.rs` repeats temp-folder setup, fixture copy, command spawn and JSON parsing.
- Replace with one `tests/common/mod.rs` (run helper, JSON helper, fixture helper). Chunk 4.

## 2. Status-transition commands and `apply` (~900 lines, src/cli, src/ops)
- `done.rs:71`, `block.rs:50`, `unblock.rs:41`, `reopen.rs:49`, `drop.rs:59`, `answer.rs:49`,
  `claim.rs:34`, `release.rs:93`, `note.rs:55` each build the same edit closure
  (`validate_transition`, `TaskEditBuilder`, forced-note, dated note).
- `apply.rs:404-700` re-implements every one of them, and `add.rs:567` re-implements `add`.
- Replace with one operation enum plus one function that builds the edits (`ops::task_ops`), used by
  the single commands and by `apply`. Chunk 3 (this task).

## 3. Command scaffolding in src/cli (~600 lines)
- 14 commands repeat: load config, call `edit_task_and_write`, then
  `if out.is_json() { emit_json_ok(META.name, &res.task) } else { write_line("<id>: <cmd> ok") }`.
- 13 repeat the `TASK` argument definition; 38 repeat the `Args as ClapArgs` import.
- Replace with one helper `cli::run_task_edit` taking only the per-command parts. Chunk 2 (this task).

## 4. Write-pipeline variants in src/ops/pipeline.rs (~400 lines)
- `edit_doc_and_write`, `edit_doc_with_state_and_write`, `edit_doc_with_found_and_state_and_write`,
  `edit_task_and_write`, `edit_task_with_found_and_write` are thin variations of one core.
- Replace with one builder or options struct. Later chunk.

## 5. Lock and history (~300 lines)
- `src/lock.rs`, `src/history.rs`, `src/cli/lock.rs`, `unlock.rs`, `history.rs`, `log.rs`
  repeat lease reading, formatting and JSON shaping. Later chunk.

## 6. Read commands (~250 lines)
- `list.rs`, `next.rs`, `search.rs`, `summary.rs`, `show.rs` each load, filter and render tasks with
  similar text/JSON row code. Shared row renderer. Later chunk.

## 7. Parser and `Context::resolve_todo_and_config_quiet` (~150 lines)
- Three near-identical `FindOptions` constructions in `cli/mod.rs:166-212`. Later chunk.

## Next chunks
1. Command layer (step 2) and batch paths (step 3), this run.
2. Write-pipeline variants (item 4) and the read commands (item 6).
3. Lock and history (item 5), parser and config lookup (item 7).
4. Test suite helpers (item 1).
