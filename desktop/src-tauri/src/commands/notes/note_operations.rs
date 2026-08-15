use std::println;



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

        let user_id = user_id_guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?;

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
        println!("USER ID {:?}", &user_id);
println!("note_id {:?}", &note_id);
println!("notes_db {:?}", &notes_db);

        let is_owner = llava_core::storage::verify_note_owner(
            &user_id,
            &note_id,
            notes_db,
        )?;

        if !is_owner {
            return Err(llava_core::Error::UserIsNotOwner);
        }

        llava_core::storage::get_note(&note_id, notes_db)?
    };

    let note_content =
        llava_core::storage::get_note_content(&note.content_path)?;

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
    state: tauri::State<'_, llava_core::AppState>,
) -> Result<(), llava_core::Error> {
    let program_paths = {
        let guard = state
            .paths
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?
            .clone()
    };

    let user_id = {
        let user_id_guard = state
            .current_user
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        let user_id = user_id_guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?;

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

        let is_owner = llava_core::storage::verify_note_owner(
            &user_id,
            &note_id,
            notes_db,
        )?;

        if !is_owner {
            return Err(llava_core::Error::UserIsNotOwner);
        }

        llava_core::storage::check_if_note_is_encrypted(
            &note_id,
            notes_db,
        )?
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

            llava_core::crypto_operations::encrypt_data(
                &notes_key,
                content,
                notes_db,
                &note_id,
            )?
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

        llava_core::storage::update_md(
            notes_db,
            note_id,
            content,
            &program_paths,
        )?;
    }

    Ok(())
}
    // TODO add to plus menu change title + add encryption of the titile when creating a note if encryption is set