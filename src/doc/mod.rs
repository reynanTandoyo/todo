//! The byte-exact line model, line edits with verification, and the parser for
//! `docs/format.md` (`TODO.md` tasks T-001, T-002, T-003).
//!
//! # Architecture
//!
//! - [`lines`]: [`LineDoc`] reads raw UTF-8 bytes into lines while keeping each line's individual
//!   ending ([`LineEnding::Lf`], [`LineEnding::Crlf`], or [`LineEnding::None`]) and any leading
//!   UTF-8 BOM. [`LineDoc::apply_edits`] applies whole-line [`LineEdit`]s without modifying any
//!   other byte.
//! - [`id`]: [`TaskId`] and [`IdConfig`] parse user input (`12`, `t12`, `T-12`, `T-012`) and
//!   format canonical IDs (`T-007`).
//! - [`parser`]: [`Document`] parses format versions 1 and 2 (`docs/format.md`), handling ignored
//!   regions (fenced code blocks and HTML comments), sections, task blocks, combined priority lines,
//!   known and unknown fields, notes, managed blocks, and every write-blocking problem or warning.
//! - [`verify`]: [`Document::apply_and_verify`] and [`verify_task_edits`] prove both byte
//!   preservation and parsed-task equivalence outside the intended [`TaskChangeIntent`] (R-EDIT-7).
//! - [`stamp`]: the tool version stamp that every write adds or raises (R-VER-1).
//! - [`migrate`]: the ordered format migrations that a write applies to a file in an older format
//!   (R-MIG-1).
//! - [`tree`]: the kinds of entry and the milestone tree (`docs/format.md` §16, §17).

pub mod id;
pub mod lines;
pub mod migrate;
pub mod parser;
pub mod stamp;
pub mod tree;
pub mod verify;

pub use id::{DEFAULT_ID_PREFIX, DEFAULT_ID_WIDTH, IdConfig, TaskId};
pub use lines::{
    AppliedLineEdits, EditError, ExpectedByteChange, Line, LineDoc, LineEdit, LineEnding, UTF8_BOM,
    Utf8DecodeError, sanitize_single_line, sanitize_trimmed_value, verify_byte_edits,
    verify_expected_byte_changes,
};
pub use migrate::{
    Migrated, MigrationError, MigrationStep, STEPS, format_line_text, migrate_document,
};
pub use parser::{
    CANONICAL_FIELD_ORDER, CANONICAL_HEADING_SEPARATOR, CANONICAL_NOBODY_OWNER,
    CURRENT_FORMAT_VERSION, CombinedPriorityLine, DependencyRef, Document, FieldEntry,
    FieldLocation, MANAGED_BLOCK_BEGIN, MANAGED_BLOCK_END, ManagedBlock, Note,
    OLDEST_FORMAT_VERSION, Problem, ProblemKind, ProblemSeverity, Section, SectionLookup, Status,
    TASK_HEADING_SEPARATORS, Task, ToolStamp, agent_names_equal, canonical_field_rank,
    inspect_problems, is_nobody_owner, is_valid_agent_name, strip_section_number,
};
pub use stamp::{stamp_bytes, tool_line_text};
pub use tree::EntryKind;
pub use verify::{AppliedDocEdits, TaskChangeIntent, verify_task_edits};
