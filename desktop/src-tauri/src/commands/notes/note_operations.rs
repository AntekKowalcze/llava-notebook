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
//! Attachments are never rewritten in place when the encryption state
//! changes: sync treats them as immutable, so each one is copied into a new
//! attachment stored the new way and the note is pointed at the copies (see
//! `switch_note_encryption`). The whole switch runs in one transaction.
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
use std::{collections::HashMap, path::PathBuf, str::FromStr};
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
/// Saves the editor content of a note and, when `next_save_to_encryption`
/// differs from the note's current state, switches its encryption.
///
/// Returns the attachments that were replaced while switching encryption as
/// `old id -> new id`; the editor must rewrite its image links with it. The
/// map is empty for an ordinary save.
#[tauri::command]
pub async fn save_note(
    note_id: String,
    content: String,
    next_save_to_encryption: Option<bool>,
    state: tauri::State<'_, llava_core::AppState>,
) -> Result<HashMap<String, String>, llava_core::Error> {
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

    let notes_db_guard = state
        .notes_db
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;

    let notes_db = notes_db_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;

    if !llava_core::storage::verify_note_owner(&user_id, &note_id, notes_db)? {
        return Err(llava_core::Error::UserIsNotOwner);
    }

    let is_encrypted = llava_core::storage::check_if_note_is_encrypted(&note_id, notes_db)?;
    let encrypt = next_save_to_encryption.unwrap_or(is_encrypted);

    let notes_key = if encrypt || is_encrypted {
        let notes_key_guard = state
            .notes_key
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        Some(
            *notes_key_guard
                .as_ref()
                .ok_or(llava_core::Error::NoKeyToDecryptANote)?,
        )
    } else {
        None
    };

    let replaced = if encrypt == is_encrypted {
        let stored = match notes_key {
            Some(key) if encrypt => {
                llava_core::crypto_operations::encrypt_data(&key, content, notes_db, &note_id)?
            }
            _ => content,
        };
        llava_core::storage::update_md(notes_db, note_id.clone(), stored, &program_paths)?;

        HashMap::new()
    } else {
        let notes_key = notes_key.ok_or(llava_core::Error::NoKeyToDecryptANote)?;
        let replaced = switch_note_encryption(
            notes_db,
            &notes_key,
            &program_paths,
            &note_id,
            content,
            encrypt,
        )?;
        delete_replaced_attachments(notes_db, notes_key, &note_id, replaced.keys());

        replaced
    };

    if let Ok(true) = llava_core::storage::check_if_note_is_synced(&note_id, notes_db) {
        let _ = llava_core::storage::change_sync_to_pending_upload(notes_db, &note_id);
    }

    Ok(replaced)
}

/// Switches a note between encrypted and plaintext storage in one database
/// transaction: a failure leaves the note, its title and its attachments
/// exactly as they were.
///
/// Attachments are not rewritten in place (sync would download the old cloud
/// copy back, and the cloud would keep the old bytes). Every attachment the
/// content references is copied into a new attachment stored the new way, and
/// the content is pointed at the copies.
fn switch_note_encryption(
    notes_db: &rusqlite::Connection,
    notes_key: &chacha20poly1305::Key,
    program_paths: &ProgramFiles,
    note_id: &str,
    content: String,
    encrypt: bool,
) -> Result<HashMap<String, String>, llava_core::Error> {
    let tx = notes_db
        .unchecked_transaction()
        .context("failed to start note encryption transaction")?;
    let mut new_files: Vec<PathBuf> = Vec::new();
    // The note file is rewritten before the commit; keep the old bytes so a
    // failed commit can put them back and file and database stay in step.
    let note_path = program_paths.notes_path.join(format!("{note_id}.md"));
    let previous_file = std::fs::read(&note_path).ok();

    let result = (|| {
        let is_synced = llava_core::storage::check_if_note_is_synced(note_id, &tx)?;
        let mut content = content;
        let mut replaced = HashMap::new();

        for (old_id, _) in llava_core::attachments::get_attachments_for_note(&tx, note_id)? {
            // Attachments the note no longer shows are left for clean_attachments.
            if !content.contains(&old_id) {
                continue;
            }

            let copy = llava_core::attachments::copy_attachment_with_encryption(
                notes_key,
                &program_paths.assets_path,
                &tx,
                &old_id,
                encrypt,
                is_synced,
            )?;
            new_files.push(copy.local_path.clone());

            let new_id = copy.attachment_id.to_string();
            content = content.replace(&old_id, &new_id);
            replaced.insert(old_id, new_id);
        }

        let title = if encrypt {
            let title = llava_core::storage::get_title(note_id, &tx)?;
            llava_core::crypto_operations::encrypt_title(notes_key, note_id, &tx, title)?
        } else {
            llava_core::crypto_operations::decrypt_title(note_id, notes_key, &tx)?
        };
        llava_core::storage::update_title(&tx, note_id, title)?;

        let stored = if encrypt {
            llava_core::crypto_operations::encrypt_data(notes_key, content, &tx, note_id)?
        } else {
            content
        };
        llava_core::storage::toggle_note_encryption(note_id.to_string(), &tx, encrypt)?;
        // Last step: the note file is the only change outside the transaction
        // (restored below if the commit fails).
        llava_core::storage::update_md(&tx, note_id.to_string(), stored, program_paths)?;

        Ok::<_, llava_core::Error>(replaced)
    })();

    match result.and_then(|replaced| {
        tx.commit()
            .context("failed to commit note encryption change")?;
        Ok(replaced)
    }) {
        Ok(replaced) => Ok(replaced),
        Err(err) => {
            // The transaction was rolled back; undo the file changes too.
            for path in new_files {
                let _ = std::fs::remove_file(path);
            }
            match &previous_file {
                Some(bytes) => {
                    let _ = std::fs::write(&note_path, bytes);
                }
                None => {
                    let _ = std::fs::remove_file(&note_path);
                }
            }
            Err(err)
        }
    }
}

/// Deletes the attachments replaced by [`switch_note_encryption`] (locally,
/// and in the cloud through sync), except ones another note still shows.
fn delete_replaced_attachments<'a>(
    notes_db: &rusqlite::Connection,
    notes_key: chacha20poly1305::Key,
    note_id: &str,
    old_ids: impl Iterator<Item = &'a String>,
) {
    let used_elsewhere =
        crate::commands::attachments::clean_attachments::attachment_ids_used_by_other_notes(
            notes_db,
            Some(notes_key),
            note_id,
        );

    for old_id in old_ids {
        if used_elsewhere.contains(old_id) {
            continue;
        }
        if let Err(err) = llava_core::attachments::delete_attachment(notes_db, old_id.clone()) {
            tracing::error!(
                task = "switch note encryption",
                attachment_id = %old_id,
                error = ?err,
                "failed to delete replaced attachment"
            );
        }
    }
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
    let title = llava_core::storage::validate_title(&title)?;

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

    // A rename is an edit too: without this a title-only change is never
    // uploaded (and a newer cloud version would later overwrite it).
    if let Ok(true) = llava_core::storage::check_if_note_is_synced(&note_id, notes_db) {
        let _ = llava_core::storage::change_sync_to_pending_upload(notes_db, &note_id);
    }

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    const NOTE_ID: &str = "4f1d2c3b-5a69-4e78-8f90-a1b2c3d4e5f6";
    const TITLE: &str = "My title";

    struct Fixture {
        dir: PathBuf,
        paths: ProgramFiles,
        db: rusqlite::Connection,
        key: chacha20poly1305::Key,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    fn fixture() -> Fixture {
        let dir = std::env::temp_dir().join(format!("llava-test-{}", uuid::Uuid::new_v4()));
        let paths = ProgramFiles {
            base: dir.clone(),
            data_base_path: dir.join("note.sqlite"),
            notes_path: dir.join("notes"),
            assets_path: dir.join("assets"),
            logs_path: dir.join("logs"),
            config_path: dir.join("config.json"),
            config_backup_path: dir.join("backup.json"),
            tmp_path: dir.join("tmp"),
            delete_tmp_path: dir.join("tmp_delete"),
            local_login_database_path: dir.join("users.sqlite"),
            device_id_path: dir.join("device_id.json"),
            active_user_path: dir.join("active_user.json"),
            app_home: dir.clone(),
        };
        std::fs::create_dir_all(&paths.notes_path).unwrap();
        std::fs::create_dir_all(&paths.assets_path).unwrap();
        let db = llava_core::storage::get_connection(&paths).unwrap();
        db.execute(
            "INSERT INTO notes (local_id, owner_id, title, summary, content_path, created_at,
                                updated_at, sync_state, encrypted, crypto_meta)
             VALUES (?1, 'owner', ?2, 's', ?3, 1, 1, 'Synced', 0,
                     '{\"title_nonce\":\"\",\"summary_nonce\":\"\",\"content_nonce\":\"\"}')",
            params![
                NOTE_ID,
                TITLE,
                paths
                    .notes_path
                    .join(format!("{NOTE_ID}.md"))
                    .to_string_lossy()
                    .to_string()
            ],
        )
        .unwrap();

        Fixture {
            dir,
            paths,
            db,
            key: *chacha20poly1305::Key::from_slice(&[3u8; 32]),
        }
    }

    fn add_image(f: &Fixture) -> String {
        llava_core::attachments::create_attachment(
            &f.key,
            &f.paths.assets_path,
            &f.db,
            NOTE_ID.into(),
            "a.png".into(),
            "image/png".into(),
            false,
            b"image bytes".to_vec(),
            true,
        )
        .unwrap()
        .attachment_id
        .to_string()
    }

    fn note_state(f: &Fixture) -> (bool, String) {
        f.db.query_row(
            "SELECT encrypted, title FROM notes WHERE local_id = ?1",
            params![NOTE_ID],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap()
    }

    fn note_file(f: &Fixture) -> String {
        std::fs::read_to_string(f.paths.notes_path.join(format!("{NOTE_ID}.md"))).unwrap()
    }

    #[test]
    fn switching_encryption_copies_attachments_and_rewrites_links() {
        let f = fixture();
        let original = add_image(&f);
        let content = format!("text\n\n![1.00](attachment://localhost/{original} \"\")\n");

        let replaced =
            switch_note_encryption(&f.db, &f.key, &f.paths, NOTE_ID, content, true).unwrap();
        let encrypted_copy = replaced[&original].clone();

        let (encrypted, title) = note_state(&f);
        assert!(encrypted);
        assert_ne!(title, TITLE, "title must be stored encrypted");
        let stored = note_file(&f);
        assert!(
            !stored.contains("attachment://"),
            "content must be stored encrypted"
        );
        let plain =
            llava_core::crypto_operations::decrypt_note(&f.key, stored, NOTE_ID, &f.db).unwrap();
        assert!(plain.contains(&encrypted_copy) && !plain.contains(&original));
        assert!(
            llava_core::attachments::check_if_attachment_is_encrypted(&encrypted_copy, &f.db)
                .unwrap()
        );

        let replaced =
            switch_note_encryption(&f.db, &f.key, &f.paths, NOTE_ID, plain, false).unwrap();
        let plain_copy = &replaced[&encrypted_copy];

        let (encrypted, title) = note_state(&f);
        assert!(!encrypted);
        assert_eq!(title, TITLE);
        assert!(note_file(&f).contains(plain_copy.as_str()));
        assert_eq!(
            llava_core::attachments::read_attachment(&f.key, &f.db, plain_copy.clone()).unwrap(),
            b"image bytes"
        );
    }

    #[test]
    fn failed_switch_leaves_the_note_unchanged() {
        let f = fixture();
        let original = add_image(&f);
        let (_, path): (String, PathBuf) =
            llava_core::attachments::get_attachments_for_note(&f.db, NOTE_ID)
                .unwrap()
                .remove(0);
        std::fs::remove_file(path).unwrap();
        let content = format!("![1.00](attachment://localhost/{original} \"\")");

        assert!(switch_note_encryption(&f.db, &f.key, &f.paths, NOTE_ID, content, true).is_err());

        assert_eq!(note_state(&f), (false, TITLE.to_string()));
        let attachments: i64 =
            f.db.query_row("SELECT COUNT(*) FROM attachments", [], |r| r.get(0))
                .unwrap();
        assert_eq!(attachments, 1, "no copy may be left behind");
        assert!(!f.paths.notes_path.join(format!("{NOTE_ID}.md")).exists());
        assert_eq!(std::fs::read_dir(&f.paths.assets_path).unwrap().count(), 0);
    }
}
