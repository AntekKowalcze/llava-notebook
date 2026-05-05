use llava_core::settings::UserConfig;
use llava_core::AppState;
use tauri::AppHandle;
use tauri::Emitter;
#[tauri::command]
pub async fn register_user_online(
    email: String,
    password: String,
    password_repeated: String,
    state: tauri::State<'_, AppState>,
    app_handle: AppHandle,
    current_settings: UserConfig,
) -> Result<(), llava_core::Error> {
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
    //do it after register/after login function same locally
    *state
        .access_token
        .lock()
        .map_err(|_| llava_core::Error::LockError)? = Some(access_token);
    *state
        .online_user_id
        .lock()
        .map_err(|_| llava_core::Error::LockError)? = Some(online_user_id);
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
    llava_core::online_auth::logout(user_id, client, &device_id, &access_token).await?;

    *state
        .online_user_id
        .lock()
        .map_err(|_| llava_core::Error::LockError)? = None;

    *state
        .access_token
        .lock()
        .map_err(|_| llava_core::Error::LockError)? = None;

    Ok(())
    //TODO server przy register device id powinno sprawdzać czy istnieje, jesli istnieje nie dodawać, jelsi nie
}

//TODO add login

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
