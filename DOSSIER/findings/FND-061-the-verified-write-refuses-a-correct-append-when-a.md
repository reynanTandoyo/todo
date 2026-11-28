<!-- dossier-format: 1 -->
# FND-061 · The verified write refuses a correct append when an empty insert comes before it at the end of a file without a final newline

- kind: finding
- status: approved
- created: 2026-11-28T02:59:41+01:00
- author: reyna
- via: reyna - claude fable 5.2
- approved: 2026-11-28T02:59:41+01:00 by reyna
- valid-from: 2026-11-28T02:59:41+01:00
- task: T-266
- tags: verify,lines,eof,mutation
- topic: format,testing
- code: src/doc/lines.rs::verify_expected_byte_changes
- history: 2026-11-28T02:59:41+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-28T02:59:41+01:00 reyna via reyna - claude fable 5.2: approved

## Observation

The check of the verified write (R-EDIT-7) refuses a correct result in one case: for the bytes 'a\nb' (no ending on the last line), the edits insert(2, []) followed by insert(2, ['c']) give the correct 'a\nb\nc', but verify_byte_edits fails with 'untouched line 2 changed bytes'; the same edits in the other order pass. Cause: verify_expected_byte_changes gives the allowance for the last line gaining its ending only to a change that adds lines, so the gap before an edit that changes nothing is checked without it. The fault errs to the safe side (a write is refused, nothing is written), and no command is known to build such a pair of edits. Found by the worker of T-266 while writing a test for mutants that the mutation run had missed; fixed by T-272.

## Evidence

Report of the worker of T-266 (test two_edits_at_the_end_of_a_file_without_a_final_newline_verify failed: 15 passed, 1 failed); src/doc/lines.rs verify_expected_byte_changes, lines 582 to 603
