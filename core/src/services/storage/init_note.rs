//! # Local note creation module
//!
//! **Purpose**: This module is responsible for creating local Markdown note files and inserting
//! their metadata into the local SQLite database.
//!
//! ## Exported items
//! * [`create_local_note`] — Creates a new note file, optionally encrypts its title, and builds
//!   the corresponding [`Note`] structure.
//! * [`add_note_to_database`] — Inserts a [`Note`] into SQLite and removes the note file if the
//!   database operation fails.
//!
//! ## Key design decisions
//! Note files are created with `create_new`, preventing accidental overwriting of an existing file.
//!
//! When encryption is enabled, the title is encrypted using ChaCha20-Poly1305 and its nonce is
//! stored in the note's crypto metadata. Sensitive note content is never written to logs.
//!
//! Database insertion is performed inside a SQLite transaction. If the insertion or transaction
//! fails, the previously created note file is removed to prevent orphaned files.
//!
//! ## Dependencies
//! - `rusqlite` — Stores note metadata in SQLite
//! - `chacha20poly1305` — Encrypts note titles
//! - `base64` — Encodes encrypted title data and nonces
//! - `serde_json` — Serialises cryptographic metadata
//! - `anyhow` — Adds context to fallible operations
//! - `uuid` — Generates unique note identifiers
//! - [`crate::constants`] — Provides database and file extension constants
//! - [`crate::crypto`] — Provides [`NoteCryptoMetadata`]
//! - [`crate::services::storage`] — Provides note activity and synchronisation state handling

use crate::Note;
use crate::constants::{INSERT_NOTE_SQL_SCHEMA, NOTE_EXTENSION};
use crate::crypto::NoteCryptoMetadata;

use anyhow::Context;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

use chacha20poly1305::aead::{Aead, OsRng};
use chacha20poly1305::{AeadCore, ChaCha20Poly1305, KeyInit};

use rusqlite::Connection;

use std::path::Path;

/// Creates a new local note file and prepares its database representation.
///
/// If encryption is enabled, the title is encrypted using ChaCha20-Poly1305 and the generated
/// nonce is stored in [`NoteCryptoMetadata`]. The note file itself is initially empty.
///
/// # Errors
/// Returns an error if the note file cannot be created, the title cannot be encrypted, or the
/// cryptographic metadata cannot be serialised.
pub async fn create_local_note(
    mut title: String,
    encryption: bool,
    synchronizing: bool,
    owner_id: &uuid::Uuid,
    path: &Path,
    notes_key: chacha20poly1305::Key,
) -> Result<Note, crate::errors::Error> {
    let id = uuid::Uuid::new_v4();

    tracing::debug!(
        task = "note creation",
        %id,
        %owner_id,
        encryption,
        synchronizing,
        "starting local note creation"
    );

    let mut note_path = path.to_path_buf();
    note_path.push(id.to_string());
    note_path.set_extension(NOTE_EXTENSION);

    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&note_path)
    {
        Ok(_) => {
            tracing::debug!(
                task = "note creation",
                status = "success",
                %id,
                "note file created successfully"
            );

            let now = crate::utils::get_time();

           let crypto_meta = if encryption {
    let cipher = ChaCha20Poly1305::new(&notes_key);

    let title_nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

    let encrypted_title = cipher
        .encrypt(&title_nonce, title.as_bytes())
        .context("failed to encrypt title")
        .map_err(|err| {
            tracing::error!(
                task = "note creation",
                status = "error",
                %id,
                error = ?err,
                "failed to encrypt note title"
            );
            err
        })?;

    title = BASE64.encode(&encrypted_title);

    let content_nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

    let encrypted_content = cipher
        .encrypt(&content_nonce, &[][..])
        .context("failed to encrypt empty note content")
        .map_err(|err| {
            tracing::error!(
                task = "note creation",
                status = "error",
                %id,
                error = ?err,
                "failed to encrypt empty note content"
            );
            err
        })?;

    std::fs::write(&note_path, BASE64.encode(&encrypted_content))
        .map_err(|err| {
            tracing::error!(
                task = "note creation",
                status = "error",
                %id,
                error = ?err,
                "failed to write encrypted note content"
            );

            crate::errors::Error::FileOperationError(err.to_string())
        })?;

    let title_nonce = BASE64.encode(title_nonce.as_slice());
    let content_nonce = BASE64.encode(content_nonce.as_slice());

    NoteCryptoMetadata {
        title_nonce,
        summary_nonce: String::new(),
        content_nonce,
    }
} else {
    NoteCryptoMetadata {
        title_nonce: String::new(),
        summary_nonce: String::new(),
        content_nonce: String::new(),
    }
};

            let string_crypto_meta = serde_json::to_string(&crypto_meta)
                .context("failed to serialize crypto metadata")
                .map_err(|err| {
                    tracing::error!(
                        task = "note creation",
                        status = "error",
                        %id,
                        error = ?err,
                        "failed to serialize crypto metadata"
                    );
                    err
                })?;

            tracing::debug!(
                task = "note creation",
                status = "success",
                %id,
                "local note prepared successfully"
            );

            Ok(Note {
                local_id: id,
                mongo_id: None,
                owner_id: *owner_id,
                title,
                summary: String::new(),
                content_path: note_path,
                created_at: now,
                updated_at: now,
                is_deleted: false,
                deleted_at: None,
                version: 1,
                cloud_version: None,
                sync_state: if synchronizing {
                    crate::services::storage::db_creation::SyncState::PendingUpload
                } else {
                    crate::services::storage::db_creation::SyncState::LocalOnly
                },
                encrypted: encryption,
                crypto_meta: string_crypto_meta,
            })
        }

        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
            tracing::error!(
                task = "note creation",
                status = "error",
                %id,
                error = ?err,
                "UUID collision while creating note"
            );

            Err(crate::errors::Error::FileAlreadyExists)
        }

        Err(err) => {
            tracing::error!(
                task = "note creation",
                status = "error",
                %id,
                error = ?err,
                "failed to create note file"
            );

            Err(crate::errors::Error::FileOperationError(
                err.to_string(),
            ))
        }
    }
}

/// Inserts a note into the local SQLite database.
///
/// The insertion is performed inside a transaction. If the database operation fails, the
/// corresponding Markdown file is removed to prevent an orphaned note file.
///
/// # Errors
/// Returns an error if the SQLite transaction, note insertion, transaction commit, or note
/// activity update fails.
pub fn add_note_to_database(
    notes_db: &mut Connection,
    note: &Note,
) -> Result<(), crate::errors::Error> {
    tracing::debug!(
        task = "database note insert",
        %note.local_id,
        "starting note database insertion"
    );

    let result = (|| {
        let tx = notes_db
            .transaction()
            .context("failed to start database transaction")?;

        tx.execute(
            INSERT_NOTE_SQL_SCHEMA,
            rusqlite::named_params! {
                ":local_id": note.local_id.to_string(),
                ":mongo_id": note.mongo_id,
                ":owner_id": note.owner_id.to_string(),
                ":title": note.title,
                ":summary": note.summary,
                ":content_path": note.content_path.to_string_lossy().to_string(),
                ":created_at": note.created_at,
                ":updated_at": note.updated_at,
                ":deleted_at": note.deleted_at,
                ":version": note.version,
                ":cloud_version": note.cloud_version,
                ":sync_state": note.sync_state,
                ":is_deleted": note.is_deleted,
                ":encrypted": note.encrypted,
                ":crypto_meta": note.crypto_meta,
            },
        )
        .context("failed to insert note")?;

        tx.commit()
            .context("failed to commit note transaction")?;

        Ok::<(), anyhow::Error>(())
    })();

    match result {
        Ok(()) => {
            tracing::debug!(
                task = "database note insert",
                status = "success",
                %note.local_id,
                "note inserted successfully"
            );

            crate::services::storage::note_utils::update_note_activity(
                note.local_id,
                notes_db,
            )?;

            Ok(())
        }

        Err(err) => {
            tracing::error!(
                task = "database note insert",
                status = "error",
                %note.local_id,
                error = ?err,
                "failed to insert note into database"
            );

            if let Err(fs_err) = std::fs::remove_file(&note.content_path) {
                tracing::error!(
                    task = "database note insert",
                    status = "error",
                    %note.local_id,
                    error = ?fs_err,
                    "failed to remove orphan note file"
                );
            } else {
                tracing::debug!(
                    task = "database note insert",
                    %note.local_id,
                    "orphan note file removed"
                );
            }

            Err(crate::errors::Error::InternalError(err.to_string()))
        }
    }
}
// todo check pending upload when sync is on, adding is correct, check also this stats adding how does it work in dashboard