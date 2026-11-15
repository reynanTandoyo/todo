//! The byte-exact line model, line edits with verification, and the parser for
//! `docs/format.md` version 1 (`TODO.md` tasks T-001, T-002, T-003).
//!
//! # Design
//!
//! - [`LineDoc`] reads raw UTF-8 bytes into lines while keeping each line's individual ending
//!   ([`LineEnding::Lf`], [`LineEnding::Crlf`], or [`LineEnding::None`]) and any leading UTF-8 BOM.
//! - [`LineDoc::apply_edits`] applies [`LineEdit`]s (replace, insert, remove whole lines) without
//!   touching any other byte.
//! - [`verify_byte_edits`] checks that every byte outside the intended edit ranges equals the
//!   original before writing to disk (R-EDIT-7).

pub mod lines;

pub use lines::{
    AppliedLineEdits, EditError, ExpectedByteChange, Line, LineDoc, LineEdit, LineEnding, UTF8_BOM,
    Utf8DecodeError, sanitize_single_line, sanitize_trimmed_value, verify_byte_edits,
    verify_expected_byte_changes,
};
