use anyhow::{anyhow, Context};
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

    let (new_uuid, new_paths, users_db, codes) = {
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
        let users_db: &mut rusqlite::Connection =
            conn_guard.as_mut().ok_or(llava_core::Error::FatalError)?;

        llava_core::register_user_offline(
            username.clone(),
            password_zeroized,
            password_repeated_zeroized,
            paths,
            users_db,
        )?
    };
    crate::commands::command_helpers::change_state_after_login(
        &state, new_uuid, users_db, new_paths, username,
    )?;

    // println!("{:#?}", state);

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

        let users_db: &mut rusqlite::Connection =
            conn_guard.as_mut().ok_or(llava_core::Error::FatalError)?;
        let paths: &llava_core::config::ProgramFiles =
            paths_guard.as_ref().ok_or(llava_core::Error::FatalError)?;
        llava_core::local_log_in(username.clone(), password_zeroized, users_db, paths).map_err(
            |e| {
              
                match &e {
                    
                    llava_core::Error::WrongPassword => {
                        if let Ok(user_uuid) = llava_core::get_user_uuid(users_db, &username) {
                            if let Ok(end_of_timeout) =
                                llava_core::check_error_count(users_db, &user_uuid)
                            {
                                if end_of_timeout > llava_core::get_time() {
                                    let timeout_duration = end_of_timeout - llava_core::get_time();
                                    return llava_core::Error::AccountLocked(timeout_duration);
                                }
                            }
                        }
                    }
                 //USER not exists handled in checing timeouts
                    _ => {
                        return llava_core::Error::FatalError;
                    }
                }
                e
            },
        )? //ZERO ERROR COUNT HERE
    };
    let mut conn_guard = state
        .users_db
        .lock()
        .map_err(|_| anyhow!("failed to lock AppState.connection"))?;
    let users_db: &mut rusqlite::Connection =
        conn_guard.as_mut().ok_or(llava_core::Error::FatalError)?;
    llava_core::zero_error_count(users_db, &new_uuid)?;
    crate::commands::command_helpers::change_state_after_login(
        &state, new_uuid, notes_conn, new_paths, username,
    )?;
    // println!("{:#?}", state);

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
    let users_db = conn_guard.as_ref().ok_or(llava_core::Error::FatalError)?;


 let user_uuid = llava_core::get_user_uuid(users_db, &username).map_err(|e| match e {
        llava_core::Error::UserNotExists => llava_core::Error::UserNotExists,
        _ => llava_core::Error::FatalError,
    })?;

    match llava_core::get_timeout(users_db, &user_uuid) {
        Ok(end_of_timeout) => {
            if end_of_timeout > llava_core::get_time() {
                Ok(end_of_timeout - llava_core::get_time())
            } else {
                Ok(0)
            }
        }
        Err(_) => Err(llava_core::Error::FatalError),
    }
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
        conn_guard.as_mut().ok_or(llava_core::Error::FatalError)?;
    llava_core::check_if_first_start(users_db)
}

#[tauri::command]
pub async fn log_with_code(
    mut code: String,
    username: String,
    state: tauri::State<'_, AppState>,
) -> Result<(String, bool), llava_core::Error> {
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
    let (paths, notes_conn, one_code) =
        llava_core::log_with_code(&paths, code, users_db, user_uuid)?;

    crate::commands::command_helpers::change_state_after_login(
        &state, user_uuid, notes_conn, paths, username,
    )?;

    Ok((user_uuid.to_string(), one_code))
}
//po znalezieniu kodu, zapisać do active user zalogowanego użytkownika,  stworzyć pliki dla tego użytkownika, wziąć baze danych dla uzytkownika
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
    let program_files = {
        let program_files_guard = state
            .paths
            .lock()
            .map_err(|_| anyhow!("Couldnt get program filesguard"))?;
        program_files_guard
            .as_ref()
            .ok_or(llava_core::Error::FatalError)?
            .clone() // ProgramFiles: Clone
    };
    let is_logged_in: SessionState = llava_core::check_if_user_logged_in(users_db, &program_files)?;

    if let SessionState::LoggedIn { user_id } = &is_logged_in {
        let parsed_user_uuid =
            uuid::Uuid::parse_str(&user_id).context("Failed to parse user_id to string")?;

        let updated_paths =
            llava_core::get_paths(program_files.app_home.clone(), &parsed_user_uuid)?;

        let notes_db = llava_core::get_connection(&updated_paths)?;

        let username = llava_core::get_username_from_uuid(users_db, user_id.clone())?;

        crate::commands::command_helpers::change_state_after_login(
            &state,
            parsed_user_uuid,
            notes_db,
            updated_paths.clone(),
            username,
        )?;
    }
    Ok(is_logged_in)
}

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
        .ok_or(llava_core::Error::FatalError)?;
    let username = llava_core::get_username_from_uuid(users_db, user_uuid)?;
    Ok(username)
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
        .ok_or(llava_core::Error::FatalError)?;
    let paths_guard = state
        .paths
        .lock()
        .map_err(|_| anyhow!("Cannot lock state"))?;
    let paths = paths_guard.as_ref().ok_or(llava_core::Error::FatalError)?;
    llava_core::local_logout(user_uuid, users_db, paths)?;
    Ok(())
}

#[tauri::command]
pub async fn get_dashboard_data(
    user_uuid: String,
    state: tauri::State<'_, AppState>,
) -> Result<llava_core::DashboardData, llava_core::Error> {
    let users_db_guard: std::sync::MutexGuard<'_, Option<rusqlite::Connection>> = state
        .users_db
        .lock()
        .map_err(|_| anyhow!("error while gettnig users_db from state"))?;

    let users_db = users_db_guard
        .as_ref()
        .ok_or(llava_core::Error::FatalError)?;
    println!("{:#?}", state);

    let notes_db_guard = state
        .notes_db
        .lock()
        .map_err(|_| anyhow!("Couldnt edit notes db in state"))?;
    println!("{:?}", notes_db_guard);
    let notes_db = notes_db_guard
        .as_ref()
        .ok_or(llava_core::Error::FatalError)?;

    return Ok(llava_core::get_dashboard_stats(
        user_uuid, &notes_db, &users_db,
    )?);
    //przetworzyć return na dashboard data
}

#[tauri::command]
pub async fn get_config_data( state: tauri::State<'_, AppState>) -> Result<llava_core::UserConfig ,llava_core::Error>{

    let paths_guard = state
            .paths
            .lock()
            .map_err(|_| anyhow!("failed to lock AppState.paths"))?;

        let paths: &llava_core::config::ProgramFiles =
            paths_guard.as_ref().ok_or(llava_core::Error::FatalError)?;

        let user_config: llava_core::UserConfig = llava_core::get_config(&paths)?;
        println!("{:?}", user_config);
        Ok(user_config)

}