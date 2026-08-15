//! # Authentication command module
//!
//! **Purpose**: This module exposes Tauri commands responsible for local authentication,
//! account registration, password recovery, session restoration, and local logout.
//!
//! ## Exports
//! * `register_command` — Registers a new local user and initialises the user's application state.
//! * `login_command` — Authenticates a user with username and password and initialises the
//!   authenticated application state.
//! * `check_if_user_exists` — Checks whether a registered user exists.
//! * `log_with_code` — Authenticates a user using a recovery code.
//! * `check_timeout_before_submit` — Returns the remaining account lockout time.
//! * `change_password` — Changes a user's password after recovery-code validation.
//! * `check_login_on_start` — Restores a previous local session and optionally checks the
//!   associated online session.
//! * `local_logout_command` — Clears the current local session and sensitive runtime state.
//! * `LoggedInOnline` — Represents the current online authentication state.
//!
//! ## Key design decisions
//! Authentication state is stored in `AppState` behind synchronisation primitives. Database
//! locks are kept for the shortest practical scope to avoid unnecessary contention.
//!
//! Passwords, recovery codes, access tokens, and encryption keys are never logged or exposed
//! to the frontend. Temporary copies of the notes encryption key are explicitly zeroized after
//! they are no longer required.
//!
//! Local authentication is independent from online authentication. A local session can be
//! restored without a working network connection. When an online account is linked, the online
//! session is checked asynchronously with a short timeout so that server unavailability does
//! not block local application startup.
//!
//! Local logout clears sensitive runtime state, including the access token, notes encryption key,
//! current user, notes database, username, configuration, and online user identifier.
//!
//! ## Dependencies
//! - `tauri` — Provides Tauri commands, application state, and event emission.
//! - `tokio` — Provides asynchronous execution and timeout handling.
//! - `zeroize` — Securely clears temporary encryption-key copies from memory.
//! - `anyhow` — Provides contextual errors for state and lock operations.
//! - `llava_core` — Provides authentication, session, database, configuration, and application
//!   state functionality.
//! - `uuid` — Handles authenticated user identifiers.
//! - `serde` — Serialises `LoggedInOnline` for frontend communication.

use anyhow::{anyhow, Context};
use llava_core::local_auth::SessionState;
use llava_core::AppState;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::Manager;
use tokio::time::{timeout, Duration};
use zeroize::Zeroize;
#[tauri::command]
pub async fn register_command(
    username: String,
    password: String,
    password_repeated: String,
    state: tauri::State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(Vec<String>, String), llava_core::Error> {
    let (new_uuid, new_paths, users_db, codes, notes_key) = {
        let paths_guard = state
            .paths
            .lock()
            .map_err(|_| anyhow!("failed to lock paths"))?;
        let mut conn_guard = state
            .users_db
            .lock()
            .map_err(|_| anyhow!("failed to lock users_db"))?;

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
        notes_key,
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
    let (new_uuid, new_paths, notes_conn, notes_key) = {
        let paths_guard = state
            .paths
            .lock()
            .map_err(|_| anyhow!("failed to lock paths"))?;
        let mut conn_guard = state
            .users_db
            .lock()
            .map_err(|_| anyhow!("failed to lock users_db"))?;

        let mut users_db = conn_guard.as_mut().ok_or(llava_core::Error::LockError)?;
        let paths = paths_guard.as_ref().ok_or(llava_core::Error::LockError)?;
        let timeout = crate::commands::handlers::local_auth::check_timeout(&username, users_db)?; //returns diff between current timestamp and end of lock timestamp
        if timeout > 0 {
            return Err(llava_core::Error::AccountLocked(0));
        }
        crate::commands::handlers::local_auth::login(username.clone(), password, paths, &mut users_db)?
    };

    let id = {
        let mut conn_guard = state
            .users_db
            .lock()
            .map_err(|_| anyhow!("failed to lock users_db"))?;
        let users_db = conn_guard.as_mut().ok_or(llava_core::Error::LockError)?;
        llava_core::local_auth::zero_error_count(users_db, &new_uuid)?;
        llava_core::local_auth::get_optional_online_id(users_db, &new_uuid)?
    }; // conn_guard dropped here

 if let Some(online_id) = id {
    crate::commands::online_auth::try_refresh_if_logged_in(online_id, app_handle.clone()).await?;
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
        notes_key,
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

    let (user_uuid, paths, notes_conn, one_code, notes_key) = {
        let paths_guard = state
            .paths
            .lock()
            .map_err(|_| anyhow!("failed to lock paths"))?;
        let users_db_guard = state
            .users_db
            .lock()
            .map_err(|_| anyhow!("failed to lock users_db"))?;

        let users_db = users_db_guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?;
        let paths = paths_guard.as_ref().ok_or(llava_core::Error::LockError)?;

        let user_uuid = llava_core::get_user_uuid(users_db, &username)?;
        let (paths, notes_conn, one_code, notes_key) =
            crate::commands::handlers::local_auth::log_with_code(code, &username, paths, users_db)?;

        (user_uuid, paths, notes_conn, one_code, notes_key)
    };


    let id = {
        let mut conn_guard = state
            .users_db
            .lock()
            .map_err(|_| anyhow!("failed to lock users_db"))?;
        let users_db = conn_guard.as_mut().ok_or(llava_core::Error::LockError)?;
        let user_uuid: uuid::Uuid = llava_core::get_user_uuid(users_db, &username)?;
        llava_core::local_auth::zero_error_count(users_db, &user_uuid)?;
        llava_core::local_auth::get_optional_online_id(users_db, &user_uuid)?
       
    };
    println!("{:?} this is an id", id);
    if let Some(online_id) = id {
      crate::commands::online_auth::try_refresh_if_logged_in(online_id, app_handle.clone()).await?;
    }
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
        notes_key,
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

#[derive(Debug, serde::Serialize)]
#[serde(tag = "status", content = "data", rename_all = "snake_case")]
pub enum LoggedInOnline {
    LoggedIn(String),
    NotLoggedIn(llava_core::Error),
    NotLinked,
    Checking,
}

#[tauri::command]
pub async fn check_login_on_start(
    app_handle: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(SessionState, LoggedInOnline), llava_core::Error> {
    let mut is_logged_in_online: LoggedInOnline = LoggedInOnline::NotLinked;
    let program_files = {
        let program_files_guard = state
            .paths
            .lock()
            .map_err(|_| anyhow!("Couldnt get program files guard"))?;
        program_files_guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?
            .clone()
    };
    let mut is_logged_in: SessionState = {
        let users_db_guard = state
            .users_db
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;
        let users_db = users_db_guard
            .as_ref()
            .ok_or(llava_core::Error::NotLoggedIn)?;
        llava_core::local_auth::check_if_user_logged_in(users_db, &program_files)?
    };

    if let SessionState::LoggedIn { user_id, notes_key } = &mut is_logged_in {
        let mut owned_key: chacha20poly1305::Key =
            chacha20poly1305::Key::clone_from_slice(&notes_key);
        let parsed_user_uuid =
            uuid::Uuid::parse_str(&user_id).context("Failed to parse user_id to string")?;

        let (updated_paths, notes_db, username, user_config, is_linked) = {
            let users_db_guard = state
                .users_db
                .lock()
                .map_err(|_| llava_core::Error::LockError)?;
            let users_db = users_db_guard
                .as_ref()
                .ok_or(llava_core::Error::NotLoggedIn)?;

            let updated_paths =
                llava_core::get_paths(program_files.app_home.clone(), &parsed_user_uuid)?;
            let notes_db = llava_core::storage::get_connection(&updated_paths)?;
            let username = llava_core::get_username_from_uuid(users_db, user_id.clone())?;
            let user_config = llava_core::settings::get_config_for_state(&updated_paths)?;
            let is_linked = llava_core::is_online_linked(&parsed_user_uuid, &users_db)?;
            (updated_paths, notes_db, username, user_config, is_linked)
        };

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
            owned_key,
        )?;

        let is_local = {
            let guard = state
                .user_config
                .lock()
                .map_err(|_| llava_core::Error::LockError)?;
            match guard
                .as_ref()
                .ok_or(llava_core::Error::LockError)?
                .clone()
                .get("local.mode")
            {
                Some(option) => option == "on",
                None => false,
            }
        };

        if !is_local && is_linked {
            let online_id = {
                let users_db_guard = state
                    .users_db
                    .lock()
                    .map_err(|_| llava_core::Error::LockError)?;
                let users_db = users_db_guard
                    .as_ref()
                    .ok_or(llava_core::Error::NotLoggedIn)?;

                match llava_core::get_online_id(&parsed_user_uuid, users_db) {
                    Ok(id) => Some(id),
                    Err(err) => {
                        is_logged_in_online = LoggedInOnline::NotLoggedIn(err);
                        None
                    }
                }
            };
            
            if let Some(online_id) = online_id {
                is_logged_in_online = LoggedInOnline::Checking;
                let app_handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app_handle.state::<AppState>();
                    let client = state.server_client.clone();
                    let res = timeout(
                        Duration::from_secs(3),
                        llava_core::online_auth::check_if_logged_in_online(&online_id, client),
                    )
                    .await;
                    let status = match res {
                        Ok(Ok(token)) => {
                            if let Ok(mut guard) = state.access_token.lock() {
                                *guard = Some(token);
                            }
                            if let Ok(mut guard) = state.online_user_id.lock() {
                                *guard = Some(online_id.clone());
                            }
                            LoggedInOnline::LoggedIn(online_id)
                        }
                        Ok(Err(err)) => {
                            if matches!(err, llava_core::Error::OnlineSessionExpired) {
                                let _ = app_handle.emit("online_session_expired", ());
                            }
                            LoggedInOnline::NotLoggedIn(err)
                        }
                        Err(_) => {
                            LoggedInOnline::NotLoggedIn(llava_core::Error::ServerNotAvailable)
                        }
                    };
                    let _ = app_handle.emit("online_login_status", &status);
                });
            }
        }
        *notes_key = vec![]; //frontend should not see the key so its correct
        owned_key.zeroize();
    }

    Ok((is_logged_in, is_logged_in_online))
}

#[tauri::command]
pub async fn local_logout_command(
    state: tauri::State<'_, AppState>,
) -> Result<(), llava_core::Error> {
    let user_uuid = {
        let mut user_uuid_guard = state
            .current_user
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;
        let id = user_uuid_guard
            .as_mut()
            .ok_or(llava_core::Error::LockError)?
            .to_string();
        id
    };

    *state
        .access_token
        .lock()
        .map_err(|_| anyhow!("couldnt edit access_token"))? = None;
    *state
        .notes_key
        .lock()
        .map_err(|_| anyhow!("couldnt edit  notes key"))? = None;
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
    *state
        .user_config
        .lock()
        .map_err(|_| anyhow!("Couldnt edit config in state"))? = None;
    *state
        .online_user_id
        .lock()
        .map_err(|_| anyhow!("Couldnt edit config in state"))? = None;
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
