use std::alloc::alloc;

use llava_core::{note_stats::NoteCard, AppState};

#[tauri::command]
pub async fn get_all_notes_data(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<NoteCard>, llava_core::Error> {
    let user_uuid = {
        let user_uuid_guard = state
            .current_user
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;
        user_uuid_guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?
            .clone()
    };

    let notes_db_guard = state
        .notes_db
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;
    let notes_db = notes_db_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;

    let user_uuid: String = user_uuid.to_string();

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

    Ok(llava_core::note_stats::get_all_notes_data(
        notes_db, &user_uuid, &notes_key,
    )?)
}

#[tauri::command]
pub async fn get_all_removed_notes_data(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<llava_core::note_stats::RemovedNote>, llava_core::Error> {
    let user_uuid = {
        let user_uuid_guard = state
            .current_user
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;
        user_uuid_guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?
            .clone()
    };

    let notes_db_guard = state
        .notes_db
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;
    let notes_db = notes_db_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;

    let user_uuid: String = user_uuid.to_string();

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
    let removed_notes: Vec<llava_core::note_stats::RemovedNote> =
        llava_core::note_stats::get_all_removed_notes_data(notes_db, user_uuid, &notes_key)?;

    Ok(removed_notes)
}
