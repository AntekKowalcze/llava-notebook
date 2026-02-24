use anyhow::anyhow;
use llava_core::{services::auth::logging::SessionState, AppState};
use zeroize::Zeroizing;

#[tauri::command]
pub async fn register_command(
    username: String,
    password: String,
    password_repeated: String,
    state: tauri::State<'_, AppState>,
) -> Result<(Vec<String>, String), llava_core::Error> {
    let password_zeroized = Zeroizing::from(password);
    let password_repeated_zeroized = Zeroizing::from(password_repeated);

    let (new_uuid, new_paths, conn, codes) = {
        let mut conn_guard = state
            .users_db
            .lock()
            .map_err(|_| anyhow!("failed to lock AppState.connection"))?;

        let paths_guard = state
            .paths
            .lock()
            .map_err(|_| anyhow!("failed to lock AppState.paths"))?;

        let paths: &llava_core::config::ProgramFiles =
            paths_guard.as_ref().ok_or(llava_core::Error::FatalError)?;
        let conn: &mut rusqlite::Connection =
            conn_guard.as_mut().ok_or(llava_core::Error::FatalError)?;

        llava_core::register_user_offline(
            username.clone(),
            password_zeroized,
            password_repeated_zeroized,
            paths,
            conn,
        )?
    };
    crate::commands::command_helpers::chagne_state_after_login(
        &state, new_uuid, conn, new_paths, username,
    )?;

    println!("{:#?}", state);

    Ok((codes, new_uuid.to_string()))
}

#[tauri::command]
pub async fn login_command(
    username: String,
    password: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, llava_core::Error> {
    let password_zeroized = Zeroizing::from(password);

    let (new_uuid, new_paths, notes_conn) = {
        let mut conn_guard = state
            .users_db
            .lock()
            .map_err(|_| anyhow!("failed to lock AppState.connection"))?;

        let paths_guard = state
            .paths
            .lock()
            .map_err(|_| anyhow!("Failed to obtain paths from state"))?;

        let conn: &mut rusqlite::Connection =
            conn_guard.as_mut().ok_or(llava_core::Error::FatalError)?;
        let paths: &llava_core::config::ProgramFiles =
            paths_guard.as_ref().ok_or(llava_core::Error::FatalError)?;
        llava_core::local_log_in(username.clone(), password_zeroized, conn, paths).map_err(|e| {
            match &e {
                llava_core::Error::WrongPassword => {
                    if let Ok(user_uuid) = llava_core::get_user_uuid(conn, &username) {
                        if let Ok(end_of_timeout) = llava_core::check_error_count(conn, &user_uuid)
                        {
                            if end_of_timeout > llava_core::get_time() {
                                let timeout_duration = end_of_timeout - llava_core::get_time();
                                return llava_core::Error::AccountLocked(timeout_duration);
                            }
                        }
                    }
                }
                llava_core::Error::UserNotExists => {
                    println!("👤 User not found");
                    return llava_core::Error::UserNotExists;
                }
                _ => {
                    return llava_core::Error::FatalError;
                }
            }
            e
        })? //ZERO ERROR COUNT HERE
    };
    let mut conn_guard = state
        .users_db
        .lock()
        .map_err(|_| anyhow!("failed to lock AppState.connection"))?;
    let conn: &mut rusqlite::Connection =
        conn_guard.as_mut().ok_or(llava_core::Error::FatalError)?;
    llava_core::zero_error_count(conn, &new_uuid)?;
    crate::commands::command_helpers::chagne_state_after_login(
        &state, new_uuid, notes_conn, new_paths, username,
    )?;
    println!("{:#?}", state);

    Ok(new_uuid.to_string())
}

#[tauri::command]

pub async fn check_timeout_before_submit(
    state: tauri::State<'_, AppState>,
    username: String,
) -> Result<i64, llava_core::errors::Error> {
    let conn_guard = state
        .users_db
        .lock()
        .map_err(|_| anyhow!("failed to lock AppState.connection"))?;
    let conn = conn_guard.as_ref().ok_or(llava_core::Error::FatalError)?;

    if let Ok(user_uuid) = llava_core::get_user_uuid(conn, &username) {
        if let Ok(end_of_timeout) = llava_core::get_timeout(conn, &user_uuid) {
            println!("{}", end_of_timeout);
            if end_of_timeout > llava_core::get_time() {
                let timeout_duration = end_of_timeout - llava_core::get_time();
                return Ok(timeout_duration);
            } else {
                return Ok(0); // No timeout, return 0 instead of error
            }
        }
    }
    Err(llava_core::Error::FatalError)
}

#[tauri::command]
pub async fn check_if_user_exists(
    state: tauri::State<'_, AppState>,
) -> Result<bool, llava_core::Error> {
    let mut conn_guard = state
        .users_db
        .lock()
        .map_err(|_| anyhow!("Failed to lock AppState.paths"))?;
    let conn: &mut rusqlite::Connection =
        conn_guard.as_mut().ok_or(llava_core::Error::FatalError)?;
    llava_core::check_if_first_start(conn)
}

#[tauri::command]
pub async fn log_with_code(
    mut code: String,
    username: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, llava_core::Error> {
    code.retain(|c| c != '-');
    let users_db_guard = state
        .users_db
        .lock()
        .map_err(|_| anyhow!("Error while locking users_db"))?;
    let users_db = users_db_guard
        .as_ref()
        .ok_or(llava_core::Error::FatalError)?;
    let user_uuid = llava_core::get_user_uuid(users_db, &username)?;
    let paths: llava_core::ProgramFiles = {
        let guard = state.paths.lock().map_err(|_| anyhow!("lock paths"))?;
        guard.as_ref().ok_or(llava_core::Error::FatalError)?.clone()
    };
    let (paths, notes_conn) = llava_core::log_with_code(&paths, code, users_db, user_uuid)?;

    crate::commands::command_helpers::chagne_state_after_login(
        &state, user_uuid, notes_conn, paths, username,
    )?;

    Ok(user_uuid.to_string())
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
    let user_db = user_db_guard
        .as_ref()
        .ok_or(llava_core::Error::FatalError)?;
    // can i just get any key from used of user and get kek from it? or do i need specially code was used last time?
    llava_core::change_password(user_db, username, password, password_repeated, code)?;
    Ok(())
}

#[tauri::command]
pub async fn check_login_on_start(
    state: tauri::State<'_, AppState>,
) -> Result<SessionState, llava_core::Error> {
    let user_db_guard = state
        .users_db
        .lock()
        .map_err(|_| anyhow!("Couldnt get user_db guard"))?;
    let users_db = user_db_guard
        .as_ref()
        .ok_or(llava_core::Error::FatalError)?;
    let is_logged_in = llava_core::check_if_user_logged_in(users_db)?;
    Ok(is_logged_in)
}
