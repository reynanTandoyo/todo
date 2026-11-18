//! Task operations: shared configuration, read/query helpers, status rules, and the write pipeline.
//!
//! # Architecture for Write Commands
//!
//! Every write command in `src/cli/` should run through the shared write pipeline in [`pipeline`]
//! rather than reading or writing `TODO.md` directly:
//!
//! 1. **Pipeline ([`edit_doc_and_write`] / [`edit_task_and_write`])**:
//!    - Enforces `--agent` / `TODO_AGENT` (`R-CLAIM-5`).
//!    - Locates `TODO.md` (`R-FIND-1..5`) and resolves [`crate::state::StateDir`] (`R-HIST-1`, `R-LOCK-7`).
//!    - Acquires the OS write lock and honours any live manual lease (`R-LOCK-1..6`).
//!    - Reads bytes under the lock, validates UTF-8 (`R-IO-4`), parses [`crate::doc::Document`],
//!      and refuses any file-wide or task-level `UNSAFE_FILE` condition (`R-EDIT-8`, `format.md` §13),
//!      including duplicate IDs against `TODO-archive.md`.
//!    - Calls the command closure to produce [`crate::doc::LineEdit`]s and a [`crate::doc::TaskChangeIntent`].
//!    - Runs [`crate::doc::Document::apply_and_verify`] (`R-EDIT-7`) to prove untouched bytes and
//!      untouched tasks stayed identical.
//!    - Atomically replaces `TODO.md` only when bytes actually changed (`R-EDIT-6`, `R-IO-1..5`).
//!
//! 2. **Status transitions and ownership ([`rules`])**:
//!    - [`validate_transition`], [`plan_claim`], and [`plan_release`] encode the `R-WRITE-9`
//!      status transition table in one place and produce `BAD_TRANSITION` errors naming the current
//!      status and allowed commands.
//!    - [`check_ownership`] encodes `R-CLAIM-3` (requiring `--force` when modifying a task claimed
//!      by another agent, except for `note`).
//!    - [`forced_note_text`] formats the canonical `--force` audit note (`docs/format.md` §9).
//!
//! 3. **Canonical whole-line edits ([`TaskEditBuilder`])**:
//!    - Edits only changed lines in canonical form (`R-EDIT-9`), leaves untouched lines byte-exact
//!      (`R-EDIT-1`), inserts missing fields in `docs/format.md` §10 canonical order, edits combined
//!      `- priority:` lines in place (`docs/format.md` §8), and appends notes after the last line of
//!      the task block (`docs/format.md` §9).

pub mod config;
pub mod edits;
pub mod identity;
pub mod pipeline;
pub mod query;
pub mod read;
pub mod rules;
pub mod task_ops;

pub use config::{
    CONFIG_FILENAME, DEFAULT_ARCHIVE_FILE, DEFAULT_CLAIM_TTL_HOURS, DEFAULT_QUESTION_SECTION,
    DEFAULT_SNAPSHOTS_KEEP, OpsConfig,
};
pub use edits::TaskEditBuilder;
pub use pipeline::{
    DocWritePlan, NewlyReadyTask, WriteDocResult, WriteTaskResult, build_claim_edits,
    build_release_edits, check_task_writable, edit_doc_and_write, edit_doc_with_state_and_write,
    edit_task_and_write, parse_doc_for_write, require_yes_or_preview,
};
pub(crate) use pipeline::{
    edit_doc_with_found_and_state_and_write, edit_task_with_found_and_write,
};
pub use query::{
    ListFilter, NextFilter, SelectedNext, StatusFilter, WhereClause, filter_tasks,
    matching_section_indices, resolve_task_by_title, resolve_task_in_doc, select_next_task,
};
pub(crate) use read::load_for_read_with_found;
pub use read::{
    DependencyStatusJson, DependentTaskJson, ListTaskJson, LoadedRead, NoteJson, TaskJson,
    build_list_task_json, build_task_json, file_label, is_task_ready, load_for_read,
    questions_about, read_optional_archive,
};
pub use rules::{
    ClaimPlan, OwnershipDecision, ReleasePlan, StatusCommand, TaskCommand, allowed_commands_from,
    bad_transition_error, check_ownership, forced_note_text, plan_claim, plan_release,
    validate_transition,
};
pub use task_ops::{NewTask, OpEnv, SetChange, TaskOp, highest_task_number};
