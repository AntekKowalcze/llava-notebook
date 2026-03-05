//! this module is responsible for updating .md file but also important fields in databases
use crate::constants::*;
use crate::utils::{Format, log_helper};
use anyhow::Context;
use std::fs::{self};

///function responsible for updating .md file contents
pub fn update_md(
    //todo change to english
    //możę usunąć note_id/name w sensie jedno z tych ale to zobacze jak będzie łatwiej zaimplementować w taurii
    //remember about changin state to pending upload when update when adding sync
    notes_db: &rusqlite::Connection,
    name: String,
    note_id: uuid::Uuid, //możliwa zmiana na uuid
    written_string: String,
    program_paths: &crate::config::ProgramFiles,
    title: String,
) -> Result<(), crate::errors::Error> {
    //see tauri notes 1
    let tmp_filename = name.clone() + TEMP_NOTE_EXTENSION;

    let tmp_filepath = program_paths.tmp_path.join(tmp_filename);
    let summary: String = written_string
        .split_whitespace()
        .take(SUMMARY_LENGTH)
        .collect::<Vec<&str>>()
        .join(" ");
    if title.split_whitespace().count() > MAX_TITLE_LENGTH {
        tracing::error!(
            task = "validating title",
            status = "error",
            "title too long"
        );
        return Err(crate::errors::Error::TitleTooLong);
    }
    fs::write(&tmp_filepath, written_string)?; //some permission error
    fs::File::open(&tmp_filepath)?.sync_all()?;

    let note_name = name.clone() + "." + NOTE_EXTENSION;
    let note_path = program_paths.notes_path.join(note_name);
    fs::rename(&tmp_filepath, note_path)?;

    notes_db
        .execute(
            UPDATE_NOTE_SQL_QUERY,
            rusqlite::named_params! {
                ":updated_time": crate::utils::get_time(),
                ":summary": summary,
                ":title" : title,
                ":id": note_id.to_string(),
            },
        )
        .context("Couldnt get needed info about note from SQL while updating")?;
    log_helper(
        "validating note name",
        "success",
        Some(Format::Display(&note_id)),
        "note updated successfully",
    );

    Ok(())
}

#[test]

fn update_test() {
    let paths = crate::config::ProgramFiles::init_in_base().unwrap();
    let name = "tttsss".to_string();
    let written_string =
        "this is test string which have to be written and now it will not overwrite".to_string();
    let sqlite_connection = crate::services::storage::db_creation::get_connection(&paths).unwrap();
    let title = "tttsss".to_string();
    update_md(
        &sqlite_connection,
        name,
        uuid::Uuid::parse_str("45943af4-6163-4816-8108-06330841e1ea").unwrap(), // this is why this test might be failing, write fucntion getting note id from name
        written_string,
        &paths,
        title,
    )
    .unwrap();
}
