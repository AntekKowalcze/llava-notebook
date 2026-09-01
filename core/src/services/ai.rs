//! # AI request module
//!
//! **Purpose**: This module provides the data structures and HTTP request
//! logic required to send AI prompts from the client to the remote server.
//!
//! It packages the document context, current text selection, and user
//! instruction into a serializable request and authenticates the request
//! using the user's online access token.
//!
//! ## Exports
//!
//! * [`AiPromptContext`] — Request payload containing the document context,
//!   selected text, and instruction that should be sent to the AI service.
//! * [`send_ai_request`] — Sends an authenticated AI request to the server
//!   and maps HTTP or transport failures to application errors.
//!
//! ## Key design decisions
//!
//! The request payload is represented by [`AiPromptContext`] and derives
//! [`Serialize`] and [`Deserialize`] so it can be encoded directly as JSON
//! when sent to the server.
//!
//! Authentication uses the online [`AccessToken`] through the HTTP
//! `Authorization: Bearer` header. The raw token is consumed only for
//! authentication and is not stored by this module.
//!
//! A `401 Unauthorized` response is mapped to
//! [`crate::errors::Error::OnlineSessionExpired`] so the caller can
//! distinguish an expired online session from other AI request failures.
//!
//! All other unsuccessful HTTP responses and request transport failures are
//! mapped to [`crate::errors::Error::AiError`]. Transport failures are also
//! logged through `tracing` without exposing the request payload or access
//! token.
//!
//! The function returns the original [`reqwest::Response`] for successful
//! requests, leaving response-body handling to the caller.
//!
//! ## Dependencies
//!
//! * [`serde`] — Serialization and deserialization of [`AiPromptContext`].
//! * [`reqwest`] — HTTP client and authenticated request handling.
//! * [`tracing`] — Logging of request transport failures.
//! * [`crate::online_auth::AccessToken`] — Online session authentication.
//! * [`crate::errors`] — Application-level AI and authentication errors.
//! * [`crate::constants`] — Server address used for the AI endpoint.

use serde::{Deserialize, Serialize};

use crate::online_auth::AccessToken;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiPromptContext {
    pub document: String,
    pub selection: String,
    pub instruction: String,
}

/// Sends an authenticated AI request containing the current document context
/// and user instruction.
///
/// # Errors
///
/// Returns [`crate::errors::Error::OnlineSessionExpired`] when the server
/// responds with HTTP `401 Unauthorized`.
///
/// Returns [`crate::errors::Error::AiError`] when the HTTP request cannot be
/// sent or the server responds with any other unsuccessful status code.
pub async fn send_ai_request(
    client: reqwest::Client,
    ctx: AiPromptContext,
    access_token: AccessToken,
) -> Result<reqwest::Response, crate::errors::Error> {
    let res = client
        .post(format!("{}ai/", crate::constants::SERVER_ADDRESS))
        .bearer_auth(access_token.0)
        .json(&ctx)
        .send()
        .await
        .map_err(|e| {
            tracing::error!(
                task = "ai request",
                status = "error",
                error = ?e,
                "failed to send ai request"
            );
            crate::errors::Error::AiError
        })?;

    if !res.status().is_success() {
        let code = res.status().as_u16();

        if code == 401 {
            return Err(crate::errors::Error::OnlineSessionExpired);
        }

        return Err(crate::errors::Error::AiError);
    }

    Ok(res)
}
