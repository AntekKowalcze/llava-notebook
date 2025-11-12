//! this module is responsible for updating .md file but also important fields in databases

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    str::FromStr,
};

use crate::{config::ProgramFiles, services::storage::update};

///function responsible for updating .md file contents
fn update_md(
    conn: &rusqlite::Connection,
    name: String,
    note_id: &str,
    written_string: String,
    program_paths: &crate::config::ProgramFiles,
) -> Result<(), crate::errors::Error> {
    //see tauri notes 1
    let tmp_filename = name.clone() + ".md.tmp";

    let tmp_filepath = program_paths.tmp_path.join(tmp_filename);
    fs::write(&tmp_filepath, written_string)?; //some permission error

    let note_name = name.clone() + ".md";
    let note_path = program_paths.notes_path.join(note_name);
    fs::rename(&tmp_filepath, note_path)?;
    let value = conn.execute(
        "UPDATE notes SET updated_at = :updated_time , version = version + 1 WHERE local_id = :id",
        rusqlite::named_params! {
            ":updated_time": crate::utils::get_time(),
            ":id": note_id,
        },
    )?;
    println!("{value}");
    crate::services::logger::log_success("successfully updated a note");

    Ok(())
}

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

fn update_test() {
    let paths = ProgramFiles::init();
    let name = "test_name".to_string();
    let written_string =
        "this is test string which have to be written and now it will not overwrite".to_string();
    let sqlite_connection = crate::services::storage::db_creation::get_connection(&paths);
    update_md(
        &sqlite_connection,
        name,
        "de8dfc04-1b31-4599-8fca-22facbf25948",
        written_string,
        &paths,
    )
    .unwrap();
}

//add deletation of note

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
