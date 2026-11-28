# Mutation testing of the guard modules

`todo` promises that several agents can share one `TODO.md` safely. The promise rests on a few
guards: the lock, the write to disk, the line model with its verified write, the status rules, the
file scopes and the commit checks. A guard that can be broken without a test going red is not
protected against the next change. Mutation testing finds such places: the tool `cargo-mutants`
changes the code one place at a time (a *mutant*) and runs the tests. A mutant that no test notices
is *missed*.

This is requirement TEST-11 in [`requirements.md`](requirements.md). It is run by hand, before a
release that changes a guard module, and never on every push: a run takes hours.

## What is examined

[`.cargo/mutants.toml`](../.cargo/mutants.toml) names the guard modules:

| Module | Guard |
|---|---|
| `src/lock.rs` | the write lock and the lease for hand edits (R-LOCK-1 to R-LOCK-7) |
| `src/atomic.rs` | the write to disk: temporary file, check before the rename, retry (R-IO-1 to R-IO-5) |
| `src/doc/lines.rs` | the line model and the verified write (R-EDIT-1 to R-EDIT-7) |
| `src/ops/rules.rs` | status changes and ownership (R-WRITE-9, R-CLAIM-3) |
| `src/ops/scope.rs` | file scopes (R-SCOPE-1 to R-SCOPE-3) |
| `src/ops/commit_files.rs` | what `done --commit` asks git (R-SCOPE-2, R-SCOPE-3) |

The report, the help texts and the other commands are not guards and are left out; the run time
grows with every module.

## How to run it

`cargo-mutants` is a tool of its own. It is installed apart and is no dependency of the crate
(NFR-9):

```powershell
cargo install cargo-mutants --locked
cargo mutants --version
```

Run it when no agent is editing the checkout: the tool copies the source tree as it is.

**First pass: every mutant against the guard tests.** Building and linking all 108 test programs
for each of several hundred mutants takes most of a day. `.cargo/mutants.toml` therefore runs each
mutant against the library tests and the 33 test programs that exercise the guards:

```powershell
New-Item -ItemType Directory -Force target\mutants\run | Out-Null
cargo mutants -j 4 -o target\mutants\run
```

**Second pass: what the first pass missed, against the whole suite.** A mutant that the guard tests
miss may still be caught by another test, so it counts as missed only after this pass.
`.cargo/mutants-full.toml` is the same configuration without the list of tests, and `--iterate`
skips every mutant that was already caught:

```powershell
cargo mutants --config .cargo\mutants-full.toml --iterate -j 4 -o target\mutants\run
```

The result is in `target\mutants\run\mutants.out`: `caught.txt`, `missed.txt`, `timeout.txt` and
`unviable.txt`, and `previously_caught.txt` for the mutants caught in an earlier pass. `target` is
ignored by git. Without `-o` the tool writes `mutants.out` into the repository root, which
`.gitignore` also covers.

- **caught**: a test failed. Good.
- **timeout**: the tests did not end (the mutant made a loop endless). The change was noticed.
- **unviable**: the mutant did not compile. It says nothing.
- **missed**: every test passed with the guard changed. Each one is judged (see below).

If a pass is stopped, run the same command again with `--iterate`: it goes on where it stopped.
`-j` sets how many mutants are built and tested at once; each job builds in its own copy of the
tree, so more jobs need more memory and disk, and tests that measure time can fail under load.

## What to do with a missed mutant

Look at the change and decide:

- **A real gap.** The guard can be broken and nothing notices. Add a task in `TODO.md` that names
  the function, the change that went unnoticed and the test that is missing. After the test is
  written, run the second pass again and see the mutant caught.
- **No gap.** The change cannot be seen from outside (it only alters a capacity hint, the exact
  instant of a time limit, or a message text), or it cannot be triggered without breaking into the
  program (two random file names that collide). Record it in the table at the end of this
  document, with the reason, so it is not judged again.

## The first run (2026-11-27 to 2026-11-28)

todo 1.1.0 plus the v1.2 changes (commit `1e2f833`), cargo-mutants 27.1.0, Windows 11, 16 logical
processors.

| Pass | Mutants tested | Time | Jobs |
|---|---|---|---|
| First pass (guard tests), in two parts because the first part was stopped | 532 | about 3.5 hours | 4, then 6 |
| Second pass (whole suite) | 110 | 2 hours | 5 |

| Result | Mutants |
|---|---|
| Caught in the first pass | 344 |
| Caught only by the whole suite | 17 |
| Timeout | 4 |
| Unviable | 78 |
| Missed | 89 |
| **All** | **532** |

Of the 454 mutants that compiled, the tests noticed 365, which is 80%.

Under load, with several jobs, a test that measures time can fail for a mutant that changes
nothing. Such a mutant then counts as caught although it is not; the numbers above may hide a few
of these.

### The 89 missed mutants

51 are real gaps. Each belongs to one of seven tasks under the milestone T-248:

| Task | What the missing tests check | Mutants |
|---|---|---|
| T-264 | Another agent's `todo lock` while a lease is live is refused and waits; the longest lease is allowed; an expired lease counts as ended at `unlock` | 5 in `src/lock.rs` |
| T-265 | The retry delay stays between 40 and 160 ms; `todo status` works when the state folder exists without a lock file | 9 in `src/lock.rs` |
| T-266 | Replaced lines keep their own line endings; the line number of invalid UTF-8; the verified write refuses a changed last line when lines are added at the end of a file without a final newline | 12 in `src/doc/lines.rs` |
| T-267 | `done --commit` from a subfolder compares paths from the folder of `TODO.md`; outside a repository a hash is kept and `HEAD` is refused | 3 in `src/ops/commit_files.rs` |
| T-268 | A blocked task can be dropped; the `ALREADY_CLAIMED` message gives the claim's age | 3 in `src/ops/rules.rs` |
| T-269 | A blocked task holds its files only with an owner and a fresh claim; the `FILES_BUSY` message names the holder; refreshing one's own claim is not checked against others | 9 in `src/ops/scope.rs` |
| T-270 | The temporary file is made in the folder of `TODO.md`; a `TODO.md` that disappears or is held by another program while the write checks it | 10 in `src/atomic.rs` |

38 are recorded as no gap:

| Where | Change | Mutants | Why it is no gap |
|---|---|---|---|
| `src/atomic.rs` `rename_with_retry` (251) | `<` to `<=` on the deadline | 1 | differs only at the exact instant of the deadline |
| `src/atomic.rs` `is_retryable_rename_error` (279) | always `true` | 1 | every rename error is then retried for 2 seconds before the same failure is reported; only slower |
| `src/atomic.rs` `is_retryable_rename_error` (286) | `==` to `!=` | 1 | the line is compiled on Linux only, and the run was on Windows |
| `src/atomic.rs` `create_temp_file_for` (292) | the guard always `true` | 1 | an empty parent folder joined with the file name is the same file as `.` joined with it |
| `src/atomic.rs` `create_temp_file_for` (317) | the retry when the temporary name exists | 7 | needs two 64-bit random names to collide; cannot be made to happen from outside |
| `src/atomic.rs` `TempFileGuard::disarm` (348) | does nothing | 1 | the guard then tries to delete a temporary file that the rename has already moved; nothing happens |
| `src/atomic.rs` `random_u64` (380) | returns 0 or 1 | 2 | temporary names still differ by process ID, and a name that exists is retried |
| `src/atomic.rs` `maybe_run_before_rename_hook` (399) | `>` to `>=` | 1 | test hook only; a pause of 0 ms is no pause |
| `src/lock.rs` `is_lease_live` (80) | `<` to `<=` | 1 | differs only at the exact instant the lease expires |
| `src/lock.rs` waiting loops (202, 325, 632) | the pause between tries is skipped | 3 | the command then tries again without pausing; the result is the same, only the processor is busier |
| `src/lock.rs` `read_existing` (466), `put_back_archive` (478, 479) | error paths of putting the archive back | 3 | they run only when replacing `TODO.md` fails after the archive was written and the archive was changed or removed in between; the tests that make the replace fail catch the other 12 mutants of this code. Open: a test would need a way to make that second failure happen |
| `src/lock.rs` `lease_duration` (658) | the test hook's `secs > 0` | 3 | test hook only |
| `src/doc/lines.rs` `LineEnding::as_str` (36) | returns another text | 2 | no write uses it; the bytes come from `as_bytes` |
| `src/doc/lines.rs` `Display for EditError` (110) | prints nothing | 1 | message text only; the commands report the error with its code |
| `src/doc/lines.rs` `LineDoc::to_bytes` (277, 280) | the size reserved for the output | 2 | a capacity hint; the bytes written are the same |
| `src/doc/lines.rs` `verify_unchanged_gap` (656) | `>` to `>=` | 1 | an empty gap has already returned before this line |
| `src/ops/rules.rs` `TaskCommand::as_str` (71) | returns another text | 2 | message text only |
| `src/ops/rules.rs` `bad_transition_error` (130) | `&&` to `\|\|` | 1 | chooses between two wordings of the same refusal |
| `src/ops/rules.rs` `validate_transition` (237) | the arm for `claim` on a claimed task is removed | 1 | `claim` never reaches this function; `plan_claim` decides it |
| `src/ops/scope.rs` `wildcard_matches` (312, 314) | `+ 1` to `* 1` when a `*` takes one more character | 2 | the match then steps back to the `*` and comes to the same answer in more steps |
| `src/ops/commit_files.rs` `resolve_commit` (213) | every failure to start git counts as git missing | 1 | differs only when git exists and cannot be started |

## Equivalent mutants recorded later

Add a row to the table above for each mutant judged as no gap in a later run, with its reason.
