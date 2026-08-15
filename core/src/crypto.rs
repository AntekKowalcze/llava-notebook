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
//!
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

use anyhow::Context;
use chacha20poly1305::{aead::OsRng};
use chacha20poly1305::aead::{self, Aead};
use rusqlite::{Connection, named_params};
use chacha20poly1305::{AeadCore, KeyInit, Nonce};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use chacha20poly1305::{ChaCha20Poly1305, Key};

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

    let nonce =
        ChaCha20Poly1305::generate_nonce(&mut aead::OsRng);

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

    let crypto_meta: NoteCryptoMetadata =
        serde_json::from_str(&crypto_meta_json)
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


#[derive(Debug, Serialize, Deserialize)]
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

    crypto_meta.title_nonce =
        BASE64.encode(nonce.as_slice());

    save_crypto_metadata(
        notes_db,
        &crypto_meta,
        note_id,
    )?;

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

    let string_crypto_meta =
        serde_json::to_string(crypto_meta)
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