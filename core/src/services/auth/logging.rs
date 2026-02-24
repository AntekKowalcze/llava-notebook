use anyhow::Context;
use argon2::{
    Argon2, PasswordHash, PasswordVerifier,
    password_hash::{self, SaltString},
};
use rusqlite::{Connection, OptionalExtension, named_params};
use serde::de;
use sha2::{Digest, Sha256};
use std::str::FromStr;
use uuid::timestamp;
use zeroize::Zeroize;

use crate::{constants::SESSION_TOKEN_TIME_ALIVE, errors, utils};

pub fn local_log_in(
    username: String,
    password: zeroize::Zeroizing<String>,
    conn: &mut rusqlite::Connection,
    paths: &crate::config::ProgramFiles,
) -> Result<(uuid::Uuid, crate::config::ProgramFiles, Connection), crate::errors::Error> {
    check_if_user_exists(&username, conn)?;
    let hash = conn
        .query_row(
            "SELECT password_hash FROM users_data WHERE username = :username",
            rusqlite::params![username],
            |row| {
                let hash: String = row.get(0)?;

                Ok(hash)
            },
        )
        .context("Couldnt get password_hash FROM users_data db ")?;

    let password_hash = PasswordHash::new(&hash)
        .context("CoPrepared using GPT-5.2 Thinkinguldnt create a password hash from password given by user in login")?;
    let password_verified = Argon2::default()
        .verify_password(&password.as_bytes(), &password_hash)
        .is_ok();
    crate::utils::log_helper(
        "logging",
        "success",
        Some(crate::utils::Format::Display(&username)),
        "password verified succesfully",
    );

    if password_verified {
        crate::services::logger::log_success("logged successfully");
    } else {
        crate::services::logger::log_error("Logging failed", crate::errors::Error::WrongPassword);
        return Err(crate::errors::Error::WrongPassword);
    }
    //after logging add resetting password for local account, and then start online accounts,
    crate::utils::log_helper(
        "logging",
        "success",
        Some(crate::utils::Format::Display(&username)),
        "user logged in succesfully",
    );
    let user_uuid: String = conn
        .query_row(
            "SELECT user_id FROM users_data WHERE username = :name",
            named_params! {
                ":name": &username,
            },
            |row| row.get("user_id"),
        )
        .context("no user with this id")?;
    let user_uuid = uuid::Uuid::parse_str(&user_uuid).context("failed to parse uuid")?;
    change_last_login(conn, &user_uuid)?;
    let paths = crate::services::auth::register::after_validation(&user_uuid, paths)?;
    session_operations(&conn, user_uuid)?;
    let conn = crate::services::storage::db_creation::get_connection(&paths)?;
    Ok((user_uuid, paths, conn))
}

pub fn log_with_code(
    paths: &crate::config::ProgramFiles,
    mut code: String,
    users_db: &rusqlite::Connection,
    user_id: uuid::Uuid,
) -> Result<(crate::config::ProgramFiles, Connection), crate::errors::Error> {
    let mut found = 0;
    let mut stmt = users_db
        .prepare("SELECT code_hash FROM recovery_keys WHERE user_id = :id AND used_at IS NULL")
        .context("failed to prepare statement")?;
    let mut handle = stmt
        .query(named_params! {
            ":id":user_id.to_string()
        })
        .context("failed to get handle to codes")?;

    if let Some(mut decoded) = base32::decode(base32::Alphabet::Crockford, &code) {
        let argon2 = Argon2::default();
        while let Ok(Some(row)) = handle.next() {
            let mut hash: String = row.get(0).context("failed to get hash")?;
            let phc = PasswordHash::new(&hash).context("failed to parse hash from db to phc")?;
            if argon2.verify_password(&decoded, &phc).is_ok() {
                found += 1;
                users_db
                    .execute(
                        "UPDATE recovery_keys SET used_at = :time WHERE code_hash = :h",
                        named_params! {
                            ":time": utils::get_time(),
                            ":h": hash//should i use phc here instead of hash
                        },
                    )
                    .context("Failed to mark code as used")?;
            }
            hash.zeroize();

            code.zeroize();
            if found > 0 {
                //change_last_login(&mut users_db, &user_id)?;
                let paths = crate::services::auth::register::after_validation(&user_id, paths)?;
                session_operations(&users_db, user_id)?;
                let conn = crate::services::storage::db_creation::get_connection(&paths)?;
                // Ok((user_uuid, paths, conn))
                decoded.zeroize();
                return Ok((paths, conn));
            }
        }
    } else {
        return Err(errors::Error::InternalError(
            "Failed to decode code".to_string(),
        ));
    }

    Err(crate::errors::Error::WrongPassword)
}

fn check_if_user_exists(
    username: &str,
    conn: &rusqlite::Connection,
) -> Result<(), crate::errors::Error> {
    let exists = conn
        .query_row(
            "SELECT username FROM users_data WHERE username = :name",
            rusqlite::params![username],
            |_row| Ok(()),
        )
        .optional()
        .context("database error while logging in, couldnt check if user exists")?
        .is_some();
    if exists {
        crate::services::logger::log_success("username exists, all correct");
        crate::utils::log_helper(
            "logging",
            "success",
            Some(crate::utils::Format::Display(&username)),
            "user exists, can log in",
        );

        return Ok(());
    } else {
        tracing::error!(task="checking if user exists in db", status="error", %username, "user do not exists in database, cant log in");
        return Err(crate::errors::Error::UserNotExists);
    }
}

pub fn change_last_login(
    users_db: &mut rusqlite::Connection,
    current_user_id: &uuid::Uuid,
) -> Result<(), crate::errors::Error> {
    let timestamp = crate::utils::get_time();
    let tx = users_db
        .transaction()
        .context("failed to create transaction")?;
    tx.execute(
        "UPDATE users_data SET last_login = :time WHERE user_id = :id",
        named_params! {
            ":time": timestamp,
            ":id": current_user_id.to_string(),
        },
    )
    .context("Failed to update users_id")?;
    tx.commit()
        .context("failed to commit transaction, rolling back")?;
    Ok(())
}

pub fn check_error_count(
    conn: &mut Connection,
    user_uuid: &uuid::Uuid,
) -> Result<i64, crate::errors::Error> {
    let mut end_of_timeout: i64 = 0;
    let tx = conn
        .transaction()
        .context("failed to create transaction in checking errors")?;

    let statement: i64 = tx
        .query_row(
            "SELECT password_errors FROM users_data WHERE user_id = :user_id",
            named_params!(
                ":user_id": user_uuid.to_string(),
            ),
            |row| row.get(0),
        )
        .context("failed to get statement")?;
    println!("{} error count", statement);
    tx.execute(
        "UPDATE users_data SET password_errors = :new_count WHERE user_id = :id",
        rusqlite::named_params! {

            ":new_count":statement+1,
            ":id": user_uuid.to_string(),
        },
    )
    .context("failed to increment error count in usersdb")?;

    if (statement + 1) % 5 == 0 {
        let multiplier = (statement + 1) / 5;
        end_of_timeout = 30 * multiplier * 1000; //miliseconds
        end_of_timeout = crate::utils::get_time() + end_of_timeout;
        tx.execute(
            "UPDATE users_data SET ending_block_timestamp = :end WHERE user_id = :id",
            rusqlite::named_params! {
                ":end": end_of_timeout,
                ":id": user_uuid.to_string()
            },
        )
        .context("failed to set ending block timestamp in users data")?;
    }

    tx.commit()
        .context("failed to commit transaciton in checking erros")?;
    Ok(end_of_timeout)
}

pub fn zero_error_count(conn: &Connection, uuid: &uuid::Uuid) -> Result<(), crate::errors::Error> {
    conn.execute(
        "UPDATE users_data SET password_errors = 0 WHERE user_id = :id",
        named_params! {
            ":id": uuid.to_string(),
        },
    )
    .inspect_err(|err| println!("{}", err))
    .context("Failed to set error count to 0")?;
    Ok(())
}
pub fn get_timeout(conn: &Connection, uuid: &uuid::Uuid) -> Result<i64, crate::errors::Error> {
    let timeout = conn
        .query_row(
            "SELECT ending_block_timestamp FROM users_data WHERE user_id = :id",
            named_params! {
                ":id": uuid.to_string(),
            },
            |row| row.get(0),
        )
        .context("Failed to get timeout")?;
    Ok(timeout)
}

pub fn session_operations(
    users_db: &Connection,
    user_id: uuid::Uuid,
) -> Result<(), crate::errors::Error> {
    let session_uuid = uuid::Uuid::new_v4();
    let session_uuid_hash = base32::encode(
        base32::Alphabet::Crockford,
        &Sha256::digest(session_uuid.to_string().as_bytes()), //changing uuid to sha and then encoding it as text / to_string() is used so hashes match when i save to keyring cuz it needs to_string also
    );
    let expires_at = crate::utils::get_time() / 100 + crate::constants::SESSION_TOKEN_TIME_ALIVE; // /100 -> to seconds
    let mut stmt = users_db.prepare("INSERT INTO session_data(hashed_token, user_id, expires_at) VALUES (:session_hash, :uid, :expires);").context("failed to prepare insert statement to session_data")?;
    stmt.execute(named_params! {
        ":session_hash": session_uuid_hash,
        ":uid": user_id.to_string(),
        ":expires": expires_at,
    })
    .context("failed to insert data into session_data table")?;
    let keyring_entry = keyring::Entry::new("llava_desktop", "session_token")
        .context("failed to create keyring entry")?;
    keyring_entry
        .set_password(&session_uuid.to_string())
        .context("failed to set secret in keyring")?;
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SessionState {
    LoggedIn { user_id: String },
    Expired,
    NotLoggedIn,
}
pub fn check_if_user_logged_in(
    users_db: &Connection,
) -> Result<SessionState, crate::errors::Error> {
    let keyring_entry = keyring::Entry::new("llava_desktop", "session_token")
        .context("failed to create keyring entry")?;

    let token = match keyring_entry.get_password() {
        Ok(t) => t,
        Err(keyring::Error::NoEntry) | Err(_) => return Ok(SessionState::NotLoggedIn),
    };

    let hashed = base32::encode(
        base32::Alphabet::Crockford,
        &Sha256::digest(token.as_bytes()),
    );

    let result = users_db.query_row(
        "SELECT user_id, expires_at FROM session_data WHERE hashed_token = ?1",
        rusqlite::params![hashed],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
    );

    match result {
        Ok((user_id, expires_at)) => {
            if expires_at > (crate::utils::get_time() / 100) {
                Ok(SessionState::LoggedIn { user_id })
            } else {
                let _ = keyring_entry.delete_credential();
                Ok(SessionState::Expired)
            }
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            let _ = keyring_entry.delete_credential();
            Ok(SessionState::NotLoggedIn)
        }
        Err(e) => Err(crate::errors::Error::InternalError(e.to_string())),
    }
}

#[test]
fn login_test() {
    let username = "twelth".to_string();
    let password = zeroize::Zeroizing::from("ToJestTest!".to_string());
    let home_path = std::env::temp_dir();
    let mut conn =
        crate::services::auth::database_creation::connect_or_create_local_login_db(&home_path)
            .unwrap();
    let paths = crate::config::ProgramFiles::init_in_base().unwrap();
    local_log_in(username, password, &mut conn, &paths).unwrap();
}
