use crate::{config::ProgramFiles, services::storage::update};

fn delete_note(
    conn: &rusqlite::Connection,
    name: String,
    note_id: &str,
    paths: &crate::config::ProgramFiles,
) -> Result<(), crate::errors::Error> {
    //create some fancy view in tauri and confirmation to delete note
    std::fs::rename(
        paths.notes_path.join(format!("{name}.md")),
        paths.delete_tmp_path.join(format!("{name}.md")),
    )?;
    let stmt = conn.execute(
        "UPDATE NOTES SET is_deleted = true, deleted_at = :deletation_time WHERE local_id = :note_id ",
        rusqlite::named_params! {
            ":deletation_time": crate::utils::get_time(),
            ":note_id": note_id},
    )?;
    println!("{stmt}");
    crate::services::logger::log_success("note successfully deleted");

    Ok(())
}

#[test]
fn test_delte_note() {
    let paths = ProgramFiles::init();
    let name = "testtt".to_string();
    let sqlite_connection = crate::services::storage::db_creation::get_connection(&paths);
    delete_note(
        &sqlite_connection,
        name.clone(),
        "de8dfc04-1b31-4599-8fca-22facbf25948",
        &paths,
    )
    .unwrap();
    std::fs::rename(
        paths.delete_tmp_path.join(format!("{name}.md")),
        paths.notes_path.join(format!("{name}.md")),
    )
    .unwrap();
}
