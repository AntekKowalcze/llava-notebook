//! This module is responsible for creating database record and .md file
use anyhow::Context;
use rusqlite::{Connection, OptionalExtension};
use std::{fs, path::PathBuf};

use crate::{
    config::ProgramFiles,
    constans::{INSERT_NOTE_SQL_SCHEMA, MAX_NOTE_NAME_LENGTH, NOTE_EXTENSION},
    services::logger,
};
//init note after new note clicked and name sumbited
///this note init note struct and creates md file
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
            crate::services::logger::log_success("created note file succesfullly");
            Ok(crate::models::note::Note {
                local_id: uuid::Uuid::new_v4(),
                mongo_id: None,
                owner_id: owner_id,

                name: name,
                title: "".to_owned(),
                summary: "".to_owned(),
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
            return Err(crate::errors::Error::FileAlreadyExists);
        }
        Err(err) => {
            crate::services::logger::log_error("couldnt create a file {}", &err);
            //dodać obsługe, poprostu nie moge utworzyć pliku, popup z tym komunikatem żeby zmienić uprawnienia i wróć do działania programu
            Err(crate::errors::Error::FileOperationError(err))
        }
    }
}

///function responsible for all operations needed to initialize note, creating struct and inserting it to sqlite
pub fn add_note_to_database(
    conn: &mut rusqlite::Connection,
    paths: &ProgramFiles,
    name: String,
) -> Result<(), crate::errors::Error> {
    let name = name.trim().to_string();
    let name = sanitise_file_name::sanitise(&name);
    if name.chars().count() == 0 {
        return Err(crate::errors::Error::NoteNameError);
    }
    let file_content = fs::read_to_string(&paths.active_user_path)?;
    let json: serde_json::Value = serde_json::from_str(&file_content)
        .context("failed to parse json of active_user.json file")?;
    let owner_id: uuid::Uuid = serde_json::from_value(json["user_uuid"].clone())
        .context("Couldnt convert user_uuid red from active_user.json to uuid")?;
    validate_note_name(&name, &conn, &owner_id)?;
    //getting current user

    let note = init_note(owner_id, &paths.notes_path, name)?;
    let transaction_result = {
        let tx = conn
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
            crate::services::logger::log_success(
                "successfully initialized note and created record in notes table",
            );
            Ok(())
        }
        Err(err) => {
            crate::services::logger::log_error(
                "Database transaction failed, deleting orphaned file",
                &err,
            );
            if let Err(fs_err) = std::fs::remove_file(&note.content_path) {
                crate::services::logger::log_error("Failed to cleanup orphaned file!", &fs_err);
            }
            Err(crate::errors::Error::InternalError(err.into()))
        }
    }
}
///function which valiates note name, it should be distinct
fn validate_note_name(
    note_name: &str,
    conn: &Connection,
    owner_id: &uuid::Uuid,
) -> Result<(), crate::errors::Error> {
    if note_name.chars().count() >= MAX_NOTE_NAME_LENGTH {
        //longest unix filename
        crate::services::logger::log_error("name too long", crate::errors::Error::NoteNameToLong);
        return Err(crate::errors::Error::NoteNameToLong);
    }
    let exists = conn
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
        crate::services::logger::log_error(
            "Note name exists",
            crate::errors::Error::NoteNameExistsError,
        );
        return Err(crate::errors::Error::NoteNameExistsError);
    } else {
        crate::services::logger::log_success("note name validated successfully");
        Ok(())
    }
}

#[test]
fn chceck_if_file_is_created() {
    let path = crate::config::ProgramFiles::init().unwrap();
    let name = "test".to_owned();
    let file_content = fs::read_to_string(&path.active_user_path).unwrap();
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
    let path = crate::config::ProgramFiles::init().unwrap();
    let mut conn = crate::services::storage::db_creation::get_connection(&path).unwrap();
    let name = "tttsss".to_owned();

    add_note_to_database(&mut conn, &path, name).unwrap();
}

#[test]
fn note_validator_test() {
    let path = crate::config::ProgramFiles::init().unwrap();
    let conn = crate::services::storage::db_creation::get_connection(&path).unwrap();
    let note_name = "ttt";
    let file_content = fs::read_to_string(&path.active_user_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&file_content).unwrap();
    let owner_id: uuid::Uuid = serde_json::from_value(json["user_uuid"].clone()).unwrap();
    validate_note_name(note_name, &conn, &owner_id).unwrap();
}
//TODO add delete user, with folder deletation and password confirmation, all notes will be deleted
