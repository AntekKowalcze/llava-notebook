///creation of user_data local database
pub fn connect_or_create_local_login_db(
    paths: &crate::config::ProgramFiles,
) -> Result<rusqlite::Connection, crate::errors::Error> {
    let local_login_conn = rusqlite::Connection::open(&paths.local_login_database_path)?;
    local_login_conn.pragma_update(None, "synchronous", &"NORMAL")?;
    local_login_conn.pragma_update(None, "cache_size", &"-2000")?;
    local_login_conn.pragma_update(None, "temp_store", &"MEMORY")?;
    local_login_conn.pragma_update(None, "journal_mode", &"WAL")?;
    let schema = r#"BEGIN; CREATE TABLE IF NOT EXISTS users_data (
                        user_id TEXT PRIMARY KEY,
                        username TEXT NOT NULL,
                        password_hash TEXT NOT NULL, 
                        password_salt TEXT NOT NULL,
                        notes_key BLOB NOT NULL,
                        nonce_notes_key BLOB NOT NULL,
                        is_online_linked INT NOT NULL DEFAULT 0, 
                        online_account_email TEXT DEFAULT NULL, 
                        device_id TEXT NOT NULL,
                        created_at int NOT NULL,  
                        last_login int NOT NULL, 
                        UNIQUE(username)
                        );
                        
                        CREATE INDEX IF NOT EXISTS idx_users_data_username ON users_data(username);
                        COMMIT;
                        "#;
    local_login_conn.execute_batch(schema)?;
    Ok(local_login_conn)
}
