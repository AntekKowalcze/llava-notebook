use anyhow::anyhow;
use llava_core::AppState;
use llava_core::stats::PanelData;

#[tauri::command]
pub async fn get_panel_data(
    state: tauri::State<'_, AppState>,
) -> Result<PanelData, llava_core::Error> {
    let user_id_guard = state
        .current_user
        .lock()
        .map_err(|_| anyhow!("Couldnt get current user id"))?;

    let user_id = user_id_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;

    let notes_db_guard = state
        .notes_db
        .lock()
        .map_err(|_| anyhow!("Couldnt access notes db in state"))?;

    let notes_db = notes_db_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;
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
    let stats = llava_core::stats::get_sliding_panel_stats(
        user_id,
        notes_db,
        &notes_key
    )?;

    Ok(stats)
}