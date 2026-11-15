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

## Status after chunk 1 (this run)
- Done: `ops::task_ops` (`TaskOp`, `NewTask`, `highest_task_number`) is the one implementation of
  claim, release, done, drop, block, unblock, reopen, answer, note, set and the task block of `add`
  (single `add`, `add --from` and `apply`). `cli::run_task_op`, `edit_one_task` and `report_task_*`
  replace the repeated scaffolding of those commands.
- Behaviour deltas in `apply` caused by sharing the single-command code (no test covered them):
  `unblock` to `claimed` now sets the `claimed` timestamp like `todo unblock`; extra `fields` named
  like a built-in key (`priority`, `type`, ...) are no longer written twice in `add`.
- Not shared yet: `move` (single and `apply` differ in blank-line handling), the read commands and
  the remaining ~25 commands (chunk 2).
- Resolved in chunk 2 step 1: `todo answer` and `apply` both write `answer — <text>` (format.md §9);
  `TaskOp::Answer` no longer has a verb parameter. No test expected `answered —`.

## Status after chunk 2, step 2 (T-083, item 1: test scaffolding)
- `tests/common/mod.rs` is the one helper module (it replaces `tests/lock_support`): `todo_bin`,
  `run_in`, `run_todo`, `run_with_stdin`, `spawn_todo`, `run_parallel_todos`, `wait_for_file`,
  `clean_todo_command`, `assert_todo`, `setup_proj`, `setup_fixture_proj`, `write_todo`,
  `write_toml`, `read_todo`, `read_text`, `fixture`, `copy_fixture`, `copy_clean_fixture`, `git`,
  `parse_json`, `stdout_text`, `stderr_text`. Every test file that needs one of them uses it.
- Removed: 7 local `todo_bin`, 8 local `setup_proj`, 6 local git helpers, 2 `copy_clean_fixture`,
  the `serde_json::from_slice(..).unwrap()` and `from_utf8_lossy` read-backs, 39 `todo_bin()` run
  chains (now `run_in`). Non-blank lines in `tests/`: 7,852 → 7,526; `src/`: 16,152 → 16,146.
- Assertions: `assert!`/`assert_eq!`/`assert_ne!` occurrences in `tests/` went 1,166 → 1,164. The two
  fewer are three `assert!` bodies of the old local git helpers, which are now one `assert!` in
  `common::git`; nothing else was removed or weakened.
- Still repeated (left for a later pass): inline `home`/`work` folder setup in files that do not use
  `setup_proj` (`join("home")` 13 times), `TempDir::new().unwrap()` boilerplate, and the
  `cargo_bin_cmd!` chains in `cmd_*` tests that need assert_cmd matchers (67 uses).
- Files with no shared helper: `atomic_io.rs`, `doc_fixtures.rs`, `doc_lines.rs`, `ops_rules.rs`
  (they test the library, not the binary, and need no process helpers).
