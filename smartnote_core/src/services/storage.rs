use rusqlite::{Connection, Result};

pub fn get_connection(paths: &crate::config::ProgramFiles) -> Connection {
    match creating_tables(paths) {
        Ok(conn) => conn,
        Err(err) => {
            crate::services::logger::log_error("error while creating tables", err);
            std::process::exit(1);
        }
    }
}
fn creating_tables(paths: &crate::config::ProgramFiles) -> Result<Connection> {
    let connection = match Connection::open(paths.data_base_path.clone()) {
        Ok(conn) => {
            //creating note table
            conn.execute(
                "CREATE TABLE IF NOT EXISTS notes (
    local_id TEXT PRIMARY KEY,
    mongo_id TEXT,
    owner_id TEXT NOT NULL,
    
    name TEXT NOT NULL,
    title TEXT NOT NULL,
    content_path TEXT NOT NULL,
    
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER,
    
    version INTEGER NOT NULL DEFAULT 1,
    cloud_version INTEGER NOT NULL DEFAULT 0,
    
    sync_state TEXT NOT NULL DEFAULT 'LocalOnly',
    is_deleted INTEGER NOT NULL DEFAULT 0,
    
    encrypted INTEGER NOT NULL DEFAULT 1,
    crypto_meta TEXT,
    
    UNIQUE(owner_id, name),
    CHECK(sync_state IN ('LocalOnly', 'PendingUpload', 'Synced', 'Conflict', 'Error'))
);",
                (),
            )?;
            // creating indexes for note table
            conn.execute("CREATE INDEX IF NOT EXISTS idx_notes_owner_updated ON notes(owner_id, updated_at DESC);
", ())?;

            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_notes_sync_state ON notes(sync_state);
",
                (),
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_notes_mongo_id ON notes(mongo_id);
",
                (),
            )?;

            //create table of attachments
            conn.execute(
                "CREATE TABLE IF NOT EXISTS attachments (
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
);",
                (),
            )?;
            //creating indexes for attachment table
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_attachments_note ON attachments(note_local_id);
",
                (),
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_attachments_cloud_key ON attachments(cloud_key);
",
                (),
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_attachments_sync_state ON attachments(sync_state);
",
                (),
            )?;

            //creating tags table
            conn.execute(
                "CREATE TABLE IF NOT EXISTS tags (
    tag_id TEXT PRIMARY KEY,
    owner_id TEXT NOT NULL,
    name TEXT NOT NULL,
    color TEXT DEFAULT '#3B82F6',
    created_at INTEGER NOT NULL,
    UNIQUE(owner_id, name)
);",
                (),
            )?;
            //creating table of note tags
            conn.execute(
                "CREATE TABLE IF NOT EXISTS note_tags (
    note_local_id TEXT NOT NULL REFERENCES notes(local_id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(tag_id) ON DELETE CASCADE,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (note_local_id, tag_id)
);",
                (),
            )?;
            //creating indexes for note tags tables
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_note_tags_tag ON note_tags(tag_id);
",
                (),
            )?;
            // create metadata table
            conn.execute(
                "CREATE TABLE IF NOT EXISTS sync_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);",
                (),
            )?;
            conn
        }
        Err(err) => {
            //display window in taurii with info that permission needs to be chaged for these paths: or user with bigger permissions
            //after window is closed close program
            crate::services::logger::log_error("couldnt create sqlite database, exitting", err);
            println!("If the problem persists, delete config.json and restart the app."); //nice message to user in tauri
            std::process::exit(1);
        }
    };

    crate::services::logger::log_success("created database successfully");
    Ok(connection)
}

#[test]
fn test_of_db() {
    let paths = crate::config::ProgramFiles::init();
    let connection = get_connection(&paths);
    println!("{:?}", connection);
}

pub enum SyncState {
    LocalOnly,
    PendingUpload,
    Synced,
    Conflict,
    Error,
}
