//! this module is responsible for updating .md file but also important fields in databases

use std::fs::{self};

use anyhow::Context;

use crate::{config::ProgramFiles, services::storage::update};

///function responsible for updating .md file contents
fn update_md(
    //remember about changin state to pending upload when update when adding sync
    conn: &rusqlite::Connection,
    name: String,
    note_id: &str,
    written_string: String,
    program_paths: &crate::config::ProgramFiles,
    title: String,
) -> Result<(), crate::errors::Error> {
    //see tauri notes 1
    let tmp_filename = name.clone() + ".md.tmp";

    let tmp_filepath = program_paths.tmp_path.join(tmp_filename);
    let summary: String = written_string
        .split(" ")
        .take(10)
        .collect::<Vec<&str>>()
        .join(" ");
    if title.split_whitespace().count() > 30 {
        return Err(crate::errors::Error::TitleTooLong);
    }
    println!("{summary}");
    fs::write(&tmp_filepath, written_string)?; //some permission error

    let note_name = name.clone() + ".md";
    let note_path = program_paths.notes_path.join(note_name);
    fs::rename(&tmp_filepath, note_path)?;
    let value = conn.execute(
        "UPDATE notes SET updated_at = :updated_time , summary = :summary ,version = version + 1, title = :title WHERE local_id = :id",
        rusqlite::named_params! {
            ":updated_time": crate::utils::get_time(),
            ":summary": summary,
            ":title" : title,
            ":id": note_id,
        },
    ).context("Couldnt get needed info about note from SQL while updating")?;
    println!("{value}");
    crate::services::logger::log_success("successfully updated a note");

    Ok(())
}

#[test]

fn update_test() {
    let paths = ProgramFiles::init().unwrap();
    let name = "tttsss".to_string();
    let written_string =
        "this is test string which have to be written and now it will not overwrite".to_string();
    let sqlite_connection = crate::services::storage::db_creation::get_connection(&paths).unwrap();
    let title = "tttsss".to_string();
    update_md(
        &sqlite_connection,
        name,
        "b486a877-21d9-4d28-abd7-451dd965fe50",
        written_string,
        &paths,
        title,
    )
    .unwrap();
}
//TODO dodać anyhow, super obsługa błędów
