//! # Online logout module
//!
//! **Purpose**: This module handles logging a user out from the online service and removes
//! the locally stored refresh token from the operating system keyring.
//!
//! ## Exported items
//! * [`logout`] — Invalidates the online session on the server and removes the associated
//!   refresh token from the local keyring.
//!
//! ## Key design decisions
//! The access token is sent to the server using the HTTP `Authorization` header and is never
//! written to logs. The refresh token is stored in the operating system keyring and is deleted
//! after a successful server-side logout.
//!
//! ## Dependencies
//! - `reqwest` — Sends the logout request to the online authentication server
//! - `keyring` — Stores and removes the local refresh token
//! - `serde` — Serialises the logout request
//! - `tracing` — Provides authentication diagnostics without logging secrets

use anyhow::Context;
use reqwest::Client;

use crate::services::online_auth::models::online_account::AccessToken;
use serde::Serialize;

#[derive(Serialize)]
struct LogoutRequest {
    pub user_id: String,
    pub device_id: uuid::Uuid,
}

/// Logs the user out from the online service and removes the local refresh token.
///
/// The server is contacted first so that the online session can be invalidated. Only after
/// a successful server response is the refresh token removed from the operating system
/// keyring.
///
/// Sensitive authentication material such as the access token and refresh token is never
/// included in logs.
///
/// # Errors
/// Returns an error if the server cannot be reached, rejects the logout request, the keyring
/// entry cannot be created, or the refresh token cannot be deleted.
pub async fn logout(
    user_id: String,
    client: Client,
    device_id: &uuid::Uuid,
    access_token: &AccessToken,
) -> Result<(), crate::errors::Error> {
    tracing::debug!(
        task = "online logout",
        %user_id,
        device_id = %device_id,
        "starting online logout"
    );

    let request = LogoutRequest {
        user_id: user_id.clone(),
        device_id: *device_id,
    };

    let res = client
        .post(format!(
            "{}auth/logout",
            crate::constants::SERVER_ADDRESS
        ))
        .bearer_auth(&access_token.0)
        .json(&request)
        .send()
        .await
        .map_err(|e| {
            tracing::error!(
                task = "online logout",
                status = "error",
                %user_id,
                error = ?e,
                "failed to send logout request"
            );

            crate::Error::ServerNotAvailable
        })?;

    if !res.status().is_success() {
        let status = res.status().as_u16();
        let body = res.text().await.unwrap_or_default();

        tracing::warn!(
            task = "online logout",
            status = "error",
            %user_id,
            http_status = status,
            "server rejected logout request"
        );

        return Err(crate::errors::Error::RequestError((status, body)));
    }

    tracing::debug!(
        task = "online logout",
        %user_id,
        "online session invalidated successfully"
    );

    let entry = keyring::Entry::new(
        "llava_desktop",
        &format!("refresh_token_id:{}", user_id),
    )
    .map_err(|e| {
        tracing::error!(
            task = "online logout",
            status = "error",
            %user_id,
            error = ?e,
            "failed to create keyring entry"
        );

        crate::errors::Error::NotLoggedIn
    })?;

    entry
        .delete_credential()
        .context("failed to delete refresh token from keyring")
        .map_err(|e| {
            tracing::error!(
                task = "online logout",
                status = "error",
                %user_id,
                error = ?e,
                "failed to delete refresh token from keyring"
            );

            e
        })?;

    tracing::debug!(
        task = "online logout",
        status = "success",
        %user_id,
        "online logout completed successfully"
    );

    Ok(())
}