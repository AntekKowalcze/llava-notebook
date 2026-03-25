use std::collections;

use anyhow::anyhow;
use llava_core::AppState;
use tauri::AppHandle;
use tauri::Emitter;

#[tauri::command]
pub async fn get_config_data(
    state: tauri::State<'_, AppState>,
) -> Result<(llava_core::settings::UserConfig, bool), llava_core::Error> {
    let paths_guard = state
        .paths
        .lock()
        .map_err(|_| anyhow!("failed to lock AppState.paths"))?;

    let paths: &llava_core::ProgramFiles =
        paths_guard.as_ref().ok_or(llava_core::Error::FatalError)?;

    let (user_config, created_default): (llava_core::settings::UserConfig, bool) =
        llava_core::settings::get_config(&paths)?;
    Ok((user_config, created_default))
}

#[tauri::command]
pub async fn update_settings(
    user_config: llava_core::settings::UserConfig,
    state: tauri::State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), llava_core::Error> {
    let paths_guard = state
        .paths
        .lock()
        .map_err(|_| anyhow!("failed to lock AppState.paths"))?;

    let paths: &llava_core::ProgramFiles =
        paths_guard.as_ref().ok_or(llava_core::Error::FatalError)?;
    let hash_config = llava_core::settings::save_config(
        &user_config,
        paths.config_path.clone(),
        paths.config_backup_path.clone(),
    )?;
    app_handle
        .emit("config-updated", &hash_config)
        .map_err(|_| llava_core::Error::FatalError)?;
    *state
        .user_config
        .lock()
        .map_err(|_| anyhow!("Couldnt edit user_config in state"))? = Some(hash_config);
    Ok(())
}

#[tauri::command]
pub async fn get_methapone_map() -> collections::HashMap<String, Vec<String>> {
    llava_core::settings::create_metaphone_map()
}
