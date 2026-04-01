use anyhow::anyhow;
use llava_core::AppState;

#[tauri::command]
pub async fn get_username_from_uuid(
    user_uuid: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, llava_core::Error> {
    let users_db_guard = state
        .users_db
        .lock()
        .map_err(|_| anyhow!("error while gettnig users_db from state"))?;
    let users_db = users_db_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;
    let username = llava_core::get_username_from_uuid(users_db, user_uuid)?;
    Ok(username)
}
