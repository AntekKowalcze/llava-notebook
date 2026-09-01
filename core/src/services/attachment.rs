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

    fs::remove_file(&local_path).context("failed to delete attachment file")?;

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

pub fn toggle_attachments_encryption_for_note(
    notes_db: &rusqlite::Connection,
    value: bool,
    note_id: &str,
) -> Result<(), crate::errors::Error> {
    let updated_at = crate::utils::get_time();
    notes_db.execute("UPDATE attachments SET encrypted = :value, updated_at = :updated_at WHERE note_local_id = :note_id",named_params! {
                ":value": value,
                ":updated_at": updated_at,
                ":note_id": note_id,
            }, ).context("failed to toggle encryption for attachments")?;
    Ok(())
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

pub fn update_attachment_file(
    path: &std::path::Path,
    content: Vec<u8>,
) -> Result<(), crate::errors::Error> {
    std::fs::write(path, content)?;
    Ok(())
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
