//! # Note storage utility module
//!
//! This module provides database and filesystem helpers for retrieving notes,
//! verifying note ownership, managing note metadata, and managing the lifecycle
//! of deleted notes in local storage.
//!
//! ## Exported items
//!
//! * [`verify_note_owner`] — Verifies that a note belongs to the specified user.
//! * [`get_note`] — Loads a complete [`Note`] from the notes database.
//! * [`get_note_content`] — Reads note content from its local filesystem path.
//! * [`check_if_note_is_encrypted`] — Checks whether encryption is enabled for a note.
//! * [`toggle_note_encryption`] — Changes the encryption state of a note.
//! * [`toggle_note_sync`] — Changes the synchronization state of a note.
//! * [`update_title`] — Updates a note title and its modification timestamp.
//! * [`get_title`] — Retrieves a note title from the database.
//! * [`get_note_struct`] — Loads a complete note and decrypts its title when necessary.
//! * [`remove_note`] — Soft-deletes a note by moving its content to temporary deleted
//!   storage and marking the database record as deleted.
//! * [`restore_deleted_note`] — Restores a soft-deleted note from temporary deleted
//!   storage.
//! * [`hard_delete_note`] — Permanently deletes a note locally and, when required,
//!   preserves the database record until its tombstone is synchronized.
//!
//! ## Key design decisions
//!
//! Note ownership is verified against the `owner_id` field stored in the local
//! notes database before accessing note data. Database lookup failures for a
//! requested note are mapped to [`crate::errors::Error::NoteNotFound`] where
//! appropriate.
//!
//! Note content is stored in the filesystem rather than directly in SQLite.
//! This module is responsible only for reading and managing the local file.
//! Encryption and decryption are handled by the cryptographic module.
//!
//! Deleted notes use a two-stage deletion model:
//!
//! 1. **Soft delete** — the note file is moved to `tmp_deleted`, while the
//!    database record is retained with `is_deleted = 1` and `deleted_at` set.
//! 2. **Hard delete** — the temporary note file is permanently removed. For
//!    synchronized notes, the database record is retained with
//!    `sync_state = 'WaitingForTombstone'` until the deletion has been
//!    propagated to the synchronization server.
//!
//! Notes that are `LocalOnly` do not require a server-side tombstone. Their
//! database record can therefore be removed immediately during hard deletion.
//!
//! Restoring a note reverses the soft-delete operation by moving its content
//! back from `tmp_deleted` to the normal notes directory, clearing
//! `is_deleted` and `deleted_at`, and updating the synchronization state.
//! A note already waiting for a tombstone cannot be restored because its
//! local content has already been permanently deleted.
//!
//! Synchronization state is used to determine whether a deletion must be
//! propagated to the server. In particular, `PendingDeleted` represents a
//! deleted note whose permanent deletion still needs to be synchronized,
//! while `WaitingForTombstone` represents a note whose local content has
//! already been permanently deleted and whose database record is retained
//! solely for tombstone synchronization.
//!
//! Metadata changes such as title and encryption-state changes update the
//! `updated_at` timestamp so that the synchronization layer can detect the
//! modification.
//!
//! Note identifiers and user identifiers are logged for diagnostics, but note
//! content and cryptographic material are never included in logs.
//!
//! ## Dependencies
//!
//! * `rusqlite` — Queries and updates the local notes database.
//! * `anyhow` — Adds context to database and filesystem errors.
//! * `std::fs` — Reads, moves, and permanently removes note files.
//! * `std::path::{Path, PathBuf}` — Represents note filesystem paths.
//! * `uuid` — Parses local and owner identifiers stored in the database.
//! * [`crate::Note`] — Application-level note model.
//! * [`crate::errors::Error`] — Application error type.
//! * [`crate::storage::SyncState`] — Represents the synchronization state of a note.
//! * [`crate::crypto::decrypt_title`] — Decrypts encrypted note titles.
//!
//! ## Security considerations
//!
//! Ownership must be verified before returning note data to callers. Callers
//! should perform this check before accessing a note whenever the operation is
//! initiated by an untrusted frontend request.
//!
//! Note content, encrypted data, cryptographic keys, and other sensitive note
//! data must never be written to diagnostic logs.
//!
//! Hard deletion of a synchronized note does not immediately remove its
//! database record. The record is intentionally retained until the associated
//! tombstone has been successfully synchronized, preventing the server from
//! losing information about the permanent deletion while the device is offline.
use anyhow::Context;
use chacha20poly1305::Key;
use rusqlite::{Connection, OptionalExtension, named_params, params};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::{Note, crypto::decrypt_title, errors::Error, storage::SyncState};

/// Verifies that a note belongs to the specified user.
///
/// # Errors
///
/// Returns [`crate::errors::Error::NoteNotFound`] if the note does not exist
/// or the ownership query cannot be completed.
pub fn verify_note_owner(
    user_id: &str,
    note_id: &str,
    notes_db: &Connection,
) -> Result<bool, crate::errors::Error> {
    tracing::debug!(
        task = "verify note ownership",
        %note_id,
        %user_id,
        "checking note ownership"
    );

    let owner = notes_db
        .query_row(
            "SELECT owner_id FROM notes WHERE local_id = :note_id",
            named_params! {
                ":note_id": note_id,
            },
            |row| {
                let id: String = row.get(0)?;
                Ok(id)
            },
        )
        .map_err(|e| {
            tracing::error!(
                task = "verify note ownership",
                status = "error",
                %note_id,
                %user_id,
                error = ?e,
                "failed to retrieve note owner"
            );

            crate::errors::Error::NoteNotFound
        })?;

    let is_owner = owner == user_id;

    tracing::debug!(
        task = "verify note ownership",
        status = "success",
        %note_id,
        %user_id,
        is_owner,
        "note ownership verified"
    );

    Ok(is_owner)
}

/// Loads a complete note from the local notes database.
///
/// # Errors
///
/// Returns [`crate::errors::Error::NoteNotFound`] if the note does not exist
/// or the database query cannot be completed.
pub fn get_note(note_id: &str, notes_db: &Connection) -> Result<Note, crate::errors::Error> {
    tracing::debug!(
        task = "get note",
        %note_id,
        "loading note from database"
    );

    let note: Note = notes_db
        .query_row(
            "SELECT
                local_id,
                mongo_id,
                owner_id,
                title,
                summary,
                content_path,
                created_at,
                updated_at,
                deleted_at,
                version,
                cloud_version,
                sync_state,
                is_deleted,
                encrypted,
                crypto_meta
             FROM notes
             WHERE local_id = :local_id",
            named_params! {
                ":local_id": note_id,
            },
            |row| {
                Ok(Note {
                    local_id: Uuid::parse_str(&row.get::<_, String>(0)?)
                        .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?,

                    mongo_id: row.get(1)?,

                    owner_id: Uuid::parse_str(&row.get::<_, String>(2)?)
                        .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?,

                    title: row.get(3)?,
                    summary: row.get(4)?,
                    content_path: PathBuf::from(row.get::<_, String>(5)?),

                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                    deleted_at: row.get(8)?,

                    version: row.get(9)?,
                    cloud_version: row.get(10)?,

                    sync_state: row.get(11)?,
                    is_deleted: row.get(12)?,
                    encrypted: row.get(13)?,
                    crypto_meta: row.get(14)?,
                })
            },
        )
        .map_err(|e| {
            tracing::error!(
                task = "get note",
                status = "error",
                %note_id,
                error = ?e,
                "failed to retrieve note"
            );

            crate::errors::Error::NoteNotFound
        })?;

    tracing::debug!(
        task = "get note",
        status = "success",
        %note_id,
        "note loaded successfully"
    );

    Ok(note)
}

/// Reads note content from its local filesystem path.
///
/// The function does not perform encryption or decryption.
///
/// # Errors
///
/// Returns an error if the note file cannot be read.
pub fn get_note_content(note_path: &PathBuf) -> Result<String, crate::errors::Error> {
    tracing::debug!(
        task = "read note content",
        path = %note_path.display(),
        "reading note content from filesystem"
    );

    let content = std::fs::read_to_string(note_path).map_err(|e| {
        tracing::error!(
            task = "read note content",
            status = "error",
            path = %note_path.display(),
            error = ?e,
            "failed to read note content"
        );

        e
    })?;

    tracing::debug!(
        task = "read note content",
        status = "success",
        path = %note_path.display(),
        "note content read successfully"
    );

    Ok(content)
}

/// Checks whether encryption is enabled for a note.
///
/// # Errors
///
/// Returns an error if the encryption state cannot be retrieved.
pub fn check_if_note_is_encrypted(
    note_id: &str,
    notes_db: &Connection,
) -> Result<bool, crate::errors::Error> {
    tracing::debug!(
        task = "check note encryption",
        %note_id,
        "checking note encryption state"
    );

    let is_encrypted: bool = notes_db
        .query_row(
            "SELECT encrypted FROM notes WHERE local_id = :note_id",
            named_params! {
                ":note_id": note_id,
            },
            |row| {
                let value: i64 = row.get(0)?;
                Ok(value != 0)
            },
        )
        .context("failed to check whether note is encrypted")
        .map_err(|e| {
            tracing::error!(
                task = "check note encryption",
                status = "error",
                %note_id,
                error = ?e,
                "failed to check note encryption state"
            );

            e
        })?;

    tracing::debug!(
        task = "check note encryption",
        status = "success",
        %note_id,
        is_encrypted,
        "note encryption state retrieved"
    );

    Ok(is_encrypted)
}

pub fn toggle_note_encryption(
    note_id: String,
    notes_db: &Connection,
    value: bool,
) -> Result<(), crate::errors::Error> {
    let updated_at = crate::utils::get_time();

    notes_db
        .execute(
            "UPDATE notes
             SET
                encrypted = :value,
                updated_at = :updated_at
             WHERE local_id = :note_id",
            named_params! {
                ":value": value,
                ":updated_at": updated_at,
                ":note_id": note_id,
            },
        )
        .context("failed to toggle encryption")?;

    Ok(())
}

pub fn toggle_note_sync(
    note_id: String,
    notes_db: &Connection,
    value: SyncState,
) -> Result<(), crate::errors::Error> {
    match value {
        SyncState::LocalOnly => {
            notes_db
                .execute(
                    "UPDATE notes SET sync_state = 'LocalOnly' WHERE local_id = :note_id",
                    named_params! {
                        ":note_id": note_id,
                    },
                )
                .context("failed to toggle sync state")?;
            notes_db.execute("UPDATE attachments SET sync_state = 'LocalOnly' WHERE note_local_id = :note_id AND sync_state != 'WaitingForTombstone'", named_params! {":note_id": note_id}).context("failed to toggle sync state for attachments")?;
        }

        SyncState::PendingUpload => {
            notes_db
                .execute(
                    "UPDATE notes SET sync_state = 'PendingUpload' WHERE local_id = :note_id",
                    named_params! {
                        ":note_id": note_id,
                    },
                )
                .context("failed to toggle sync state")?;
            notes_db.execute("UPDATE attachments SET sync_state = 'PendingUpload' WHERE note_local_id = :note_id AND sync_state != 'WaitingForTombstone'", named_params! {":note_id": note_id}).context("failed to toggle sync state for attachments")?;
        }

        _ => return Err(crate::errors::Error::FatalError),
    }

    Ok(())
}

pub fn update_title(
    notes_db: &Connection,
    note_id: &str,
    title: String,
) -> Result<(), crate::errors::Error> {
    let updated_at = crate::utils::get_time();

    notes_db
        .execute(
            "UPDATE notes
             SET
                title = :content,
                updated_at = :updated_at
             WHERE local_id = :note_id",
            named_params! {
                ":content": title,
                ":updated_at": updated_at,
                ":note_id": note_id,
            },
        )
        .context("Failed to update note title")?;

    Ok(())
}

pub fn get_title(note_id: &str, notes_db: &Connection) -> Result<String, crate::errors::Error> {
    let data = notes_db
        .query_row(
            "SELECT title FROM notes WHERE local_id = :note_id",
            named_params! {
                ":note_id": note_id,
            },
            |row| {
                let title: String = row.get(0)?;
                Ok(title)
            },
        )
        .context("Failed to get title")?;

    Ok(data)
}

pub fn get_note_struct(
    notes_key: &Key,
    note_id: String,
    notes_db: &Connection,
) -> Result<Note, crate::errors::Error> {
    let row = notes_db
        .query_row(
            "SELECT local_id, mongo_id, owner_id, title, summary, content_path,
                    created_at, updated_at, deleted_at, version, cloud_version,
                    sync_state, is_deleted, encrypted, crypto_meta
             FROM notes
             WHERE local_id = ?1",
            params![note_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, Option<i64>>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, Option<i64>>(10)?,
                    row.get::<_, SyncState>(11)?,
                    row.get::<_, bool>(12)?,
                    row.get::<_, bool>(13)?,
                    row.get::<_, Option<String>>(14)?,
                ))
            },
        )
        .optional()
        .context("Failed to get note from Db")?
        .ok_or(Error::NoteNotFound)?;

    let (
        raw_local_id,
        mongo_id,
        raw_owner_id,
        raw_title,
        summary,
        raw_content_path,
        created_at,
        updated_at,
        deleted_at,
        version,
        cloud_version,
        sync_state,
        is_deleted,
        encrypted,
        raw_crypto_meta,
    ) = row;

    let local_id = Uuid::parse_str(&raw_local_id)
        .map_err(|_| Error::InternalError("local_id is not a valid UUID".into()))?;

    let owner_id = Uuid::parse_str(&raw_owner_id)
        .map_err(|_| Error::InternalError("owner_id is not a valid UUID".into()))?;

    let content_path = PathBuf::from(raw_content_path);

    let title = if encrypted {
        decrypt_title(&note_id, notes_key, notes_db)?
    } else {
        raw_title
    };

    let crypto_meta = raw_crypto_meta.unwrap_or_default();

    Ok(Note {
        local_id,
        mongo_id,
        owner_id,
        title,
        summary,
        content_path,
        created_at,
        updated_at,
        is_deleted,
        deleted_at,
        version,
        cloud_version,
        sync_state,
        encrypted,
        crypto_meta,
    })
}

pub fn remove_note(
    notes_db: &Connection,
    note_id: &str,
    tmp_delete_path: &Path,
) -> Result<(), crate::errors::Error> {
    let tx: rusqlite::Transaction<'_> = notes_db
        .unchecked_transaction()
        .context("failed to start remove note transaction")?;

    let (content_path, is_deleted, sync_state): (String, i64, String) = tx
        .query_row(
            r#"
            SELECT content_path, is_deleted, sync_state
            FROM notes
            WHERE local_id = ?1
            "#,
            params![note_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .context("failed to get note before deletion")
        .map_err(|e| crate::errors::Error::InternalError(e.to_string()))?;

    if is_deleted != 0 {
        return Ok(());
    }

    let new_sync_state = match sync_state.as_str() {
        "LocalOnly" => "LocalOnly",
        _ => "PendingDeleted",
    };

    let source = PathBuf::from(content_path);

    if !source.exists() {
        return Err(crate::errors::Error::FileOperationError(
            "note content path does not exist".to_string(),
        ));
    }

    fs::create_dir_all(tmp_delete_path)
        .context("failed to create tmp_delete directory")
        .map_err(|e| crate::errors::Error::FileOperationError(e.to_string()))?;

    let target = tmp_delete_path.join(format!("{}.{}", note_id, crate::constants::NOTE_EXTENSION));
    fs::rename(&source, &target)
        .context("failed to move note to tmp_delete")
        .map_err(|e| crate::errors::Error::FileOperationError(e.to_string()))?;

    let deleted_at = crate::utils::get_time();
    tx.execute(
        r#"
        UPDATE notes
        SET
            is_deleted = 1,
            deleted_at = ?1,
            sync_state = ?2,
            updated_at = ?1
        WHERE local_id = ?3
        "#,
        params![deleted_at, new_sync_state, note_id],
    )
    .context("failed to mark note as deleted")
    .map_err(|e| crate::errors::Error::InternalError(e.to_string()))?;

    tx.commit()
        .context("failed to commit note deletion")
        .map_err(|e| crate::errors::Error::InternalError(e.to_string()))?;

    crate::utils::log_helper(
        "remove note",
        "success",
        Some(crate::utils::Format::Display(&note_id.to_string())),
        "note moved to tmp_delete and marked as deleted",
    );

    Ok(())
}

pub fn restore_deleted_note(
    notes_db: &Connection,
    tmp_deleted_path: PathBuf,
    notes_path: &PathBuf,
    note_id: &str,
) -> Result<(), crate::errors::Error> {
    let temp_deleted_note_path =
        tmp_deleted_path.join(format!("{}.{}", note_id, crate::constants::NOTE_EXTENSION));
    let target = notes_path.join(format!("{}.{}", note_id, crate::constants::NOTE_EXTENSION));
    // Get current note state.
    let (is_deleted, sync_state): (i64, SyncState) = notes_db
        .query_row(
            r#"
            SELECT is_deleted, sync_state
            FROM notes
            WHERE local_id = :id
            "#,
            named_params! {
                ":id": note_id,
            },
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .context("failed to get note state while restoring note")
        .map_err(|e| crate::errors::Error::InternalError(e.to_string()))?;

    if is_deleted == 0 {
        return Err(crate::errors::Error::InternalError(
            "note is not deleted".to_string(),
        ));
    }

    // A note waiting for a tombstone has already been
    // permanently deleted locally and should not be restorable.
    if sync_state == SyncState::WaitingForTombstone {
        return Err(crate::errors::Error::InternalError(
            "cannot restore note waiting for tombstone".to_string(),
        ));
    }

    if !temp_deleted_note_path.exists() {
        return Err(crate::errors::Error::FileOperationError(
            "deleted note file does not exist in tmp_deleted".to_string(),
        ));
    }

    if target.exists() {
        return Err(crate::errors::Error::FileOperationError(
            "target note file already exists".to_string(),
        ));
    }

    // Restore the file first.
    fs::rename(&temp_deleted_note_path, &target)
        .context("failed to restore note file")
        .map_err(|e| crate::errors::Error::FileOperationError(e.to_string()))?;

    let new_sync_state = match sync_state {
        SyncState::LocalOnly => "LocalOnly",
        _ => "PendingUpload",
    };

    let now: i64 = crate::utils::get_time();

    let tx = notes_db
        .unchecked_transaction()
        .context("failed to start restore note transaction")?;

    tx.execute(
        r#"
        UPDATE notes
        SET
            is_deleted = 0,
            deleted_at = NULL,
            sync_state = :sync_state,
            updated_at = :updated_at
        WHERE local_id = :id
        "#,
        named_params! {
            ":sync_state": new_sync_state,
            ":updated_at": now,
            ":id": note_id,
        },
    )
    .context("failed to restore note state")
    .map_err(|e| crate::errors::Error::InternalError(e.to_string()))?;

    tx.commit()
        .context("failed to commit note restoration")
        .map_err(|e| crate::errors::Error::InternalError(e.to_string()))?;

    crate::utils::log_helper(
        "restore note",
        "success",
        Some(crate::utils::Format::Display(&note_id.to_string())),
        "note restored from tmp_deleted",
    );

    Ok(())
}

pub fn hard_delete_note(
    notes_db: &Connection,
    tmp_deleted_path: &Path,
    note_id: &str,
) -> Result<(), crate::errors::Error> {
    let delete_path =
        tmp_deleted_path.join(format!("{}.{}", note_id, crate::constants::NOTE_EXTENSION));
    if !delete_path.exists() {
        tracing::error!(
            task = "hard delete note",
            status = "error",
            note_id,
            path = ?delete_path,
            "note file does not exist in tmp_deleted"
        );

        return Err(crate::errors::Error::FileOperationError(
            "note does not exist in tmp_deleted".to_string(),
        ));
    }

    let (is_deleted, sync_state): (i64, SyncState) = notes_db
        .query_row(
            r#"
            SELECT is_deleted, sync_state
            FROM notes
            WHERE local_id = :id
            "#,
            named_params! {
                ":id": note_id,
            },
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .context("failed to get note state while hard deleting note")
        .map_err(|e| {
            tracing::error!(
                task = "hard delete note",
                status = "error",
                note_id,
                error = ?e,
                "failed to get note state"
            );

            crate::errors::Error::InternalError(e.to_string())
        })?;

    if is_deleted == 0 {
        tracing::error!(
            task = "hard delete note",
            status = "error",
            note_id,
            "attempted to hard delete a note that is not deleted"
        );

        return Err(crate::errors::Error::InternalError(
            "note is not deleted".to_string(),
        ));
    }
    // TODO make filename in attachments as uuid so there are no problems with privacy
    /*
     * For synchronized notes, keep the database row as an outbox entry
     * for the tombstone.
     *
     * The transaction guarantees that if removing the file fails,
     * WaitingForTombstone is not committed.
     */
    if sync_state == SyncState::PendingDeleted {
        let attachments: Vec<(String, PathBuf)> =
            crate::services::attachment::get_attachments_for_note(notes_db, note_id)?;
        for (_, path) in attachments {
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
        let tx = notes_db
            .unchecked_transaction()
            .context("failed to start hard delete transaction")?;

        tx.execute(
            r#"
            UPDATE notes
            SET sync_state = 'WaitingForTombstone'
            WHERE local_id = :id
            "#,
            named_params! {
                ":id": note_id,
            },
        )
        .context("failed to update sync state after hard delete")
        .map_err(|e| {
            tracing::error!(
                task = "hard delete note",
                status = "error",
                note_id,
                error = ?e,
                "failed to set WaitingForTombstone"
            );

            crate::errors::Error::InternalError(e.to_string())
        })?;

        tx.execute(
            "UPDATE attachments SET sync_state = 'WaitingForTombstone' WHERE note_local_id = :id",
            named_params! {":id": note_id},
        )
        .context("Failed to put attachments to tombstone")?;

        // The file is no longer needed locally.
        fs::remove_file(&delete_path)
            .context("failed to remove note file from tmp_deleted")
            .map_err(|e| {
                tracing::error!(
                    task = "hard delete note",
                    status = "error",
                    note_id,
                    path = ?delete_path,
                    error = ?e,
                    "failed to remove note file; rolling back sync state"
                );

                crate::errors::Error::FileOperationError(e.to_string())
            })?;

        tx.commit()
            .context("failed to commit hard delete transaction")
            .map_err(|e| {
                tracing::error!(
                    task = "hard delete note",
                    status = "error",
                    note_id,
                    error = ?e,
                    "failed to commit WaitingForTombstone state"
                );

                crate::errors::Error::InternalError(e.to_string())
            })?;

        tracing::info!(
            task = "hard delete note",
            status = "success",
            note_id,
            "note file permanently deleted; waiting for tombstone synchronization"
        );
    } else {
        /*
         * LocalOnly means that this deletion does not need to be
         * propagated to the synchronization server.
         *
         * The local database row can therefore be removed immediately.
         */
        fs::remove_file(&delete_path)
            .context("failed to remove note file from tmp_deleted")
            .map_err(|e| {
                tracing::error!(
                    task = "hard delete note",
                    status = "error",
                    note_id,
                    path = ?delete_path,
                    error = ?e,
                    "failed to remove note file"
                );

                crate::errors::Error::FileOperationError(e.to_string())
            })?;

        notes_db
            .execute(
                r#"
                DELETE FROM notes
                WHERE local_id = :id
                "#,
                named_params! {
                    ":id": note_id,
                },
            )
            .context("failed to delete local note row")
            .map_err(|e| {
                tracing::error!(
                    task = "hard delete note",
                    status = "error",
                    note_id,
                    error = ?e,
                    "failed to delete local note row"
                );

                crate::errors::Error::InternalError(e.to_string())
            })?;

        tracing::info!(
            task = "hard delete note",
            status = "success",
            note_id,
            "note permanently deleted locally"
        );
    }

    Ok(())
}

pub fn check_if_note_is_synced(
    note_id: &str,
    notes_db: &Connection,
) -> Result<bool, crate::errors::Error> {
    let sync_status: SyncState = notes_db
        .query_row(
            "SELECT sync_state FROM notes WHERE local_id = :id",
            named_params! {":id": note_id},
            |row| Ok(row.get(0)?),
        )
        .context("Failed to get is synced")?;
    let synced = match sync_status {
        SyncState::LocalOnly => false,
        _ => true,
    };
    Ok(synced)
}
