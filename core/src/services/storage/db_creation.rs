//! # Notes database creation module
//! **Purpose**: Opens a connection to the notes SQLite database, applies all required PRAGMAs for
//! performance and integrity, and ensures the full schema exists. Also defines [`SyncState`] with
//! the `ToSql` / `FromSql` impls needed to bind it directly in queries.
//!
//! ## Exported items
//! * [`get_connection`] — Public entry point; calls the internal setup, logs the result, and
//!   returns a ready-to-use [`Connection`]
//! * [`SyncState`] — Enum representing the cloud sync status of a note or attachment
//!   (`LocalOnly`, `PendingUpload`, `Synced`, `Conflict`, `Error`, `PendingDeleted`);
//!   serialisable and bindable directly to SQLite TEXT columns
//!
//! ## Key design decisions
//! Schema creation runs inside a transaction so the batch is atomic — either all tables and
//! indexes exist or none do. PRAGMAs are set on every connection open: `foreign_keys = ON`,
//! `journal_mode = WAL`, `synchronous = NORMAL`, `cache_size = -64000` (64 MB), `temp_store =
//! MEMORY`, and `busy_timeout = 5000 ms`. WAL mode is chosen to allow concurrent reads during
//! writes, which matters for the Tauri frontend fetching notes while a sync worker writes.
//!
//! ## Dependencies
//! - `rusqlite` — SQLite connection, pragma updates, transaction, `ToSql`/`FromSql` traits
//! - `anyhow` — `.context()` error propagation

use crate::utils::{Format, log_helper};
use anyhow::Context;
use rusqlite::{Connection, Result};

use crate::constants::NOTE_DB_SCHEMA;
use crate::migrations::migration::run_notes_migration;

pub fn get_connection(
    paths: &crate::config::ProgramFiles,
) -> Result<Connection, crate::errors::Error> {
    let notes_db = creating_tables(paths)
        .inspect_err(|err| crate::services::logger::log_error("error while creating tables", &err))
        .context("couldnt create tables in getting connection foruser connection to db")?;
    log_helper(
        "connecting to notes database and creating it",
        "success",
        None::<Format<String>>,
        "Successfully got connection",
    );
    Ok(notes_db)
}

fn creating_tables(
    paths: &crate::config::ProgramFiles,
) -> Result<Connection, crate::errors::Error> {
    let notes_db_res = Connection::open(&paths.data_base_path);
    if let Err(ref e) = notes_db_res {
        tracing::error!(
            task = "opening notes db",
            path = ?paths.data_base_path,
            error = ?e,
            "Failed to open notes database file"
        );
    }
    let mut notes_db = notes_db_res.context("couldnt establish connection to notes database")?;

    notes_db
        .pragma_update(None, "foreign_keys", "ON")
        .context("Pragma error while creating notes db, foreign_keys")?;
    notes_db
        .pragma_update(None, "synchronous", "NORMAL")
        .context("Pragma error while creating notes db, synchronous")?;
    notes_db
        .pragma_update(None, "cache_size", "-64000")
        .context("Pragma error while creating notes db, cache size")?;
    notes_db
        .pragma_update(None, "temp_store", "MEMORY")
        .context("Pragma error while creating notes db, temp_store")?;
    notes_db
        .pragma_update(None, "busy_timeout", "5000")
        .context("Pragma error while creating notes db, busy timeout")?;
    notes_db
        .pragma_update(None, "journal_mode", "WAL")
        .context("Pragma error while creating notes db, journal mode")?;

    // opcjonalne potwierdzenie (tylko do logów)
    if let Ok(mode) = notes_db.pragma_query_value(None, "journal_mode", |r| r.get::<_, String>(0)) {
        crate::services::logger::log_success(&format!("Journal mode set to {}", mode));
    }
    let tx = notes_db
        .transaction()
        .inspect_err(|e| {
            tracing::error!(
                task = "creating transaction",
                path = ?paths.data_base_path,
                error = ?e,
                "failed to start transaction for notes db"
            );
        })
        .context("failed to create database for notes")?;
    let schema = NOTE_DB_SCHEMA;
    tx.execute_batch(schema)
        .inspect_err(|e| {
            tracing::error!(
                task = "execute schema",
                path = ?paths.data_base_path,
                error = ?e,
                "failed to execute notes DB schema"
            );
        })
        .context("Couldnt create notes database SQL ERROR")?;
    tx.commit()
        .inspect_err(|e| {
            tracing::error!(
                task = "commit schema tx",
                path = ?paths.data_base_path,
                error = ?e,
                "failed to commit notes DB schema transaction"
            );
        })
        .context("failed to create database")?;
    run_notes_migration(&notes_db).context("failed while running notes db migration")?;
    log_helper(
        "ensuring notes db schema",
        "success",
        None::<Format<String>>,
        "db schema correct",
    );
    Ok(notes_db)
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
    let paths = crate::config::ProgramFiles::init_in_base().unwrap();
    let connection = get_connection(&paths);
    println!("{:?}", connection);
}
