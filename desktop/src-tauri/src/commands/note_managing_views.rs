use llava_core::{note_stats::NoteCard, AppState};

/// Retrieves metadata for all active notes belonging to the current user.
///
/// The command obtains the authenticated user identifier, local notes
/// database, and note encryption key from [`AppState`] and delegates the
/// database query and title decryption to [`llava_core::note_stats::get_all_notes_data`].
///
/// # Errors
///
/// Returns [`llava_core::Error::LockError`] when any required application
/// state mutex cannot be acquired or when the corresponding state value is
/// not initialized.
///
/// Returns [`llava_core::Error::NoKeyToDecryptANote`] when the note encryption
/// key required to decrypt encrypted note titles is unavailable.
///
/// Returns any error produced by
/// [`llava_core::note_stats::get_all_notes_data`].
#[tauri::command]
pub async fn get_all_notes_data(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<NoteCard>, llava_core::Error> {
    let user_uuid = {
        let user_uuid_guard = state
            .current_user
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;
        *user_uuid_guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?
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

        *notes_key_guard
            .as_ref()
            .ok_or(llava_core::Error::NoKeyToDecryptANote)?
    };

    llava_core::note_stats::get_all_notes_data(notes_db, &user_uuid, &notes_key)
}

/// Retrieves recently removed notes belonging to the current user.
///
/// The command obtains the authenticated user identifier, local notes
/// database, and note encryption key from [`AppState`] and delegates retrieval
/// and title decryption to [`llava_core::note_stats::get_all_removed_notes_data`].
///
/// # Errors
///
/// Returns [`llava_core::Error::LockError`] when any required application
/// state mutex cannot be acquired or when the corresponding state value is
/// not initialized.
///
/// Returns [`llava_core::Error::NoKeyToDecryptANote`] when the note encryption
/// key required to decrypt encrypted note titles is unavailable.
///
/// Returns any error produced by
/// [`llava_core::note_stats::get_all_removed_notes_data`].
#[tauri::command]
pub async fn get_all_removed_notes_data(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<llava_core::note_stats::RemovedNote>, llava_core::Error> {
    let user_uuid = {
        let user_uuid_guard = state
            .current_user
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;
        *user_uuid_guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?
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

        *notes_key_guard
            .as_ref()
            .ok_or(llava_core::Error::NoKeyToDecryptANote)?
    };
    let removed_notes: Vec<llava_core::note_stats::RemovedNote> =
        llava_core::note_stats::get_all_removed_notes_data(notes_db, user_uuid, &notes_key)?;

    Ok(removed_notes)
}
