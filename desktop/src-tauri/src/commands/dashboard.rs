use anyhow::anyhow;
use llava_core::AppState;
#[tauri::command]
pub async fn get_dashboard_data(
    user_uuid: String,
    state: tauri::State<'_, AppState>,
) -> Result<llava_core::stats::DashboardData, llava_core::Error> {
    let users_db_guard: std::sync::MutexGuard<'_, Option<rusqlite::Connection>> = state
        .users_db
        .lock()
        .map_err(|_| anyhow!("error while gettnig users_db from state"))?;

    let users_db = users_db_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;

    let notes_db_guard = state
        .notes_db
        .lock()
        .map_err(|_| anyhow!("Couldnt edit notes db in state"))?;
    let notes_db = notes_db_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;

    return Ok(llava_core::stats::get_dashboard_stats(
        user_uuid, &notes_db, &users_db,
    )?);
}
