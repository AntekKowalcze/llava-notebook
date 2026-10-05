//! # Cloud synchronization module
//!
//! **Purpose**: This module coordinates synchronization between the local
//! SQLite database and the remote cloud service.
//!
//! It prepares local notes and attachments for synchronization, communicates
//! with the synchronization API, uploads and downloads attachment data,
//! applies synchronization results to the local database, and maintains
//! transactional database state through queued operations.
//!
//! ## Exports
//!
//! * [`sync`] — Sends the local synchronization state to the server and
//!   retrieves the operations required to bring the client up to date.
//! * [`get_all_notes_to_sync`] — Collects all locally stored notes and their
//!   attachment metadata that need to participate in synchronization.
//! * [`get_note_for_upload`] — Loads a note and its content from local storage
//!   in the representation required by the server.
//! * [`get_attachment_for_upload`] — Loads an attachment and its metadata from
//!   local storage for upload.
//! * [`execute_db_operation`] — Applies one queued synchronization database
//!   operation to an existing SQLite transaction.
//! * [`execute_db_operations`] — Applies a sequence of synchronization
//!   operations atomically in a single SQLite transaction.
//! * [`DbOperation`] — Represents a database mutation produced by a
//!   synchronization operation.
//! * [`execute_server_operations`] — Executes pending note and attachment
//!   uploads/downloads and returns the database operations required to record
//!   their results locally.
//! * [`upload_notes`] — Uploads new or modified notes and creates the database
//!   operations required to store their cloud identifiers and versions.
//! * [`upload_attachments`] — Uploads attachment contents to S3 using
//!   presigned URLs and prepares the corresponding local synchronization
//!   operations.
//! * [`download_attachment`] — Downloads attachments from their presigned
//!   URLs, verifies their integrity, stores them locally, and creates the
//!   corresponding database operations.
//! * [`handle_attachments_to_hard_delete`] — Removes locally stored attachment
//!   files and creates database operations for their permanent deletion.
//! * [`handle_attachment_synced`] — Creates database operations marking a set
//!   of attachments as synchronized.
//! * [`handle_notes_synced`] — Creates database operations marking a set of
//!   notes as synchronized.
//! * [`handle_notes_to_hard_delete`] — Removes local note content files and
//!   creates database operations for their permanent deletion.
//! * [`handle_notes_to_download`] — Stores notes received from the cloud on
//!   local disk and creates database operations for inserting or updating
//!   their local records.
//! * [`complete_attachment_upload`] — Notifies the server that an attachment
//!   uploaded through a presigned URL has completed.
//!
//! ## Key design decisions
//!
//! Synchronization is split into two stages. The server first determines the
//! required synchronization actions through [`sync`], after which the client
//! executes those actions and converts their results into [`DbOperation`]
//! values. Local database changes are then applied atomically through
//! [`execute_db_operations`].
//!
//! Local synchronization state is represented explicitly through [`SyncState`]
//! and is used to distinguish synchronized data, pending changes, and
//! tombstones that require permanent deletion.
//!
//! Cloud note versions are used as optimistic concurrency metadata. The local
//! database stores the server-provided cloud version and uses it when
//! constructing subsequent synchronization requests, preventing stale local
//! state from silently overwriting newer cloud data.
//!
//! Note content and attachment data are handled separately from their SQLite
//! metadata. File contents are read from or written to the filesystem while
//! SQLite stores their paths and synchronization metadata.
//!
//! Encrypted content is treated as opaque synchronization data by this
//! module. Encrypted note content is sent and stored without attempting to
//! decrypt it, while cryptographic metadata is serialized alongside the
//! corresponding object.
//!
//! Attachment uploads use presigned S3 URLs. The synchronization service
//! therefore transfers attachment bytes directly between the client and S3
//! rather than routing file contents through the application server.
//!
//! Downloaded attachments are verified before being committed locally. The
//! reported size must match the received content and the BLAKE3 checksum of
//! the downloaded bytes must match the checksum provided by the server.
//!
//! Database mutations are represented as [`DbOperation`] values and committed
//! in a single SQLite transaction. This prevents partial synchronization state
//! from being persisted when one operation in a synchronization batch fails.
//!
//! Files created while processing a batch of downloaded notes are tracked so
//! that filesystem changes can be rolled back when a later step fails before
//! the database transaction is committed.
//!
//! Network operations use [`reqwest`] and map authentication failures to the
//! application's online-session error so that an expired access token can be
//! handled differently from other synchronization failures.
//!
//! ## Dependencies
//!
//! * [`reqwest`] — HTTP communication with the synchronization API and
//!   downloads from presigned URLs.
//! * [`rusqlite`] — Local SQLite queries, transactions, and synchronization
//!   state persistence.
//! * [`serde`] — Serialization and deserialization of synchronization payloads
//!   and cryptographic metadata.
//! * [`serde_json`] — Serialization and deserialization of cryptographic
//!   metadata stored with notes and attachments.
//! * [`base64`] — Encoding and decoding unencrypted note content.
//! * [`blake3`] — Integrity verification of downloaded attachment data.
//! * [`uuid`] — UUID generation and parsing for local and attachment
//!   identifiers.
//! * [`tokio`] — Asynchronous filesystem and network operations.
//! * [`anyhow`] — Adds contextual information to database, filesystem, and
//!   serialization failures.
//! * [`crate::constants`] — Provides server and note-storage configuration.
//! * [`crate::models::sync`] — Defines synchronization request, response, note,
//!   and attachment data structures.
//! * [`crate::services::attachment`] — Provides attachment cryptographic
//!   metadata structures.
//! * [`crate::storage`] — Provides local synchronization state definitions.
//! * [`crate::errors`] — Application-level synchronization and authentication
//!   errors.
//! * [`crate::models::online_account::AccessToken`] — Provides the access token
//!   used to authenticate cloud synchronization requests.

use crate::constants::{NOTE_EXTENSION, SERVER_ADDRESS, TEMP_NOTE_EXTENSION};
use crate::models::online_account::AccessToken;
use crate::models::sync::{
    AttachmentForUpload, AttachmentSyncCheck, CheckNoteSyncStatus, CheckSyncRequest,
    CheckSyncResponse, DownloadAttachment, DownloadNote, NoteForUpload,
};
use crate::{services::attachment::AttachmentCryptoMetadata, storage::SyncState};

use anyhow::Context;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use reqwest::{
    Client,
    header::{HeaderMap, HeaderValue},
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Deserialize;
use std::path;
use std::str::FromStr;

pub async fn sync(
    client: Client,
    notes_to_sync: Vec<CheckNoteSyncStatus>,
    access_token: &AccessToken,
    full_sync: bool,
) -> Result<CheckSyncResponse, crate::errors::Error> {
    let request = CheckSyncRequest {
        notes: notes_to_sync,
        full_sync,
    };

    let response = client
        .post(format!("{}sync/sync-check", SERVER_ADDRESS))
        .bearer_auth(&access_token.0)
        .json(&request)
        .send()
        .await
        .map_err(|e| {
            tracing::error!(
                task = "sync",
                error = ?e,
                full_sync,
                "failed to send sync-check"
            );

            crate::errors::Error::SyncFailed
        })?;

    let status = response.status();

    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(crate::errors::Error::OnlineSessionExpired);
    }

    if status == reqwest::StatusCode::CONFLICT {
        tracing::debug!(task = "sync", "server reports a sync already in progress");

        return Err(crate::errors::Error::SyncInProgress);
    }

    if !status.is_success() {
        tracing::error!(
            task = "sync",
            http_status = status.as_u16(),
            full_sync,
            "sync-check request rejected"
        );

        return Err(crate::errors::Error::SyncFailed);
    }

    response.json::<CheckSyncResponse>().await.map_err(|e| {
        tracing::error!(
            task = "sync",
            error = ?e,
            "failed to decode sync response"
        );

        crate::errors::Error::InternalError("Failed to decode response".to_string())
    })
}

/// Removes download instructions for notes the user switched to local-only.
///
/// Local-only notes are not part of the sync request, so the server treats a
/// note that was synced earlier as missing on this client and offers it for
/// download. Applying that would overwrite the local copy and flip it back to
/// `Synced`, silently undoing the user's choice and discarding local edits.
pub fn drop_instructions_for_local_only_notes(
    notes_db: &Connection,
    response: &mut CheckSyncResponse,
) -> Result<(), crate::errors::Error> {
    let mut stmt = notes_db
        .prepare(
            "SELECT mongo_id FROM notes WHERE sync_state = 'LocalOnly' AND mongo_id IS NOT NULL",
        )
        .context("failed to prepare local-only notes query")?;

    let local_only: std::collections::HashSet<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .context("failed to query local-only notes")?
        .filter_map(Result::ok)
        .collect();

    if local_only.is_empty() {
        return Ok(());
    }

    response
        .notes_to_download
        .retain(|note| !local_only.contains(&note.cloud_id));
    response
        .attachments_to_download
        .retain(|attachment| !local_only.contains(&attachment.note_cloud_id));

    Ok(())
}

pub fn get_all_notes_to_sync(
    notes_db: &Connection,
) -> Result<Vec<CheckNoteSyncStatus>, crate::errors::Error> {
    let mut stmt = notes_db
        .prepare(
            r#"
            SELECT
                n.local_id,
                n.mongo_id,
                n.sync_state,
                n.cloud_version,
                a.attachment_id,
                a.checksum_encrypted,
                a.size_bytes,
                a.encrypted,
                a.filename,
                a.mime_type,
                a.created_at,
                a.updated_at,
                a.crypto_meta,
                a.sync_state
            FROM notes n
            LEFT JOIN attachments a
                ON n.local_id = a.note_local_id
            WHERE n.sync_state != 'LocalOnly'
              AND (
                  a.sync_state IS NULL
                  OR a.sync_state != 'LocalOnly'
              )
            "#,
        )
        .context("failed to prepare sync query")?;

    let mut notes = std::collections::HashMap::<
        String,
        (
            String,
            Option<String>,
            SyncState,
            bool,
            Option<i64>,
            Vec<AttachmentSyncCheck>,
        ),
    >::new();

    let mut rows = stmt.query([]).context("failed to query notes to sync")?;

    while let Some(row) = rows.next().context("failed to read row")? {
        let local_id: String = row.get(0).context("failed to get local_id")?;

        let cloud_id: Option<String> = row.get(1).context("failed to get mongo_id")?;

        let sync_state: SyncState = row.get(2).context("failed to get sync_state")?;

        let cloud_version: Option<i64> = row.get(3).context("failed to get cloud_version")?;

        let attachment_id: Option<String> = row.get(4).context("failed to get attachment_id")?;

        let checksum_encrypted: Option<String> =
            row.get(5).context("failed to get checksum_encrypted")?;

        let size_bytes: Option<i64> = row.get(6).context("failed to get size_bytes")?;

        let encrypted: Option<bool> = row.get(7).context("failed to get encrypted")?;

        let file_name: Option<String> = row.get(8).context("failed to get filename")?;

        let mime_type: Option<String> = row.get(9).context("failed to get mime_type")?;

        let created_at: Option<i64> = row.get(10).context("failed to get created_at")?;

        let updated_at: Option<i64> = row.get(11).context("failed to get updated_at")?;

        let crypto_metadata: Option<String> = row.get(12).context("failed to get crypto_meta")?;

        let attachment_sync_state: Option<SyncState> =
            row.get(13).context("failed to get attachment sync_state")?;

        let note_hard_deleted = sync_state == SyncState::WaitingForTombstone;

        let entry = notes.entry(local_id.clone()).or_insert_with(|| {
            (
                local_id.clone(),
                cloud_id.clone(),
                sync_state,
                note_hard_deleted,
                cloud_version,
                Vec::new(),
            )
        });

        if attachment_id.is_none()
            || checksum_encrypted.is_none()
            || size_bytes.is_none()
            || encrypted.is_none()
            || file_name.is_none()
            || mime_type.is_none()
            || created_at.is_none()
            || updated_at.is_none()
            || attachment_sync_state.is_none()
        {
            continue;
        }

        let metadata = match crypto_metadata {
            Some(value) => match serde_json::from_str::<AttachmentCryptoMetadata>(&value) {
                Ok(meta) => Some(meta),
                Err(err) => {
                    tracing::error!(
                        task = "sync",
                        local_id = %local_id,
                        error = ?err,
                        "failed to parse attachment crypto metadata"
                    );

                    continue;
                }
            },
            None => None,
        };

        let attachment_id = match uuid::Uuid::from_str(&attachment_id.unwrap()) {
            Ok(id) => id,
            Err(err) => {
                tracing::error!(
                    task = "sync",
                    local_id = %local_id,
                    error = ?err,
                    "invalid attachment UUID"
                );

                continue;
            }
        };

        entry.5.push(AttachmentSyncCheck {
            attachment_id,
            checksum_encrypted: checksum_encrypted.unwrap(),
            size_bytes: size_bytes.unwrap(),
            is_encrypted: encrypted.unwrap(),
            hard_deleted: attachment_sync_state.unwrap() == SyncState::WaitingForTombstone,
            file_name: file_name.unwrap(),
            mime_type: mime_type.unwrap(),
            created_at: created_at.unwrap(),
            updated_at: updated_at.unwrap(),
            crypto_metadata: metadata,
        });
    }
    Ok(notes
        .into_values()
        .map(
            |(local_id, cloud_id, sync_state, hard_deleted, cloud_version, attachments)| {
                CheckNoteSyncStatus {
                    local_id,
                    cloud_id,
                    sync_state,
                    hard_deleted,
                    cloud_version,
                    attachments: if attachments.is_empty() {
                        None
                    } else {
                        Some(attachments)
                    },
                }
            },
        )
        .collect())
}
pub fn get_note_for_upload(
    conn: &Connection,
    local_id: &str,
) -> Result<NoteForUpload, crate::errors::Error> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                local_id,
                mongo_id,
                owner_id,
                title,
                summary,
                content_path,
                created_at,
                updated_at,
                deleted_at,
                version,
                cloud_version,
                sync_state,
                is_deleted,
                encrypted,
                crypto_meta
            FROM notes
            WHERE local_id = ?1
            "#,
        )
        .context("failed to prepare note query")?;

    let mut rows = stmt
        .query(params![local_id])
        .context("failed to query note")?;

    if let Some(row) = rows.next().context("failed to read note")? {
        let note_path: String = row.get(5).context("failed to get content path")?;

        let encrypted: bool = row.get(13).context("failed to get encrypted")?;

        let content = if encrypted {
            std::fs::read_to_string(&note_path)?
        } else {
            BASE64.encode(std::fs::read(&note_path)?)
        };
        let ss: SyncState = row.get(11).context("failed to get sync_state")?;
        let is_hard_deleted = ss == SyncState::WaitingForTombstone;

        Ok(NoteForUpload {
            local_id: row.get(0).context("failed to get local_id")?,
            mongo_id: row.get(1).context("failed to get mongo_id")?,
            owner_id: row.get(2).context("failed to get owner_id")?,
            title: row.get(3).context("failed to get title")?,
            summary: row.get(4).context("failed to get summary")?,
            content_path: None,
            created_at: row.get(6).context("failed to get created_at")?,
            updated_at: row.get(7).context("failed to get updated_at")?,
            deleted_at: row.get(8).context("failed to get deleted_at")?,
            version: row.get(9).context("failed to get version")?,
            cloud_version: row.get(10).context("failed to get cloud_version")?,
            sync_state: row.get(11).context("failed to get sync_state")?,
            hard_deleted: is_hard_deleted,
            is_deleted: row.get(12).context("failed to get is_deleted")?,
            encrypted,
            crypto_meta: row.get(14).context("failed to get crypto_meta")?,
            content,
        })
    } else {
        Err(crate::errors::Error::NoteNotFound)
    }
}

pub fn get_attachment_for_upload(
    conn: &Connection,
    attachment_id: &str,
) -> Result<AttachmentForUpload, crate::errors::Error> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                a.attachment_id,
                a.note_local_id,
                a.filename,
                a.mime_type,
                a.size_bytes,
                a.local_path,
                a.cloud_key,
                a.checksum_encrypted,
                a.encrypted,
                a.crypto_meta,
                a.sync_state,
                a.created_at,
                a.updated_at,
                n.mongo_id
            FROM attachments a
            INNER JOIN notes n
                ON a.note_local_id = n.local_id
            WHERE a.attachment_id = ?1
            "#,
        )
        .context("failed to prepare attachment query")?;

    let mut rows = stmt
        .query(params![attachment_id])
        .context("failed to query attachment")?;

    if let Some(row) = rows.next().context("failed to read attachment")? {
        let attachment_path: String = row.get(5).context("failed to get attachment path")?;

        let content = std::fs::read(&attachment_path)?;

        Ok(AttachmentForUpload {
            attachment_id: row.get(0).context("failed to get attachment_id")?,
            note_local_id: row.get(1).context("failed to get note_local_id")?,
            filename: row.get(2).context("failed to get filename")?,
            mime_type: row.get(3).context("failed to get mime_type")?,
            size_bytes: row.get(4).context("failed to get size_bytes")?,
            local_path: None,
            cloud_key: row.get(6).context("failed to get cloud_key")?,
            checksum_encrypted: row.get(7).context("failed to get checksum_encrypted")?,
            encrypted: row.get(8).context("failed to get encrypted")?,
            crypto_meta: row.get(9).context("failed to get crypto_meta")?,
            sync_state: row.get(10).context("failed to get sync_state")?,
            created_at: row.get(11).context("failed to get created_at")?,
            updated_at: row.get(12).context("failed to get updated_at")?,
            note_cloud_id: row.get(13).context("failed to get note_cloud_id")?,
            content,
        })
    } else {
        Err(crate::errors::Error::SyncFailed)
    }
}

pub fn execute_db_operation(
    tx: &rusqlite::Transaction<'_>,
    operation: DbOperation,
) -> Result<(), crate::errors::Error> {
    match operation {
        // Only valid after the commit; execute_db_operations defers it.
        DbOperation::ReplaceNoteFile { .. } => {
            return Err(crate::errors::Error::InternalError(
                "note file replacement must run through execute_db_operations".to_string(),
            ));
        }

        DbOperation::InsertNote {
            local_id,
            mongo_id,
            owner_id,
            title,
            summary,
            content_path,
            created_at,
            updated_at,
            deleted_at,
            version,
            cloud_version,
            sync_state,
            is_deleted,
            encrypted,
            crypto_meta,
        } => {
            tx.execute(
                r#"
                INSERT INTO notes (
                    local_id,
                    mongo_id,
                    owner_id,
                    title,
                    summary,
                    content_path,
                    created_at,
                    updated_at,
                    deleted_at,
                    version,
                    cloud_version,
                    sync_state,
                    is_deleted,
                    encrypted,
                    crypto_meta
                )
                VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8,
                    ?9, ?10, ?11, ?12, ?13, ?14, ?15
                )
                "#,
                params![
                    local_id,
                    mongo_id,
                    owner_id,
                    title,
                    summary,
                    content_path,
                    created_at,
                    updated_at,
                    deleted_at,
                    version,
                    cloud_version,
                    sync_state,
                    is_deleted,
                    encrypted,
                    crypto_meta,
                ],
            )
            .context("failed to insert note")?;
        }

        DbOperation::UpdateNoteFromCloud {
            local_id,
            mongo_id,
            owner_id,
            cloud_version,
            title,
            summary,
            content_path,
            created_at,
            updated_at,
            is_deleted,
            deleted_at,
            encrypted,
            crypto_meta,
        } => {
            tx.execute(
                r#"
                UPDATE notes
                SET
                    mongo_id = ?1,
                    owner_id = ?2,
                    cloud_version = ?3,
                    title = ?4,
                    summary = ?5,
                    content_path = ?6,
                    created_at = ?7,
                    updated_at = ?8,
                    is_deleted = ?9,
                    deleted_at = ?10,
                    encrypted = ?11,
                    crypto_meta = ?12,
                    sync_state = 'Synced'
                WHERE local_id = ?13
                "#,
                params![
                    mongo_id,
                    owner_id,
                    cloud_version,
                    title,
                    summary,
                    content_path,
                    created_at,
                    updated_at,
                    is_deleted,
                    deleted_at,
                    encrypted,
                    crypto_meta,
                    local_id,
                ],
            )
            .context("failed to update note from cloud")?;
        }

        // Applied when the server reports a note as already in sync. The note
        // may have been edited (or deleted) after the sync-check request was
        // built; flipping such a note back to 'Synced' would drop its pending
        // upload, so only idle states are touched.
        DbOperation::MarkNoteSynced { local_id } => {
            tx.execute(
                r#"
                UPDATE notes
                SET sync_state = 'Synced'
                WHERE local_id = ?1
                  AND sync_state NOT IN (
                      'PendingUpload', 'PendingDeleted', 'WaitingForTombstone', 'LocalOnly'
                  )
                "#,
                params![local_id],
            )
            .context("failed to mark note synced")?;
        }

        // Applied after this client uploaded the note. `updated_at` is the
        // value the uploaded snapshot had: if the user edited the note while
        // the upload was in flight it differs, and the note must stay
        // PendingUpload so the newer edit is uploaded on the next sync.
        // A note permanently deleted (WaitingForTombstone) or made local-only
        // meanwhile keeps that state: neither changes `updated_at`, and
        // flipping it to Synced would drop the tombstone or undo the choice.
        DbOperation::MarkNoteSyncedIfUnchanged {
            local_id,
            updated_at,
        } => {
            tx.execute(
                r#"
                UPDATE notes
                SET sync_state = 'Synced'
                WHERE local_id = ?1
                  AND updated_at = ?2
                  AND sync_state NOT IN ('WaitingForTombstone', 'LocalOnly')
                "#,
                params![local_id, updated_at],
            )
            .context("failed to mark uploaded note synced")?;
        }

        DbOperation::InsertConflictCopy {
            local_id,
            owner_id,
            title,
            summary,
            content_path,
            created_at,
            updated_at,
            encrypted,
            crypto_meta,
        } => {
            tx.execute(
                r#"
                INSERT INTO notes (
                    local_id,
                    mongo_id,
                    owner_id,
                    title,
                    summary,
                    content_path,
                    created_at,
                    updated_at,
                    deleted_at,
                    version,
                    cloud_version,
                    sync_state,
                    is_deleted,
                    encrypted,
                    crypto_meta
                )
                VALUES (
                    ?1, NULL, ?2, ?3, ?4, ?5, ?6, ?7,
                    NULL, 0, NULL, 'PendingUpload', 0, ?8, ?9
                )
                "#,
                params![
                    local_id,
                    owner_id,
                    title,
                    summary,
                    content_path,
                    created_at,
                    updated_at,
                    encrypted,
                    crypto_meta,
                ],
            )
            .context("failed to insert conflict copy")?;
        }

        DbOperation::SetCloudVersion {
            local_id,
            cloud_version,
        } => {
            tx.execute(
                r#"
                UPDATE notes
                SET cloud_version = ?1
                WHERE local_id = ?2
                "#,
                params![cloud_version, local_id],
            )
            .context("failed to set cloud version")?;
        }

        DbOperation::DeleteNote { local_id } => {
            tx.execute(
                r#"
                DELETE FROM notes
                WHERE local_id = ?1
                "#,
                params![local_id],
            )
            .context("failed to delete note")?;
        }

        DbOperation::MarkNoteError { local_id } => {
            tx.execute(
                r#"
                UPDATE notes
                SET sync_state = 'Error'
                WHERE local_id = ?1
                "#,
                params![local_id],
            )
            .context("failed to mark note error")?;
        }

        DbOperation::InsertAttachment {
            attachment_id,
            note_cloud_id,
            filename,
            mime_type,
            size_bytes,
            local_path,
            cloud_key,
            checksum_encrypted,
            encrypted,
            crypto_meta,
            sync_state,
            created_at,
            updated_at,
        } => {
            let local_id: String = tx
                .query_row(
                    r#"
                    SELECT local_id
                    FROM notes
                    WHERE mongo_id = ?1
                    "#,
                    params![note_cloud_id],
                    |row| row.get(0),
                )
                .context("failed to query local_id for attachment")?;

            tx.execute(
                r#"
                INSERT INTO attachments (
                    attachment_id,
                    note_local_id,
                    filename,
                    mime_type,
                    size_bytes,
                    local_path,
                    cloud_key,
                    checksum_encrypted,
                    encrypted,
                    crypto_meta,
                    sync_state,
                    created_at,
                    updated_at
                )
                VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7,
                    ?8, ?9, ?10, ?11, ?12, ?13
                )
                ON CONFLICT(attachment_id) DO UPDATE SET
                    filename = excluded.filename,
                    mime_type = excluded.mime_type,
                    size_bytes = excluded.size_bytes,
                    local_path = excluded.local_path,
                    cloud_key = excluded.cloud_key,
                    checksum_encrypted = excluded.checksum_encrypted,
                    encrypted = excluded.encrypted,
                    crypto_meta = excluded.crypto_meta,
                    sync_state = excluded.sync_state,
                    created_at = excluded.created_at,
                    updated_at = excluded.updated_at
                "#,
                params![
                    attachment_id,
                    local_id,
                    filename,
                    mime_type,
                    size_bytes,
                    local_path,
                    cloud_key,
                    checksum_encrypted,
                    encrypted,
                    crypto_meta,
                    sync_state,
                    created_at,
                    updated_at,
                ],
            )
            .context("failed to insert attachment")?;
        }

        DbOperation::UpdateAttachmentFromCloud {
            attachment_id,
            filename,
            mime_type,
            size_bytes,
            cloud_key,
            checksum_encrypted,
            encrypted,
            crypto_meta,
            created_at,
            updated_at,
        } => {
            tx.execute(
                r#"
                UPDATE attachments
                SET
                    filename = ?1,
                    mime_type = ?2,
                    size_bytes = ?3,
                    cloud_key = ?4,
                    checksum_encrypted = ?5,
                    encrypted = ?6,
                    crypto_meta = ?7,
                    created_at = ?8,
                    updated_at = ?9,
                    sync_state = 'Synced'
                WHERE attachment_id = ?10
                "#,
                params![
                    filename,
                    mime_type,
                    size_bytes,
                    cloud_key,
                    checksum_encrypted,
                    encrypted,
                    crypto_meta,
                    created_at,
                    updated_at,
                    attachment_id,
                ],
            )
            .context("failed to update attachment from cloud")?;
        }

        DbOperation::MarkAttachmentSynced {
            attachment_id,
            cloud_key,
        } => {
            tx.execute(
                r#"
                UPDATE attachments
                SET
                    sync_state = 'Synced',
                    cloud_key = ?1
                WHERE attachment_id = ?2
                "#,
                params![cloud_key, attachment_id],
            )
            .context("failed to mark attachment synced")?;
        }

        DbOperation::DeleteAttachment { attachment_id } => {
            tx.execute(
                r#"
                DELETE FROM attachments
                WHERE attachment_id = ?1
                "#,
                params![attachment_id],
            )
            .context("failed to delete attachment")?;
        }

        DbOperation::SetOnlineId { local_id, mongo_id } => {
            tx.execute(
                r#"
                UPDATE notes
                SET mongo_id = ?1
                WHERE local_id = ?2
                "#,
                params![mongo_id, local_id],
            )
            .context("failed to set online id")?;
        }
    }

    Ok(())
}

pub fn execute_db_operations(
    conn: &mut Connection,
    operations: Vec<DbOperation>,
) -> Result<(), crate::errors::Error> {
    let tx = conn.transaction().context("failed to create transaction")?;
    let mut file_replacements = Vec::new();

    for operation in operations {
        match operation {
            DbOperation::ReplaceNoteFile {
                staged,
                target,
                stale,
            } => {
                file_replacements.push((staged, target, stale));
            }
            operation => execute_db_operation(&tx, operation)?,
        }
    }

    tx.commit().context("failed to commit transaction")?;

    for (staged, target, stale) in file_replacements {
        replace_note_file(&staged, &target, &stale);
    }

    Ok(())
}

/// Moves staged cloud content into place after its database update was
/// committed. Failures are logged: the database already describes the new
/// version, and the staged file is kept so the content is not lost.
fn replace_note_file(staged: &str, target: &str, stale: &str) {
    if let Err(err) = std::fs::rename(staged, target) {
        tracing::error!(
            task = "sync",
            staged = %staged,
            target = %target,
            error = ?err,
            "failed to move downloaded note content into place"
        );
        return;
    }

    if let Err(err) = std::fs::remove_file(stale) {
        if err.kind() != std::io::ErrorKind::NotFound {
            tracing::error!(
                task = "sync",
                path = %stale,
                error = ?err,
                "failed to remove stale note file"
            );
        }
    }
}

/// Suffix of cloud content staged by [`handle_notes_to_download`] until the
/// database update for the note is committed.
const STAGED_SUFFIX: &str = ".incoming";

#[derive(Debug)]
pub enum DbOperation {
    /// Moves downloaded content (`staged`) over an existing note's file
    /// (`target`) and removes the note's copy in the other folder (`stale`).
    /// Applied only after the transaction commits, see
    /// [`execute_db_operations`].
    ReplaceNoteFile {
        staged: String,
        target: String,
        stale: String,
    },

    InsertNote {
        local_id: String,
        mongo_id: String,
        owner_id: String,
        title: String,
        summary: String,
        content_path: String,
        created_at: i64,
        updated_at: i64,
        deleted_at: Option<i64>,
        version: i64,
        cloud_version: i64,
        sync_state: String,
        is_deleted: bool,
        encrypted: bool,
        crypto_meta: Option<String>,
    },

    UpdateNoteFromCloud {
        local_id: String,
        mongo_id: String,
        owner_id: String,
        cloud_version: i64,
        title: String,
        summary: String,
        content_path: String,
        created_at: i64,
        updated_at: i64,
        is_deleted: bool,
        deleted_at: Option<i64>,
        encrypted: bool,
        crypto_meta: Option<String>,
    },

    MarkNoteSynced {
        local_id: String,
    },

    MarkNoteSyncedIfUnchanged {
        local_id: String,
        updated_at: i64,
    },

    /// Local copy of a note whose unsynced edits would otherwise be
    /// overwritten by a newer cloud version.
    InsertConflictCopy {
        local_id: String,
        owner_id: String,
        title: String,
        summary: String,
        content_path: String,
        created_at: i64,
        updated_at: i64,
        encrypted: bool,
        crypto_meta: Option<String>,
    },

    SetCloudVersion {
        local_id: String,
        cloud_version: i64,
    },

    DeleteNote {
        local_id: String,
    },

    MarkNoteError {
        local_id: String,
    },

    InsertAttachment {
        attachment_id: String,
        note_cloud_id: String,
        filename: String,
        mime_type: String,
        size_bytes: i64,
        local_path: Option<String>,
        cloud_key: Option<String>,
        checksum_encrypted: String,
        encrypted: bool,
        crypto_meta: Option<String>,
        sync_state: String,
        created_at: i64,
        updated_at: i64,
    },

    UpdateAttachmentFromCloud {
        attachment_id: String,
        filename: String,
        mime_type: String,
        size_bytes: i64,
        cloud_key: String,
        checksum_encrypted: String,
        encrypted: bool,
        crypto_meta: Option<String>,
        created_at: i64,
        updated_at: i64,
    },

    MarkAttachmentSynced {
        attachment_id: String,
        cloud_key: String,
    },

    DeleteAttachment {
        attachment_id: String,
    },

    SetOnlineId {
        local_id: String,
        mongo_id: String,
    },
}
pub async fn execute_server_operations(
    client: Client,
    attachments_upload_data: Vec<(AttachmentForUpload, String)>,
    note_upload_data: Vec<NoteForUpload>,
    operation_queue: CheckSyncResponse,
    access_token: AccessToken,
    online_user_id: String,
    attachments_path: &std::path::Path,
) -> Result<Vec<DbOperation>, crate::errors::Error> {
    let mut db_operations = Vec::new();

    let (upload_notes_result, upload_attachments_result, download_attachments_result) = tokio::join!(
        upload_notes(client.clone(), note_upload_data, access_token.clone(),),
        upload_attachments(
            client.clone(),
            attachments_upload_data,
            online_user_id,
            access_token.clone()
        ),
        download_attachment(
            client,
            operation_queue.attachments_to_download,
            attachments_path,
        ),
    );

    match upload_notes_result {
        Ok(mut operations) => {
            db_operations.append(&mut operations);
        }

        Err(err) => {
            return Err(err);
        }
    }

    match upload_attachments_result {
        Ok(mut operations) => {
            db_operations.append(&mut operations);
        }

        Err(err) => {
            return Err(err);
        }
    }

    match download_attachments_result {
        Ok(mut operations) => {
            db_operations.append(&mut operations);
        }

        Err(err) => {
            return Err(err);
        }
    }

    Ok(db_operations)
}
#[derive(Debug, Deserialize)]
struct UploadNoteResponse {
    mongo_id: String,
    cloud_version: i64,
    /// False when the note already existed in the cloud (an earlier upload's
    /// response was lost) and this upload's content was not stored. Servers
    /// that do not send it always created the note.
    #[serde(default = "default_true")]
    created: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
struct UpdateNoteResponse {
    cloud_version: i64,
}

/// Sends one note to the cloud: a new note is created, an existing one is
/// updated by its cloud ID.
async fn send_note_upload(
    client: &Client,
    access_token: &AccessToken,
    note: &NoteForUpload,
    as_new_note: bool,
) -> Result<reqwest::Response, reqwest::Error> {
    let request = match (as_new_note, note.mongo_id.as_ref()) {
        (false, Some(mongo_id)) => {
            client.put(format!("{}sync/update-note/{}", SERVER_ADDRESS, mongo_id))
        }
        _ => client.post(format!("{}sync/upload-note", SERVER_ADDRESS)),
    };

    request.bearer_auth(&access_token.0).json(note).send().await
}

pub async fn upload_notes(
    client: Client,
    notes_to_upload: Vec<NoteForUpload>,
    access_token: AccessToken,
) -> Result<Vec<DbOperation>, crate::errors::Error> {
    let mut operations = Vec::new();

    for note in notes_to_upload {
        let mut is_new_note = note
            .mongo_id
            .as_ref()
            .map_or(true, |value| value.is_empty());

        let local_id = note.local_id.clone();
        let uploaded_updated_at = note.updated_at;

        let mut response = match send_note_upload(&client, &access_token, &note, is_new_note).await
        {
            Ok(response) => response,

            Err(err) => {
                tracing::error!(
                    task = "sync",
                    local_id = %local_id,
                    error = ?err,
                    "failed to upload note"
                );

                continue;
            }
        };

        if !is_new_note && response.status() == reqwest::StatusCode::NOT_FOUND {
            // The server has no such note for this account (for example it was
            // uploaded under another account or the cloud copy was lost), so
            // updating it can never succeed. Create it again; the upload
            // endpoint never overwrites a note that already exists.
            tracing::warn!(
                task = "sync",
                local_id = %local_id,
                "note is missing in the cloud, uploading it again as a new note"
            );

            is_new_note = true;

            response = match send_note_upload(&client, &access_token, &note, true).await {
                Ok(response) => response,

                Err(err) => {
                    tracing::error!(
                        task = "sync",
                        local_id = %local_id,
                        error = ?err,
                        "failed to upload note"
                    );

                    continue;
                }
            };
        }

        let status = response.status();

        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(crate::errors::Error::OnlineSessionExpired);
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();

            tracing::error!(
                task = "sync",
                local_id = %local_id,
                http_status = status.as_u16(),
                response_body = %body,
                "note upload request rejected"
            );

            continue;
        }

        if is_new_note {
            let result: UploadNoteResponse = match response.json().await {
                Ok(result) => result,

                Err(err) => {
                    tracing::error!(
                        task = "sync",
                        local_id = %local_id,
                        error = ?err,
                        "failed to parse upload response"
                    );

                    continue;
                }
            };

            operations.push(DbOperation::SetOnlineId {
                local_id: local_id.clone(),
                mongo_id: result.mongo_id,
            });

            operations.push(DbOperation::SetCloudVersion {
                local_id: local_id.clone(),
                cloud_version: result.cloud_version,
            });

            // An existing cloud note did not receive this content: leave the
            // note pending so the next sync updates it by id (or resolves a
            // conflict) instead of treating these edits as uploaded.
            if result.created {
                operations.push(DbOperation::MarkNoteSyncedIfUnchanged {
                    local_id,
                    updated_at: uploaded_updated_at,
                });
            }
        } else {
            let result: UpdateNoteResponse = match response.json().await {
                Ok(result) => result,

                Err(err) => {
                    tracing::error!(
                        task = "sync",
                        local_id = %local_id,
                        error = ?err,
                        "failed to parse update response"
                    );

                    continue;
                }
            };

            operations.push(DbOperation::SetCloudVersion {
                local_id: local_id.clone(),
                cloud_version: result.cloud_version,
            });

            operations.push(DbOperation::MarkNoteSyncedIfUnchanged {
                local_id,
                updated_at: uploaded_updated_at,
            });
        }
    }

    Ok(operations)
}

pub async fn upload_attachments(
    client: Client,
    attachments_upload_data: Vec<(AttachmentForUpload, String)>,
    online_user_id: String,
    token: AccessToken,
) -> Result<Vec<DbOperation>, crate::errors::Error> {
    let mut operations = Vec::new();

    for (attachment, url) in attachments_upload_data {
        if attachment.size_bytes > 20 * 1024 * 1024 {
            tracing::warn!(
                task = "sync",
                attachment_id = %attachment.attachment_id,
                size_bytes = attachment.size_bytes,
                "skipping attachment larger than 20 MB"
            );

            continue;
        }

        let mut headers = HeaderMap::new();

        let values = [
            ("x-amz-meta-checksum", attachment.checksum_encrypted.clone()),
            ("x-amz-meta-size_bytes", attachment.size_bytes.to_string()),
            ("x-amz-meta-is_encrypted", attachment.encrypted.to_string()),
            ("x-amz-meta-file_name", attachment.filename.clone()),
            ("x-amz-meta-mime_type", attachment.mime_type.clone()),
            (
                "x-amz-meta-crypto_meta",
                attachment.crypto_meta.clone().unwrap_or_default(),
            ),
            ("x-amz-meta-note_cloud_id", attachment.note_cloud_id.clone()),
            ("x-amz-meta-created_at", attachment.created_at.to_string()),
            ("x-amz-meta-updated_at", attachment.updated_at.to_string()),
        ];

        let mut invalid_header = false;

        for (name, value) in values {
            match HeaderValue::from_str(&value) {
                Ok(value) => {
                    headers.insert(name, value);
                }

                Err(err) => {
                    tracing::error!(
                        task = "sync",
                        attachment_id = %attachment.attachment_id,
                        header = name,
                        error = ?err,
                        "failed to create attachment header"
                    );

                    invalid_header = true;
                    break;
                }
            }
        }

        if invalid_header {
            continue;
        }

        let attachment_id = attachment.attachment_id.clone();

        let response = match client
            .put(url)
            .headers(headers)
            .body(attachment.content)
            .send()
            .await
        {
            Ok(response) => response,

            Err(err) => {
                tracing::error!(
                    task = "sync",
                    attachment_id = %attachment_id,
                    error = ?err,
                    "failed to upload attachment"
                );

                continue;
            }
        };

        let status = response.status();

        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<failed to read response body>".to_string());

            tracing::error!(
                task = "sync",
                attachment_id = %attachment_id,
                http_status = status.as_u16(),
                response_body = %body,
                "attachment upload rejected"
            );

            continue;
        }
        let _ = complete_attachment_upload(&client, token.clone(), &attachment_id).await;

        operations.push(DbOperation::MarkAttachmentSynced {
            attachment_id: attachment_id.clone(),
            cloud_key: format!("{}/attachments/{}", online_user_id, attachment_id),
        });
    }

    Ok(operations)
}

pub async fn download_attachment(
    client: Client,
    attachments_to_download: Vec<DownloadAttachment>,
    attachments_path: &std::path::Path,
) -> Result<Vec<DbOperation>, crate::errors::Error> {
    let mut operations = Vec::new();

    for attachment in attachments_to_download {
        let attachment_id = attachment.attachment_id;

        let response = match client.get(&attachment.download_url).send().await {
            Ok(response) => response,

            Err(err) => {
                tracing::error!(
                    task = "sync",
                    attachment_id = %attachment_id,
                    error = ?err,
                    "failed to download attachment"
                );

                continue;
            }
        };

        let status = response.status();

        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(crate::errors::Error::OnlineSessionExpired);
        }

        if !status.is_success() {
            tracing::error!(
                task = "sync",
                attachment_id = %attachment_id,
                http_status = status.as_u16(),
                "attachment download rejected"
            );

            continue;
        }

        let extension = path::Path::new(&attachment.file_name)
            .extension()
            .and_then(|extension| extension.to_str());

        let local_path = match extension {
            Some(extension) => attachments_path.join(format!("{}.{}", attachment_id, extension)),

            None => attachments_path.join(attachment_id.to_string()),
        };

        let crypto_meta = match attachment.crypto_meta {
            Some(metadata) => match serde_json::to_string(&metadata) {
                Ok(value) => Some(value),

                Err(err) => {
                    tracing::error!(
                        task = "sync",
                        attachment_id = %attachment_id,
                        error = ?err,
                        "failed to serialize attachment crypto metadata"
                    );

                    continue;
                }
            },

            None => None,
        };

        let content = match response.bytes().await {
            Ok(content) => content,

            Err(err) => {
                tracing::error!(
                    task = "sync",
                    attachment_id = %attachment_id,
                    error = ?err,
                    "failed to read attachment"
                );

                continue;
            }
        };

        if content.len() as i64 != attachment.size_bytes {
            tracing::error!(
                task = "sync",
                attachment_id = %attachment_id,
                expected = attachment.size_bytes,
                actual = content.len(),
                "attachment size mismatch"
            );

            continue;
        }

        let checksum = blake3::hash(&content).to_hex().to_string();

        if checksum != attachment.checksum_encrypted {
            tracing::error!(
                task = "sync",
                attachment_id = %attachment_id,
                expected = %attachment.checksum_encrypted,
                actual = %checksum,
                "attachment checksum mismatch"
            );

            continue;
        }

        if let Err(err) = tokio::fs::write(&local_path, &content).await {
            tracing::error!(
                task = "sync",
                attachment_id = %attachment_id,
                path = %local_path.display(),
                error = ?err,
                "failed to write attachment"
            );

            continue;
        }

        operations.push(DbOperation::InsertAttachment {
            attachment_id: attachment_id.to_string(),
            note_cloud_id: attachment.note_cloud_id,
            filename: attachment.file_name,
            mime_type: attachment.mime_type,
            size_bytes: attachment.size_bytes,
            local_path: Some(local_path.to_string_lossy().to_string()),
            cloud_key: Some(attachment.cloud_key),
            checksum_encrypted: attachment.checksum_encrypted,
            encrypted: attachment.is_encrypted,
            crypto_meta,
            sync_state: "Synced".to_string(),
            created_at: attachment.created_at,
            updated_at: attachment.updated_at,
        });
    }

    Ok(operations)
}

pub fn handle_attachments_to_hard_delete(
    notes_db: &Connection,
    attachment_ids: Vec<String>,
) -> Result<Vec<DbOperation>, crate::errors::Error> {
    let mut operations = Vec::new();

    for attachment_id in attachment_ids {
        let local_path: Option<String> = notes_db
            .query_row(
                r#"
                SELECT local_path
                FROM attachments
                WHERE attachment_id = ?1
                "#,
                params![attachment_id],
                |row| row.get(0),
            )
            .map_err(|err| {
                tracing::error!(
                    task = "sync",
                    attachment_id = %attachment_id,
                    error = ?err,
                    "failed to get attachment path"
                );

                crate::errors::Error::SyncFailed
            })?;

        if let Some(local_path) = local_path {
            if let Err(err) = std::fs::remove_file(&local_path) {
                if err.kind() != std::io::ErrorKind::NotFound {
                    tracing::error!(
                        task = "sync",
                        attachment_id = %attachment_id,
                        path = %local_path,
                        error = ?err,
                        "failed to remove attachment"
                    );

                    continue;
                }
            }
        }

        operations.push(DbOperation::DeleteAttachment { attachment_id });
    }

    Ok(operations)
}

pub fn handle_attachment_synced(
    attachment_ids: Vec<String>,
    online_user_id: String,
) -> Vec<DbOperation> {
    attachment_ids
        .into_iter()
        .map(|id| DbOperation::MarkAttachmentSynced {
            cloud_key: format!("{}/attachments/{}", online_user_id, id),
            attachment_id: id,
        })
        .collect()
}

pub fn handle_notes_synced(note_ids: Vec<String>) -> Vec<DbOperation> {
    note_ids
        .into_iter()
        .map(|local_id| DbOperation::MarkNoteSynced { local_id })
        .collect()
}

pub fn handle_notes_to_hard_delete(
    note_ids: Vec<String>,
    notes_db: &Connection,
) -> Result<Vec<DbOperation>, crate::errors::Error> {
    let mut operations = Vec::new();

    for local_id in note_ids {
        let content_path: String = notes_db
            .query_row(
                r#"
                SELECT content_path
                FROM notes
                WHERE local_id = ?1
                "#,
                params![&local_id],
                |row| row.get(0),
            )
            .map_err(|err| {
                tracing::error!(
                    task = "sync",
                    local_id = %local_id,
                    error = ?err,
                    "failed to get note content path"
                );

                crate::errors::Error::SyncFailed
            })?;

        if let Err(err) = std::fs::remove_file(&content_path) {
            if err.kind() != std::io::ErrorKind::NotFound {
                tracing::error!(
                    task = "sync",
                    local_id = %local_id,
                    path = %content_path,
                    error = ?err,
                    "failed to remove note content file"
                );

                continue;
            }
        }

        operations.push(DbOperation::DeleteNote { local_id });
    }

    Ok(operations)
}

pub fn handle_notes_to_download(
    notes_to_download: Vec<DownloadNote>,
    notes_db: &Connection,
    notes_path: &std::path::Path,
    tmp_deleted_path: &std::path::Path,
    user_id: String,
) -> Result<Vec<DbOperation>, crate::errors::Error> {
    let mut operations = Vec::with_capacity(notes_to_download.len());

    let mut added_files = Vec::with_capacity(notes_to_download.len());

    for note in notes_to_download {
        let mongo_id = note.cloud_id.clone();

        let existing_local_id: Option<String> = notes_db
            .query_row(
                r#"
                SELECT local_id
                FROM notes
                WHERE mongo_id = ?1
                "#,
                params![&mongo_id],
                |row| row.get(0),
            )
            .optional()
            .context("failed to check existing local note")?;

        let local_id = existing_local_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let bytes_to_write = if note.is_encrypted {
            note.content.clone().into_bytes()
        } else {
            BASE64
                .decode(&note.content)
                .context("failed to decode note content")?
        };

        // Another device changed this note while it still has unsynced local
        // edits here. Keep those edits as a separate note before the cloud
        // version overwrites them.
        if existing_local_id.is_some() {
            match conflict_copy_operation(
                notes_db,
                &local_id,
                &note,
                &bytes_to_write,
                notes_path,
                &mut added_files,
            ) {
                Ok(Some(operation)) => operations.push(operation),
                Ok(None) => {}
                Err(err) => {
                    rollback_files(&added_files);
                    return Err(err);
                }
            }
        }

        // A deleted note lives in tmp_deleted, any other in notes; the copy
        // in the other folder (if any) is stale.
        let (file_path, stale_path) = if note.is_deleted {
            (
                tmp_deleted_path.join(format!("{}{}", local_id, TEMP_NOTE_EXTENSION)),
                notes_path.join(format!("{}.{}", local_id, NOTE_EXTENSION)),
            )
        } else {
            (
                notes_path.join(format!("{}.{}", local_id, NOTE_EXTENSION)),
                tmp_deleted_path.join(format!("{}{}", local_id, TEMP_NOTE_EXTENSION)),
            )
        };

        // An existing note's files must not change until the database update
        // for it is committed: if this sync pass fails later, the note would
        // otherwise hold cloud content while the database (and the conflict
        // check on the next pass) still treat it as the unsynced local
        // version, and the local edits would be lost. The cloud content is
        // staged next to the file and moved in after the commit.
        let replace_file = if existing_local_id.is_some() {
            let staged = path::PathBuf::from(format!("{}{}", file_path.display(), STAGED_SUFFIX));
            if let Err(err) = std::fs::write(&staged, &bytes_to_write) {
                rollback_files(&added_files);
                return Err(err.into());
            }
            added_files.push(staged.clone());

            Some(DbOperation::ReplaceNoteFile {
                staged: staged.to_string_lossy().into_owned(),
                target: file_path.to_string_lossy().into_owned(),
                stale: stale_path.to_string_lossy().into_owned(),
            })
        } else {
            if let Err(err) = std::fs::write(&file_path, &bytes_to_write) {
                rollback_files(&added_files);
                return Err(err.into());
            }
            added_files.push(file_path.clone());
            None
        };

        let crypto_meta = match note.crypto_meta {
            Some(meta) => Some(
                serde_json::to_string(&meta).context("failed to serialize note crypto metadata")?,
            ),

            None => None,
        };

        match existing_local_id {
            Some(local_id) => {
                operations.push(DbOperation::UpdateNoteFromCloud {
                    local_id,
                    mongo_id,
                    owner_id: user_id.clone(),
                    cloud_version: note.cloud_version,
                    title: note.title,
                    summary: note.summary,
                    content_path: file_path.to_string_lossy().into_owned(),
                    created_at: note.created_at,
                    updated_at: note.updated_at,
                    is_deleted: note.is_deleted,
                    deleted_at: note.deleted_at,
                    encrypted: note.is_encrypted,
                    crypto_meta,
                });
                operations.extend(replace_file);
            }

            None => {
                operations.push(DbOperation::InsertNote {
                    local_id,
                    mongo_id,
                    owner_id: user_id.clone(),
                    title: note.title,
                    summary: note.summary,
                    content_path: file_path.to_string_lossy().into_owned(),
                    created_at: note.created_at,
                    updated_at: note.updated_at,
                    deleted_at: note.deleted_at,
                    version: 0,
                    cloud_version: note.cloud_version,
                    sync_state: "Synced".to_string(),
                    is_deleted: note.is_deleted,
                    encrypted: note.is_encrypted,
                    crypto_meta,
                });
            }
        }
    }

    Ok(operations)
}

/// Builds the operation that preserves unsynced local edits of `local_id`
/// before `incoming` (a newer cloud version) overwrites them.
///
/// Returns `None` when there is nothing to preserve: the local note has no
/// pending edits, or its content and title already equal the cloud version
/// (for example an upload that succeeded but whose response was lost).
/// Attachments are not duplicated; the copy keeps referencing the original
/// note's attachments by id.
fn conflict_copy_operation(
    notes_db: &Connection,
    local_id: &str,
    incoming: &DownloadNote,
    incoming_bytes: &[u8],
    notes_path: &path::Path,
    added_files: &mut Vec<path::PathBuf>,
) -> Result<Option<DbOperation>, crate::errors::Error> {
    type LocalNoteRow = (
        SyncState,
        String,
        String,
        String,
        i64,
        bool,
        Option<String>,
        String,
    );

    let row: Option<LocalNoteRow> = notes_db
        .query_row(
            r#"
            SELECT sync_state, content_path, title, summary, created_at,
                   encrypted, crypto_meta, owner_id
            FROM notes
            WHERE local_id = ?1
            "#,
            params![local_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            },
        )
        .optional()
        .context("failed to read local note before conflict check")?;

    let Some((
        sync_state,
        content_path,
        title,
        summary,
        created_at,
        encrypted,
        crypto_meta,
        owner_id,
    )) = row
    else {
        return Ok(None);
    };

    // Error means an earlier sync of the note failed with its edits unsent.
    if !matches!(sync_state, SyncState::PendingUpload | SyncState::Error) {
        return Ok(None);
    }

    let local_bytes = match std::fs::read(&content_path) {
        Ok(bytes) => bytes,
        Err(err) => {
            tracing::warn!(
                task = "sync",
                local_id = %local_id,
                error = ?err,
                "cannot read local note to preserve it as a conflict copy"
            );

            return Ok(None);
        }
    };

    if local_bytes == incoming_bytes && title == incoming.title {
        return Ok(None);
    }

    let copy_id = uuid::Uuid::new_v4().to_string();
    let copy_path = notes_path.join(format!("{}.{}", copy_id, NOTE_EXTENSION));

    std::fs::write(&copy_path, &local_bytes)?;
    added_files.push(copy_path.clone());

    tracing::warn!(
        task = "sync",
        local_id = %local_id,
        copy_id = %copy_id,
        "local edits conflict with a newer cloud version, kept them as a copy"
    );

    // Encrypted titles are opaque blobs, so only plain titles can be labelled.
    let copy_title = if encrypted {
        title
    } else {
        format!("{} (conflict copy)", title)
    };

    Ok(Some(DbOperation::InsertConflictCopy {
        local_id: copy_id,
        owner_id,
        title: copy_title,
        summary,
        content_path: copy_path.to_string_lossy().into_owned(),
        created_at,
        updated_at: crate::utils::get_time(),
        encrypted,
        crypto_meta,
    }))
}

fn rollback_files(files: &[std::path::PathBuf]) {
    for file in files {
        let _ = std::fs::remove_file(file);
    }
}

pub async fn complete_attachment_upload(
    client: &reqwest::Client,
    token: AccessToken,
    attachment_id: &str,
) -> Result<(), reqwest::Error> {
    let url = format!("{}sync/upload-compleated/{}", SERVER_ADDRESS, attachment_id);

    let response = client.post(url).bearer_auth(token.0).send().await?;

    response.error_for_status()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::NOTE_DB_SCHEMA;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(NOTE_DB_SCHEMA).unwrap();
        conn
    }

    fn insert_note(conn: &Connection, id: &str, state: &str, path: &path::Path, title: &str) {
        conn.execute(
            "INSERT INTO notes (local_id, mongo_id, owner_id, title, summary, content_path,
                                created_at, updated_at, version, cloud_version, sync_state,
                                is_deleted, encrypted)
             VALUES (?1, 'mongo-1', 'owner', ?2, 'sum', ?3, 1, 100, 1, 3, ?4, 0, 0)",
            params![id, title, path.to_string_lossy().to_string(), state],
        )
        .unwrap();
    }

    fn cloud_note(content: &str, title: &str) -> DownloadNote {
        DownloadNote {
            cloud_id: "mongo-1".into(),
            owner_id: String::new(),
            cloud_version: 4,
            title: title.into(),
            summary: "sum".into(),
            content: BASE64.encode(content),
            created_at: 1,
            updated_at: 200,
            is_deleted: false,
            deleted_at: None,
            hard_deleted: false,
            is_encrypted: false,
            deleted_attachments: None,
            crypto_meta: None,
        }
    }

    fn apply(conn: &mut Connection, ops: Vec<DbOperation>) {
        execute_db_operations(conn, ops).unwrap();
    }

    fn sync_state(conn: &Connection, id: &str) -> String {
        conn.query_row(
            "SELECT sync_state FROM notes WHERE local_id = ?1",
            params![id],
            |r| r.get(0),
        )
        .unwrap()
    }

    #[test]
    fn local_only_note_is_not_overwritten_by_cloud_download() {
        let conn = db();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.md");
        std::fs::write(&file, "x").unwrap();
        insert_note(&conn, "a", "LocalOnly", &file, "t");

        let mut response = CheckSyncResponse {
            to_upload: vec![],
            notes_to_download: vec![cloud_note("cloud", "t")],
            notes_synced: vec![],
            attachments_to_upload: vec![],
            attachments_to_download: vec![],
            attachments_synced: vec![],
            attachments_to_hard_delete: vec![],
            notes_to_hard_delete: vec![],
            notes_failed: vec![],
            quota_exceeded: false,
        };

        drop_instructions_for_local_only_notes(&conn, &mut response).unwrap();
        assert!(response.notes_to_download.is_empty());

        // A different cloud note is untouched.
        let mut other = cloud_note("cloud", "t");
        other.cloud_id = "mongo-2".into();
        response.notes_to_download.push(other);
        drop_instructions_for_local_only_notes(&conn, &mut response).unwrap();
        assert_eq!(response.notes_to_download.len(), 1);
    }

    #[test]
    fn edit_made_during_upload_stays_pending() {
        let mut conn = db();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.md");
        std::fs::write(&file, "x").unwrap();
        insert_note(&conn, "a", "PendingUpload", &file, "t");

        // Upload snapshot had updated_at = 100, then the user edited (updated_at moved).
        conn.execute("UPDATE notes SET updated_at = 150 WHERE local_id = 'a'", [])
            .unwrap();
        apply(
            &mut conn,
            vec![DbOperation::MarkNoteSyncedIfUnchanged {
                local_id: "a".into(),
                updated_at: 100,
            }],
        );
        assert_eq!(sync_state(&conn, "a"), "PendingUpload");

        apply(
            &mut conn,
            vec![DbOperation::MarkNoteSyncedIfUnchanged {
                local_id: "a".into(),
                updated_at: 150,
            }],
        );
        assert_eq!(sync_state(&conn, "a"), "Synced");
    }

    #[test]
    fn delete_or_local_only_during_upload_is_kept() {
        let mut conn = db();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.md");
        std::fs::write(&file, "x").unwrap();
        insert_note(&conn, "deleted", "WaitingForTombstone", &file, "t");
        insert_note(&conn, "local", "LocalOnly", &file, "t");

        apply(
            &mut conn,
            ["deleted", "local"]
                .into_iter()
                .map(|id| DbOperation::MarkNoteSyncedIfUnchanged {
                    local_id: id.into(),
                    updated_at: 100,
                })
                .collect(),
        );

        assert_eq!(sync_state(&conn, "deleted"), "WaitingForTombstone");
        assert_eq!(sync_state(&conn, "local"), "LocalOnly");
    }

    #[test]
    fn server_synced_report_does_not_clear_pending_edits() {
        let mut conn = db();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.md");
        std::fs::write(&file, "x").unwrap();
        insert_note(&conn, "a", "PendingUpload", &file, "t");
        insert_note(&conn, "b", "Error", &file, "t");

        apply(
            &mut conn,
            handle_notes_synced(vec!["a".into(), "b".into()])
                .into_iter()
                .collect(),
        );

        assert_eq!(sync_state(&conn, "a"), "PendingUpload");
        assert_eq!(sync_state(&conn, "b"), "Synced");
    }

    #[test]
    fn newer_cloud_version_keeps_unsynced_local_edits_as_copy() {
        let mut conn = db();
        let dir = tempfile::tempdir().unwrap();
        let notes = dir.path().join("notes");
        let tmp = dir.path().join("tmp");
        std::fs::create_dir_all(&notes).unwrap();
        std::fs::create_dir_all(&tmp).unwrap();
        let file = notes.join("a.md");
        std::fs::write(&file, "my offline edit").unwrap();
        insert_note(&conn, "a", "PendingUpload", &file, "Title");

        let ops = handle_notes_to_download(
            vec![cloud_note("their edit", "Title")],
            &conn,
            &notes,
            &tmp,
            "owner".into(),
        )
        .unwrap();
        apply(&mut conn, ops);

        // Original now holds the cloud version...
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "their edit");
        assert_eq!(sync_state(&conn, "a"), "Synced");

        // ...and the local edit survives as a new, not-yet-uploaded note.
        let (copy_id, title, path, state, mongo): (String, String, String, String, Option<String>) =
            conn.query_row(
                "SELECT local_id, title, content_path, sync_state, mongo_id
                 FROM notes WHERE local_id != 'a'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .unwrap();
        assert_ne!(copy_id, "a");
        assert_eq!(title, "Title (conflict copy)");
        assert_eq!(state, "PendingUpload");
        assert_eq!(mongo, None);
        assert_eq!(std::fs::read_to_string(path).unwrap(), "my offline edit");
    }

    #[test]
    fn aborted_sync_pass_keeps_local_edits_for_the_next_pass() {
        let mut conn = db();
        let dir = tempfile::tempdir().unwrap();
        let notes = dir.path().join("notes");
        let tmp = dir.path().join("tmp");
        std::fs::create_dir_all(&notes).unwrap();
        std::fs::create_dir_all(&tmp).unwrap();
        let file = notes.join("a.md");
        std::fs::write(&file, "my offline edit").unwrap();
        insert_note(&conn, "a", "PendingUpload", &file, "Title");

        // First pass downloads, then fails before its database operations run.
        let _dropped = handle_notes_to_download(
            vec![cloud_note("their edit", "Title")],
            &conn,
            &notes,
            &tmp,
            "owner".into(),
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "my offline edit",
            "the note file must not change before the database update commits"
        );

        // The next pass still sees the local edit and keeps it as a copy.
        let ops = handle_notes_to_download(
            vec![cloud_note("their edit", "Title")],
            &conn,
            &notes,
            &tmp,
            "owner".into(),
        )
        .unwrap();
        apply(&mut conn, ops);

        assert_eq!(std::fs::read_to_string(&file).unwrap(), "their edit");
        assert!(!notes.join(format!("a.md{STAGED_SUFFIX}")).exists());
        let copy: String = conn
            .query_row(
                "SELECT content_path FROM notes WHERE local_id != 'a'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(std::fs::read_to_string(copy).unwrap(), "my offline edit");
    }

    #[test]
    fn no_copy_when_cloud_already_has_the_local_content_or_note_is_synced() {
        let mut conn = db();
        let dir = tempfile::tempdir().unwrap();
        let notes = dir.path().join("notes");
        let tmp = dir.path().join("tmp");
        std::fs::create_dir_all(&notes).unwrap();
        std::fs::create_dir_all(&tmp).unwrap();
        let file = notes.join("a.md");

        // Upload succeeded but its response was lost: same content on both sides.
        std::fs::write(&file, "same").unwrap();
        insert_note(&conn, "a", "PendingUpload", &file, "Title");
        let ops = handle_notes_to_download(
            vec![cloud_note("same", "Title")],
            &conn,
            &notes,
            &tmp,
            "owner".into(),
        )
        .unwrap();
        apply(&mut conn, ops);
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM notes", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );

        // A synced note simply takes the cloud version.
        conn.execute("UPDATE notes SET sync_state = 'Synced'", [])
            .unwrap();
        let ops = handle_notes_to_download(
            vec![cloud_note("newer", "Title")],
            &conn,
            &notes,
            &tmp,
            "owner".into(),
        )
        .unwrap();
        apply(&mut conn, ops);
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM notes", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
    }

    #[test]
    fn downloading_an_attachment_the_client_already_has_does_not_fail_the_batch() {
        let mut conn = db();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.md");
        std::fs::write(&file, "x").unwrap();
        insert_note(&conn, "a", "Synced", &file, "t");

        let insert = |checksum: &str| DbOperation::InsertAttachment {
            attachment_id: "att-1".into(),
            note_cloud_id: "mongo-1".into(),
            filename: "f.png".into(),
            mime_type: "image/png".into(),
            size_bytes: 3,
            local_path: Some("/tmp/f.png".into()),
            cloud_key: Some("u/attachments/att-1".into()),
            checksum_encrypted: checksum.into(),
            encrypted: false,
            crypto_meta: None,
            sync_state: "Synced".into(),
            created_at: 1,
            updated_at: 1,
        };

        apply(&mut conn, vec![insert("old")]);
        apply(&mut conn, vec![insert("new")]);

        let checksum: String = conn
            .query_row("SELECT checksum_encrypted FROM attachments", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(checksum, "new");
    }
}
