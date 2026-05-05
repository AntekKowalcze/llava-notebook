use anyhow::Context;
use reqwest::Client;
use serde::Serialize;

use crate::services::online_auth::models::online_account::RefreshResponse;

#[derive(Serialize, Debug)]
struct RefreshRequest {
    refresh_token: String,
}
pub async fn check_if_logged_in_online(
    online_id: &str,
    client: Client,
) -> Result<crate::services::online_auth::models::online_account::AccessToken, crate::errors::Error>
{
    let entry = keyring::Entry::new("llava_desktop", &format!("refresh_token_id:{}", online_id))
        .map_err(|_| crate::errors::Error::NotLoggedIn)?;
    let refresh_token = entry
        .get_password()
        .map_err(|_| crate::errors::Error::NotLoggedIn)?;
    let req: RefreshRequest = RefreshRequest { refresh_token };
    let response = client
        .post(format!("{}auth/refresh", crate::constants::SERVER_ADDRESS))
        .json(&req)
        .send()
        .await
        .map_err(|err| crate::errors::Error::from_reqwest_error(&err))?;
    if !response.status().is_success() {
        if response.status().as_u16() == 500 {
            return Err(crate::errors::Error::RequestError((
                500,
                "Internal server error, you will be not logged in".to_string(),
            ))); //how to handle this? try again in some time?
        } else if response.status().as_u16() == 401 {
            return Err(crate::errors::Error::NotLoggedIn);
        } else {
            return Err(crate::errors::Error::NotLoggedIn);
        }
    }

    let tokens = response
        .json::<RefreshResponse>()
        .await
        .context("failed to parse response")?;
    entry
        .set_password(&tokens.refresh_token.0)
        .context("failed to save refresh token in keyring")?;
    Ok(tokens.access_token)
}
