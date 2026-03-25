//! This module is responsible for creating database record and .md file

use crate::utils::{Format, log_helper};
use crate::{
    config::ProgramFiles,
    constants::{INSERT_NOTE_SQL_SCHEMA, MAX_NOTE_NAME_LENGTH, NOTE_EXTENSION},
};
use anyhow::Context;
use rusqlite::{Connection, OptionalExtension};
use std::path::PathBuf;
//init note after new note clicked and name sumbited
///this note init note struct and creates md file
//TODO every note should be encrypted on disk, with same key as in database, title as plaintext should be deleted from sql schema, it should be deriven from constatnt not editable # as first sign in md file (title sign and first line content is title) and name of the note should be changed to uuid of note, to getting note file content should be just export,
// TODO after function which saves note content manually there should be command which increents activity number in sql database
// TODO TREAT CRETING NOTE AS EDITING SO RUN ADDING ACTIVITY COUNT AFTER CREATION AND DELETATION look at queries in dashboard to create good insert queries in adding note
fn init_note(
    owner_id: uuid::Uuid, //get it from current user file
    path: &PathBuf,       //path of notes
    name: String,
) -> Result<crate::models::note::Note, crate::errors::Error> {
    let mut new_note_path = path.clone();
    new_note_path.push(&name);
    new_note_path.set_extension(NOTE_EXTENSION);
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&new_note_path)
    {
        Ok(_) => {
            log_helper(
                "note initialization",
                "success",
                None::<Format<String>>,
                "Note created successfully",
            );
            crate::services::logger::log_success("created note file succesfullly");
            Ok(crate::models::note::Note {
                local_id: uuid::Uuid::new_v4(),
                mongo_id: None,
                owner_id: owner_id,

                name: name,
                title: "".to_owned(),
                summary: "".to_owned(), //Smth here is off why should i store name and summary unecrypted?
                content_path: new_note_path,

                created_at: crate::utils::get_time(),
                updated_at: crate::utils::get_time(),
                is_deleted: false,
                deleted_at: None,

                version: 1,
                cloud_version: None,
                sync_state: crate::services::storage::db_creation::SyncState::LocalOnly,

                encrypted: false,

                crypto_meta: None, //change it after adding encription
            })
        }
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
            tracing::error!(
                task = "note initialization",
                status = "error",
                "note with this name already exists"
            );
            return Err(crate::errors::Error::FileAlreadyExists);
        }
        Err(err) => {
            tracing::error!(
                task = "note initialization",
                status = "error",
                "couldnt init a note"
            );

            Err(crate::errors::Error::FileOperationError(err.to_string()))
        }
    }
}

///function responsible for all operations needed to initialize note, creating struct and inserting it to sqlite
pub fn add_note_to_database(
    notes_db: &mut rusqlite::Connection,
    paths: &ProgramFiles,
    name: String,
    owner_id: uuid::Uuid,
) -> Result<(), crate::errors::Error> {
    let name = name.trim().to_string();
    let name = sanitise_file_name::sanitise(&name);
    if name.chars().count() == 0 {
        return Err(crate::errors::Error::NoteNameError);
    }
    //  let file_content = fs::read_to_string(&paths.active_user_path)?;
    // let json: serde_json::Value = serde_json::from_str(&file_content)
    //     .context("failed to parse json of active_user.json file")?;
    // let owner_id: uuid::Uuid = serde_json::from_value(json["user_uuid"].clone())
    //     .context("Couldnt convert user_uuid red from active_user.json to uuid")?;
    validate_note_name(&name, &notes_db, &owner_id)?;
    //getting current user

    let note = init_note(owner_id, &paths.notes_path, name)?;
    let transaction_result = {
        let tx = notes_db
            .transaction()
            .context("Couldnt commit transaction while inserting a note, sql error")?;
        tx.execute(
            INSERT_NOTE_SQL_SCHEMA,
            rusqlite::named_params! {
                    ":local_id": note.local_id.to_string(),
                    ":mongo_id": note.mongo_id,
                    ":owner_id": note.owner_id.to_string(),
                    ":name": note.name,
                    ":title": note.title,
                    ":summary": note.summary,
                    ":content_path": note.content_path.to_string_lossy().to_string(),
                    ":created_at": note.created_at,
                    ":updated_at": note.updated_at,
                    ":deleted_at": note.deleted_at,
                    ":version": note.version ,
                    ":cloud_version": note.cloud_version ,
                    ":sync_state": note.sync_state,
                    ":is_deleted": note.is_deleted,
                    ":encrypted": note.encrypted ,
                    ":crypto_meta": note.crypto_meta,
            },
        )
        .context("Couldnt insert into database")?;
        tx.commit()
    };

    match transaction_result {
        Ok(_) => {
            log_helper(
                "addign note to db",
                "success",
                None::<Format<String>>,
                "successfully created a note",
            );

            Ok(())
        }
        Err(err) => {
            if let Err(fs_err) = std::fs::remove_file(&note.content_path) {
                tracing::error!(
                    task = "adding note to db",
                    status = "error",
                    ?fs_err,
                    "couldnt remove orphaned file",
                );
            }
            tracing::error!(
                task = "adding note to db",
                status = "error",
                ?err,
                "error while inserting note to db, deleting orphaned file",
            );
            Err(crate::errors::Error::InternalError(err.to_string()))
        }
    }
}
///function which valiates note name, it should be distinct
fn validate_note_name(
    note_name: &str,
    notes_db: &Connection,
    owner_id: &uuid::Uuid,
) -> Result<(), crate::errors::Error> {
    if note_name.chars().count() >= MAX_NOTE_NAME_LENGTH {
        //longest unix filename
        tracing::error!(
            task = "validating note name",
            status = "error",
            "note name too long",
        );
        return Err(crate::errors::Error::NoteNameTooLong);
    }
    let exists = notes_db
        .query_row(
            "SELECT 1 FROM notes WHERE owner_id = :owner_id AND name = :note_name",
            rusqlite::named_params! {
                ":owner_id": owner_id.to_string(),
                ":note_name": note_name,
            },
            |_row| Ok(()),
        )
        .optional()
        .context("SQL query failed while validating note name")?
        .is_some();

    if exists {
        tracing::error!(
            task = "validating note name",
            status = "error",
            "note name exists",
        );
        return Err(crate::errors::Error::NoteNameExistsError);
    } else {
        log_helper(
            "validating note name",
            "success",
            None::<Format<String>>,
            "note name validated successfully",
        );
        Ok(())
    }
}

#[test]
fn check_if_file_is_created() {
    let path = crate::config::ProgramFiles::init_in_base().unwrap();
    let name = "test".to_owned();
    let file_content = std::fs::read_to_string(&path.active_user_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&file_content).unwrap();
    let owner_id: uuid::Uuid = serde_json::from_value(json["user_uuid"].clone()).unwrap();

    init_note(owner_id, &path.notes_path, name.clone()).unwrap();
    let mut new_note_path = path.notes_path;
    new_note_path.push(name);
    new_note_path.set_extension("md");
    assert!(std::path::Path::exists(&new_note_path));
}
#[test]
fn add_to_db() {
    let path = crate::config::ProgramFiles::init_in_base().unwrap();
    let mut notes_db = crate::services::storage::db_creation::get_connection(&path).unwrap();
    let name = "tttsss".to_owned();
    let file_content = std::fs::read_to_string(&path.active_user_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&file_content)
        .context("failed to parse json of active_user.json file")
        .unwrap();
    let owner_id: uuid::Uuid = serde_json::from_value(json["user_uuid"].clone())
        .context("Couldnt convert user_uuid red from active_user.json to uuid")
        .unwrap();
    add_note_to_database(&mut notes_db, &path, name, owner_id).unwrap();
}

#[test]
fn note_validator_test() {
    let path = crate::config::ProgramFiles::init_in_base().unwrap();
    let notes_db = crate::services::storage::db_creation::get_connection(&path).unwrap();
    let note_name = "ttt";
    let file_content = std::fs::read_to_string(&path.active_user_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&file_content).unwrap();
    let owner_id: uuid::Uuid = serde_json::from_value(json["user_uuid"].clone()).unwrap();
    validate_note_name(note_name, &notes_db, &owner_id).unwrap();
}
