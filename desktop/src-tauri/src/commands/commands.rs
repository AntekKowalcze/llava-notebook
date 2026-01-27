use anyhow::{anyhow, Context};
use llava_core::{config::change_active_user, AppState};
use tauri::State;
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

    println!("{:#?}", state);

    Ok(())
}

#[tauri::command]
pub async fn login_command(
    username: String,
    password: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), llava_core::Error> {
    let password_zeroized = Zeroizing::from(password);

    let (new_uuid, new_paths, conn) = {
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

        llava_core::local_log_in(username.clone(), password_zeroized, conn, paths)?
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

    println!("{:#?}", state);

    Ok(())
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
