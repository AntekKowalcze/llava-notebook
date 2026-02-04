use crate::constants::LOCAL_LOGIN_DB_SCHEMA;
use crate::utils::{Format, log_helper};
use anyhow::Context;
//llava/users/local_login_db.sqlite
///creation of user_data local database
pub fn connect_or_create_local_login_db(
    local_login_db_path: &std::path::Path,
) -> Result<rusqlite::Connection, crate::errors::Error> {
    // let home_path = dirs_next::data_local_dir().ok_or(crate::errors::Error::FatalError)?;
    //let mut local_login_db_path = home_path.join("llava/users");
    // std::fs::create_dir_all(&local_login_db_path.parent())?;
    tracing::info!("Local login db dirs created");
    //local_login_db_path = home_path.join("llava/users/local_login_db.sqlite");

    let mut local_login_conn = rusqlite::Connection::open(local_login_db_path).context(
        "Couldnt create, read or find local_login database, couldnt establish connection.",
    )?;
    tracing::info!("Created connection to local_login_db");
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
    local_login_conn
        .pragma_update(None, "foreign_keys", "ON")
        .context("Failed to update pragma for enabling fks")?;
    let tx = local_login_conn
        .transaction()
        .context("Couldnt create local login database, couldnt create transaction")?;
    tx.execute_batch(LOCAL_LOGIN_DB_SCHEMA)
        .context("Couldnt create database of local users in database creation")?;
    tx.commit()
        .context("Couldnt create local login db, couldnt commit transaction")?;
    log_helper::<String>(
        "creation of database",
        "success",
        None::<Format<String>>,
        "user log in database created successfully",
    );
    Ok(local_login_conn)
}
