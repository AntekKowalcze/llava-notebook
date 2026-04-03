use anyhow::{anyhow, Context};
use llava_core::local_auth::SessionState;
use llava_core::AppState;
use tauri::AppHandle;
use tauri::Emitter;
#[tauri::command]
pub async fn register_command(
    username: String,
    password: String,
    password_repeated: String,
    state: tauri::State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(Vec<String>, String), llava_core::Error> {
    let (new_uuid, new_paths, users_db, codes) = {
        let mut conn_guard = state
            .users_db
            .lock()
            .map_err(|_| anyhow!("failed to lock users_db"))?;
        let paths_guard = state
            .paths
            .lock()
            .map_err(|_| anyhow!("failed to lock paths"))?;

        let paths = paths_guard.as_ref().ok_or(llava_core::Error::LockError)?;
        let users_db = conn_guard.as_mut().ok_or(llava_core::Error::LockError)?;

        crate::commands::handlers::local_auth::register(
            username.clone(),
            password,
            password_repeated,
            paths,
            users_db,
        )?
    };

    let user_config = llava_core::settings::get_config_for_state(&new_paths)?;

    app_handle
        .emit("config-updated", &user_config)
        .map_err(|_| llava_core::Error::FatalError)?;

    crate::commands::command_helpers::change_state_after_login(
        &state,
        new_uuid,
        users_db,
        new_paths,
        username,
        user_config,
    )?;

    Ok((codes, new_uuid.to_string()))
}

// commands/auth.rs

#[tauri::command]
pub async fn login_command(
    username: String,
    password: String,
    state: tauri::State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<String, llava_core::Error> {
    let (new_uuid, new_paths, notes_conn) = {
        let mut conn_guard = state
            .users_db
            .lock()
            .map_err(|_| anyhow!("failed to lock users_db"))?;
        let paths_guard = state
            .paths
            .lock()
            .map_err(|_| anyhow!("failed to lock paths"))?;

        let users_db = conn_guard.as_mut().ok_or(llava_core::Error::LockError)?;
        let paths = paths_guard.as_ref().ok_or(llava_core::Error::LockError)?;

        crate::commands::handlers::local_auth::login(username.clone(), password, paths, users_db)?
    };

    {
        let mut conn_guard = state
            .users_db
            .lock()
            .map_err(|_| anyhow!("failed to lock users_db"))?;
        let users_db = conn_guard.as_mut().ok_or(llava_core::Error::LockError)?;
        llava_core::local_auth::zero_error_count(users_db, &new_uuid)?;
    }

    let user_config = llava_core::settings::get_config_for_state(&new_paths)?;

    app_handle
        .emit("config-updated", &user_config)
        .map_err(|_| llava_core::Error::FatalError)?;

    crate::commands::command_helpers::change_state_after_login(
        &state,
        new_uuid,
        notes_conn,
        new_paths,
        username,
        user_config,
    )?;

    Ok(new_uuid.to_string())
}

#[tauri::command]
pub async fn check_if_user_exists(
    state: tauri::State<'_, AppState>,
) -> Result<bool, llava_core::Error> {
    let mut conn_guard = state
        .users_db
        .lock()
        .map_err(|_| anyhow!("Failed to lock AppState.paths"))?;
    let users_db: &mut rusqlite::Connection =
        conn_guard.as_mut().ok_or(llava_core::Error::LockError)?;
    llava_core::local_auth::check_if_first_start(users_db)
}

#[tauri::command]
pub async fn log_with_code(
    mut code: String,
    username: String,
    state: tauri::State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(String, bool), llava_core::Error> {
    code.retain(|c| c != '-');

    let (user_uuid, paths, notes_conn, one_code) = {
        let users_db_guard = state
            .users_db
            .lock()
            .map_err(|_| anyhow!("failed to lock users_db"))?;
        let paths_guard = state
            .paths
            .lock()
            .map_err(|_| anyhow!("failed to lock paths"))?;

        let users_db = users_db_guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?;
        let paths = paths_guard.as_ref().ok_or(llava_core::Error::LockError)?;

        let user_uuid = llava_core::get_user_uuid(users_db, &username)?;
        let (paths, notes_conn, one_code) =
            crate::commands::handlers::local_auth::log_with_code(code, &username, paths, users_db)?;

        (user_uuid, paths, notes_conn, one_code)
    };

    let user_config = llava_core::settings::get_config_for_state(&paths)?;

    app_handle
        .emit("config-updated", &user_config)
        .map_err(|_| llava_core::Error::FatalError)?;

    crate::commands::command_helpers::change_state_after_login(
        &state,
        user_uuid,
        notes_conn,
        paths,
        username,
        user_config,
    )?;

    Ok((user_uuid.to_string(), one_code))
}

#[tauri::command]
pub async fn check_timeout_before_submit(
    username: String,
    state: tauri::State<'_, AppState>,
) -> Result<i64, llava_core::Error> {
    let conn_guard = state
        .users_db
        .lock()
        .map_err(|_| anyhow!("failed to lock users_db"))?;
    let users_db = conn_guard.as_ref().ok_or(llava_core::Error::LockError)?;

    crate::commands::handlers::local_auth::check_timeout(&username, users_db)
}
#[tauri::command]
pub async fn change_password(
    username: String,
    password: String,
    password_repeated: String,
    mut code: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), llava_core::Error> {
    code.retain(|c| c != '-');
    let user_db_guard = state
        .users_db
        .lock()
        .map_err(|_| anyhow!("failed to get users_db from state"))?;
    let user_db = user_db_guard.as_ref().ok_or(llava_core::Error::LockError)?;
    llava_core::local_auth::change_password(user_db, username, password, password_repeated, code)?;
    Ok(())
}

#[tauri::command]
pub async fn check_login_on_start(
    app_handle: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<SessionState, llava_core::Error> {
    let user_db_guard = state
        .users_db
        .lock()
        .map_err(|_| anyhow!("Couldnt get user_db guard"))?;
    let users_db = user_db_guard.as_ref().ok_or(llava_core::Error::LockError)?;
    let program_files = {
        let program_files_guard = state
            .paths
            .lock()
            .map_err(|_| anyhow!("Couldnt get program filesguard"))?;
        program_files_guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?
            .clone()
    };
    let is_logged_in: SessionState =
        llava_core::local_auth::check_if_user_logged_in(users_db, &program_files)?;

    if let SessionState::LoggedIn { user_id } = &is_logged_in {
        let parsed_user_uuid =
            uuid::Uuid::parse_str(&user_id).context("Failed to parse user_id to string")?;

        let updated_paths =
            llava_core::get_paths(program_files.app_home.clone(), &parsed_user_uuid)?;

        let notes_db = llava_core::storage::get_connection(&updated_paths)?;

        let username = llava_core::get_username_from_uuid(users_db, user_id.clone())?;
        let user_config = llava_core::settings::get_config_for_state(&updated_paths)?;
        app_handle
            .emit("config-updated", &user_config)
            .map_err(|_| llava_core::Error::FatalError)?;

        crate::commands::command_helpers::change_state_after_login(
            &state,
            parsed_user_uuid,
            notes_db,
            updated_paths.clone(),
            username,
            user_config,
        )?;
    }
    println!("{:#?}", state);
    Ok(is_logged_in)
}

#[tauri::command]
pub async fn local_logout_command(
    user_uuid: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), llava_core::Error> {
    *state
        .current_user
        .lock()
        .map_err(|_| anyhow!("couldnt edit current user"))? = None;
    *state
        .notes_db
        .lock()
        .map_err(|_| anyhow!("Couldnt edit notes db in state"))? = None;

    *state
        .username
        .lock()
        .map_err(|_| anyhow!("Couldnt edit username in state"))? = None;
    let users_db_guard = state
        .users_db
        .lock()
        .map_err(|_| anyhow!("Cannot lock state"))?;
    let users_db = users_db_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;
    let paths_guard = state
        .paths
        .lock()
        .map_err(|_| anyhow!("Cannot lock state"))?;
    let paths = paths_guard.as_ref().ok_or(llava_core::Error::LockError)?;
    llava_core::local_auth::local_logout(user_uuid, users_db, paths)?;
    Ok(())
}
