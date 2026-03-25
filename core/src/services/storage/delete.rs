//! this module contains function for soft deleting notes locally

use crate::utils::{Format, log_helper};

use anyhow::Context;

use crate::services::storage::db_creation::SyncState;
/// soft delete note
pub fn delete_note(
    notes_db: &rusqlite::Connection,
    name: String,
    note_id: uuid::Uuid,
    paths: &crate::config::ProgramFiles,
) -> Result<(), crate::errors::Error> {
    let current_sync_status: SyncState = notes_db
        .query_row(
            "SELECT sync_state FROM notes WHERE local_id = :note_id",
            rusqlite::named_params! { ":note_id": note_id.to_string() },
            |row| row.get("sync_state"),
        )
        .context("couldnt get currenct sync status for rollback in deletation of note SQL ERROR")?;

    notes_db.execute(
        "UPDATE notes SET sync_state = 'PendingDeleted', deleted_at = :time WHERE local_id = :note_id",
        rusqlite::named_params! {
            ":time": crate::utils::get_time(),
            ":note_id": note_id.to_string()
        },
    ).context("couldnt update sync state and deleted time in deletation of note")?;

    if let Err(err) = std::fs::rename(
        paths.notes_path.join(format!("{name}.md")),
        paths.delete_tmp_path.join(format!("{name}.md")),
    ) {
        tracing::error!(task="deleting", status="error", %note_id, "Soft delete failed, rolling back DB");
        notes_db.execute(
            "UPDATE notes SET sync_state = :status_before, deleted_at = NULL WHERE local_id = :note_id",
            rusqlite::named_params! {
                ":status_before": current_sync_status,
                ":note_id": note_id.to_string()
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

//run test only with note id!``
// #[test]
// fn test_delte_note() {
//     let paths = crate::config::ProgramFiles::init_in_base().unwrap();
//     let name: String = "tttsss".to_string();
//     let sqlite_connection = crate::services::storage::db_creation::get_connection(&paths).unwrap();
//     delete_note(
//         &sqlite_connection,
//         name.clone(),
//         uuid::Uuid::parse_str("45943af4-6163-4816-8108-06330841e1ea").unwrap(),
//         &paths,
//     )
//     .unwrap();
//     std::fs::rename(
//         paths.delete_tmp_path.join(format!("{name}.md")),
//         paths.notes_path.join(format!("{name}.md")),
//     )
//     .unwrap();
// }
//("45943af4-6163-4816-8108-06330841e1ea")

//deletation not visible because its cleaning
