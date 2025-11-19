//! Module used for creating and checking database settings, this module also gives connection to db

use rusqlite::{Connection, Result};
///This function gives connection to or returns db error and log it to user
pub fn get_connection(paths: &crate::config::ProgramFiles) -> Connection {
    match creating_tables(paths) {
        Ok(conn) => conn,
        Err(err) => {
            crate::services::logger::log_error("error while creating tables", err);
            std::process::exit(1); //TODO When adding tauri part just change it to Result<Connection, AppError> propagate to tauri error handler (catch) and display popup with options chnage permissions or leave program
        }
    }
}

fn creating_tables(paths: &crate::config::ProgramFiles) -> Result<Connection> {
    let conn = Connection::open(&paths.data_base_path)?;

    // ustawienia pragm
    conn.pragma_update(None, "foreign_keys", &"ON")?;
    conn.pragma_update(None, "synchronous", &"NORMAL")?;
    conn.pragma_update(None, "cache_size", &"-64000")?;
    conn.pragma_update(None, "temp_store", &"MEMORY")?;
    conn.pragma_update(None, "busy_timeout", &"5000")?;
    conn.pragma_update(None, "journal_mode", &"WAL")?;

    // opcjonalne potwierdzenie (tylko do logów)
    if let Ok(mode) = conn.pragma_query_value(None, "journal_mode", |r| r.get::<_, String>(0)) {
        crate::services::logger::log_success(&format!("Journal mode set to {}", mode));
    }

    let schema = r#"
    BEGIN;
    CREATE TABLE IF NOT EXISTS notes (
        local_id TEXT PRIMARY KEY,
        mongo_id TEXT,
        owner_id TEXT NOT NULL,
    
        name TEXT NOT NULL,
        title TEXT NOT NULL,
        summary TEXT NOT NULL,
        content_path TEXT NOT NULL,
        
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL,
        deleted_at INTEGER,
        
        version INTEGER NOT NULL DEFAULT 1,
        cloud_version INTEGER DEFAULT NULL,
        
        sync_state TEXT NOT NULL DEFAULT 'LocalOnly',
        is_deleted INTEGER NOT NULL DEFAULT 0,
        
        encrypted INTEGER NOT NULL DEFAULT 1,
        crypto_meta TEXT,
        
        UNIQUE(owner_id, name),
        CHECK(sync_state IN ('LocalOnly', 'PendingUpload', 'Synced', 'Conflict', 'Error'))
    );

    CREATE INDEX IF NOT EXISTS idx_notes_owner_updated ON notes(owner_id, updated_at DESC);
    CREATE INDEX IF NOT EXISTS idx_notes_sync_state ON notes(sync_state);
    CREATE INDEX IF NOT EXISTS idx_notes_mongo_id ON notes(mongo_id);

    CREATE TABLE IF NOT EXISTS attachments (
        attachment_id TEXT PRIMARY KEY,
        note_local_id TEXT NOT NULL REFERENCES notes(local_id) ON DELETE CASCADE,
        
        filename TEXT NOT NULL,
        mime_type TEXT NOT NULL,
        size_bytes INTEGER NOT NULL,
        
        local_path TEXT,
        cloud_key TEXT,
       
        checksum_encrypted TEXT NOT NULL,
        
        encrypted INTEGER NOT NULL DEFAULT 1,
        crypto_meta TEXT,
        
        sync_state TEXT NOT NULL DEFAULT 'LocalOnly',
        
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL,
        
        CHECK(sync_state IN ('LocalOnly', 'PendingUpload', 'Synced', 'Error'))
    );

    CREATE INDEX IF NOT EXISTS idx_attachments_note ON attachments(note_local_id);
    CREATE INDEX IF NOT EXISTS idx_attachments_cloud_key ON attachments(cloud_key);
    CREATE INDEX IF NOT EXISTS idx_attachments_sync_state ON attachments(sync_state);

    CREATE TABLE IF NOT EXISTS tags (
        tag_id TEXT PRIMARY KEY,
        owner_id TEXT NOT NULL,
        name TEXT NOT NULL,
        color TEXT DEFAULT '#3B82F6',
        created_at INTEGER NOT NULL,
        UNIQUE(owner_id, name)
    );

    CREATE TABLE IF NOT EXISTS note_tags (
        note_local_id TEXT NOT NULL REFERENCES notes(local_id) ON DELETE CASCADE,
        tag_id TEXT NOT NULL REFERENCES tags(tag_id) ON DELETE CASCADE,
        created_at INTEGER NOT NULL,
        PRIMARY KEY (note_local_id, tag_id)
    );

    CREATE INDEX IF NOT EXISTS idx_note_tags_tag ON note_tags(tag_id);

    CREATE TABLE IF NOT EXISTS sync_metadata (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL,
        updated_at INTEGER NOT NULL
    );
    COMMIT;
    "#;

    conn.execute_batch(schema)?;

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
}

impl rusqlite::ToSql for SyncState {
    fn to_sql(&self) -> Result<rusqlite::types::ToSqlOutput<'_>> {
        let s = match self {
            Self::LocalOnly => "LocalOnly",
            Self::PendingUpload => "PendingUpload",
            Self::Synced => "Synced",
            Self::Conflict => "Conflict",
            Self::Error => "Error",
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
