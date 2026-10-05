//! # Storage usage module
//!
//! **Purpose**: Reports how much space the user's notes and attachments take,
//! on this device and in the cloud.
//!
//! ## Exported items
//! * [`LocalStorageUsage`] — Bytes and counts of notes and attachments stored
//!   on this device.
//! * [`CloudStorageUsage`] — The user's attachment quota on the server.
//! * [`get_local_storage_usage`] — Measures the files referenced by the local
//!   notes database.
//! * [`fetch_cloud_storage_usage`] — Asks the server for the quota usage.
//!
//! ## Key design decisions
//! Local usage is measured from the files on disk rather than the sizes stored
//! in SQLite, so it reflects what the device really holds (encrypted bytes,
//! notes in the trash included). Files that are missing count as zero.
//!
//! Only attachments count towards the cloud quota; note text does not.
//!
//! ## Dependencies
//! - `rusqlite` — Lists note and attachment files
//! - `reqwest` — Requests the cloud usage
//! - `serde` — Serialises the usage for the frontend

use anyhow::Context;
use reqwest::Client;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::constants::SERVER_ADDRESS;
use crate::models::online_account::AccessToken;

#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct LocalStorageUsage {
    pub notes_count: i64,
    pub notes_bytes: u64,
    pub attachments_count: i64,
    pub attachments_bytes: u64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct CloudStorageUsage {
    pub used_bytes: i64,
    pub reserved_bytes: i64,
    pub limit_bytes: i64,
}

/// Sums the sizes of the files behind all local notes and attachments.
///
/// # Errors
/// Returns an error if the database cannot be queried.
pub fn get_local_storage_usage(
    notes_db: &Connection,
) -> Result<LocalStorageUsage, crate::errors::Error> {
    let (notes_count, notes_bytes) = sum_file_sizes(
        notes_db,
        "SELECT content_path FROM notes WHERE sync_state != 'WaitingForTombstone'",
    )?;
    let (attachments_count, attachments_bytes) =
        sum_file_sizes(notes_db, "SELECT local_path FROM attachments")?;

    Ok(LocalStorageUsage {
        notes_count,
        notes_bytes,
        attachments_count,
        attachments_bytes,
    })
}

/// Counts the files that exist and sums their sizes; `query` selects one path
/// column.
fn sum_file_sizes(notes_db: &Connection, query: &str) -> Result<(i64, u64), crate::errors::Error> {
    let mut statement = notes_db
        .prepare(query)
        .context("failed to prepare storage usage query")?;

    let paths = statement
        .query_map([], |row| row.get::<_, Option<String>>(0))
        .context("failed to query storage usage")?;

    let mut count = 0;
    let mut bytes = 0;
    for path in paths {
        let Some(path) = path.context("failed to read storage usage row")? else {
            continue;
        };
        if let Ok(metadata) = std::fs::metadata(&path) {
            count += 1;
            bytes += metadata.len();
        }
    }

    Ok((count, bytes))
}

/// Asks the server how much of the attachment quota is used.
///
/// Returns `Ok(None)` when the server does not offer the endpoint yet.
///
/// # Errors
/// Returns [`crate::errors::Error::OnlineSessionExpired`] when the access
/// token was rejected, and [`crate::errors::Error::ServerNotAvailable`] when
/// the request failed.
pub async fn fetch_cloud_storage_usage(
    client: Client,
    access_token: &AccessToken,
) -> Result<Option<CloudStorageUsage>, crate::errors::Error> {
    let response = client
        .get(format!("{}sync/storage", SERVER_ADDRESS))
        .bearer_auth(&access_token.0)
        .send()
        .await
        .map_err(|e| {
            tracing::error!(task = "storage usage", error = ?e, "failed to request storage usage");
            crate::errors::Error::ServerNotAvailable
        })?;

    let status = response.status();

    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(crate::errors::Error::OnlineSessionExpired);
    }

    if status == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }

    if !status.is_success() {
        tracing::error!(
            task = "storage usage",
            http_status = status.as_u16(),
            "storage usage request rejected"
        );
        return Err(crate::errors::Error::ServerNotAvailable);
    }

    let usage = response.json::<CloudStorageUsage>().await.map_err(|e| {
        tracing::error!(task = "storage usage", error = ?e, "failed to decode storage usage");
        crate::errors::Error::InternalError("Failed to decode storage usage".to_string())
    })?;

    Ok(Some(usage))
}
