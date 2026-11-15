//! Task operations: shared configuration, read/query helpers, and the write pipeline.

pub mod config;
pub mod query;
pub mod read;

pub use config::{DEFAULT_ARCHIVE_FILE, DEFAULT_CLAIM_TTL_HOURS, OpsConfig};
pub use query::{
    ListFilter, NextFilter, SelectedNext, StatusFilter, WhereClause, filter_tasks,
    matching_section_indices, select_next_task,
};
pub use read::{
    DependencyStatusJson, DependentTaskJson, ListTaskJson, LoadedRead, NoteJson, TaskJson,
    build_list_task_json, build_task_json, file_label, is_task_ready, load_for_read,
    read_optional_archive,
};
