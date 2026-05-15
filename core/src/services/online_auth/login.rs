use crate::constants::KEY_ENCRYPTED_KEY_LENGTH;
use crate::constants::SERVER_ADDRESS;
use crate::services::logger::log_error;
use crate::services::online_auth::models::online_account::AccessToken;
use crate::services::online_auth::models::online_account::ArgonParams;
use crate::services::online_auth::models::online_account::RefreshResponse;
use crate::services::online_auth::models::online_account::RefreshToken;
use anyhow::Context;
use argon2::Argon2;
use argon2::PasswordHasher;
use argon2::password_hash::SaltString;
use base64::Engine;
use chacha20poly1305::KeyInit;
use chacha20poly1305::aead::Aead;
use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
use zeroize::Zeroize;

#[derive(Serialize, Debug)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
struct LoginErrorResponse {
    error: Option<String>,
    timeout: Option<i64>,
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
        .await.map_err(|_| crate::Error::ServerNotAvailable)?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body_text = response.text().await.unwrap_or_default();
        if status == 500 {
            return Err(crate::errors::Error::RequestError((
                500,
                "Internal server error, you will be not logged in".to_string(),
            )));
        } else if status == 401 {
            if body_text.contains("session_expired") {
                return Err(crate::errors::Error::OnlineSessionExpired);
            }

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

#[derive(Debug, Serialize, Deserialize)]
struct PreLoginRequest {
    pub email: String,
}

//create login request struct
#[derive(Debug, Serialize, Deserialize)]

struct LoginRequest {
    pub email: String,
    pub password_hash: String,
    pub device_id: String,
}

#[derive(serde::Deserialize)]
struct PreLoginResponse {
    password_salt: String,
}

#[derive(serde::Deserialize)]
struct LoginResponse {
    access_token: AccessToken,
    refresh_token: RefreshToken,
    user_id: String,
    master_key_enc: String,
    master_key_nonce: String,
    kek_salt: String,
    params: ArgonParams,
}

pub async fn login(
    email: String,
    password: zeroize::Zeroizing<String>,
    client: Client,
    device_id: &uuid::Uuid,
) -> Result<(AccessToken, String, Vec<u8>), crate::errors::Error> {
    let argon2 = Argon2::default();
    crate::services::online_auth::register::verify_email(&email)
        .map_err(|_| crate::errors::Error::WrongEmail)?;

    let request = PreLoginRequest {
        email: email.clone(),
    };
    let response = client
        .post(format!("{}auth/pre-login", SERVER_ADDRESS))
        .json(&request)
        .send()
        .await.map_err(|_| crate::Error::ServerNotAvailable)?;
    if !response.status().is_success() {
        return Err(crate::errors::Error::RequestError((
            response.status().as_u16(),
            "Error while logging in".to_string(),
        )));
    }

    let response = response.json::<PreLoginResponse>().await.map_err(|_| {
        crate::errors::Error::InternalError("Failed to decode response".to_string())
    })?;
    let salt =
        SaltString::from_b64(&response.password_salt).context("failed to create salt string")?;
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .context("failed to hash password")?;

    let login_request: LoginRequest = LoginRequest {
        email: email.clone(),
        password_hash: hash.to_string(),
        device_id: device_id.to_string(),
    };

    let result = client
        .post(format!("{}auth/login", SERVER_ADDRESS))
        .json(&login_request)
        .send()
        .await.map_err(|_| crate::Error::ServerNotAvailable)?;

    if !result.status().is_success() {
        let status = result.status().as_u16();
        let body = result.text().await.unwrap_or_default();
        if status == 401 {
            if let Ok(server_error) = serde_json::from_str::<LoginErrorResponse>(&body) {
                if let Some(timeout_until) = server_error.timeout {
                    let timeout_left = timeout_until.saturating_sub(crate::utils::get_time());
                    return Err(crate::errors::Error::AccountLocked(timeout_left.max(0)));
                }

                if let Some(error) = server_error.error {
                    return match error.as_str() {
                        "wrong password" => Err(crate::errors::Error::WrongPassword),
                        "invalid_credentials" => Err(crate::errors::Error::WrongCredentials),
                        _ => Err(crate::errors::Error::WrongCredentials),
                    };
                }
            }

            if body.contains("wrong password") {
                return Err(crate::errors::Error::WrongPassword);
            }
            if body.contains("invalid_credentials") {
                return Err(crate::errors::Error::WrongCredentials);
            }
            if body.contains("timeout") {
                return Err(crate::errors::Error::WrongCredentials);
            }
            return Err(crate::errors::Error::WrongCredentials);
        }
        if status == 500 {
            return Err(crate::errors::Error::RequestError((
                500,
                "Error while logging in".to_string(),
            )));
        }

        let err = body;
        return Err(anyhow::anyhow!("server error: {}", err).into());
    }

    let result = result
        .json::<LoginResponse>()
        .await
        .context("Failed to decode response from server")?;

    let entry = keyring::Entry::new(
        "llava_desktop",
        &format!("refresh_token_id:{}", &result.user_id),
    )
    .context("failed to create keyring entry")?;
    entry
        .set_password(&result.refresh_token.0)
        .context("failed to store refresh token in keyring")?;

    let params = argon2::Params::new(
        result.params.m_cost,
        result.params.t_cost,
        result.params.p_cost,
        None,
    )
    .context("failed to create params")?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    let mut kek_bytes = [0u8; KEY_ENCRYPTED_KEY_LENGTH];
    argon2
        .hash_password_into(
            password.as_bytes(),
            result.kek_salt.as_bytes(),
            &mut kek_bytes,
        )
        .context("failed to derive online KEK")?;
    let master_key_enc = base64::engine::general_purpose::STANDARD
        .decode(result.master_key_enc)
        .context("failed to decode from base64")?;
    let master_key_nonce = base64::engine::general_purpose::STANDARD
        .decode(result.master_key_nonce)
        .context("failed to decode from base64")?;

    let kek = chacha20poly1305::ChaCha20Poly1305::new(&kek_bytes.into());
    let nonce = chacha20poly1305::Nonce::from_slice(&master_key_nonce);
    let notes_key = kek
        .decrypt(nonce, master_key_enc.as_ref())
        .map_err(|_| anyhow::anyhow!("failed to decrypt master key"))
        .context("master_key_enc decryption failed")?;

    kek_bytes.zeroize();
    

    Ok((result.access_token, result.user_id, notes_key))
}

//TODO logged in online account indicator 4. if not logged in but linked initialize workek which tries to login every while when online 5. mailer active account link + password recovery (it may be done later)
