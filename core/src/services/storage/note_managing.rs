//! # Note list data module
//!
//! **Purpose**: This module prepares note metadata for presentation in the
//! user interface.
//!
//! It retrieves active and recently removed notes from the local SQLite
//! database, resolves their synchronization state and tag relationships, and
//! decrypts note titles when encryption is enabled.
//!
//! ## Exports
//!
//! * [`NoteCard`] — Serializable representation of an active note containing
//!   the data required by the note list UI.
//! * [`get_all_notes_data`] — Retrieves active notes for a user, loads their
//!   tags, converts database synchronization states, and decrypts encrypted
//!   note titles.
//! * [`RemovedNote`] — Serializable representation of a recently removed note
//!   shown in the deleted-notes UI.
//! * [`get_all_removed_notes_data`] — Retrieves notes that are still within the
//!   configured deletion-retention period and resolves their display titles.
//!
//! ## Key design decisions
//!
//! Note list queries are scoped by `owner_id` so that only notes belonging to
//! the requested local user are returned.
//!
//! Soft-deleted notes are excluded from the active note list. They are exposed
//! separately through [`get_all_removed_notes_data`] while they remain within
//! the configured [`crate::constants::HARD_DELETE_TIME`] retention period.
//!
//! Encrypted note titles are decrypted only when data is prepared for the
//! user interface. The database therefore continues to store the protected
//! representation while the UI receives the readable title.
//!
//! Tags are loaded in a separate query and grouped by local note identifier.
//! This avoids executing a separate SQL query for every note and allows tag
//! data to be attached to [`NoteCard`] values efficiently.
//!
//! Synchronization states are stored in SQLite as strings and explicitly
//! converted into [`SyncState`] variants. Unknown database values are treated
//! as internal errors rather than silently falling back to a valid state.
//!
//! The module exposes only the metadata required by the UI through
//! [`NoteCard`] and [`RemovedNote`] instead of returning database rows
//! directly.
//!
//! ## Dependencies
//!
//! * [`rusqlite`] — Queries the local SQLite database for notes and tags.
//! * [`serde`] — Serialization and deserialization of UI-facing note data.
//! * [`chacha20poly1305`] — Provides the note encryption key type required by
//!   title decryption.
//! * [`anyhow`] — Adds context to database operations and intermediate
//!   failures.
//! * [`std::collections::HashMap`] — Groups tags by note identifier.
//! * [`tracing`] — Logs note retrieval, tag processing, decryption, and
//!   synchronization-state failures.
//! * [`crate::crypto`] — Decrypts titles of encrypted notes.
//! * [`crate::storage`] — Provides the [`SyncState`] synchronization states.
//! * [`crate::tags`] — Provides the [`UiTag`] representation used by the UI.
//! * [`crate::constants`] — Provides the hard-delete retention period.
//! * [`crate::errors`] — Provides application-level errors returned by this
//!   module.
//! * [`crate::utils`] — Provides shared structured logging helpers.

use crate::{storage::SyncState, tags::UiTag};
use anyhow::Context;
use chacha20poly1305::Key;
use rusqlite::{Connection, named_params, params};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::error;

#[derive(Debug, Serialize)]
pub struct NoteCard {
    pub local_id: String,
    pub title: String,
    pub updated_at: i64,
    pub encrypted: bool,
    pub sync_state: SyncState,
    pub tags: Vec<UiTag>,
}

pub fn get_all_notes_data(
    notes_db: &Connection,
    user_id: &str,
    notes_key: &Key,
) -> Result<Vec<NoteCard>, crate::errors::Error> {
    tracing::info!(
        task = "get all notes data",
        status = "started",
        %user_id,
        "starting notes list retrieval"
    );

    let mut notes_stmt = notes_db
        .prepare(
            r#"
            SELECT
                local_id,
                title,
                updated_at,
                encrypted,
                sync_state
            FROM notes
            WHERE owner_id = ?1
              AND is_deleted = 0
              AND deleted_at IS NULL
            ORDER BY updated_at DESC
            "#,
        )
        .context("failed to prepare statement")
        .map_err(|e| {
            error!(
                task = "get all notes data",
                status = "error",
                %user_id,
                error = ?e,
                "failed to prepare notes query"
            );

            crate::errors::Error::InternalError(e.to_string())
        })?;

    struct NoteRow {
        local_id: String,
        title: String,
        updated_at: i64,
        encrypted: bool,
        sync_state: String,
    }

    let note_rows = notes_stmt
        .query_map(params![user_id], |row| {
            Ok(NoteRow {
                local_id: row.get(0)?,
                title: row.get(1)?,
                updated_at: row.get(2)?,
                encrypted: row.get::<_, i64>(3)? != 0,
                sync_state: row.get(4)?,
            })
        })
        .context("failed to map results to NoteRow struct")
        .map_err(|e| {
            error!(
                task = "get all notes data",
                status = "error",
                %user_id,
                error = ?e,
                "failed to map notes query results"
            );

            crate::errors::Error::InternalError(e.to_string())
        })?
        .collect::<Result<Vec<_>, _>>()
        .context("failed to collect results")
        .map_err(|e| {
            error!(
                task = "get all notes data",
                status = "error",
                %user_id,
                error = ?e,
                "failed to collect notes query results"
            );

            crate::errors::Error::InternalError(e.to_string())
        })?;

    tracing::info!(
        task = "get all notes data",
        status = "success",
        %user_id,
        note_count = note_rows.len(),
        "notes metadata loaded"
    );

    let mut tags_by_note: HashMap<String, Vec<UiTag>> = HashMap::new();

    let mut tags_stmt = notes_db
        .prepare(
            r#"
            SELECT
                nt.note_local_id,
                t.tag_id,
                t.name,
                t.color
            FROM note_tags nt
            INNER JOIN tags t
                ON t.tag_id = nt.tag_id
            INNER JOIN notes n
                ON n.local_id = nt.note_local_id
            WHERE n.owner_id = ?1
              AND n.is_deleted = 0
              AND n.deleted_at IS NULL
            ORDER BY t.name COLLATE NOCASE
            "#,
        )
        .context("failed to fetch notes data")
        .map_err(|e| {
            error!(
                task = "get note tags",
                status = "error",
                %user_id,
                error = ?e,
                "failed to prepare tags query"
            );

            crate::errors::Error::InternalError(e.to_string())
        })?;

    let tag_rows = tags_stmt
        .query_map(params![user_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                UiTag {
                    tag_id: row.get(1)?,
                    name: row.get(2)?,
                    color: row.get(3)?,
                },
            ))
        })
        .context("failed to map tags")
        .map_err(|e| {
            error!(
                task = "get note tags",
                status = "error",
                %user_id,
                error = ?e,
                "failed to map tag query results"
            );

            crate::errors::Error::InternalError(e.to_string())
        })?;

    let mut tag_count = 0usize;

    for tag_row in tag_rows {
        let (note_id, tag) = tag_row
            .context("error while destructurizing output")
            .map_err(|e| {
                error!(
                    task = "get note tags",
                    status = "error",
                    %user_id,
                    error = ?e,
                    "failed to process tag row"
                );

                crate::errors::Error::InternalError(e.to_string())
            })?;

        tags_by_note.entry(note_id).or_default().push(tag);

        tag_count += 1;
    }

    tracing::info!(
        task = "get note tags",
        status = "success",
        %user_id,
        tag_count,
        "note tags loaded"
    );

    let mut result = Vec::with_capacity(note_rows.len());

    for note in note_rows {
        let title = if note.encrypted {
            tracing::info!(
                task = "decrypt note title",
                status = "started",
                %user_id,
                note_id = %note.local_id,
                "decrypting encrypted note title"
            );

            let decrypted_title = crate::crypto::decrypt_title(&note.local_id, notes_key, notes_db)
                .map_err(|e| {
                    error!(
                        task = "decrypt note title",
                        status = "error",
                        %user_id,
                        note_id = %note.local_id,
                        error = ?e,
                        "failed to decrypt note title"
                    );

                    e
                })?;

            tracing::info!(
                task = "decrypt note title",
                status = "success",
                %user_id,
                note_id = %note.local_id,
                "encrypted note title decrypted successfully"
            );

            decrypted_title
        } else {
            note.title
        };

        let sync_state = match note.sync_state.as_str() {
            "LocalOnly" => SyncState::LocalOnly,
            "PendingUpload" => SyncState::PendingUpload,
            "Synced" => SyncState::Synced,
            "Conflict" => SyncState::Conflict,
            "Error" => SyncState::Error,
            "PendingDeleted" => SyncState::PendingDeleted,

            value => {
                error!(
                    task = "parse sync state",
                    status = "error",
                    %user_id,
                    note_id = %note.local_id,
                    sync_state = %value,
                    "unknown sync state received from database"
                );

                return Err(crate::errors::Error::InternalError(format!(
                    "Unknown sync state: {value}"
                )));
            }
        };

        let tags = tags_by_note.remove(&note.local_id).unwrap_or_default();

        result.push(NoteCard {
            local_id: note.local_id,
            title,
            updated_at: note.updated_at,
            encrypted: note.encrypted,
            sync_state,
            tags,
        });
    }

    crate::utils::log_helper(
        "notes list",
        "success",
        Some(crate::utils::Format::Display(&user_id.to_string())),
        "notes data retrieved successfully",
    );

    Ok(result)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RemovedNote {
    pub local_id: String,
    pub title: String,
    pub removed_at: i64,
}

pub fn get_all_removed_notes_data(
    notes_db: &Connection,
    user_id: String,
    notes_key: &Key,
) -> Result<Vec<RemovedNote>, crate::errors::Error> {
    let current_removal_border = crate::utils::get_time() - crate::constants::HARD_DELETE_TIME;

    let mut stmt = notes_db
        .prepare(
            "SELECT local_id, title, deleted_at, encrypted
             FROM notes
             WHERE is_deleted = 1
               AND deleted_at > :time AND sync_state != 'WaitingForTombstone'",
        )
        .context("Failed to prepare removed notes query")
        .map_err(|e| crate::errors::Error::InternalError(e.to_string()))?;

    let removed_rows = stmt
        .query_map(
            named_params! {
                ":time": current_removal_border
            },
            |row| {
                let id: String = row.get(0)?;
                let encrypted: bool = row.get(3)?;

                let title = if encrypted {
                    crate::crypto::decrypt_title(&id, notes_key, notes_db).map_err(|e| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(
                            e.to_string(),
                        )))
                    })?
                } else {
                    row.get(1)?
                };

                Ok(RemovedNote {
                    local_id: id,
                    title,
                    removed_at: row.get(2)?,
                })
            },
        )
        .context("Failed to query removed notes")
        .map_err(|e| {
            error!(
                task = "get all removed notes data",
                status = "error",
                %user_id,
                error = ?e,
                "failed to collect removed notes query results"
            );

            crate::errors::Error::InternalError(e.to_string())
        })?
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to collect removed notes")
        .map_err(|e| {
            error!(
                task = "get all removed notes data",
                status = "error",
                %user_id,
                error = ?e,
                "failed to collect removed notes"
            );

            crate::errors::Error::InternalError(e.to_string())
        })?;

    Ok(removed_rows)
}
