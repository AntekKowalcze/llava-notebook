//! Module used for creating and checking database settings, this module also gives connection to db

use anyhow::Context;
use rusqlite::{Connection, Result};

use crate::constans::NOTE_DB_SCHEMA;
///This function gives connection to or returns db error and log it to user
pub fn get_connection(
    paths: &crate::config::ProgramFiles,
) -> Result<Connection, crate::errors::Error> {
    let conn = creating_tables(paths)
        .inspect_err(|err| crate::services::logger::log_error("error while creating tables", &err))
        .context("couldnt create tables in getting connection foruser connection to db")?;
    Ok(conn)
}

fn creating_tables(
    paths: &crate::config::ProgramFiles,
) -> Result<Connection, crate::errors::Error> {
    let mut conn = Connection::open(&paths.data_base_path)
        .context("couldnt establish connection to notes database")?;

    // ustawienia pragm
    conn.pragma_update(None, "foreign_keys", &"ON")
        .context("Pragma error while creating notes db, foreign_keys")?;
    conn.pragma_update(None, "synchronous", &"NORMAL")
        .context("Pragma error while creating notes db, synchronous")?;
    conn.pragma_update(None, "cache_size", &"-64000")
        .context("Pragma error while creating notes db, cache size")?;
    conn.pragma_update(None, "temp_store", &"MEMORY")
        .context("Pragma error while creating notes db, temp_store")?;
    conn.pragma_update(None, "busy_timeout", &"5000")
        .context("Pragma error while creating notes db, busy timeout")?;
    conn.pragma_update(None, "journal_mode", &"WAL")
        .context("Pragma error while creating notes db, journal mode")?;

    // opcjonalne potwierdzenie (tylko do logów)
    if let Ok(mode) = conn.pragma_query_value(None, "journal_mode", |r| r.get::<_, String>(0)) {
        crate::services::logger::log_success(&format!("Journal mode set to {}", mode));
    }
    let tx = conn
        .transaction()
        .context("failed to create database for notes")?;
    let schema = NOTE_DB_SCHEMA;
    tx.execute_batch(schema)
        .context("Couldnt create notes database SQL ERROR")?;
    tx.commit().context("failed to create database")?;

    crate::services::logger::log_success("Database schema ensured");
    Ok(conn)
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum SyncState {
    LocalOnly,
    PendingUpload,
    Synced,
    Conflict,
    Error,
    PendingDeleted,
}

impl rusqlite::ToSql for SyncState {
    fn to_sql(&self) -> Result<rusqlite::types::ToSqlOutput<'_>> {
        let s = match self {
            Self::LocalOnly => "LocalOnly",
            Self::PendingUpload => "PendingUpload",
            Self::Synced => "Synced",
            Self::Conflict => "Conflict",
            Self::Error => "Error",
            Self::PendingDeleted => "PendingDeleted",
        };
        Ok(rusqlite::types::ToSqlOutput::from(s))
    }
}

impl rusqlite::types::FromSql for SyncState {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        match value.as_str()? {
            "LocalOnly" => Ok(SyncState::LocalOnly),
            "PendingUpload" => Ok(SyncState::PendingUpload),
            "Synced" => Ok(SyncState::Synced),
            "Conflict" => Ok(SyncState::Conflict),
            "Error" => Ok(SyncState::Error),
            "PendingDeleted" => Ok(SyncState::PendingDeleted),
            _ => Err(rusqlite::types::FromSqlError::InvalidType),
        }
    }
}

#[test]
fn test_of_db() {
    let paths = crate::config::ProgramFiles::init().unwrap();
    let connection = get_connection(&paths);
    println!("{:?}", connection);
}
