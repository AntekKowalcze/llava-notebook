use llava_core::AppState;

#[tauri::command]
pub async fn create_note(
    title: String,
    encryption: bool,
    synchronizing: bool,
    state: tauri::State<'_, AppState>,
) -> Result<(), llava_core::Error> {
    println!("SYNC {:?}", synchronizing);
    if title.trim().is_empty() {
        return Err(llava_core::Error::NoteNameError);
    }

    let user_id = {
        let guard = state
            .current_user
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        *guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?
    };

    let notes_path = {
        let guard = state
            .paths
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?
            .notes_path
            .clone()
    };

    let note = llava_core::storage::create_local_note(
        title,
        encryption,
        synchronizing,
        &user_id,
        &notes_path,
    )
    .await?;

    let mut db_guard = state
        .notes_db
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;

    let db = db_guard
        .as_mut()
        .ok_or(llava_core::Error::LockError)?;

    llava_core::storage::add_note_to_database(db, note)?;

    Ok(())
}