use anyhow::Context;
use rusqlite::{Connection, named_params};
pub fn update_note_activity(
    note_id: uuid::Uuid,
    notes_db: &mut Connection,
) -> Result<(), crate::errors::Error> {
    let result = (|| {
        let tx = notes_db
            .transaction()
            .context("Could not start transaction")?;

        tx.execute(
            "INSERT INTO user_activity (note_id) VALUES (:note_id)",
            named_params! {":note_id": note_id.to_string()},
        )
        .context("Could not change user activity")?;
        tx.commit().context("Could not commit transaction")?;

        Ok::<(), anyhow::Error>(())
    })();
    result?;

    Ok(())
}
