//! # Online authentication module
//!
//! **Purpose**: This module handles authentication against the online backend, including
//! refresh-token based session restoration, password-based login, online key derivation,
//! and recovery of the local notes encryption key.
//!
//! ## Exported items
//! * [`RefreshRequest`] — Request payload used to refresh an online session.
//! * [`check_if_logged_in_online`] — Restores an online session using the refresh token stored
//!   in the system keyring.
//! * [`login`] — Authenticates the user against the online server and decrypts the user's
//!   notes encryption key.
//!
//! ## Key design decisions
//! Passwords are received as [`zeroize::Zeroizing<String>`] so the password buffer is cleared
//! when it is dropped. The derived KEK is stored in a fixed-size byte array and explicitly
//! zeroized after use.
//!
//! Refresh tokens are stored in the operating system keyring rather than application files.
//! Access tokens, refresh tokens, passwords, password hashes, encryption keys, and other
//! sensitive cryptographic material are never written to logs.
//!
//! The online login flow consists of two requests: a pre-login request retrieves the password
//! salt, then the password is hashed locally and submitted to the login endpoint. The server
//! returns an encrypted master key, which is decrypted locally using a KEK derived from the
//! user's password.
//!
//! ## Dependencies
//! - `reqwest` — HTTP communication with the online authentication server
//! - `argon2` — Password hashing and KEK derivation
//! - `chacha20poly1305` — Decryption of the encrypted notes key
//! - `keyring` — Secure storage of refresh tokens
//! - `zeroize` — Clearing sensitive key material from memory
//! - `serde` — Serialisation and deserialisation of authentication payloads
//! - `base64` — Encoding and decoding encrypted key material
//! - `tracing` — Authentication diagnostics without logging secrets

use crate::constants::KEY_ENCRYPTED_KEY_LENGTH;
use crate::constants::SERVER_ADDRESS;
use crate::models::online_account::{AccessToken,RefreshToken,RefreshResponse,ArgonParams};
use anyhow::Context;
use argon2::Argon2;
use argon2::PasswordHasher;
use argon2::password_hash::SaltString;
use base64::Engine;
use chacha20poly1305::Key;
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

/// Attempts to restore an online session using the refresh token stored in the system keyring.
///
/// A successful refresh replaces the stored refresh token with the newly issued one.
/// Sensitive token values are intentionally excluded from logs.
///
/// # Errors
/// Returns an error if the refresh token cannot be retrieved, the server is unavailable,
/// the session has expired, the response cannot be decoded, or the new refresh token
/// cannot be stored.
pub async fn check_if_logged_in_online(
    online_id: &str,
    client: Client,
) -> Result<crate::models::online_account::AccessToken, crate::errors::Error>
{
    tracing::debug!(
        task = "online session refresh",
        %online_id,
        "starting online session refresh"
    );

    let entry = keyring::Entry::new("llava_desktop", &format!("refresh_token_id:{}", online_id))
        .map_err(|e| {
            tracing::error!(
                task = "online session refresh",
                status = "error",
                %online_id,
                error = ?e,
                "failed to create keyring entry"
            );

            crate::errors::Error::NotLoggedIn
        })?;

    let refresh_token = entry.get_password().map_err(|e| {
        tracing::error!(
            task = "online session refresh",
            status = "error",
            %online_id,
            error = ?e,
            "failed to retrieve refresh token from keyring"
        );

        crate::errors::Error::NotLoggedIn
    })?;

    let req = RefreshRequest { refresh_token };

    let response = client
        .post(format!("{}auth/refresh", SERVER_ADDRESS))
        .json(&req)
        .send()
        .await
        .map_err(|e| {
            tracing::error!(
                task = "online session refresh",
                status = "error",
                %online_id,
                error = ?e,
                "server request failed"
            );

            crate::Error::ServerNotAvailable
        })?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body_text = response.text().await.unwrap_or_default();

        tracing::warn!(
            task = "online session refresh",
            status = "error",
            %online_id,
            http_status = status,
            "online session refresh rejected by server"
        );

        if status == 500 {
            return Err(crate::errors::Error::RequestError((
                500,
                "Internal server error, you will be not logged in".to_string(),
            )));
        }

        if status == 401 {
            if body_text.contains("session_expired") {
                tracing::info!(
                    task = "online session refresh",
                    %online_id,
                    "online session expired"
                );

                return Err(crate::errors::Error::OnlineSessionExpired);
            }

            return Err(crate::errors::Error::NotLoggedIn);
        }

        return Err(crate::errors::Error::NotLoggedIn);
    }

    let tokens = response
        .json::<RefreshResponse>()
        .await
        .context("failed to parse response")
        .map_err(|e| {
            tracing::error!(
                task = "online session refresh",
                status = "error",
                %online_id,
                error = ?e,
                "failed to decode refresh response"
            );

            e
        })?;

    entry
        .set_password(&tokens.refresh_token.0)
        .context("failed to save refresh token in keyring")
        .map_err(|e| {
            tracing::error!(
                task = "online session refresh",
                status = "error",
                %online_id,
                error = ?e,
                "failed to store refreshed token in keyring"
            );

            e
        })?;

    tracing::debug!(
        task = "online session refresh",
        status = "success",
        %online_id,
        "online session refreshed successfully"
    );

    Ok(tokens.access_token)
}

#[derive(Debug, Serialize, Deserialize)]
struct PreLoginRequest {
    pub email: String,
}

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

/// Authenticates a user and locally decrypts the user's notes encryption key.
///
/// The password is first used to reproduce the server-side password hash. After successful
/// authentication, a second Argon2 derivation produces the KEK used to decrypt the encrypted
/// notes key returned by the server.
///
/// # Errors
/// Returns an error if email validation, network communication, password hashing, response
/// decoding, key derivation, key decryption, or refresh-token storage fails.
pub async fn login(
    email: String,
    password: zeroize::Zeroizing<String>,
    client: Client,
    device_id: &uuid::Uuid,
) -> Result<(AccessToken, String, Vec<u8>), crate::errors::Error> {
    tracing::debug!(task = "online login", "starting online login");

    let argon2 = Argon2::default();

    crate::services::online_auth::register::verify_email(&email).map_err(|e| {
        tracing::warn!(
            task = "online login",
            status = "error",
            error = ?e,
            "email validation failed"
        );

        crate::errors::Error::WrongEmail
    })?;

    tracing::debug!(task = "online login", "email validation successful");

    let request = PreLoginRequest {
        email: email.clone(),
    };

    let response = client
        .post(format!("{}auth/pre-login", SERVER_ADDRESS))
        .json(&request)
        .send()
        .await
        .map_err(|e| {
            tracing::error!(
                task = "online pre-login",
                status = "error",
                error = ?e,
                "pre-login request failed"
            );

            crate::Error::ServerNotAvailable
        })?;

    if !response.status().is_success() {
        let status = response.status().as_u16();

        tracing::warn!(
            task = "online pre-login",
            status = "error",
            http_status = status,
            "pre-login request rejected by server"
        );

        return Err(crate::errors::Error::RequestError((
            status,
            "Error while logging in".to_string(),
        )));
    }

    let response = response.json::<PreLoginResponse>().await.map_err(|e| {
        tracing::error!(
            task = "online pre-login",
            status = "error",
            error = ?e,
            "failed to decode pre-login response"
        );

        crate::errors::Error::InternalError("Failed to decode response".to_string())
    })?;

    tracing::debug!(task = "online login", "received password salt from server");

    let salt = SaltString::from_b64(&response.password_salt)
        .context("failed to create salt string")
        .map_err(|e| {
            tracing::error!(
                task = "online login",
                status = "error",
                error = ?e,
                "failed to parse password salt"
            );
            e
        })?;
    //issues may come from chagned hash method in go login method
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .context("failed to hash password")
        .map_err(|e| {
            tracing::error!(
                task = "online login",
                status = "error",
                error = ?e,
                "failed to hash password"
            );

            e
        })?;

    let login_request = LoginRequest {
        email: email.clone(),
        password_hash: hash.to_string(),
        device_id: device_id.to_string(),
    };

    tracing::debug!(task = "online login", "sending authentication request");

    let result = client
        .post(format!("{}auth/login", SERVER_ADDRESS))
        .json(&login_request)
        .send()
        .await
        .map_err(|e| {
            tracing::error!(
                task = "online login",
                status = "error",
                error = ?e,
                "login request failed"
            );

            crate::Error::ServerNotAvailable
        })?;

    if !result.status().is_success() {
        let status = result.status().as_u16();
        let body = result.text().await.unwrap_or_default();

        tracing::warn!(
            task = "online login",
            status = "error",
            http_status = status,
            "authentication request rejected by server"
        );

        if status == 401 {
            if let Ok(server_error) = serde_json::from_str::<LoginErrorResponse>(&body) {
                if let Some(timeout_until) = server_error.timeout {
                    let timeout_left = timeout_until.saturating_sub(crate::utils::get_time());

                    tracing::warn!(
                        task = "online login",
                        status = "error",
                        timeout_left,
                        "account is temporarily locked"
                    );

                    return Err(crate::errors::Error::AccountLocked(timeout_left.max(0)));
                }

                if let Some(error) = server_error.error {
                    return match error.as_str() {
                        "wrong password" => {
                            tracing::warn!(
                                task = "online login",
                                status = "error",
                                "incorrect password"
                            );

                            Err(crate::errors::Error::WrongPassword)
                        }

                        "invalid_credentials" => {
                            tracing::warn!(
                                task = "online login",
                                status = "error",
                                "invalid login credentials"
                            );

                            Err(crate::errors::Error::WrongCredentials)
                        }

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
            tracing::error!(
                task = "online login",
                status = "error",
                "server returned internal error during login"
            );

            return Err(crate::errors::Error::RequestError((
                500,
                "Error while logging in".to_string(),
            )));
        }

        return Err(anyhow::anyhow!("server error: {}", body).into());
    }

    let result = result
        .json::<LoginResponse>()
        .await
        .context("failed to decode response from server")
        .map_err(|e| {
            tracing::error!(
                task = "online login",
                status = "error",
                error = ?e,
                "failed to decode login response"
            );

            e
        })?;

    tracing::debug!(
        task = "online login",
        user_id = %result.user_id,
        "authentication successful"
    );

    let params = argon2::Params::new(
        result.params.m_cost,
        result.params.t_cost,
        result.params.p_cost,
        None,
    )
    .context("failed to create params")
    .map_err(|e| {
        tracing::error!(
            task = "online login",
            status = "error",
            user_id = %result.user_id,
            error = ?e,
            "failed to create Argon2 parameters"
        );

        e
    })?;

    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    let mut kek_bytes = [0u8; KEY_ENCRYPTED_KEY_LENGTH];

    argon2
        .hash_password_into(
            password.as_bytes(),
            result.kek_salt.as_bytes(),
            &mut kek_bytes,
        )
        .context("failed to derive online KEK")
        .map_err(|e| {
            tracing::error!(
                task = "online login",
                status = "error",
                user_id = %result.user_id,
                error = ?e,
                "failed to derive online KEK"
            );
            e
        })?;

    let master_key_enc = base64::engine::general_purpose::STANDARD
        .decode(&result.master_key_enc)
        .context("failed to decode encrypted master key")
        .map_err(|e| {
            tracing::error!(
                task = "online login",
                status = "error",
                user_id = %result.user_id,
                error = ?e,
                "failed to decode encrypted master key"
            );
            e
        })?;

    let master_key_nonce = base64::engine::general_purpose::STANDARD
        .decode(&result.master_key_nonce)
        .context("failed to decode master key nonce")
        .map_err(|e| {
            tracing::error!(
                task = "online login",
                status = "error",
                user_id = %result.user_id,
                error = ?e,
                "failed to decode master key nonce"
            );
            e
        })?;

    if master_key_nonce.len() != 12 {
        kek_bytes.zeroize();

        return Err(anyhow::anyhow!(
            "invalid master key nonce length: expected 12 bytes, got {}",
            master_key_nonce.len()
        )
        .into());
    }

    let kek = chacha20poly1305::ChaCha20Poly1305::new(Key::from_slice(&kek_bytes));

    let nonce = chacha20poly1305::Nonce::from_slice(&master_key_nonce);

    let notes_key = kek
        .decrypt(nonce, master_key_enc.as_ref())
        .map_err(|_| {
            tracing::error!(
                task = "online login",
                status = "error",
                user_id = %result.user_id,
                "failed to decrypt notes encryption key"
            );

            anyhow::anyhow!("failed to decrypt master key")
        })
        .context("master_key_enc decryption failed")?;

    kek_bytes.zeroize();

    if notes_key.len() != 32 {
        return Err(anyhow::anyhow!(
            "invalid notes key length: expected 32 bytes, got {}",
            notes_key.len()
        )
        .into());
    }

    let entry = keyring::Entry::new(
        "llava_desktop",
        &format!("refresh_token_id:{}", &result.user_id),
    )
    .context("failed to create keyring entry")
    .map_err(|e| {
        tracing::error!(
            task = "online login",
            status = "error",
            user_id = %result.user_id,
            error = ?e,
            "failed to create keyring entry"
        );
        e
    })?;

    entry
        .set_password(&result.refresh_token.0)
        .context("failed to store refresh token in keyring")
        .map_err(|e| {
            tracing::error!(
                task = "online login",
                status = "error",
                user_id = %result.user_id,
                error = ?e,
                "failed to store refresh token in keyring"
            );
            e
        })?;

    tracing::debug!(
        task = "online login",
        status = "success",
        user_id = %result.user_id,
        "online login completed successfully"
    );

    Ok((result.access_token, result.user_id, notes_key))
}
