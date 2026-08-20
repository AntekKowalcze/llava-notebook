//! # Online account registration module
//!
//! **Purpose**: This module handles registration of a local account with the online service,
//! validation of the user's email address, and synchronisation of online account information
//! with the local SQLite database.
//!
//! ## Exports
//! * [`register`] — Derives the password hash and online key-encryption key, wraps the local
//!   notes encryption key, registers the user and device with the server, and securely stores
//!   the returned refresh token in the system keyring.
//! * [`verify_email`] — Validates the supplied email address.
//! * [`change_email_in_database`] — Stores the online account email and online account identifier
//!   in the local users database.
//!
//! ## Key design decisions
//! The user's password is never sent to the server in plaintext. Argon2 is used both for the
//! password hash and for deriving the key-encryption key used to wrap the local notes encryption
//! key.
//!
//! The derived KEK material is stored in a temporary byte array and explicitly zeroized after
//! the notes key has been wrapped. Password arguments use `Zeroizing<String>` so that their
//! contents are cleared when they are dropped.
//!
//! The local notes encryption key is never logged or sent to the server in plaintext. Only its
//! encrypted representation and the corresponding nonce are included in the registration
//! request.
//!
//! Refresh tokens are stored using the operating system keyring rather than the application
//! database or regular filesystem storage.
//!
//! Authentication failures and server errors are logged without logging passwords, tokens,
//! encryption keys, or other sensitive authentication material.
//!
//! ## Dependencies
//! - `argon2` — Password hashing and key derivation
//! - `chacha20poly1305` — Encryption of the local notes encryption key
//! - `zeroize` — Securely clears sensitive temporary key material
//! - `reqwest` — Communicates with the online authentication server
//! - `keyring` — Securely stores the refresh token
//! - `rusqlite` — Updates local online-account information
//! - `regex` — Validates email addresses
//! - `anyhow` — Adds error context

use crate::constants::SERVER_ADDRESS;
use crate::services::online_auth::models::online_account::{
    AccessToken, ArgonParams, RegisterDevicePayload, RegisterRequest, RegisterUserPayload, Tokens,
};
use anyhow::Context;
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use chacha20poly1305::{AeadCore, ChaCha20Poly1305, Key, KeyInit, aead::Aead};
use regex::Regex;
use rusqlite::{Connection, named_params};
use zeroize::Zeroize;
pub async fn register(
    client: &reqwest::Client,
    email: String,
    password: zeroize::Zeroizing<String>,
    password_repeated: zeroize::Zeroizing<String>,
    device_id: uuid::Uuid,
    notes_key: &chacha20poly1305::Key,
) -> Result<(AccessToken, String), crate::errors::Error> {
    tracing::debug!(
        task = "online registration",
        "starting online account registration"
    );

    verify_email(&email)?;

    crate::services::local_auth::register::password_validation(&password, &password_repeated)?;

    let argon2 = Argon2::default();

    let password_salt = SaltString::generate(&mut OsRng);

    let password_hashed = argon2
        .hash_password(password.as_bytes(), &password_salt)
        .context("failed to generate password hash")?
        .to_string();

    let password_salt = password_salt
        .as_str()
        .split('$')
        .last()
        .unwrap()
        .to_string();

    let kek_salt = SaltString::generate(&mut OsRng);

    let mut kek_bytes = [0u8; 32];

    argon2
        .hash_password_into(
            password.as_bytes(),
            kek_salt.as_str().as_bytes(),
            &mut kek_bytes,
        )
        .context("failed to derive online KEK")?;

    let kek = ChaCha20Poly1305::new(Key::from_slice(&kek_bytes));

    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

    let wrapped_notes_key = kek
        .encrypt(&nonce, notes_key.as_slice())
        .map_err(|_| anyhow::anyhow!("failed to encrypt notes key with online KEK"))
        .context("failed to wrap notes_key for online registration")?;

    kek_bytes.zeroize();

    tracing::debug!(
        task = "online registration",
        status = "success",
        "password hash and notes key wrapping prepared"
    );

    let params = argon2::Params::default();

    let argon2_params = ArgonParams {
        m_cost: params.m_cost(),
        t_cost: params.t_cost(),
        p_cost: params.p_cost(),
    };

    let user = RegisterUserPayload {
        email,
        password_hash: password_hashed,
        password_salt,
        kek_salt: kek_salt.to_string(),
        master_key_enc: wrapped_notes_key,
        master_key_nonce: nonce.to_vec(),
        argon2_params,
    };

    let device = RegisterDevicePayload {
        device_id,
        device_name: crate::utils::get_host_name().context("failed to get hostname")?,
    };

    let request = RegisterRequest { user, device };

    let res = client
        .post(format!("{}auth/register", SERVER_ADDRESS))
        .json(&request)
        .send()
        .await
        .map_err(|_| {
            tracing::error!(
                task = "online registration",
                status = "error",
                "server is not available"
            );

            crate::Error::ServerNotAvailable
        })?;

    if !res.status().is_success() {
        let status = res.status().as_u16();

        if status == 409 {
            tracing::warn!(
                task = "online registration",
                status = "error",
                "registration rejected because email is already used"
            );

            return Err(crate::errors::Error::EmailAlreadyUsed);
        }

        let err = res.text().await.unwrap_or_default();

        tracing::error!(
            task = "online registration",
            status = "error",
            http_status = status,
            error = %err,
            "server rejected registration request"
        );

        return Err(anyhow::anyhow!("server error: {}", err).into());
    }

    let tokens = res
        .json::<Tokens>()
        .await
        .context("failed to parse response")
        .map_err(|e| {
            tracing::error!(
                task = "online registration",
                status = "error",
                error = ?e,
                "failed to parse registration response"
            );

            e
        })?;

    let entry = keyring::Entry::new(
        "llava_desktop",
        &format!("refresh_token_id:{}", &tokens.user_id),
    )
    .context("failed to create keyring entry")
    .map_err(|e| {
        tracing::error!(
            task = "online registration",
            status = "error",
            error = ?e,
            "failed to create refresh token keyring entry"
        );

        e
    })?;

    entry
        .set_password(&tokens.refresh_token.0)
        .context("failed to store refresh token in keyring")
        .map_err(|e| {
            tracing::error!(
                task = "online registration",
                status = "error",
                error = ?e,
                "failed to store refresh token in keyring"
            );

            e
        })?;

    tracing::info!(
        task = "online registration",
        status = "success",
        %tokens.user_id,
        "online account registered successfully"
    );

    Ok((tokens.access_token, tokens.user_id))
}
pub fn verify_email(email: &str) -> Result<(), crate::errors::Error> {
    let re = Regex::new(
        r"[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*@(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?",
    )
    .unwrap();

    if !re.is_match(email) {
        tracing::warn!(
            task = "email validation",
            status = "error",
            "invalid email address"
        );

        Err(crate::errors::Error::WrongEmail)
    } else {
        tracing::debug!(
            task = "email validation",
            status = "success",
            "email address validated successfully"
        );

        Ok(())
    }
}

pub fn change_email_in_database(
    email: &str,
    users_db: &Connection,
    id: &str,
    user_id: String,
) -> Result<(), crate::errors::Error> {
    tracing::debug!(
        task = "update online account",
        %user_id,
        "updating local online account information"
    );

    users_db
        .execute(
            "UPDATE users_data
             SET online_account_email = :mail,
                 online_account_id = :online_id,
                 is_online_linked = 1
             WHERE user_id = :id;",
            named_params! {
                ":mail": email,
                ":online_id": id,
                ":id": user_id,
            },
        )
        .context("failed to update mail in users_db locally")
        .map_err(|e| {
            tracing::error!(
                task = "update online account",
                status = "error",
                error = ?e,
                "failed to update local online account information"
            );

            e
        })?;

    tracing::debug!(
        task = "update online account",
        status = "success",
        "local online account information updated successfully"
    );

    Ok(())
}
