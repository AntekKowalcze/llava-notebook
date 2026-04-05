use llava_core::AppState;
#[tauri::command]
pub async fn register_user_online(
    email: String,
    password: String,
    password_repeated: String,
    state: tauri::State<'_, AppState>,
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
    let password = zeroize::Zeroizing::new(password);
    let password_repeated = zeroize::Zeroizing::new(password_repeated);

    let access_token = llava_core::online_auth::register(
        email,
        password,
        password_repeated,
        device_id.clone(),
        &notes_key,
    )
    .await?;
    *state
        .access_token
        .lock()
        .map_err(|_| llava_core::Error::LockError)? = Some(access_token);

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
// TODO Change go backend to response with user id
