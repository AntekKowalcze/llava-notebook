//! # Note storage utility module
//!
//! **Purpose**: This module provides database and filesystem helpers for retrieving notes,
//! verifying note ownership, checking encryption state, and reading note content from local
//! storage.
//!
//! ## Exported items
//! * [`verify_note_owner`] — Verifies that a note belongs to the specified user.
//! * [`get_note`] — Loads a complete [`Note`] from the notes database.
//! * [`get_note_content`] — Reads the plaintext or encrypted note content from its local file.
//! * [`check_if_note_is_encrypted`] — Checks whether encryption is enabled for a note.
//!
//! ## Key design decisions
//! Note ownership is verified against the `owner` field stored in the notes database before
//! accessing note data. Database lookup failures for a requested note are mapped to
//! [`crate::errors::Error::NoteNotFound`].
//!
//! Note content is stored in the filesystem rather than directly in SQLite. This module only
//! reads the file; encryption and decryption are handled by the cryptographic module.
//!
//! Note identifiers and user identifiers are logged for diagnostics, but note content and
//! cryptographic material are never included in logs.
//!
//! ## Dependencies
//! - `rusqlite` — Queries the local notes database
//! - `anyhow` — Adds context to database errors
//! - `std::fs` — Reads note content from local storage
//! - `std::path::PathBuf` — Represents note file paths
//! - [`crate::Note`] — Application-level note model
//! - [`crate::errors::Error`] — Application error type
//!
//! # Security considerations
//! Ownership must be verified before returning note data to callers. Callers should perform
//! this check before accessing a note whenever the operation is initiated by an untrusted
//! frontend request.

use rusqlite::{named_params, Connection};
use std::path::PathBuf;

use anyhow::Context;

use crate::Note;

/// Verifies that a note belongs to the specified user.
///
/// # Errors
/// Returns [`crate::errors::Error::NoteNotFound`] if the note does not exist or the ownership
/// query cannot be completed.
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
/// Returns [`crate::errors::Error::NoteNotFound`] if the note does not exist or the database
/// query cannot be completed.
pub fn get_note(
    note_id: &str,
    notes_db: &Connection,
) -> Result<Note, crate::errors::Error> {
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
local_id: uuid::Uuid::parse_str(&row.get::<_, String>(0)?).map_err(|_| rusqlite::Error::QueryReturnedNoRows )?,
mongo_id: row.get(1)?,
                    owner_id: uuid::Uuid::parse_str(&row.get::<_, String>(2)?).map_err(|_| rusqlite::Error::QueryReturnedNoRows )?,

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
/// The function does not perform encryption or decryption. Encrypted content remains encoded
/// until it is passed to the cryptographic layer.
///
/// # Errors
/// Returns an error if the note file cannot be read.
pub fn get_note_content(
    note_path: &PathBuf,
) -> Result<String, crate::errors::Error> {
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
/// Returns an error if the encryption state cannot be retrieved from the database.
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