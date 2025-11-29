//! in this module paths and config/meta values are stored

//config.rs
//folders
pub const USER_DIR_PATTERN: &str = "smartnote/users";
pub const SUBDIRS: &[&str; 5] = &["notes", "assets", "logs", "tmp", "tmp_delete"];
//jsons
pub const CONFIG_FILE: &str = "config.json";
pub const ACTIVE_USER_JSON_PATH: &str = "smartnote/active_user.json";
pub const DEVICE_ID_FILE: &str = "smartnote/device_id.json";
pub const DEVICE_ID_JSON_KEY: &str = "device_id";
pub const ACTIVE_USER_JSON_KEY: &str = "user_uuid";

//sqlite
pub const NOTES_DB: &str = "note.sqlite";
pub const LOCAL_USERS_DB: &str = "smartnote/users/local_login_db.sqlite";

pub const LOGS_PATH: &str = "logs/app.log";

//register.rs
//SQL
pub const LOCAL_USER_DB_INSERT_SQL_SCHEMA: &str = r#"INSERT INTO users_data (
        user_id,
        username,
        password_hash,
        password_salt,
        notes_key,
        nonce_notes_key,
        is_online_linked,
        online_account_email, 
        device_id, created_at, 
        last_login
        ) VALUES (
       :user_id,
       :username, 
       :password_hash, 
       :password_salt, 
       :notes_key, 
       :nonce_notes_key, 
       :is_online_linked, 
       :online_account_email, 
       :device_id, 
       :created_at, 
       :last_login)"#;

//Encryption
pub const KEY_ENCRYPTED_KEY_LENGTH: usize = 32;
pub const MINIMAL_PASSWORD_LENGTH: usize = 8;

//init_note.rs
//extensions
pub const NOTE_EXTENSION: &str = "md";

//limits
pub const MAX_NOTE_NAME_LENGTH: usize = 255;

//SQL
pub const INSERT_NOTE_SQL_SCHEMA: &str = "INSERT INTO notes (local_id, mongo_id, owner_id, name, title, summary, content_path, created_at, updated_at, deleted_at, version, cloud_version, sync_state, is_deleted, encrypted, crypto_meta) VALUES (:local_id, :mongo_id, :owner_id, :name, :title, :summary, :content_path, :created_at, :updated_at, :deleted_at, :version, :cloud_version, :sync_state, :is_deleted, :encrypted, :crypto_meta); ";

//update.rs

//extensions
pub const TEMP_NOTE_EXTENSION: &str = ".md.tmp";
//Limits
pub const SUMMARY_LENGTH: usize = 10;
pub const MAX_TITLE_LENGTH: usize = 30;
//SQL
pub const UPDATE_NOTE_SQL_QUERY: &str = "UPDATE notes SET updated_at = :updated_time , summary = :summary ,version = version + 1, title = :title WHERE local_id = :id";

//db_creation
pub const NOTE_DB_SCHEMA: &str = r#"
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
        CHECK(sync_state IN ('LocalOnly', 'PendingUpload', 'Synced', 'Conflict', 'Error', 'PendingDeleted'))
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
    "#;

//local login db creation

pub const LOCAL_LOGIN_DB_SCHEMA: &str = r#" CREATE TABLE IF NOT EXISTS users_data (
                        user_id TEXT PRIMARY KEY,
                        username TEXT NOT NULL,
                        password_hash TEXT NOT NULL, 
                        password_salt TEXT NOT NULL,
                        notes_key BLOB NOT NULL,
                        nonce_notes_key BLOB NOT NULL,
                        is_online_linked INTEGER NOT NULL DEFAULT 0, 
                        online_account_email TEXT DEFAULT NULL, 
                        device_id TEXT NOT NULL,
                        created_at INTEGER NOT NULL,  
                        last_login INTEGER NOT NULL, 
                        UNIQUE(username)
                        );
                        
                        CREATE INDEX IF NOT EXISTS idx_users_data_username ON users_data(username);
                        
                        "#;
