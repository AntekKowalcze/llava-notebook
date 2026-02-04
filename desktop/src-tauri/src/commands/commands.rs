use anyhow::{anyhow, Context};
use llava_core::{config::change_active_user, AppState};
use tauri::{App, State};
use zeroize::Zeroizing;

#[tauri::command]
pub async fn register_command(
    username: String,
    password: String,
    password_repeated: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), llava_core::Error> {
    let password_zeroized = Zeroizing::from(password);
    let password_repeated_zeroized = Zeroizing::from(password_repeated);

    let (new_uuid, new_paths, conn) = {
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

    *state
        .current_user
        .lock()
        .map_err(|_| anyhow!("couldnt edit current user"))? = Some(new_uuid);

    *state
        .notes_db
        .lock()
        .map_err(|_| anyhow!("Couldnt edit notes db in state"))? = Some(conn);

    *state
        .paths
        .lock()
        .map_err(|_| anyhow!("Couldnt edit paths in state"))? = Some(new_paths);

    *state
        .username
        .lock()
        .map_err(|_| anyhow!("Couldnt edit username in state"))? = Some(username.clone());

    // println!("{:#?}", state);

    Ok(())
}

#[tauri::command]
pub async fn login_command(
    username: String,
    password: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), llava_core::Error> {
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
    llava_core::zero_error_count(&conn, &new_uuid)?;

    *state
        .current_user
        .lock()
        .map_err(|_| anyhow!("couldnt edit current user"))? = Some(new_uuid);
    *state
        .notes_db
        .lock()
        .map_err(|_| anyhow!("Couldnt edit notes db in state"))? = Some(notes_conn);
    *state
        .paths
        .lock()
        .map_err(|_| anyhow!("Couldnt edit paths in state"))? = Some(new_paths);
    *state
        .username
        .lock()
        .map_err(|_| anyhow!("Couldnt edit username in state"))? = Some(username.clone());

    // println!("{:#?}", state);

    Ok(())
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

pub async fn generate_recovery_keys(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<String>, llava_core::Error> {
    let user_id_guard = state
        .current_user
        .lock()
        .map_err(|_| anyhow!("Failed to obtain user id from state"))?;
    let conn_guard = state
        .users_db
        .lock()
        .map_err(|_| anyhow!("Failed to get conn from state"))?;
    let users_db = conn_guard.as_ref().ok_or(llava_core::Error::FatalError)?;

    let user_uuid = user_id_guard
        .as_ref()
        .ok_or(llava_core::Error::FatalError)?;

    let keys = llava_core::recovery_code_handling(user_uuid, users_db)?;
    Ok(keys)
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
    return llava_core::check_if_first_start(conn);
}
