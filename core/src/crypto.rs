//! # Note encryption and cryptographic metadata module
//!
//! **Purpose**: This module provides authenticated encryption and decryption for note content
//! and titles using ChaCha20-Poly1305. It also manages the per-note nonces required to decrypt
//! encrypted fields.
//!
//! ## Exported items
//! * [`NoteCryptoMetadata`] — Stores Base64-encoded nonces for encrypted note fields.
//! * [`decrypt_note`] — Decrypts note content using the stored content nonce.
//! * [`encrypt_data`] — Encrypts note content and updates its content nonce in the database.
//! * [`encrypt_title`] — Encrypts a note title and updates its title nonce in the database.
//! * [`update_title_metadata`] — Updates the stored title nonce for a note.
//! * [`decrypt title`] — Decrypts a title
//! ## Key design decisions
//! Each encrypted field uses a newly generated random nonce. Nonces are not secret and are
//! stored as Base64-encoded strings in the note's `crypto_meta` JSON object.
//!
//! Ciphertext is also stored as Base64 because encrypted data is binary and cannot be assumed
//! to contain valid UTF-8.
//!
//! The encryption key is passed by reference to avoid unnecessary copies of key material.
//! The key itself is owned and managed by the application state and is not stored in the
//! note database.
//!
//! ChaCha20-Poly1305 provides authenticated encryption, so decryption fails if the ciphertext
//! or authentication tag has been modified or if an incorrect key or nonce is supplied.
//!
//! ## Dependencies
//! - `chacha20poly1305` — ChaCha20-Poly1305 authenticated encryption and nonce generation
//! - `base64` — Encoding ciphertext and nonces for string and database storage
//! - `serde` / `serde_json` — Serialisation of [`NoteCryptoMetadata`]
//! - `rusqlite` — Reading and updating cryptographic metadata in the notes database
//! - `anyhow` — Adding context to cryptographic, encoding, serialisation, and database errors
//! - `tracing` — Logging encryption, decryption, and metadata operations without logging
//!   sensitive cryptographic material
use std::path::{Path, PathBuf};

use anyhow::Context;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use chacha20poly1305::aead::OsRng;
use chacha20poly1305::aead::{self, Aead};
use chacha20poly1305::{AeadCore, KeyInit, Nonce};
use chacha20poly1305::{ChaCha20Poly1305, Key};
use rusqlite::{Connection, named_params};
use serde::{Deserialize, Serialize};

use crate::services::attachment::AttachmentCryptoMetadata;

pub fn decrypt_note(
    notes_key: &Key,
    content: String,
    note_id: &str,
    notes_db: &Connection,
) -> Result<String, crate::errors::Error> {
    tracing::debug!(
        task = "decrypt note",
        %note_id,
        "starting note decryption"
    );

    let crypto_metadata = get_current_crypto_metadata(note_id, notes_db)?;

    let base64_nonce = crypto_metadata.content_nonce;

    let nonce = BASE64
        .decode(base64_nonce)
        .context("failed to decode nonce for content")
        .map_err(|e| {
            tracing::error!(
                task = "decrypt note",
                status = "error",
                %note_id,
                error = ?e,
                "failed to decode content nonce"
            );
            e
        })?;

    let content = BASE64
        .decode(content)
        .context("failed to decode content from base64")
        .map_err(|e| {
            tracing::error!(
                task = "decrypt note",
                status = "error",
                %note_id,
                error = ?e,
                "failed to decode encrypted content"
            );
            e
        })?;

    let cipher = ChaCha20Poly1305::new(notes_key);
    let nonce = Nonce::from_slice(&nonce);

    let decrypted = cipher
        .decrypt(nonce, content.as_ref())
        .context("failed to decrypt note content")
        .map_err(|e| {
            tracing::error!(
                task = "decrypt note",
                status = "error",
                %note_id,
                error = ?e,
                "failed to decrypt note content"
            );
            e
        })?;

    let decrypted_content = String::from_utf8(decrypted)
        .context("failed to convert decrypted content to UTF-8")
        .map_err(|e| {
            tracing::error!(
                task = "decrypt note",
                status = "error",
                %note_id,
                error = ?e,
                "decrypted content is not valid UTF-8"
            );
            e
        })?;

    tracing::debug!(
        task = "decrypt note",
        status = "success",
        %note_id,
        "note decrypted successfully"
    );

    Ok(decrypted_content)
}

pub fn encrypt_data(
    notes_key: &Key,
    content: String,
    notes_db: &Connection,
    note_id: &str,
) -> Result<String, crate::errors::Error> {
    tracing::debug!(
        task = "encrypt note",
        %note_id,
        "starting note encryption"
    );

    let cipher = ChaCha20Poly1305::new(notes_key);

    let nonce = ChaCha20Poly1305::generate_nonce(&mut aead::OsRng);

    let encrypted_data = cipher
        .encrypt(&nonce, content.as_bytes())
        .context("failed to encrypt note content")
        .map_err(|e| {
            tracing::error!(
                task = "encrypt note",
                status = "error",
                %note_id,
                error = ?e,
                "failed to encrypt note content"
            );
            e
        })?;

    let encrypted_string = BASE64.encode(encrypted_data);
    let nonce_string = BASE64.encode(nonce.as_slice());

    let mut crypto_meta = get_current_crypto_metadata(note_id, notes_db)?;

    crypto_meta.content_nonce = nonce_string;

    save_crypto_metadata(notes_db, &crypto_meta, note_id)?;

    tracing::debug!(
        task = "encrypt note",
        status = "success",
        %note_id,
        "note encrypted successfully"
    );

    Ok(encrypted_string)
}

fn get_current_crypto_metadata(
    note_id: &str,
    notes_db: &Connection,
) -> Result<NoteCryptoMetadata, crate::errors::Error> {
    tracing::debug!(
        task = "get crypto metadata",
        %note_id,
        "fetching crypto metadata"
    );

    let crypto_meta_json: String = notes_db
        .query_row(
            "SELECT crypto_meta FROM notes WHERE local_id = :note_id",
            named_params! {
                ":note_id": note_id,
            },
            |row| row.get::<_, String>(0),
        )
        .context("failed to get crypto_meta")
        .map_err(|e| {
            tracing::error!(
                task = "get crypto metadata",
                status = "error",
                %note_id,
                error = ?e,
                "failed to get crypto metadata"
            );
            e
        })?;

    let crypto_meta: NoteCryptoMetadata = serde_json::from_str(&crypto_meta_json)
        .context("failed to parse crypto_meta")
        .map_err(|e| {
            tracing::error!(
                task = "get crypto metadata",
                status = "error",
                %note_id,
                error = ?e,
                "failed to parse crypto metadata"
            );
            e
        })?;

    tracing::debug!(
        task = "get crypto metadata",
        status = "success",
        %note_id,
        "crypto metadata loaded successfully"
    );

    Ok(crypto_meta)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NoteCryptoMetadata {
    pub title_nonce: String,
    pub summary_nonce: String,
    pub content_nonce: String,
}

pub fn encrypt_title(
    notes_key: &Key,
    note_id: &str,
    notes_db: &Connection,
    content: String,
) -> Result<String, crate::errors::Error> {
    tracing::debug!(
        task = "encrypt title",
        %note_id,
        "starting title encryption"
    );

    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

    let cipher = ChaCha20Poly1305::new(notes_key);

    let encrypted_data = cipher
        .encrypt(&nonce, content.as_bytes())
        .context("failed to encrypt title")
        .map_err(|e| {
            tracing::error!(
                task = "encrypt title",
                status = "error",
                %note_id,
                error = ?e,
                "failed to encrypt title"
            );
            e
        })?;

    update_title_metadata(notes_db, note_id, nonce)?;

    tracing::debug!(
        task = "encrypt title",
        status = "success",
        %note_id,
        "title encrypted successfully"
    );

    Ok(BASE64.encode(encrypted_data))
}

pub fn decrypt_title(
    note_id: &str,
    notes_key: &Key,
    notes_db: &Connection,
) -> Result<String, crate::errors::Error> {
    tracing::debug!(
        task = "decrypt title",
        %note_id,
        "starting title decryption"
    );

    let crypto_metadata = get_current_crypto_metadata(note_id, notes_db)?;

    let base64_nonce = crypto_metadata.title_nonce;

    let nonce = BASE64
        .decode(base64_nonce)
        .context("failed to decode nonce for title")
        .map_err(|e| {
            tracing::error!(
                task = "decrypt title",
                status = "error",
                %note_id,
                error = ?e,
                "failed to decode title nonce"
            );
            e
        })?;
    let title = notes_db
        .query_row(
            "SELECT title FROM notes WHERE local_id = :note_id",
            named_params! {":note_id": note_id},
            |row| {
                let title: String = row.get(0)?;
                Ok(title)
            },
        )
        .context("Failed to get title from database")?;

    let content = BASE64
        .decode(title)
        .context("failed to decode content from base64")
        .map_err(|e| {
            tracing::error!(
                task = "decrypt title",
                status = "error",
                %note_id,
                error = ?e,
                "failed to decode encrypted title"
            );
            e
        })?;

    let cipher = ChaCha20Poly1305::new(notes_key);
    let nonce = Nonce::from_slice(&nonce);

    let decrypted = cipher
        .decrypt(nonce, content.as_ref())
        .context("failed to decrypt note title")
        .map_err(|e| {
            tracing::error!(
                task = "decrypt title",
                status = "error",
                %note_id,
                error = ?e,
                "failed to decrypt note title"
            );
            e
        })?;

    let decrypted_content = String::from_utf8(decrypted)
        .context("failed to convert decrypted title to UTF-8")
        .map_err(|e| {
            tracing::error!(
                task = "decrypt title",
                status = "error",
                %note_id,
                error = ?e,
                "decrypted title is not valid UTF-8"
            );
            e
        })?;

    tracing::debug!(
        task = "decrypt title",
        status = "success",
        %note_id,
        "title decrypted successfully"
    );

    Ok(decrypted_content)
}

pub fn update_title_metadata(
    notes_db: &Connection,
    note_id: &str,
    nonce: Nonce,
) -> Result<(), crate::errors::Error> {
    tracing::debug!(
        task = "update title metadata",
        %note_id,
        "updating title nonce"
    );

    let mut crypto_meta = get_current_crypto_metadata(note_id, notes_db)?;

    crypto_meta.title_nonce = BASE64.encode(nonce.as_slice());

    save_crypto_metadata(notes_db, &crypto_meta, note_id)?;

    tracing::debug!(
        task = "update title metadata",
        status = "success",
        %note_id,
        "title metadata updated successfully"
    );

    Ok(())
}

fn save_crypto_metadata(
    notes_db: &Connection,
    crypto_meta: &NoteCryptoMetadata,
    note_id: &str,
) -> Result<(), crate::errors::Error> {
    tracing::debug!(
        task = "save crypto metadata",
        %note_id,
        "saving crypto metadata"
    );

    let string_crypto_meta = serde_json::to_string(crypto_meta)
        .context("failed to serialize crypto_meta")
        .map_err(|e| {
            tracing::error!(
                task = "save crypto metadata",
                status = "error",
                %note_id,
                error = ?e,
                "failed to serialize crypto metadata"
            );
            e
        })?;

    notes_db
        .execute(
            "UPDATE notes
             SET crypto_meta = :crypto_meta
             WHERE local_id = :note_id",
            named_params! {
                ":crypto_meta": string_crypto_meta,
                ":note_id": note_id,
            },
        )
        .context("failed to update crypto_meta")
        .map_err(|e| {
            tracing::error!(
                task = "save crypto metadata",
                status = "error",
                %note_id,
                error = ?e,
                "failed to update crypto metadata"
            );
            e
        })?;

    tracing::debug!(
        task = "save crypto metadata",
        status = "success",
        %note_id,
        "crypto metadata saved successfully"
    );

    Ok(())
}

pub fn encrypt_attachment(
    notes_key: &Key,
    notes_db: &rusqlite::Connection,
    file_content: &[u8],
    attachment_id: String,
) -> Result<Vec<u8>, crate::errors::Error> {
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

    let cipher = ChaCha20Poly1305::new(notes_key);

    let encrypted_file = cipher
        .encrypt(&nonce, file_content)
        .context("failed to encrypt attachment")?;

    let crypto_meta = AttachmentCryptoMetadata {
        attachment_nonce: BASE64.encode(nonce),
    };
    let encrypted_checksum = blake3::hash(&encrypted_file);

    let crypto_meta_json = serde_json::to_string(&crypto_meta)
        .context("failed to serialize attachment crypto metadata")?;
    let size_bytes = encrypted_file.len() as i64;
    notes_db
        .execute(
            r#"
            UPDATE attachments
            SET
                crypto_meta = ?1,
                encrypted = 1,
                updated_at = ?2,
                checksum_encrypted = ?3,
                size_bytes = ?4

            WHERE attachment_id = ?5
            "#,
            rusqlite::params![
                crypto_meta_json,
                crate::utils::get_time(),
                encrypted_checksum.to_string(),
                size_bytes,
                attachment_id
            ],
        )
        .context("failed to update attachment crypto metadata")?;

    Ok(encrypted_file)
}

pub fn decrypt_attachment(
    notes_key: &Key,
    notes_db: &rusqlite::Connection,
    attachment_id: String,
) -> Result<Vec<u8>, crate::errors::Error> {
    let (local_path, crypto_meta_json, encrypted): (String, String, bool) = notes_db
        .query_row(
            r#"
            SELECT
                local_path,
                crypto_meta,
                encrypted
            FROM attachments
            WHERE attachment_id = ?1
            "#,
            rusqlite::params![attachment_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .context("failed to get attachment metadata")?;

    if !encrypted {
        return Err(anyhow::anyhow!("attachment is not encrypted").into());
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

    let encrypted_file =
        std::fs::read(&local_path).context("failed to read encrypted attachment")?;

    let cipher = ChaCha20Poly1305::new(notes_key);

    let decrypted_file = cipher
        .decrypt(nonce, encrypted_file.as_ref())
        .context("failed to decrypt attachment")?;
    let decrypted_checksum = blake3::hash(&decrypted_file);
    let size_bytes = decrypted_file.len() as i64;

    notes_db
        .execute(
            "UPDATE attachments SET checksum_encrypted = ?1, size_bytes = ?2 WHERE attachment_id = ?3",
            rusqlite::params![decrypted_checksum.to_string(), size_bytes ,attachment_id],
        )
        .context("failed to update checksum in db")?;
    Ok(decrypted_file)
}

pub fn reencrypt_db(
    notes_db: &Connection,
    old_key: &Key,
    new_key: &Key,
) -> Result<(), crate::errors::Error> {
    validate_reencrypt_db(notes_db)?;
    let mut stmt = notes_db
        .prepare("SELECT local_id, content_path FROM notes WHERE encrypted = 1;")
        .context("Failed to prepare statement")?;
    let mut handle = stmt
        .query(rusqlite::params![])
        .context("Failed to get encrypted notes")?;
    while let Some(row) = handle.next().context("failed to get next row")? {
        let id: String = row.get(0).context("failed to get next id")?;
        let path: String = row.get(1).context("failed to get content path")?;
        let path = PathBuf::from(path);
        //DO not changing updated at  here it would provide mess
        let mut new_stmt = notes_db.prepare("SELECT attachment_id, local_path FROM attachments WHERE encrypted = 1 AND note_local_id = ?1").context("Failed to prepare attachemtn statemetn")?;
        let mut attachment_handle = new_stmt
            .query(rusqlite::params![&id])
            .context("failed to get attachment handle")?;

        let current_saved_content = crate::storage::get_note_content(&path)?; //encrypted content
        let decrypted_content = decrypt_note(old_key, current_saved_content, &id, notes_db)?;

        let decrypted_title = decrypt_title(&id, old_key, notes_db)?;

        let encrypted_content = encrypt_data(new_key, decrypted_content, notes_db, &id)?;
        let encrypted_title = encrypt_title(new_key, &id, notes_db, decrypted_title)?;
        atomic_write(&path, &encrypted_content.as_bytes())?;
        notes_db
            .execute(
                "UPDATE notes SET title = ?1 WHERE local_id = ?2",
                rusqlite::params![encrypted_title, &id],
            )
            .context("Failed to update title")?;

        while let Some(attachment_row) = attachment_handle
            .next()
            .context("failed to get next attachment row")?
        {
            let attachment_id: String = attachment_row.get(0).context("failed to get next id")?;
            let attachment_path: String = attachment_row
                .get(1)
                .context("failed to get content path")?;
            let attachment_path: PathBuf = PathBuf::from(attachment_path);
            let decypted_attachment = decrypt_attachment(old_key, notes_db, attachment_id.clone())?;
            let encrypted_with_new_key =
            encrypt_attachment(new_key, notes_db, &decypted_attachment, attachment_id)?;
            atomic_write(&attachment_path, &encrypted_with_new_key)?;
        }
    }
    Ok(())
}

fn atomic_write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let tmp_path = path.with_extension("rekey_tmp");

    std::fs::write(&tmp_path, data)?;
    std::fs::rename(&tmp_path, path)?;

    Ok(())
}

fn validate_reencrypt_db(notes_db: &Connection) -> Result<(), crate::errors::Error> {
    // Validate encrypted notes.
    let mut stmt = notes_db
        .prepare(
            r#"
            SELECT local_id, content_path, crypto_meta
            FROM notes
            WHERE encrypted = 1
            "#,
        )
        .context("failed to prepare encrypted notes validation query")?;

    let mut rows = stmt.query([]).context("failed to query encrypted notes")?;

    while let Some(row) = rows.next().context("failed to get next encrypted note")? {
        let note_id: String = row.get(0).context("failed to get note id")?;

        let content_path: String = row.get(1).context("failed to get note content path")?;

        let crypto_meta_json: String = row.get(2).context("failed to get note crypto metadata")?;

        let content_path = PathBuf::from(&content_path);

        if !content_path.is_file() {
            return Err(anyhow::anyhow!(
                "encrypted content file for note {} does not exist: {}",
                note_id,
                content_path.display()
            )
            .into());
        }

        let crypto_meta: NoteCryptoMetadata = serde_json::from_str(&crypto_meta_json)
            .with_context(|| format!("failed to parse crypto metadata for note {}", note_id))?;

        validate_nonce(&crypto_meta.content_nonce, "content", &note_id)?;
        validate_nonce(&crypto_meta.title_nonce, "title", &note_id)?;
    }

    let mut stmt = notes_db
        .prepare(
            r#"
            SELECT attachment_id, local_path, crypto_meta
            FROM attachments
            WHERE encrypted = 1
            "#,
        )
        .context("failed to prepare encrypted attachments validation query")?;

    let mut rows = stmt
        .query([])
        .context("failed to query encrypted attachments")?;

    while let Some(row) = rows
        .next()
        .context("failed to get next encrypted attachment")?
    {
        let attachment_id: String = row.get(0).context("failed to get attachment id")?;

        let local_path: String = row.get(1).context("failed to get attachment path")?;

        let crypto_meta_json: String = row
            .get(2)
            .context("failed to get attachment crypto metadata")?;

        let local_path = PathBuf::from(&local_path);

        if !local_path.is_file() {
            return Err(anyhow::anyhow!(
                "encrypted attachment {} does not exist: {}",
                attachment_id,
                local_path.display()
            )
            .into());
        }

        let crypto_meta: AttachmentCryptoMetadata = serde_json::from_str(&crypto_meta_json)
            .with_context(|| {
                format!(
                    "failed to parse crypto metadata for attachment {}",
                    attachment_id
                )
            })?;

        validate_nonce(&crypto_meta.attachment_nonce, "attachment", &attachment_id)?;
    }

    Ok(())
}

fn validate_nonce(nonce_base64: &str, field: &str, id: &str) -> Result<(), crate::errors::Error> {
    let nonce = BASE64
        .decode(nonce_base64)
        .with_context(|| format!("failed to decode {} nonce for {} {}", field, field, id))?;

    if nonce.len() != 12 {
        return Err(anyhow::anyhow!(
            "invalid {} nonce length for {}: expected 12 bytes, got {}",
            field,
            id,
            nonce.len()
        )
        .into());
    }

    Ok(())
}
