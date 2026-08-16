use llava_core::{AppState, tags_handling::find_tag_id};
use anyhow::anyhow;
#[tauri::command]
pub async fn add_tag_to_note (state: tauri::State<'_, AppState>, note_id: String, tag_name: String, tag_color: String) -> Result<(), llava_core::Error> {
 let notes_db_guard = state
        .notes_db
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;

    let notes_db = notes_db_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;
    let user_id_guard = state
        .current_user
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;

    let user_id: &uuid::Uuid = user_id_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;
    let user_id = user_id.to_string();

   let optional_tag_id =  find_tag_id(notes_db, &user_id, &tag_name)?;
   if let Some(tag_id) = optional_tag_id{
    llava_core::tags_handling::add_tag_to_note(notes_db, note_id, tag_id)?;

   }else{
    let new_tag_id: String = llava_core::tags_handling::add_tag_to_database(notes_db, tag_name, tag_color, &user_id)?;
    llava_core::tags_handling::add_tag_to_note(notes_db, note_id, new_tag_id)?;
   }
  Ok(())
}


#[tauri::command]
pub async fn remove_tag_from_note(
    state: tauri::State<'_, AppState>,
    note_id: String,
    tag_name: String,
) -> Result<(), llava_core::Error> {
    let notes_db_guard = state
        .notes_db
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;

    let notes_db = notes_db_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;

    let user_id_guard = state
        .current_user
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;

    let user_id = user_id_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?
        .to_string();

    let tag_id = llava_core::tags_handling::find_tag_id(
        notes_db,
        &user_id,
        &tag_name,
    )?;

    if let Some(tag_id) = tag_id {
        llava_core::tags_handling::remove_tag_from_note(
            notes_db,
            &note_id,
            &tag_id,
        )?;
    }

    Ok(())
}

#[tauri::command]
pub async fn get_all_tags(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<llava_core::tags_handling::UiTag>, llava_core::Error> {
    let notes_db_guard = state
        .notes_db
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;

    let notes_db = notes_db_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;

    let user_id_guard = state
        .current_user
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;

    let user_id = user_id_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?
        .to_string();

    Ok(llava_core::tags_handling::get_all_tags(
        notes_db,
        &user_id,
    )?)
}



#[tauri::command]
pub async fn get_all_tags_for_note(
    state: tauri::State<'_, AppState>,
    note_id: String,
) -> Result<Vec<llava_core::tags_handling::UiTag>, llava_core::Error> {
    let notes_db_guard = state
        .notes_db
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;

    let notes_db = notes_db_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;

    Ok(llava_core::tags_handling::get_all_tags_for_note(
        notes_db,
        &note_id,
    )?)
}