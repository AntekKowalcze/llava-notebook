//! # Deleted note cleanup module
//!
//! **Purpose**: This module identifies notes whose soft-deletion retention
//! period has expired and permanently removes them from local storage.
//!
//! It queries the local SQLite database for deleted notes older than the
//! configured hard-delete threshold and delegates the actual permanent
//! deletion to the storage layer.
//!
//! ## Exports
//!
//! * [`hard_deletes_terminated_notes`] — Finds notes whose deletion period has
//!   expired, permanently removes them from local storage, and returns their
//!   local identifiers.
//!
//! ## Key design decisions
//!
//! Hard deletion is performed only after the configured retention period has
//! elapsed. The expiration timestamp is calculated from the current UTC time
//! and [`crate::constants::HARD_DELETE_TIME`].
//!
//! The database query is performed before deleting any files so that the set
//! of notes to process is determined consistently for the cleanup operation.
//!
//! Individual deletion failures are logged but do not abort the cleanup of
//! other expired notes. This allows the cleanup process to make progress even
//! when one note cannot currently be removed.
//!
//! The actual deletion logic is delegated to
//! [`crate::storage::hard_delete_note`], keeping filesystem and database
//! deletion responsibilities outside this module.
//!
//! ## Dependencies
//!
//! * [`rusqlite`] — Queries the local SQLite database for expired deleted
//!   notes.
//! * [`anyhow`] — Adds context to database query failures.
//! * [`std::path::Path`] — Provides the path to temporary deleted-note
//!   storage.
//! * [`crate::constants`] — Provides the configured hard-delete retention
//!   period.
//! * [`crate::storage`] — Performs permanent deletion of note data.
//! * [`crate::utils`] — Provides the current UTC timestamp.
//! * [`crate::errors`] — Application-level error type returned by the cleanup
//!   operation.
//! * [`tracing`] — Logs successful cleanup completion and individual deletion
//!   failures.

use anyhow::Context;
use rusqlite::{Connection, named_params};
use std::path::Path;

pub fn hard_deletes_terminated_notes(
    notes_db: &Connection,
    tmp_deleted_path: &Path,
) -> Result<Vec<String>, crate::errors::Error> {
    let expired_time = crate::utils::get_time() - crate::constants::HARD_DELETE_TIME;

    let mut stmt = notes_db
        .prepare(
            r#"
            SELECT local_id
            FROM notes
            WHERE is_deleted = 1
              AND deleted_at IS NOT NULL
              AND deleted_at < :time
            "#,
        )
        .context("failed to prepare statement for expired notes")?;

    let mut id_vec: Vec<String> = Vec::new();

    for row in stmt
        .query_map(
            named_params! {
                ":time": expired_time,
            },
            |row| row.get::<_, String>(0),
        )
        .context("failed to query expired notes")?
    {
        id_vec.push(row.context("failed to read expired note id")?);
    }

    for id in &id_vec {
        if let Err(e) = crate::storage::hard_delete_note(notes_db, tmp_deleted_path, id) {
            tracing::error!(
                task = "cleanup expired deleted notes",
                note_id = %id,
                error = ?e,
                "failed to permanently delete expired note"
            );
        }
    }

    tracing::info!(
        task = "cleanup expired deleted notes",
        count = id_vec.len(),
        "expired deleted notes cleanup completed"
    );

    Ok(id_vec)
}
