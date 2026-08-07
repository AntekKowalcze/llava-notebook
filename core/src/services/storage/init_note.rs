//! Responsible for creating local note files and database records.

use crate::Note;

use crate::services::online_auth::models::online_account::ArgonParams;

use crate::utils::{Format, log_helper};

use crate::constants::{INSERT_NOTE_SQL_SCHEMA, NOTE_EXTENSION};

use anyhow::Context;

use rusqlite::{Connection};

use std::path::Path;
use crate::services::storage::note_utils;

/// Creates a new local note.
///
/// Responsibilities:
/// - generates UUID
/// - creates markdown file
/// - prepares Note struct
///
/// Database insertion is handled separately.
pub async fn create_local_note(
    title: String,
    encryption: bool,
    synchronizing: bool,
    owner_id: &uuid::Uuid,
    path: &Path,
) -> Result<Note, crate::errors::Error> {
    let id = uuid::Uuid::new_v4();
                    println!("SYNCHRONIZING {:?}", synchronizing);

    let mut note_path = path.to_path_buf();

    note_path.push(id.to_string());

    note_path.set_extension(NOTE_EXTENSION);

    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&note_path)
    {
        Ok(_) => {
            log_helper(
                "note initialization",
                "success",
                None::<Format<String>>,
                "Note file created",
            );

            let now = crate::utils::get_time();

            let crypto_meta = if encryption {
                let params = argon2::Params::default();
                let argon_params = ArgonParams {
                    m_cost: params.m_cost(),
                    t_cost: params.t_cost(),
                    p_cost: params.p_cost(),
                };

                Some(
                    serde_json::to_value(argon_params)
                        .context("Failed to convert ArgonParams to JSON")?,
                )
            } else {
                None
            };

            Ok(Note {
                local_id: id,

                mongo_id: None,

                owner_id: *owner_id,

                title,

                summary: String::new(),

                content_path: note_path,

                created_at: now,

                updated_at: now,

                is_deleted: false,

                deleted_at: None,

                version: 1,

                cloud_version: None,

                sync_state: if synchronizing {
                    crate::services::storage::db_creation::SyncState::PendingUpload
                } else {
                    crate::services::storage::db_creation::SyncState::LocalOnly
                },

                encrypted: encryption,

                crypto_meta,
            })
        }

        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
            tracing::error!(task = "note creation", "UUID collision while creating note");

            Err(crate::errors::Error::FileAlreadyExists)
        }

        Err(err) => {
            tracing::error!(task = "note creation", ?err, "Could not create note file");

            Err(crate::errors::Error::FileOperationError(err.to_string()))
        }
    }
}

/// Inserts note into SQLite.
///
/// If database insertion fails,
/// created file is removed to avoid orphan files.
pub fn add_note_to_database(
    notes_db: &mut Connection,
    note: Note,
) -> Result<(), crate::errors::Error> {
    let result = (|| {
        let tx = notes_db
            .transaction()
            .context("Could not start transaction")?;

        let crypto_meta = note
            .crypto_meta
            .as_ref()
            .map(|v| serde_json::to_string(v))
            .transpose()
            .context("Could not serialize crypto metadata")?;

        tx.execute(
            INSERT_NOTE_SQL_SCHEMA,
            rusqlite::named_params! {
                ":local_id":
                    note.local_id.to_string(),
                ":mongo_id":
                    note.mongo_id,
                ":owner_id":
                    note.owner_id.to_string(),
                ":title":
                    note.title,
                ":summary":
                    note.summary,
                ":content_path":
                    note.content_path.to_string_lossy()
                        .to_string(),
                ":created_at":
                    note.created_at,
                ":updated_at":
                    note.updated_at,
                ":deleted_at":
                    note.deleted_at,
                ":version":
                    note.version,
                ":cloud_version":
                    note.cloud_version,
                ":sync_state":
                    note.sync_state,
                ":is_deleted":
                    note.is_deleted,
                ":encrypted":
                    note.encrypted,
                ":crypto_meta":
                    crypto_meta,

            },
        )
        .context("Could not insert note")?;
        tx.commit().context("Could not commit transaction")?;
        Ok::<(), anyhow::Error>(())
    })();
    
    match result {
        Ok(_) => {
            log_helper(
                "database note insert",
                "success",
                None::<Format<String>>,
                "Note inserted successfully",
            );
            crate::services::storage::note_utils::update_note_activity(note.local_id.clone(), notes_db)?;
            Ok(())
        }

        Err(err) => {
            if let Err(fs_err) = std::fs::remove_file(&note.content_path) {
                tracing::error!(?fs_err, "Could not remove orphan file");
            }
            Err(crate::errors::Error::InternalError(err.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn create_note_creates_file() {
        let temp = tempfile::tempdir().unwrap();

        let owner = uuid::Uuid::new_v4();

        let note = create_local_note("Test".into(), false, false, &owner, temp.path())
            .await
            .unwrap();

        assert!(note.content_path.exists());

        assert_eq!(note.owner_id, owner);
    }
}

// todo check pending upload when sync is on, adding is correct, check also this stats adding how does it work in dashboard