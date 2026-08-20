use llava_core::settings::UserConfig;
use llava_core::AppState;

use crate::commands::local_auth::LoggedInOnline;
use anyhow::anyhow;
use chacha20poly1305::aead::generic_array;
use chacha20poly1305::aead::generic_array::GenericArray;
use subtle::ConstantTimeEq;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::Manager;
use zeroize::Zeroizing;
#[tauri::command]
pub async fn register_user_online(
    email: String,
    password: String,
    password_repeated: String,
    state: tauri::State<'_, AppState>,
    app_handle: AppHandle,
    current_settings: Option<UserConfig>,
) -> Result<(), llava_core::Error> {
    let (server_connected, internet_connected) = match llava_core::check_connection().await {
        Ok((server, internet)) => (server, internet),
        Err(_) => (false, false),
    };
    if let Ok(mut lock) = state.server_connection.lock() {
        *lock = server_connected;
    }
    if let Ok(mut lock) = state.internet_connection.lock() {
        *lock = internet_connected;
    }
    crate::commands::utils::check_connection_before_request(state.clone())?;

    let device_id = {
        let guard = state
            .device_id
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;
        guard.as_ref().ok_or(llava_core::Error::LockError)?.clone()
    }; // guard dropped here

    let notes_key: chacha20poly1305::Key = {
        let guard = state
            .notes_key
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;
        guard.as_ref().ok_or(llava_core::Error::LockError)?.clone()
    }; // guard dropped here

    let client = &state.server_client;
    let password = zeroize::Zeroizing::new(password);
    let password_repeated = zeroize::Zeroizing::new(password_repeated);

    let (access_token, online_user_id) = llava_core::online_auth::register(
        client,
        email.clone(),
        password,
        password_repeated,
        device_id.clone(),
        &notes_key,
    )
    .await?;

    let conn_guard = state
        .users_db
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;
    let users_db = conn_guard.as_ref().ok_or(llava_core::Error::LockError)?;
    let user_id_guard_ = state
        .current_user
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;
    let user_id = user_id_guard_
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;

    llava_core::online_auth::change_email_in_database(
        &email,
        &users_db,
        &online_user_id,
        user_id.to_string(),
    )?;
    *state
        .access_token
        .lock()
        .map_err(|_| llava_core::Error::LockError)? = Some(access_token);
    *state
        .online_user_id
        .lock()
        .map_err(|_| llava_core::Error::LockError)? = Some(online_user_id);
    let current_settings = match current_settings {
        Some(settings) => settings,
        None => {
            let paths = {
                let guard = state
                    .paths
                    .lock()
                    .map_err(|_| llava_core::Error::LockError)?;
                guard.as_ref().ok_or(llava_core::Error::LockError)?.clone()
            };
            let (settings, _created_default) = llava_core::settings::get_config(&paths)?;
            settings
        }
    };
    after_login(&current_settings, &state, &app_handle)?;
    Ok(())
}
// #[derive(serde::Deserialize)]
// pub struct Claims {
//     pub sub: String,
//     pub exp: i64,
//     pub iat: i64,
//     pub aud: Vec<String>,
//     pub device_id: uuid::Uuid,
// }

#[tauri::command]
pub async fn online_logout(state: tauri::State<'_, AppState>) -> Result<(), llava_core::Error> {
    crate::commands::utils::check_connection_before_request(state.clone())?;

    let user_id = {
        let guard = state
            .online_user_id
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;
        guard
            .as_ref()
            .ok_or(llava_core::Error::NotLoggedIn)?
            .clone()
    };

    let access_token = {
        let guard = state
            .access_token
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;
        guard
            .as_ref()
            .ok_or(llava_core::Error::NotLoggedIn)?
            .clone() // owned AccessToken
    };

    let device_id = {
        let guard = state
            .device_id
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;
        guard
            .as_ref()
            .ok_or(llava_core::Error::NotLoggedIn)?
            .clone()
    };
    let client = state.server_client.clone();

    llava_core::online_auth::logout(user_id.clone(), client, &device_id, &access_token).await?;

    let users_db_guard = state
        .users_db
        .lock()
        .map_err(|_| anyhow!("failed to lock users_db"))?;

    let users_db = users_db_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;

    llava_core::online_auth::set_account_to_offline_in_db(user_id, users_db)?;

    *state
        .online_user_id
        .lock()
        .map_err(|_| llava_core::Error::LockError)? = None;

    *state
        .access_token
        .lock()
        .map_err(|_| llava_core::Error::LockError)? = None;

    Ok(())
}

#[tauri::command]
pub async fn get_email_from_id(
    online_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, llava_core::Error> {
    let conn_guard = state
        .users_db
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;
    let users_db = conn_guard.as_ref().ok_or(llava_core::Error::LockError)?;
    Ok(llava_core::get_email_from_online_id(&online_id, users_db)?)
}

#[tauri::command]
pub async fn login_online(
    email: String,
    password: String,
    state: tauri::State<'_, AppState>,
    app_handle: AppHandle,
    current_settings: Option<UserConfig>,
) -> Result<String, llava_core::Error> {
    let (server_connected, internet_connected) = match llava_core::check_connection().await {
        Ok((server, internet)) => (server, internet),
        Err(_) => (false, false),
    };
    if let Ok(mut lock) = state.server_connection.lock() {
        *lock = server_connected;
    }
    if let Ok(mut lock) = state.internet_connection.lock() {
        *lock = internet_connected;
    }
    crate::commands::utils::check_connection_before_request(state.clone())?;

    let client: reqwest::Client = state.server_client.clone();

    let device_id = {
        let guard = state
            .device_id
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;
        guard.as_ref().ok_or(llava_core::Error::LockError)?.clone()
    };

    let (access_token, online_user_id, notes_key) =
        llava_core::online_auth::login(email.clone(), Zeroizing::new(password), client, &device_id)
            .await?;
    *state
        .access_token
        .lock()
        .map_err(|_| llava_core::Error::LockError)? = Some(access_token);
    let state_notes_key = {
        let guard = state
            .notes_key
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;
        guard.as_ref().ok_or(llava_core::Error::LockError)?.clone()
    };
    *state
        .online_user_id
        .lock()
        .map_err(|_| llava_core::Error::LockError)? = Some(online_user_id.clone());
    if notes_key
        .as_slice()
        .ct_eq(state_notes_key.as_slice())
        .into()
    {
        // keys match, nothing to do
    } else {
        let arr: GenericArray<u8, generic_array::typenum::U32> =
            *GenericArray::from_slice(&notes_key);
        *state
            .notes_key
            .lock()
            .map_err(|_| llava_core::Error::LockError)? = Some(arr);
        todo!()
    }
    let users_data_guard = state
        .users_db
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;
    let users_data = users_data_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;
    let user_id_guard = state
        .current_user
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;
    let user_id = user_id_guard.as_ref().ok_or(llava_core::Error::LockError)?;
    llava_core::change_account_link_status(users_data, user_id)?;
    llava_core::online_auth::change_email_in_database(
        &email,
        users_data,
        &online_user_id,
        user_id.to_string(),
    )?;

    let current_settings = match current_settings {
        Some(settings) => settings,
        None => {
            let paths = {
                let guard = state
                    .paths
                    .lock()
                    .map_err(|_| llava_core::Error::LockError)?;
                guard.as_ref().ok_or(llava_core::Error::LockError)?.clone()
            };
            let (settings, _created_default) = llava_core::settings::get_config(&paths)?;
            settings
        }
    };
    after_login(&current_settings, &state, &app_handle)?;

    Ok(online_user_id)
}

fn after_login(
    current_settings: &UserConfig,
    state: &tauri::State<'_, AppState>,
    app_handle: &AppHandle,
) -> Result<(), llava_core::Error> {
    let mut config_map_guard = state
        .user_config
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;
    let config_map = config_map_guard
        .as_mut()
        .ok_or(llava_core::Error::LockError)?;
    if let Some(value) = config_map.get_mut("local.mode") {
        *value = "off".to_string();
    } else {
        config_map.insert("local.mode".to_string(), "off".to_string());
    }
    let paths = {
        let guard = state
            .paths
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;
        guard.as_ref().ok_or(llava_core::Error::LockError)?.clone()
    };
    let hash_config = llava_core::settings::save_config(
        &current_settings,
        paths.config_path.clone(),
        paths.config_backup_path.clone(),
    )?;
    app_handle
        .emit("config-updated", &hash_config)
        .map_err(|_| llava_core::Error::FatalError)?;
    Ok(())
}

// after refresh -> after login

#[tauri::command]
pub async fn try_login_if_connected_with_server(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), llava_core::Error> {
    let user_uuid = {
        let user_uuid_guard = state
            .current_user
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;
        user_uuid_guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?
            .clone()
    };

    let id = {
        let mut conn_guard = state
            .users_db
            .lock()
            .map_err(|_| anyhow!("failed to lock users_db"))?;
        let users_db = conn_guard.as_mut().ok_or(llava_core::Error::LockError)?;
        llava_core::local_auth::get_optional_online_id(users_db, &user_uuid)?
    };

    if let Some(online_id) = id {
        try_refresh_if_logged_in(online_id, app_handle.clone()).await?;
    }
    Ok(())
}

pub async fn try_refresh_if_logged_in(
    online_id: String,
    app_handle: tauri::AppHandle,
) -> Result<(), llava_core::Error> {
    let state = app_handle.state::<AppState>();
    let is_connected_to_server = {
        let guard = state
            .server_connection
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;
        guard.clone()
    };
    let is_online = {
        let guard = state
            .internet_connection
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;
        guard.clone()
    };

    if !is_online || !is_connected_to_server {
        let status = if !is_online {
            LoggedInOnline::NotLoggedIn(llava_core::Error::NoInternetConnection)
        } else {
            LoggedInOnline::NotLoggedIn(llava_core::Error::ServerNotAvailable)
        };
        let _ = app_handle.emit("online_login_status", &status);
        return Ok(());
    }

    let app_handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        let state = app_handle.state::<AppState>();
        let client = state.server_client.clone();
        let res = tokio::time::timeout(
            tokio::time::Duration::from_secs(3),
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
            Err(_) => LoggedInOnline::NotLoggedIn(llava_core::Error::ServerNotAvailable),
        };
        let _ = app_handle.emit("online_login_status", &status);
    });
    Ok(())
}
