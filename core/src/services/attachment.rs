//! # Attachment management module
//!
//! **Purpose**: This module is responsible for the local lifecycle of note
//! attachments, including creation, encryption, decryption, deletion,
//! synchronization-state management, metadata handling, and filesystem
//! operations.
//!
//! Attachments are stored as files on the local filesystem while their
//! metadata and synchronization state are maintained in the local SQLite
//! database.
//!
//! ## Exports
//!
//! * [`AttachmentCryptoMetadata`] — Stores cryptographic metadata required to
//!   decrypt an encrypted attachment.
//! * [`create_attachment`] — Creates a new attachment, optionally encrypts its
//!   contents, stores the resulting bytes on disk, and creates its database
//!   record.
//! * [`read_attachment`] — Reads an attachment from disk and decrypts it when
//!   the attachment is encrypted.
//! * [`copy_attachment_with_encryption`] — Copies an attachment into a new
//!   one stored encrypted or as plaintext (used when a note's encryption is
//!   switched; attachments are never rewritten in place).
//! * [`delete_attachment`] — Removes an attachment from local storage and
//!   either deletes its database record or marks it for cloud deletion.
//! * [`check_if_attachment_is_encrypted`] — Retrieves the encryption state of
//!   an attachment from the local database.
//! * [`toggle_attachments_sync_for_note`] — Changes the synchronization state
//!   of attachments belonging to a note when transitioning them away from a
//!   deletion state.
//! * [`get_attachments_for_note`] — Retrieves local attachment identifiers and
//!   filesystem paths for a note.
//! * [`check_attachment_existance`] — Checks whether an attachment exists in
//!   the local database.
//!
//! ## Key design decisions
//!
//! Attachment encryption uses [`ChaCha20Poly1305`] with a freshly generated
//! random nonce for every encrypted attachment. The nonce is stored as
//! Base64-encoded metadata in SQLite so the attachment can be decrypted later.
//!
//! The encryption key is supplied by the caller through [`Key`]. This module
//! does not persist the encryption key and only stores the nonce required for
//! decryption.
//!
//! The checksum and size recorded for an attachment always describe the exact
//! bytes stored on disk. For encrypted attachments this therefore means the
//! encrypted ciphertext, including the authentication tag, rather than the
//! original plaintext. The same representation can consequently be verified
//! after synchronization to and from remote storage.
//!
//! Attachment filenames on disk are based on the generated UUID rather than
//! the original filename. The original filename is retained as metadata while
//! the UUID-based filesystem name avoids collisions and keeps local storage
//! independent of user-provided filenames.
//!
//! Local attachment synchronization is represented through [`SyncState`].
//! Attachments that exist only locally are removed immediately from the
//! database when deleted, while synchronized attachments are retained as
//! tombstones until the cloud synchronization process permanently removes
//! them.
//!
//! Attachment encryption and synchronization state are managed separately.
//! Changing the state of one does not implicitly perform encryption or file
//! rewriting; the higher-level application logic is responsible for any
//! required content transformation.
//!
//! ## Dependencies
//!
//! * [`chacha20poly1305`] — ChaCha20-Poly1305 authenticated encryption and
//!   random nonce generation.
//! * [`base64`] — Encoding and decoding attachment nonces.
//! * [`rusqlite`] — Local SQLite persistence for attachment metadata and
//!   synchronization state.
//! * [`serde`] — Serialization and deserialization of cryptographic metadata.
//! * [`serde_json`] — Encoding cryptographic metadata stored in SQLite.
//! * [`uuid`] — Generation and parsing of attachment and note identifiers.
//! * [`blake3`] — Calculation of checksums for stored attachment bytes.
//! * [`anyhow`] — Adds contextual information to filesystem, database,
//!   encryption, and serialization failures.
//! * [`std::fs`] — Local attachment file creation, reading, writing, and
//!   deletion.
//! * [`std::path`] — Filesystem path handling for local attachment storage.
//! * [`crate::attachments`] — Provides the [`Attachment`] data structure.
//! * [`crate::storage`] — Provides the [`SyncState`] synchronization state.
//! * [`crate::utils`] — Provides timestamps used by attachment records.
//! * [`crate::errors`] — Application-level errors returned by attachment
//!   operations.

use crate::{attachments::Attachment, storage::SyncState};
use anyhow::Context;
use chacha20poly1305::AeadCore;
use chacha20poly1305::{
    ChaCha20Poly1305, Key, KeyInit, Nonce,
    aead::{Aead, OsRng},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AttachmentCryptoMetadata {
    pub attachment_nonce: String,
}
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use rusqlite::{named_params, params};
use std::{fs, path::Path};
use uuid::Uuid;
pub fn create_attachment(
    notes_key: &Key,
    assets_path: &PathBuf,
    notes_db: &rusqlite::Connection,
    note_id: String,
    file_name: String,
    mime_type: String,
    is_note_encrypted: bool,
    file_content: Vec<u8>,
    is_synced: bool,
) -> Result<Attachment, crate::errors::Error> {
    let created_time = crate::utils::get_time();
    let updated_time = created_time;

    let attachment_id = Uuid::new_v4();

    let note_local_id = Uuid::parse_str(&note_id).context("failed to parse note id")?;

    // Bytes that will actually be stored on disk and uploaded to S3.
    let (stored_content, crypto_meta) = if is_note_encrypted {
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

        let cipher = ChaCha20Poly1305::new(notes_key);

        let encrypted_file = cipher
            .encrypt(&nonce, file_content.as_ref())
            .context("failed to encrypt attachment")?;

        let crypto_meta = AttachmentCryptoMetadata {
            attachment_nonce: BASE64.encode(nonce),
        };

        let crypto_meta = serde_json::to_string(&crypto_meta)
            .context("failed to serialize attachment crypto metadata")?;

        (encrypted_file, crypto_meta)
    } else {
        // Store plaintext directly.
        (file_content, String::new())
    };
    // Checksum of the exact bytes stored locally / remotely.
    let checksum: String = blake3::hash(&stored_content).to_hex().to_string();

    // Size of the exact bytes stored locally / remotely.
    let size_bytes = stored_content.len() as i64;

    // assets/<attachment_id>.<extension>
    let extension = Path::new(&file_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .filter(|ext| !ext.is_empty());

    let stored_filename = match extension {
        Some(ext) => format!("{}.{}", attachment_id, ext),
        None => attachment_id.to_string(),
    };

    let local_path = assets_path.join(&stored_filename);

    fs::create_dir_all(assets_path).context("failed to create assets directory")?;

    // Save the exact bytes that will be synced to S3.
    fs::write(&local_path, &stored_content).context("failed to write attachment to disk")?;

    let sync_state = if is_synced {
        crate::services::storage::db_creation::SyncState::PendingUpload
    } else {
        crate::services::storage::db_creation::SyncState::LocalOnly
    };
    notes_db
        .execute(
            r#"
        INSERT INTO attachments (
            attachment_id,
            note_local_id,
            filename,
            mime_type,
            size_bytes,
            local_path,
            cloud_key,
            checksum_encrypted,
            encrypted,
            crypto_meta,
            sync_state,
            created_at,
            updated_at
        )
        VALUES (
            :id,
            :note_id,
            :filename,
            :mime_type,
            :size_bytes,
            :local_path,
            NULL,
            :checksum,
            :encrypted,
            :crypto_meta,
            :sync_state,
            :created_at,
            :updated_at
        )
        "#,
            named_params! {
                ":id": attachment_id.to_string(),
                ":note_id": note_local_id.to_string(),
                ":filename": stored_filename,
                ":mime_type": mime_type,
                ":size_bytes": size_bytes,
                ":local_path": local_path.to_string_lossy().to_string(),
                ":checksum": checksum,
                ":encrypted": is_note_encrypted,
                ":crypto_meta": crypto_meta,
                ":sync_state": sync_state,
                ":created_at": created_time,
                ":updated_at": updated_time,
            },
        )
        .context("failed to insert attachment into database")?;

    Ok(Attachment {
        attachment_id,
        note_local_id,
        filename: file_name,
        mime_type,
        size_bytes,
        local_path,
        cloud_key: None,
        checksum_encrypted: checksum,
        encrypted: is_note_encrypted,
        crypto_meta,
        sync_state,
        created_at: created_time,
        updated_at: updated_time,
    })
}

pub fn read_attachment(
    notes_key: &Key,
    notes_db: &rusqlite::Connection,
    attachment_id: String,
) -> Result<Vec<u8>, crate::errors::Error> {
    let (local_path, encrypted, crypto_meta_json): (String, bool, String) = notes_db
        .query_row(
            r#"
                SELECT
                    local_path,
                    encrypted,
                    crypto_meta
                FROM attachments
                WHERE attachment_id = ?1
                "#,
            params![attachment_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .context("failed to get attachment metadata")?;

    let stored_content = fs::read(&local_path).context("failed to read attachment")?;

    if !encrypted {
        return Ok(stored_content);
    }

    let crypto_meta: AttachmentCryptoMetadata = serde_json::from_str(&crypto_meta_json)
        .context("failed to parse attachment crypto metadata")?;

    let nonce_bytes = BASE64
        .decode(&crypto_meta.attachment_nonce)
        .context("failed to decode attachment nonce")?;

    if nonce_bytes.len() != 12 {
        return Err(anyhow::anyhow!(
            "invalid attachment nonce length: expected 12 bytes, got {}",
            nonce_bytes.len()
        )
        .into());
    }

    let nonce = Nonce::from_slice(&nonce_bytes);

    let cipher = ChaCha20Poly1305::new(notes_key);

    cipher
        .decrypt(nonce, stored_content.as_ref())
        .context("failed to decrypt attachment")
        .map_err(Into::into)
}

pub fn delete_attachment(
    notes_db: &rusqlite::Connection,
    attachment_id: String,
) -> Result<(), crate::errors::Error> {
    let (local_path, sync_state): (String, SyncState) = notes_db
        .query_row(
            r#"
            SELECT local_path, sync_state
            FROM attachments
            WHERE attachment_id = ?1
            "#,
            params![attachment_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .context("failed to get attachment")?;

    // A file that is already gone must not block removing the database entry,
    // otherwise cleanup would fail on the same attachment forever.
    match fs::remove_file(&local_path) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => {
            return Err(anyhow::Error::from(err)
                .context("failed to delete attachment file")
                .into());
        }
    }

    match sync_state {
        SyncState::LocalOnly => {
            notes_db
                .execute(
                    r#"
                    DELETE FROM attachments
                    WHERE attachment_id = ?1
                    "#,
                    params![attachment_id],
                )
                .context("failed to delete attachment from database")?;
        }

        _ => {
            notes_db
                .execute(
                    r#"
                    UPDATE attachments
                    SET sync_state = 'WaitingForTombstone'
                    WHERE attachment_id = ?1
                    "#,
                    params![attachment_id],
                )
                .context("failed to mark attachment for deletion")?;
        }
    }

    Ok(())
}
/// Copies an attachment into a new attachment of the same note, stored
/// encrypted or as plaintext according to `encrypt`.
///
/// Used when a note's encryption is switched. Attachments are never rewritten
/// in place: sync treats them as immutable (a changed checksum makes it
/// download the cloud copy back), so a rewritten file would be reverted and
/// the cloud would keep the old bytes. A copy gets a new id, is uploaded as a
/// new object, and the old attachment can be deleted everywhere.
///
/// # Errors
/// Returns an error if the attachment cannot be read or the copy cannot be
/// stored.
pub fn copy_attachment_with_encryption(
    notes_key: &Key,
    assets_path: &PathBuf,
    notes_db: &rusqlite::Connection,
    attachment_id: &str,
    encrypt: bool,
    is_synced: bool,
) -> Result<Attachment, crate::errors::Error> {
    let (note_id, filename, mime_type): (String, String, String) = notes_db
        .query_row(
            "SELECT note_local_id, filename, mime_type FROM attachments WHERE attachment_id = ?1",
            params![attachment_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .context("failed to get attachment metadata")?;

    let plaintext = read_attachment(notes_key, notes_db, attachment_id.to_string())?;

    create_attachment(
        notes_key,
        assets_path,
        notes_db,
        note_id,
        filename,
        mime_type,
        encrypt,
        plaintext,
        is_synced,
    )
}

pub fn check_if_attachment_is_encrypted(
    attachment_id: &str,
    notes_db: &rusqlite::Connection,
) -> Result<bool, crate::errors::Error> {
    tracing::debug!(
        task = "check note encryption",
        %attachment_id,
        "checking note encryption state"
    );

    let is_encrypted: bool = notes_db
        .query_row(
            "SELECT encrypted FROM attachments WHERE attachment_id = :attachment_id",
            named_params! {
                ":attachment_id": attachment_id,
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
                %attachment_id,
                error = ?e,
                "failed to check note encryption state"
            );

            e
        })?;

    tracing::debug!(
        task = "check note encryption",
        status = "success",
        %attachment_id,
        is_encrypted,
        "note encryption state retrieved"
    );

    Ok(is_encrypted)
}

pub fn toggle_attachments_sync_for_note(
    notes_db: &rusqlite::Connection,
    value: SyncState,
    note_id: &str,
) -> Result<(), crate::errors::Error> {
    match value {
        SyncState::LocalOnly => {
            notes_db
                .execute(
                    "UPDATE attachments SET sync_state = 'LocalOnly' WHERE note_local_id = :note_id AND sync_state = 'WaitingForTombstone'",
                    named_params! {
                        ":note_id": note_id,
                    },
                )
                .context("failed to toggle sync state")?;
        }

        SyncState::PendingUpload => {
            notes_db
                .execute(
                    "UPDATE attachments SET sync_state = 'PendingUpload' WHERE note_local_id = :note_id AND sync_state = 'WaitingForTombstone'",
                    named_params! {
                        ":note_id": note_id,
                    },
                )
                .context("failed to toggle sync state")?;
        }

        _ => return Err(crate::errors::Error::FatalError),
    }

    Ok(())
}

pub fn get_attachments_for_note(
    notes_db: &rusqlite::Connection,
    note_id: &str,
) -> Result<Vec<(String, PathBuf)>, crate::errors::Error> {
    let mut stmt = notes_db
        .prepare(
            "SELECT attachment_id, local_path
             FROM attachments
             WHERE note_local_id = :id
               AND sync_state != 'WaitingForTombstone'
               AND local_path IS NOT NULL",
        )
        .context("Failed to prepare query for getting note attachments")?;

    let mut rows = stmt
        .query(named_params! { ":id": note_id })
        .context("Failed to execute query")?;

    let mut return_vec = Vec::new();

    while let Some(row) = rows.next().context("Failed to get next row")? {
        let attachment_id: String = row.get(0).context("Failed to get attachment_id")?;
        let path: String = row.get(1).context("failed to get local attachment path")?;
        let path: PathBuf = PathBuf::from(path);
        return_vec.push((attachment_id, path));
    }

    Ok(return_vec)
}

pub fn check_attachment_existance(notes_db: &rusqlite::Connection, attachment_id: &str) -> bool {
    notes_db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM attachments WHERE attachment_id = ?1)",
            params![attachment_id],
            |row| row.get::<_, bool>(0),
        )
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::NOTE_DB_SCHEMA;

    const NOTE_ID: &str = "0b9f6a8e-3c0d-4a51-9a52-1d0f2f6a7c11";

    fn setup() -> (rusqlite::Connection, tempfile::TempDir, Key) {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(NOTE_DB_SCHEMA).unwrap();
        conn.execute(
            "INSERT INTO notes (local_id, owner_id, title, summary, content_path, created_at, updated_at)
             VALUES (?1, 'owner', 't', 's', 'p', 1, 1)",
            params![NOTE_ID],
        )
        .unwrap();
        (
            conn,
            tempfile::tempdir().unwrap(),
            *Key::from_slice(&[7u8; 32]),
        )
    }

    fn row(conn: &rusqlite::Connection, id: &str) -> (bool, String, String) {
        conn.query_row(
            "SELECT encrypted, sync_state, local_path FROM attachments WHERE attachment_id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap()
    }

    #[test]
    fn copy_switches_encryption_and_keeps_the_original() {
        let (conn, dir, key) = setup();
        let assets = dir.path().to_path_buf();
        let image = b"\x89PNG fake image bytes".to_vec();

        let original = create_attachment(
            &key,
            &assets,
            &conn,
            NOTE_ID.into(),
            "a.png".into(),
            "image/png".into(),
            false,
            image.clone(),
            true,
        )
        .unwrap();
        let original_id = original.attachment_id.to_string();

        let encrypted =
            copy_attachment_with_encryption(&key, &assets, &conn, &original_id, true, true)
                .unwrap();
        let encrypted_id = encrypted.attachment_id.to_string();

        assert_ne!(encrypted_id, original_id);
        let (is_encrypted, sync_state, path) = row(&conn, &encrypted_id);
        assert!(is_encrypted);
        assert_eq!(sync_state, "PendingUpload");
        assert_ne!(
            std::fs::read(&path).unwrap(),
            image,
            "stored bytes must be ciphertext"
        );
        assert_eq!(
            read_attachment(&key, &conn, encrypted_id.clone()).unwrap(),
            image
        );

        // The original is untouched (it is deleted separately).
        let (original_encrypted, _, original_path) = row(&conn, &original_id);
        assert!(!original_encrypted);
        assert_eq!(std::fs::read(original_path).unwrap(), image);

        let plain =
            copy_attachment_with_encryption(&key, &assets, &conn, &encrypted_id, false, false)
                .unwrap();
        let (plain_encrypted, plain_state, plain_path) =
            row(&conn, &plain.attachment_id.to_string());
        assert!(!plain_encrypted);
        assert_eq!(plain_state, "LocalOnly");
        assert_eq!(std::fs::read(plain_path).unwrap(), image);
    }
}
