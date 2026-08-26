//! # Attachment protocol module
//! **Purpose**: Registers and implements the custom `attachment://` URI scheme protocol,
//! which serves decrypted attachment bytes (images, files) to the frontend webview by
//! attachment UUID. This lets the note editor reference attachments as ordinary URLs
//! (e.g. `attachment://<uuid>`) without ever exposing plaintext attachment data on disk
//! or over a network-visible channel.
//!
//! ## Exported items
//! * [`register`] — Registers the `attachment` scheme handler onto a [`tauri::Builder`].
//!   Call this once during app setup, before `.run()`.
//!
//! ## Key design decisions
//! The attachment id is read from the request URI's **host** component, not its path.
//! `attachment://<uuid>` places the identifier right after `//`, which the URI spec
//! parses as authority/host, not path.
//!
//! Registered via `register_asynchronous_uri_scheme_protocol` rather than the sync
//! variant, since resolving a request means locking shared state, running a SQLite
//! query, and decrypting the payload — none of which should block the UI thread. Work
//! is dispatched onto `std::thread::spawn`; swap for `tauri::async_runtime::spawn` if
//! `read_attachment` ever becomes a true `async fn`.
//!
//!
//! ## Dependencies
//! - `tauri` — `Builder::register_asynchronous_uri_scheme_protocol`, `UriSchemeContext`, `AppHandle`
//! - `http` — request/response types and status codes
//! - `mime` — `Content-Type` value for plain-text error bodies
//! - `rusqlite` — attachment metadata lookup (`mime_type` column)
//! - `llava_core::AppState`, `llava_core::attachments::read_attachment` — shared state and decryption

use llava_core::AppState;
use tauri::Manager;

/// Registers the `attachment://<uuid>` protocol handler on the given builder.
///
/// # Errors
/// Never returns a `Result` itself — the underlying protocol API has no error channel
/// back to the caller. Failures are surfaced as HTTP status codes in the response
/// instead: `401` for missing session state (not logged in), `404` for an unknown
/// attachment id, `500` for lock/DB/decryption failures.
pub fn register(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder.register_asynchronous_uri_scheme_protocol("attachment", handle_request)
}

#[cfg(not(windows))]
fn handle_request(
    ctx: tauri::UriSchemeContext<'_, tauri::Wry>,
    request: http::Request<Vec<u8>>,
    responder: tauri::UriSchemeResponder,
) {
    #[cfg(windows)]
    let attachment_id = request.uri().path().trim_start_matches('/').to_string();

    #[cfg(not(windows))]
    let attachment_id = request.uri().host().unwrap_or_default().to_string();

    let app_handle = ctx.app_handle().clone();

    std::thread::spawn(move || {
        responder.respond(resolve_attachment(&app_handle, &attachment_id));
    });
}

/// Looks up and decrypts a single attachment, returning the finished response.
/// Kept separate from [`handle_request`] so each failure path is one early `return`
/// instead of nested nested `match` arms all calling `responder.respond`.
fn resolve_attachment(
    app_handle: &tauri::AppHandle,
    attachment_id: &str,
) -> http::Response<Vec<u8>> {
    let state = app_handle.state::<AppState>();

    let notes_db_guard = match state.notes_db.lock() {
        Ok(guard) => guard,
        Err(_) => {
            return error_response(
                http::StatusCode::INTERNAL_SERVER_ERROR,
                "failed to lock notes database",
            )
        }
    };
    let Some(notes_db) = notes_db_guard.as_ref() else {
        return error_response(
            http::StatusCode::UNAUTHORIZED,
            "notes database is not available",
        );
    };

    let notes_key_guard = match state.notes_key.lock() {
        Ok(guard) => guard,
        Err(_) => {
            return error_response(
                http::StatusCode::INTERNAL_SERVER_ERROR,
                "failed to lock notes key",
            )
        }
    };
    let Some(notes_key) = notes_key_guard.as_ref() else {
        return error_response(http::StatusCode::UNAUTHORIZED, "notes key is not available");
    };

    let mime_type: String = match notes_db.query_row(
        "SELECT mime_type FROM attachments WHERE attachment_id = ?1",
        rusqlite::params![attachment_id],
        |row| row.get(0),
    ) {
        Ok(mime_type) => mime_type,
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            return error_response(http::StatusCode::NOT_FOUND, "attachment not found");
        }
        Err(_) => {
            return error_response(
                http::StatusCode::INTERNAL_SERVER_ERROR,
                "failed to get attachment metadata",
            )
        }
    };

    let data = match llava_core::attachments::read_attachment(
        notes_key,
        notes_db,
        attachment_id.to_string(),
    ) {
        Ok(data) => data,
        Err(_) => {
            return error_response(
                http::StatusCode::INTERNAL_SERVER_ERROR,
                "failed to read attachment",
            )
        }
    };

    http::Response::builder()
        .status(http::StatusCode::OK)
        .header(http::header::CONTENT_TYPE, mime_type)
        .body(data)
        .unwrap()
}

fn error_response(status: http::StatusCode, message: &'static str) -> http::Response<Vec<u8>> {
    http::Response::builder()
        .status(status)
        .header(http::header::CONTENT_TYPE, mime::TEXT_PLAIN.essence_str())
        .body(message.as_bytes().to_vec())
        .unwrap()
}
