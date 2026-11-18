//! Migration of `TODO.md` and its archive by a write (`docs/format.md` §18, R-MIG-1).
//!
//! A write command on a file in an older format applies every pending step first, under the write
//! lock, after the usual snapshot and in the same write as its own change. This module plans those
//! steps and checks their result; the pipeline and the commands write the bytes it returns.

use std::path::Path;

use serde::Serialize;

use crate::atomic::read_file_with_snapshot;
use crate::doc::{
    CURRENT_FORMAT_VERSION, Document, Migrated, MigrationError, MigrationStep, STEPS,
    migrate_document,
};
use crate::error::{ErrorCode, Result, TodoError};
use crate::lock::{ArchiveWrite, MigrationWrite};
use crate::ops::OpsConfig;
use crate::ops::pipeline::parse_doc_for_write;
use crate::version::RUNNING_VERSION;

/// One file that a migration moves to the current format, such as `TODO.md` from 1 to 2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MigratedFile {
    /// The file name, as shown to people (`TODO.md`, `TODO-archive.md`).
    pub file: String,
    /// The format version it was in.
    pub from: u32,
    /// The format version it is in now.
    pub to: u32,
}

/// The migrations that a write or `todo migrate` applies to `TODO.md` and its archive (R-MIG-1).
///
/// Nothing is written while the plan is made; the caller writes the bytes it returns, under the
/// lock, and only when its own change goes ahead.
#[derive(Debug)]
pub(crate) struct PendingMigration {
    /// `TODO.md` after the pending steps; `None` when it is already in the current format.
    pub todo: Option<Migrated>,
    /// The archive file after the pending steps, with the snapshot its write is guarded by;
    /// `None` when there is no archive, it is already current, or the caller writes it itself.
    pub archive: Option<ArchiveWrite>,
    /// The files that are migrated, in the order they are named.
    pub files: Vec<MigratedFile>,
}

impl PendingMigration {
    /// Plans the migration of a parsed `TODO.md`.
    ///
    /// `archive` is the parsed archive file when the caller has read it; its migration is planned
    /// with a fresh read of `archive_path`, so the write can detect a change made since. Pass
    /// `None` when the caller migrates the archive itself.
    pub(crate) fn plan(
        doc: &Document,
        label: &str,
        archive: Option<&Document>,
        archive_path: &Path,
        cfg: &OpsConfig,
    ) -> Result<Self> {
        let mut files = Vec::new();
        let todo = migrate_for_write(doc, label)?;
        if let Some(m) = &todo {
            files.push(MigratedFile {
                file: label.to_string(),
                from: m.from,
                to: m.to,
            });
        }

        let mut archive_write = None;
        if let Some(arch) = archive
            && arch.effective_format_version() < CURRENT_FORMAT_VERSION
            && let Some((write, from, to)) = plan_archive(archive_path, cfg)?
        {
            files.push(MigratedFile {
                file: cfg.archive_file.clone(),
                from,
                to,
            });
            archive_write = Some(write);
        }

        Ok(Self {
            todo,
            archive: archive_write,
            files,
        })
    }

    /// Splits the plan into the migrated `TODO.md`, for the write to continue from, and the record
    /// of the migration that the write carries into its journal entry and archive write. The record
    /// is `None` when nothing is migrated.
    pub(crate) fn split(self) -> (Option<Migrated>, Option<MigrationWrite>) {
        if self.files.is_empty() {
            return (self.todo, None);
        }
        let record = MigrationWrite {
            summary: describe(&self.files),
            archive: self.archive,
        };
        (self.todo, Some(record))
    }
}

/// Names migrated files for the standard error line and the journal, such as
/// `TODO.md: format 1 to 2; TODO-archive.md: format 1 to 2`.
#[must_use]
pub(crate) fn describe(files: &[MigratedFile]) -> String {
    files
        .iter()
        .map(|f| format!("{}: format {} to {}", f.file, f.from, f.to))
        .collect::<Vec<_>>()
        .join("; ")
}

/// Migrates a parsed document to the current format (R-MIG-1). `None` when it is already current.
///
/// The migrated text must parse without write-blocking problems (`docs/format.md` §13); otherwise
/// the migration is refused with `UNSAFE_FILE` (exit 4) and nothing is written.
pub(crate) fn migrate_for_write(doc: &Document, label: &str) -> Result<Option<Migrated>> {
    migrate_checked(doc, label, STEPS, CURRENT_FORMAT_VERSION)
}

/// [`migrate_for_write`] with the steps and the target version given, so that tests can run other
/// steps through the same refusal.
pub(crate) fn migrate_checked(
    doc: &Document,
    label: &str,
    steps: &[MigrationStep],
    target: u32,
) -> Result<Option<Migrated>> {
    let migrated = migrate_document(doc, steps, target).map_err(|e| migration_error(&e, label))?;
    if let Some(m) = &migrated
        && let Some(problem) = m.doc.problems.iter().find(|p| p.blocks_all_writes())
    {
        return Err(TodoError::new(
            ErrorCode::UnsafeFile,
            format!(
                "cannot migrate {label} to format {target}: {}",
                problem.message
            ),
        )
        .with_file(label)
        .with_line(problem.line));
    }
    Ok(migrated)
}

/// Plans the migration of the archive file on disk. `None` when it is already current.
fn plan_archive(archive_path: &Path, cfg: &OpsConfig) -> Result<Option<(ArchiveWrite, u32, u32)>> {
    let snap = read_file_with_snapshot(archive_path)?;
    let parsed = parse_doc_for_write(&snap.bytes, &cfg.archive_file, cfg)?;
    let Some(migrated) = migrate_for_write(&parsed, &cfg.archive_file)? else {
        return Ok(None);
    };
    let new_bytes = stamped_bytes(&migrated.doc, &cfg.archive_file)?;
    let write = ArchiveWrite {
        path: archive_path.to_path_buf(),
        expected: Some(snap.snapshot),
        new_bytes,
    };
    Ok(Some((write, migrated.from, migrated.to)))
}

/// The bytes of `doc` with the tool stamp added or raised to the running version (R-VER-1).
pub(crate) fn stamped_bytes(doc: &Document, label: &str) -> Result<Vec<u8>> {
    match doc
        .with_tool_stamp(RUNNING_VERSION)
        .map_err(|e| TodoError::new(ErrorCode::WriteFailed, e.to_string()).with_file(label))?
    {
        Some(applied) => Ok(applied.bytes),
        None => Ok(doc.to_bytes()),
    }
}

fn migration_error(err: &MigrationError, label: &str) -> TodoError {
    TodoError::new(
        ErrorCode::WriteFailed,
        format!("cannot migrate {label}: {err}"),
    )
    .with_file(label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::LineEdit;
    use crate::error::{ErrorCode, ExitCode};

    /// A step whose result is a format line this build does not know, so it must be refused.
    fn unknown_version_edits(doc: &Document) -> Vec<LineEdit> {
        vec![LineEdit::replace_line(
            doc.format_line.unwrap_or(0),
            "<!-- todo-format: 99 -->",
        )]
    }

    const UNKNOWN_VERSION_STEP: MigrationStep = MigrationStep {
        from: 1,
        to: 2,
        edits: unknown_version_edits,
    };

    fn parse(text: &str) -> Document {
        Document::parse_bytes(text.as_bytes()).expect("valid UTF-8")
    }

    #[test]
    fn a_migration_that_leaves_a_blocking_problem_is_refused_with_exit_four() {
        let doc = parse("<!-- todo-format: 1 -->\n# Title\n");
        let err = migrate_checked(&doc, "TODO.md", &[UNKNOWN_VERSION_STEP], 2)
            .expect_err("the migrated text must be refused");
        assert_eq!(err.code, ErrorCode::UnsafeFile);
        assert_eq!(err.exit_code, ExitCode::FileError);
        assert!(
            err.message
                .starts_with("cannot migrate TODO.md to format 2"),
            "{}",
            err.message
        );
        assert_eq!(err.details.line, Some(1));
    }

    #[test]
    fn a_document_already_in_the_current_format_has_nothing_to_migrate() {
        let doc = parse(&format!(
            "<!-- todo-format: {CURRENT_FORMAT_VERSION} -->\n# Title\n"
        ));
        let migrated = migrate_for_write(&doc, "TODO.md").expect("no error");
        assert!(migrated.is_none());
    }

    #[test]
    fn a_format_one_document_migrates_to_the_current_format() {
        let doc = parse("<!-- todo-format: 1 -->\n# Title\n");
        let migrated = migrate_for_write(&doc, "TODO.md")
            .expect("no error")
            .expect("format 1 is migrated");
        assert_eq!((migrated.from, migrated.to), (1, CURRENT_FORMAT_VERSION));
        assert_eq!(
            migrated.doc.effective_format_version(),
            CURRENT_FORMAT_VERSION
        );
    }

    #[test]
    fn describe_names_each_migrated_file_with_its_versions() {
        let files = vec![
            MigratedFile {
                file: "TODO.md".to_string(),
                from: 1,
                to: 2,
            },
            MigratedFile {
                file: "TODO-archive.md".to_string(),
                from: 1,
                to: 2,
            },
        ];
        assert_eq!(
            describe(&files),
            "TODO.md: format 1 to 2; TODO-archive.md: format 1 to 2"
        );
    }
}
