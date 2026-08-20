use llava_core::{storage::SyncState, Note, ProgramFiles};
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

    let note_content = llava_core::storage::get_note_content(&note.content_path)?;

    if note.encrypted {
        let notes_key = {
            let notes_key_guard = state
                .notes_key
                .lock()
                .map_err(|_| llava_core::Error::LockError)?;

            notes_key_guard
                .as_ref()
                .ok_or(llava_core::Error::NoKeyToDecryptANote)?
                .clone()
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

        Ok(content)
    } else {
        Ok(note_content)
    }
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

            notes_key_guard
                .as_ref()
                .ok_or(llava_core::Error::NoKeyToDecryptANote)?
                .clone()
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

                notes_key_guard
                    .as_ref()
                    .ok_or(llava_core::Error::NoKeyToDecryptANote)?
                    .clone()
            };

            if !next_value {
                let title =
                    llava_core::crypto_operations::decrypt_title(&note_id, &notes_key, notes_db)?;
                llava_core::storage::update_title(notes_db, &note_id, title)?;
            } else {
                let unencrypted_title = llava_core::storage::get_title(&note_id, notes_db)?;
                let title = llava_core::crypto_operations::encrypt_title(
                    &notes_key,
                    &note_id,
                    notes_db,
                    unencrypted_title,
                )?;
                llava_core::storage::update_title(notes_db, &note_id, title)?
            }
            llava_core::storage::toggle_note_encryption(note_id, notes_db, next_value)?;
        }
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

        notes_key_guard
            .as_ref()
            .ok_or(llava_core::Error::NoKeyToDecryptANote)?
            .clone()
    };

    Ok(llava_core::storage::get_note_struct(
        &notes_key, note_id, notes_db,
    )?)
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

            notes_key_guard
                .as_ref()
                .ok_or(llava_core::Error::NoKeyToDecryptANote)?
                .clone()
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

    Ok(llava_core::storage::toggle_note_encryption(
        note_id, notes_db, value,
    )?)
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
