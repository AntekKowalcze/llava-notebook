use anyhow::Context;

use crate::constans::LOCAL_LOGIN_DB_SCHEMA;
//smartnote/users/local_login_db.sqlite
///creation of user_data local database
pub fn connect_or_create_local_login_db() -> Result<rusqlite::Connection, crate::errors::Error> {
    let home_path = dirs_next::data_local_dir().ok_or(crate::errors::Error::FatalError)?;
    let mut local_login_db_path = home_path.join("smartnote/users");
    std::fs::create_dir_all(&local_login_db_path)?;
    local_login_db_path = home_path.join("smartnote/users/local_login_db.sqlite");

    let mut local_login_conn = rusqlite::Connection::open(local_login_db_path).context(
        "Couldnt create, read or find local_login database, couldnt establish connection.",
    )?;
    local_login_conn
        .pragma_update(None, "synchronous", &"NORMAL")
        .context("Pragma error while creating local users db, synchronous")?;
    local_login_conn
        .pragma_update(None, "cache_size", &"-2000")
        .context("Pragma error while creating local users db, cache_size")?;
    local_login_conn
        .pragma_update(None, "temp_store", &"MEMORY")
        .context("Pragma error while creating local users db, temp_store")?;
    local_login_conn
        .pragma_update(None, "journal_mode", &"WAL")
        .context("Pragma error while creating local users db, journal_mode")?;
    let tx = local_login_conn
        .transaction()
        .context("Couldnt create local login database, couldnt create transaction")?;
    tx.execute_batch(LOCAL_LOGIN_DB_SCHEMA)
        .context("Couldnt create database of local users in database creation")?;
    tx.commit()
        .context("Couldnt create local login db, couldnt commit transaction")?;

    Ok(local_login_conn)
}
