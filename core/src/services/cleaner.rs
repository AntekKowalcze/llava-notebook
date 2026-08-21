use anyhow::Context;
use rusqlite::{Connection, named_params};
use std::path::Path;
pub fn hard_deletes_terminated_notes(
    notes_db: &Connection,
    tmp_deleted_path: &Path,
) -> Result<Vec<String>, crate::errors::Error> {
    let expired_time = crate::utils::get_time() - crate::constants::HARD_DELETE_TIME;

    let mut stmt = notes_db
        .prepare(
            r#"
            SELECT local_id
            FROM notes
            WHERE is_deleted = 1
              AND deleted_at IS NOT NULL
              AND deleted_at < :time
            "#,
        )
        .context("failed to prepare statement for expired notes")?;

    let mut id_vec: Vec<String> = Vec::new();

    for row in stmt
        .query_map(
            named_params! {
                ":time": expired_time,
            },
            |row| row.get::<_, String>(0),
        )
        .context("failed to query expired notes")?
    {
        id_vec.push(row.context("failed to read expired note id")?);
    }

    for id in &id_vec {
        if let Err(e) = crate::storage::hard_delete_note(notes_db, tmp_deleted_path, id) {
            tracing::error!(
                task = "cleanup expired deleted notes",
                note_id = %id,
                error = ?e,
                "failed to permanently delete expired note"
            );
        }
    }

    tracing::info!(
        task = "cleanup expired deleted notes",
        count = id_vec.len(),
        "expired deleted notes cleanup completed"
    );

    Ok(id_vec)
}
