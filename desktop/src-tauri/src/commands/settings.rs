use std::collections;

use anyhow::anyhow;
use llava_core::AppState;
use tauri::AppHandle;
use tauri::Emitter;
use zeroize::Zeroize;

#[tauri::command]
pub async fn get_config_data(
    state: tauri::State<'_, AppState>,
) -> Result<(llava_core::settings::UserConfig, bool), llava_core::Error> {
    let paths_guard = state
        .paths
        .lock()
        .map_err(|_| anyhow!("failed to lock AppState.paths"))?;

    let paths: &llava_core::ProgramFiles =
        paths_guard.as_ref().ok_or(llava_core::Error::LockError)?;

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
        paths_guard.as_ref().ok_or(llava_core::Error::LockError)?;
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

#[tauri::command]
pub async fn load_backup_config(
    state: tauri::State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), llava_core::Error> {
    let paths_guard = state
        .paths
        .lock()
        .map_err(|_| anyhow!("failed to lock paths"))?;
    let paths = paths_guard.as_ref().ok_or(llava_core::Error::LockError)?;

    llava_core::settings::load_config_backup(&paths.config_backup_path, &paths.config_path)?;
    let (user_config, _created_default): (llava_core::settings::UserConfig, bool) =
        llava_core::settings::get_config(&paths)?;
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
pub async fn get_logfile_content(
    state: tauri::State<'_, AppState>,
) -> Result<String, llava_core::Error> {
    let paths_guard = state
        .paths
        .lock()
        .map_err(|_| anyhow!("failed to lock paths"))?;
    let paths = paths_guard.as_ref().ok_or(llava_core::Error::LockError)?;
    Ok(llava_core::settings::logfile_contents(&paths.logs_path)?)
}

#[tauri::command]
pub async fn get_recovery_codes(
    state: tauri::State<'_, AppState>,
    mut password: String,
) -> Result<Vec<String>, llava_core::Error> {
    let users_db_lock = state
        .users_db
        .lock()
        .map_err(|_| anyhow!("failed to lock users_db"))?;
    let users_db = users_db_lock.as_ref().ok_or(llava_core::Error::LockError)?;
    let username_lock = state
        .username
        .lock()
        .map_err(|_| anyhow!("failed to lock username"))?;
    let username = username_lock.as_ref().ok_or(llava_core::Error::LockError)?;

    if llava_core::auth::autorization(&username, &password, &users_db)? {
        let codes = llava_core::auth::recovery_code_handling(username, users_db, &password)?;

        password.zeroize();
        Ok(codes)
    } else {
        password.zeroize();
        Err(llava_core::Error::PasswordValidation)
    }
}

#[tauri::command]
pub async fn change_username(
    state: tauri::State<'_, AppState>,
    new_username: String,
) -> Result<(), llava_core::Error> {
    let user_id_guard: std::sync::MutexGuard<'_, Option<uuid::Uuid>> = state
        .current_user
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;
    let user_id = user_id_guard.as_ref().ok_or(llava_core::Error::LockError)?;

    let users_db_lock = state
        .users_db
        .lock()
        .map_err(|_| anyhow!("failed to lock users_db"))?;
    let users_db = users_db_lock.as_ref().ok_or(llava_core::Error::LockError)?;

    llava_core::settings::change_username(&user_id, &new_username, &users_db)?;

    *state
        .username
        .lock()
        .map_err(|_| anyhow!("Couldnt edit username in state"))? = Some(new_username);
    Ok(())
}
