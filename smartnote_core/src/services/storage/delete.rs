//! this module contains function for soft deleting notes locally
use crate::utils::{Format, log_helper};

use anyhow::Context;

use crate::{
    config::ProgramFiles,
    models::note,
    services::storage::{db_creation::SyncState, update},
};
/// soft delete note
fn delete_note(
    conn: &rusqlite::Connection,
    name: String,
    note_id: &str,
    paths: &crate::config::ProgramFiles,
) -> Result<(), crate::errors::Error> {
    let current_sync_status: SyncState = conn
        .query_row(
            "SELECT sync_state FROM notes WHERE local_id = :note_id",
            rusqlite::named_params! { ":note_id": note_id },
            |row| row.get("sync_state"),
        )
        .context("couldnt get currenct sync status for rollback in deletation of note SQL ERROR")?;

    conn.execute(
        "UPDATE notes SET sync_state = 'PendingDeleted', deleted_at = :time WHERE local_id = :note_id",
        rusqlite::named_params! {
            ":time": crate::utils::get_time(),
            ":note_id": note_id
        },
    ).context("couldnt update sync state and deleted time in deletation of note")?;

    if let Err(err) = std::fs::rename(
        paths.notes_path.join(format!("{name}.md")),
        paths.delete_tmp_path.join(format!("{name}.md")),
    ) {
        tracing::error!(task="deleting", status="error", %note_id, "Soft delete failed, rolling back DB");
        conn.execute(
            "UPDATE notes SET sync_state = :status_before, deleted_at = NULL WHERE local_id = :note_id",
            rusqlite::named_params! {
                ":status_before": current_sync_status,
                ":note_id": note_id
            },
        ).context("Couldnt get sync state, and delte status while rollback in note deletation SQL ERROR")?;

        return Err(crate::errors::Error::from(err));
    }

    log_helper(
        "deleting note",
        "success",
        Some(Format::Display(&note_id)),
        "note deleted succesfully",
    );

    Ok(())
}

#[test]
fn test_delte_note() {
    let paths = ProgramFiles::init().unwrap();
    let name: String = "tttsss".to_string();
    let sqlite_connection = crate::services::storage::db_creation::get_connection(&paths).unwrap();
    delete_note(
        &sqlite_connection,
        name.clone(),
        "45943af4-6163-4816-8108-06330841e1ea",
        &paths,
    )
    .unwrap();
    std::fs::rename(
        paths.delete_tmp_path.join(format!("{name}.md")),
        paths.notes_path.join(format!("{name}.md")),
    )
    .unwrap();
}

//deletation not visible because its cleaning
