//! # Note command module
//!
//! **Purpose**: This module provides Tauri commands for reading, modifying,
//! synchronizing, encrypting, deleting, and restoring local notes.
//!
//! It acts as the command-layer bridge between the frontend and the core note,
//! storage, attachment, and cryptographic services.
//!
//! ## Exports
//!
//! * [`get_note_content`] — Retrieves the content of a note after verifying
//!   ownership and decrypts it when the note is encrypted.
//! * [`save_note`] — Saves note content locally and optionally changes the
//!   note's encryption state, including reprocessing its title and attachments.
//! * [`toggle_note_sync`] — Changes whether a note participates in cloud
//!   synchronization.
//! * [`get_note_object`] — Retrieves the complete note representation prepared
//!   by the core storage layer.
//! * [`change_note_title`] — Updates a note title and encrypts it when the note
//!   is encrypted.
//! * [`toggle_note_encryption`] — Changes the stored encryption state of a
//!   note.
//! * [`remove_note`] — Soft-deletes a note and moves its local content into
//!   the configured deleted-note storage.
//! * [`hard_delete_note`] — Permanently removes a note and its local data.
//! * [`restore_note`] — Restores a previously soft-deleted note and its local
//!   content.
//!
//! ## Key design decisions
//!
//! Note ownership is verified before reading or modifying note content. The
//! command layer therefore does not rely solely on the caller-provided note
//! identifier when accessing protected local data.
//!
//! Encrypted note content is transparently decrypted for the editor and
//! encrypted again before being persisted. The encryption key itself is stored
//! in application state and is never derived or persisted by this command
//! layer.
//!
//! Note encryption applies to more than the Markdown content. When encryption
//! is enabled or disabled, the note title and all associated attachments are
//! transformed to keep the complete note consistently encrypted or
//! unencrypted.
//!
//! Attachment contents are rewritten when the note encryption state changes.
//! The attachment database metadata is updated alongside the file contents so
//! the synchronization layer can later operate on the correct representation.
//!
//! Synchronization state is delegated to the storage layer. This command
//! module only selects the requested local synchronization state and does not
//! directly communicate with the remote service.
//!
//! Soft deletion and hard deletion are intentionally separate operations.
//! Soft deletion allows the synchronization and recovery workflows to retain
//! the necessary metadata, while hard deletion permanently removes local note
//! data.
//!
//! The command layer acquires only the application-state locks required for a
//! particular operation and passes references to the core services rather than
//! duplicating storage or cryptographic logic.
//!
//! ## Dependencies
//!
//! * [`tauri`] — Tauri commands and access to managed application state.
//! * [`llava_core`] — Core note, storage, attachment, encryption, and error
//!   handling functionality.
//! * [`chacha20poly1305`] — Provides the note encryption key type.
//! * [`anyhow`] — Adds context to UUID parsing and other command-layer
//!   failures.
//! * [`uuid`] — Parsing and formatting note and user identifiers.
//! * [`std::path::PathBuf`] — Represents local attachment paths.
//! * [`crate::commands`] — Provides the application's command-layer module
//!   structure.
//!
//! The actual persistence and cryptographic implementations are intentionally
//! kept in `llava_core`; this module is responsible primarily for application
//! state access, authorization checks, and orchestration.
use anyhow::Context;
use llava_core::{storage::SyncState, Note, ProgramFiles};
use std::{path::PathBuf, str::FromStr};
#[tauri::command]
pub async fn get_note_content(
    note_id: String,
    state: tauri::State<'_, llava_core::AppState>,
) -> Result<String, llava_core::Error> {
    let user_id = {
        let user_id_guard = state
            .current_user
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        let user_id = user_id_guard.as_ref().ok_or(llava_core::Error::LockError)?;

        uuid::Uuid::to_string(user_id)
    };

    let note = {
        let notes_db_guard = state
            .notes_db
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        let notes_db = notes_db_guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?;

        let is_owner = llava_core::storage::verify_note_owner(&user_id, &note_id, notes_db)?;

        if !is_owner {
            return Err(llava_core::Error::UserIsNotOwner);
        }

        llava_core::storage::get_note(&note_id, notes_db)?
    };
    *state
        .current_note
        .lock()
        .map_err(|_| llava_core::Error::LockError)? =
        Some(uuid::Uuid::from_str(&note_id).context("failed to parse uuid")?);
    let mut note_content = llava_core::storage::get_note_content(&note.content_path)?;

    if note.encrypted {
        let notes_key = {
            let notes_key_guard = state
                .notes_key
                .lock()
                .map_err(|_| llava_core::Error::LockError)?;

            *notes_key_guard
                .as_ref()
                .ok_or(llava_core::Error::NoKeyToDecryptANote)?
        };

        let content = {
            let notes_db_guard = state
                .notes_db
                .lock()
                .map_err(|_| llava_core::Error::LockError)?;

            let notes_db = notes_db_guard
                .as_ref()
                .ok_or(llava_core::Error::LockError)?;

            llava_core::crypto_operations::decrypt_note(
                &notes_key,
                note_content,
                &note_id,
                notes_db,
            )?
        };

        note_content = content;
    } 
    note_content = llava_core::storage::resolve_attachment_protocol(&note_content);
    Ok(note_content)
}
#[tauri::command]
pub async fn save_note(
    note_id: String,
    content: String,
    next_save_to_encryption: Option<bool>,
    state: tauri::State<'_, llava_core::AppState>,
) -> Result<(), llava_core::Error> {
    let program_paths = {
        let guard = state
            .paths
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        guard.as_ref().ok_or(llava_core::Error::LockError)?.clone()
    };

    let user_id = {
        let user_id_guard = state
            .current_user
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        let user_id = user_id_guard.as_ref().ok_or(llava_core::Error::LockError)?;

        user_id.to_string()
    };

    let is_encrypted = {
        let notes_db_guard = state
            .notes_db
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        let notes_db = notes_db_guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?;

        let is_owner = llava_core::storage::verify_note_owner(&user_id, &note_id, notes_db)?;

        if !is_owner {
            return Err(llava_core::Error::UserIsNotOwner);
        }
        if let Some(encryption_status) = next_save_to_encryption {
            encryption_status
        } else {
            llava_core::storage::check_if_note_is_encrypted(&note_id, notes_db)?
        }
    };

    let content = if is_encrypted {
        let notes_key = {
            let notes_key_guard = state
                .notes_key
                .lock()
                .map_err(|_| llava_core::Error::LockError)?;

            *notes_key_guard
                .as_ref()
                .ok_or(llava_core::Error::NoKeyToDecryptANote)?
        };

        let encrypted_content = {
            let notes_db_guard = state
                .notes_db
                .lock()
                .map_err(|_| llava_core::Error::LockError)?;

            let notes_db = notes_db_guard
                .as_ref()
                .ok_or(llava_core::Error::LockError)?;

            llava_core::crypto_operations::encrypt_data(&notes_key, content, notes_db, &note_id)?
        };

        encrypted_content
    } else {
        content
    };

    {
        let notes_db_guard = state
            .notes_db
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        let notes_db = notes_db_guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?;

        llava_core::storage::update_md(notes_db, note_id.clone(), content, &program_paths)?;
        if let Some(next_value) = next_save_to_encryption {
            let notes_key = {
                let notes_key_guard = state
                    .notes_key
                    .lock()
                    .map_err(|_| llava_core::Error::NoKeyToDecryptANote)?;

                *notes_key_guard
                    .as_ref()
                    .ok_or(llava_core::Error::NoKeyToDecryptANote)?
            };
            let all_attachments_ids: Vec<(String, PathBuf)> =
                llava_core::attachments::get_attachments_for_note(notes_db, &note_id)?;

            if !next_value {
                let title =
                    llava_core::crypto_operations::decrypt_title(&note_id, &notes_key, notes_db)?;
                llava_core::storage::update_title(notes_db, &note_id, title)?;
                for (id, path) in all_attachments_ids {
                    let attachment = llava_core::crypto_operations::decrypt_attachment(
                        &notes_key, notes_db, id,
                    )?;
                    llava_core::attachments::update_attachment_file(&path, attachment)?;
                }
                llava_core::attachments::toggle_attachments_encryption_for_note(
                    notes_db, false, &note_id,
                )?;
            } else {
                let unencrypted_title = llava_core::storage::get_title(&note_id, notes_db)?;
                let title = llava_core::crypto_operations::encrypt_title(
                    &notes_key,
                    &note_id,
                    notes_db,
                    unencrypted_title,
                )?;
                llava_core::storage::update_title(notes_db, &note_id, title)?;

                for (id, path) in all_attachments_ids {
                    let attachment =
                        llava_core::attachments::read_attachment(&notes_key, notes_db, id.clone())?;
                    let encrypted_attachment = llava_core::crypto_operations::encrypt_attachment(
                        &notes_key,
                        notes_db,
                        &attachment,
                        id,
                    )?;
                    llava_core::attachments::update_attachment_file(&path, encrypted_attachment)?;
                }
                llava_core::attachments::toggle_attachments_encryption_for_note(
                    notes_db, true, &note_id,
                )?;
            }
            llava_core::storage::toggle_note_encryption(note_id.clone(), notes_db, next_value)?;
        }

        let _ = llava_core::storage::change_sync_to_pending_upload(notes_db, &note_id);
    }

    Ok(())
}

#[tauri::command]
pub async fn toggle_note_sync(
    note_id: String,
    state: tauri::State<'_, llava_core::AppState>,
    value: String,
) -> Result<(), llava_core::Error> {
    let guard = state
        .notes_db
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;

    let notes_db = guard.as_ref().ok_or(llava_core::Error::LockError)?;
    if value == "off" {
        llava_core::storage::toggle_note_sync(note_id, notes_db, SyncState::LocalOnly)?;
    } else {
        llava_core::storage::toggle_note_sync(note_id, notes_db, SyncState::PendingUpload)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_note_object(
    note_id: String,
    state: tauri::State<'_, llava_core::AppState>,
) -> Result<Note, llava_core::Error> {
    let guard = state
        .notes_db
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;

    let notes_db = guard.as_ref().ok_or(llava_core::Error::LockError)?;

    let notes_key = {
        let notes_key_guard = state
            .notes_key
            .lock()
            .map_err(|_| llava_core::Error::NoKeyToDecryptANote)?;

        *notes_key_guard
            .as_ref()
            .ok_or(llava_core::Error::NoKeyToDecryptANote)?
    };

    llava_core::storage::get_note_struct(&notes_key, note_id, notes_db)
}

#[tauri::command]
pub async fn change_note_title(
    note_id: String,
    state: tauri::State<'_, llava_core::AppState>,
    title: String,
) -> Result<(), llava_core::Error> {
    let notes_db_guard = state
        .notes_db
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;

    let notes_db = notes_db_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;

    let is_encrypted = llava_core::storage::check_if_note_is_encrypted(&note_id, notes_db)?;
    if is_encrypted {
        let notes_key = {
            let notes_key_guard = state
                .notes_key
                .lock()
                .map_err(|_| llava_core::Error::LockError)?;

            *notes_key_guard
                .as_ref()
                .ok_or(llava_core::Error::NoKeyToDecryptANote)?
        };
        let encrypted_title =
            llava_core::crypto_operations::encrypt_title(&notes_key, &note_id, notes_db, title)?;
        llava_core::storage::update_title(notes_db, &note_id, encrypted_title)?;
    } else {
        llava_core::storage::update_title(notes_db, &note_id, title)?;
    }

    Ok(())
}

#[tauri::command]
pub async fn toggle_note_encryption(
    note_id: String,
    state: tauri::State<'_, llava_core::AppState>,
    value: bool,
) -> Result<(), llava_core::Error> {
    let notes_db_guard = state
        .notes_db
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;

    let notes_db = notes_db_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;

    llava_core::storage::toggle_note_encryption(note_id, notes_db, value)
}

#[tauri::command]
pub fn remove_note(
    note_id: String,
    state: tauri::State<'_, llava_core::AppState>,
) -> Result<(), llava_core::Error> {
    let notes_db_guard = state
        .notes_db
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;

    let notes_db = notes_db_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;

    let paths: ProgramFiles = {
        let guard = state
            .paths
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;
        guard.as_ref().ok_or(llava_core::Error::LockError)?.clone()
    };

    llava_core::storage::remove_note(notes_db, &note_id, &paths.delete_tmp_path)
}

#[tauri::command]
pub fn hard_delete_note(
    note_id: String,
    state: tauri::State<'_, llava_core::AppState>,
) -> Result<(), llava_core::Error> {
    let notes_db_guard = state
        .notes_db
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;

    let notes_db = notes_db_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;
    let program_paths: ProgramFiles = {
        let guard = state
            .paths
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        guard.as_ref().ok_or(llava_core::Error::LockError)?.clone()
    };

    llava_core::storage::hard_delete_note(notes_db, &program_paths.delete_tmp_path, &note_id)?;

    Ok(())
}

#[tauri::command]
pub fn restore_note(
    note_id: String,
    state: tauri::State<'_, llava_core::AppState>,
) -> Result<(), llava_core::Error> {
    let notes_db_guard = state
        .notes_db
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;

    let notes_db = notes_db_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;
    let program_paths: ProgramFiles = {
        let guard = state
            .paths
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        guard.as_ref().ok_or(llava_core::Error::LockError)?.clone()
    };

    llava_core::storage::restore_deleted_note(
        notes_db,
        program_paths.delete_tmp_path,
        &program_paths.notes_path,
        &note_id,
    )?;

    Ok(())
}
