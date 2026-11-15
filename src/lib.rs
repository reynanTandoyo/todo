//! `todo`: a concurrency-safe editor for `TODO.md` task lists.
//!
//! What each module must do is in `docs/requirements.md`; the file format is in
//! `docs/format.md`; the tasks that build each module are in `TODO.md`.

pub mod agents;
pub mod atomic;
pub mod cli;
pub mod doc;
pub mod error;
pub mod find;
pub mod history;
pub mod lock;
pub mod ops;
pub mod output;
pub mod state;
