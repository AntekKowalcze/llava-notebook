use llava_core::settings::UserConfig;
use llava_core::AppState;
use tauri_plugin_clipboard_manager::ClipboardExt;
use anyhow::Context;
use crate::commands::local_auth::LoggedInOnline;
use crate::commands::sync::sync::synchronize_all;
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
        *guard.as_ref().ok_or(llava_core::Error::LockError)?
    }; // guard dropped here

    let notes_key: chacha20poly1305::Key = {
        let guard = state
            .notes_key
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;
        *guard.as_ref().ok_or(llava_core::Error::LockError)?
    }; // guard dropped here

    let client = &state.server_client;
    let password = zeroize::Zeroizing::new(password);
    let password_repeated = zeroize::Zeroizing::new(password_repeated);

    let (access_token, online_user_id) = llava_core::online_auth::register(
        client,
        email.clone(),
        password,
        password_repeated,
        device_id,
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
        users_db,
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
pub async fn online_logout(app_handle: AppHandle, state: tauri::State<'_, AppState>, sync: bool) -> Result<(), llava_core::Error> {
    crate::commands::utils::check_connection_before_request(state.clone())?;
    if sync {
   let res = synchronize_all(state.clone(), app_handle).await;

   if res.is_err() {
    return Err(llava_core::Error::SyncFailed)
   }
   let mut notes_db_guard = state
        .notes_db
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;

    let notes_db = notes_db_guard
        .as_mut()
        .ok_or(llava_core::Error::LockError)?;
     let local_user_id: uuid::Uuid = {
        let user_uuid_guard = state
            .current_user
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;
        *user_uuid_guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?
    };
    llava_core::online_auth::delete_synced_notes_on_logout(notes_db, local_user_id.to_string())?;

    }
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
        *guard.as_ref().ok_or(llava_core::Error::NotLoggedIn)?
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
    llava_core::get_email_from_online_id(&online_id, users_db)
}
#[tauri::command]
pub async fn login_online(
    email: String,
    password: String,
    state: tauri::State<'_, AppState>,
    app_handle: AppHandle,
    current_settings: Option<UserConfig>,
    local_password: String,
) -> Result<String, llava_core::Error> {
    tracing::info!(
        task = "online login",
        status = "starting",
        "starting online login"
    );

    {
        tracing::debug!(
            task = "online login",
            step = "local_authorization",
            "checking local password"
        );

        let username_guard = state
            .username
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        let username = username_guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?;

        let users_db_guard = state
            .users_db
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        let users_db = users_db_guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?;

        if !llava_core::local_auth::autorization(
            username,
            &local_password,
            users_db,
        )? {
            tracing::warn!(
                task = "online login",
                status = "error",
                step = "local_authorization",
                "local password verification failed"
            );

            return Err(llava_core::Error::WrongPassword);
        }

        tracing::debug!(
            task = "online login",
            status = "success",
            step = "local_authorization",
            "local password verified"
        );
    }

    tracing::debug!(
        task = "online login",
        step = "connection_check",
        "checking server and internet connection"
    );

    let (server_connected, internet_connected) =
        match llava_core::check_connection().await {
            Ok((server, internet)) => (server, internet),
            Err(e) => {
                tracing::warn!(
                    task = "online login",
                    status = "error",
                    step = "connection_check",
                    error = ?e,
                    "connection check failed"
                );

                (false, false)
            }
        };

    if let Ok(mut lock) = state.server_connection.lock() {
        *lock = server_connected;
    }

    if let Ok(mut lock) = state.internet_connection.lock() {
        *lock = internet_connected;
    }

    tracing::debug!(
        task = "online login",
        step = "connection_check",
        server_connected,
        internet_connected,
        "connection check completed"
    );

    crate::commands::utils::check_connection_before_request(state.clone())?;

    let client = state.server_client.clone();

    let device_id = {
        let guard = state
            .device_id
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        *guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?
    };

    tracing::debug!(
        task = "online login",
        step = "online_authentication",
        "starting online authentication"
    );

    let (access_token, online_user_id, new_notes_key) =
        llava_core::online_auth::login(
            email.clone(),
            Zeroizing::new(password),
            client,
            &device_id,
        )
        .await?;

    tracing::info!(
        task = "online login",
        status = "success",
        step = "online_authentication",
        online_user_id = %online_user_id,
        "online authentication successful"
    );

    let state_notes_key = {
        let guard = state
            .notes_key
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        *guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?
    };

    let user_id = {
        let guard = state
            .current_user
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        *guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?
    };

    let keys_match: bool = new_notes_key
        .as_slice()
        .ct_eq(state_notes_key.as_slice())
        .into();

    tracing::debug!(
        task = "online login",
        step = "key_comparison",
        keys_match,
        "compared online and local notes keys"
    );

    {
        let users_db_guard = state
            .users_db
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        let users_db = users_db_guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?;

        if keys_match {
            tracing::info!(
                task = "online login",
                step = "session_rotation",
                user_id = %user_id,
                "online notes key matches local notes key; rotating local session"
            );

            llava_core::local_auth::invalidate_session_for_user(
                &user_id.to_string(),
                users_db,
            )?;

            tracing::debug!(
                task = "online login",
                step = "session_rotation",
                "previous sessions invalidated"
            );

            llava_core::local_auth::session_operations(
                users_db,
                user_id,
                &state_notes_key,
            )?;

            tracing::info!(
                task = "online login",
                status = "success",
                step = "session_rotation",
                "new local session created"
            );
        } else {
            tracing::info!(
                task = "online login",
                step = "key_rotation",
                user_id = %user_id,
                "online notes key differs from local notes key; starting database re-encryption"
            );

            let _ = app_handle.emit("reencrypting_db", ());

            let new_key: GenericArray<u8, generic_array::typenum::U32> =
                *GenericArray::from_slice(&new_notes_key);

            let old_key = state_notes_key;

            tracing::debug!(
                task = "online login",
                step = "database_reencryption",
                "acquiring notes database"
            );

            {
                let notes_db_guard = state
                    .notes_db
                    .lock()
                    .map_err(|_| llava_core::Error::LockError)?;

                let notes_db = notes_db_guard
                    .as_ref()
                    .ok_or(llava_core::Error::LockError)?;

                tracing::info!(
                    task = "online login",
                    step = "database_reencryption",
                    "starting database re-encryption"
                );

                llava_core::crypto_operations::reencrypt_db(
                    notes_db,
                    &old_key,
                    &new_key,
                )?;

                tracing::info!(
                    task = "online login",
                    status = "success",
                    step = "database_reencryption",
                    "database re-encryption completed"
                );
            }

            tracing::debug!(
                task = "online login",
                step = "local_key_rewrap",
                "re-wrapping notes key for local password"
            );

            llava_core::local_auth::rewrap_key(
                &new_key,
                users_db,
                &local_password,
                &user_id.to_string(),
            )?;

            tracing::info!(
                task = "online login",
                status = "success",
                step = "local_key_rewrap",
                "local notes key re-wrapped successfully"
            );

            tracing::debug!(
                task = "online login",
                step = "recovery_key_rotation",
                "invalidating previous recovery codes"
            );

            llava_core::local_auth::invalidate_recovery_keys(
                users_db,
                &user_id.to_string(),
            )?;

            tracing::debug!(
                task = "online login",
                step = "recovery_key_rotation",
                "generating new recovery codes"
            );

            let new_codes =
                llava_core::local_auth::generate_recovery_codes_with_new_key(
                    &new_key,
                    users_db,
                    &user_id.to_string(),
                )?;

            tracing::info!(
                task = "online login",
                status = "success",
                step = "recovery_key_rotation",
                count = new_codes.len(),
                "new recovery codes generated"
            );

            tracing::debug!(
                task = "online login",
                step = "session_rotation",
                "invalidating previous local sessions"
            );

            llava_core::local_auth::invalidate_session_for_user(
                &user_id.to_string(),
                users_db,
            )?;

            llava_core::local_auth::session_operations(
                users_db,
                user_id,
                &new_key,
            )?;

            tracing::info!(
                task = "online login",
                status = "success",
                step = "session_rotation",
                "new local session created with new notes key"
            );

            *state
                .notes_key
                .lock()
                .map_err(|_| llava_core::Error::LockError)? = Some(new_key);

            tracing::debug!(
                task = "online login",
                step = "state_update",
                "local notes key updated"
            );

            app_handle
                .clipboard()
                .write_text(new_codes.join(",\n"))
                .context("failed to copy recovery codes")?;

            tracing::info!(
                task = "online login",
                step = "recovery_key_rotation",
                "new recovery codes copied to clipboard"
            );

            let _ = app_handle.emit("reencrypting_db_finished", ());

            tracing::info!(
                task = "online login",
                status = "success",
                step = "key_rotation",
                "key rotation flow completed"
            );
        }
    }

    *state
        .online_user_id
        .lock()
        .map_err(|_| llava_core::Error::LockError)? = Some(online_user_id.clone());

    *state
        .access_token
        .lock()
        .map_err(|_| llava_core::Error::LockError)? = Some(access_token);

    tracing::debug!(
        task = "online login",
        step = "state_update",
        "online authentication state updated"
    );

    {
        let users_data_guard = state
            .users_db
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        let users_data = users_data_guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?;

        llava_core::change_account_link_status(
            users_data,
            &user_id,
        )?;

        tracing::debug!(
            task = "online login",
            step = "account_link",
            "account link status updated"
        );

        llava_core::online_auth::change_email_in_database(
            &email,
            users_data,
            &online_user_id,
            user_id.to_string(),
        )?;

        tracing::debug!(
            task = "online login",
            step = "account_link",
            "online account email updated locally"
        );
    }

    let current_settings = match current_settings {
        Some(settings) => settings,
        None => {
            let paths = {
                let guard = state
                    .paths
                    .lock()
                    .map_err(|_| llava_core::Error::LockError)?;

                guard
                    .as_ref()
                    .ok_or(llava_core::Error::LockError)?
                    .clone()
            };

            let (settings, _created_default) =
                llava_core::settings::get_config(&paths)?;

            settings
        }
    };

    after_login(
        &current_settings,
        &state,
        &app_handle,
    )?;
    let _ =  synchronize_all(state, app_handle);
    tracing::info!(
        task = "online login",
        status = "success",
        user_id = %user_id,
        online_user_id = %online_user_id,
        "online login flow completed successfully"
    );

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
        current_settings,
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
        *user_uuid_guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?
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
        *guard
    };
    let is_online = {
        let guard = state
            .internet_connection
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;
        *guard
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
