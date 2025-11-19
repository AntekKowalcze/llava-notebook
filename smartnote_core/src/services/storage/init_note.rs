//! This module is responsible for creating database record and .md file
use std::path::PathBuf;

use rusqlite::{Connection, OptionalExtension};

use crate::{config::ProgramFiles, services::logger};
//init note after new note clicked and name sumbited

fn init_note(
    //TODO add here owner id after added
    path: &PathBuf, //path of notes
    name: String,
) -> Result<crate::models::note::Note, crate::errors::Error> {
    let mut new_note_path = path.clone();
    new_note_path.push(&name);
    new_note_path.set_extension("md");
    match std::fs::File::create(&new_note_path) {
        Ok(_) => {
            crate::services::logger::log_success("created note file succesfullly");
            Ok(crate::models::note::Note {
                local_id: uuid::Uuid::new_v4(),
                mongo_id: None,
                owner_id: uuid::Uuid::new_v4(),

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
        Err(err) => {
            crate::services::logger::log_error("couldnt create a file {}", err);
            todo!(); //dodać obsługe, poprostu nie moge utworzyć pliku, popup z tym komunikatem żeby zmienić uprawnienia i wróć do działania programu
            Err(crate::errors::Error::FileOperationError(err))
        }
    }
}

///function responsible for all operations needed to initialize note, creating struct and inserting it to sqlite
pub fn add_note_to_database(
    conn: &mut rusqlite::Connection,
    path: &PathBuf,
    name: String,
) -> Result<(), crate::errors::Error> {
    let name = name.trim().to_string();
    validate_note_name(&name, &conn)?;
    if let Ok(note) = init_note(path, name) {
        let tx = conn.transaction()?;
        tx.execute("INSERT INTO notes (local_id, mongo_id, owner_id, name, title, summary, content_path, created_at, updated_at, deleted_at, version, cloud_version, sync_state, is_deleted, encrypted, crypto_meta) VALUES (:local_id, :mongo_id, :owner_id, :name, :title, :summary, :content_path, :created_at, :updated_at, :deleted_at, :version, :cloud_version, :sync_state, :is_deleted, :encrypted, :crypto_meta); "
        , rusqlite::named_params!{
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
    })?;
        tx.commit()?;
        crate::services::logger::log_success(
            "successfully initialized note and created record in notes table",
        );
        Ok(())
    } else {
        //TODO add error handling when tauri added
        todo!() //tutaj będzie poprostu powrót do strony frontendowej + dodanie structu do to_save.json
    }
}

fn validate_note_name(note_name: &str, conn: &Connection) -> Result<(), crate::errors::Error> {
    let exists = conn
        .query_row(
            "SELECT 1 FROM notes WHERE name = :note_name",
            rusqlite::params![note_name],
            |_row| Ok(()),
        )
        .optional()?
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
    let name = "tesg".to_owned();
    init_note(&path.notes_path, name.clone()).unwrap();
    let mut new_note_path = path.notes_path;
    new_note_path.push(name);
    new_note_path.set_extension("md");
    assert!(std::path::Path::exists(&new_note_path));
} //TODO add name validation and whitespace deletation if needed
#[test]
fn add_to_db() {
    let path = crate::config::ProgramFiles::init().unwrap();
    let mut conn = crate::services::storage::db_creation::get_connection(&path);
    let name = "test".to_owned();
    add_note_to_database(&mut conn, &path.notes_path, name).unwrap();
}

#[test]
fn note_validator_test() {
    let path = crate::config::ProgramFiles::init().unwrap();
    let conn = crate::services::storage::db_creation::get_connection(&path);
    let note_name = "test";
    validate_note_name(note_name, &conn).unwrap();
}
//TODO add logs in appropriate places
