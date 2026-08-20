//! # Note file update module
//!
//! **Purpose**: This module is responsible for safely updating local Markdown note files and
//! synchronising the note's `updated_at` timestamp in the local SQLite database.
//!
//! ## Exported items
//! * [`update_md`] — Atomically replaces a note's Markdown file contents and updates its
//!   database modification timestamp.
//!
//! ## Key design decisions
//! Note contents are first written to a temporary file and synchronised to disk before the
//! temporary file replaces the existing note file. This reduces the risk of leaving a partially
//! written note file if the application crashes during the write operation.
//!
//! The database timestamp is updated only after the filesystem operation succeeds.
//!
//! Note content is never written to logs. Only the note identifier and operation status are
//! logged for diagnostics.
//!
//! ## Dependencies
//! - `std::fs` — Writes and replaces local note files
//! - `rusqlite` — Updates note metadata in the local database
//! - `anyhow` — Adds context to database errors
//! - [`crate::config::ProgramFiles`] — Provides the notes and temporary directories
//! - [`crate::constants`] — Provides note filename and SQL constants
//! - [`crate::utils`] — Provides timestamps

use crate::constants::*;
use anyhow::Context;

use std::io::Write;
use std::path::Path;

/// Updates the contents of a local Markdown note and its database modification timestamp.
///
/// The note is written to a temporary file and synchronised before replacing the existing file.
/// This prevents the target file from containing partially written content if the application
/// crashes during the write operation.
///
/// # Errors
/// Returns an error if the temporary file cannot be created or written, the data cannot be
/// synchronised, the target file cannot be replaced, or the database timestamp cannot be updated.
pub fn update_md(
    notes_db: &rusqlite::Connection,
    note_id: String,
    written_string: String,
    program_paths: &crate::config::ProgramFiles,
) -> Result<(), crate::errors::Error> {
    tracing::debug!(
        task = "update note",
        %note_id,
        "starting note update"
    );

    let note_name = format!("{}.{}", note_id, NOTE_EXTENSION);
    let note_path = program_paths.notes_path.join(note_name);

    atomic_write(&note_path, written_string.as_bytes()).map_err(|e| {
        tracing::error!(
            task = "update note",
            status = "error",
            %note_id,
            error = ?e,
            "failed to atomically write note"
        );

        e
    })?;

    notes_db
        .execute(
            UPDATE_NOTE_SQL_QUERY,
            rusqlite::named_params! {
                ":updated_time": crate::utils::get_time(),
                ":id": note_id,
            },
        )
        .context("could not update note timestamp in database")
        .map_err(|e| {
            tracing::error!(
                task = "update note",
                status = "error",
                %note_id,
                error = ?e,
                "failed to update note timestamp in database"
            );

            e
        })?;

    tracing::debug!(
        task = "update note",
        status = "success",
        %note_id,
        "note updated successfully"
    );

    Ok(())
}

fn atomic_write(path: &Path, content: &[u8]) -> std::io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("file has no parent directory"))?;

    let mut temp_file = tempfile::NamedTempFile::new_in(parent)?;

    temp_file.write_all(content)?;
    temp_file.as_file().sync_all()?;

    temp_file.persist(path).map_err(|e| e.error)?;

    Ok(())
}
