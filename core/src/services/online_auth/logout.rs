use anyhow::Context;
use reqwest::Client;

use crate::services::online_auth::models::online_account::AccessToken;
use serde::Serialize;
#[derive(Serialize)]
struct LogoutRequest {
    pub user_id: String,
    pub device_id: uuid::Uuid,
}

pub async fn logout(
    user_id: String,
    client: Client,
    device_id: &uuid::Uuid,
    access_token: &AccessToken,
) -> Result<(), crate::errors::Error> {
    let request = LogoutRequest {
        user_id: user_id.clone(),
        device_id: *device_id,
    };

    let res = client
        .post(format!("{}auth/logout", crate::constants::SERVER_ADDRESS))
        .bearer_auth(&access_token.0)
        .json(&request)
        .send()
        .await.map_err(|_| crate::Error::ServerNotAvailable)?;

    if !res.status().is_success() {
        return Err(crate::errors::Error::RequestError((
            res.status().as_u16(),
            res.text().await.unwrap_or_default(),
        )));
    }

    let entry = keyring::Entry::new("llava_desktop", &format!("refresh_token_id:{}", user_id))
        .map_err(|_| crate::errors::Error::NotLoggedIn)?;
    entry
        .delete_credential()
        .context("Failed to delete credential")?;

    Ok(())
}
