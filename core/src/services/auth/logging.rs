//! # Local user login module
//! **Purpose**: This module is responsible for all actions taken during local user authentication.
//! It handles password verification, recovery code login, session token management, keyring storage,
//! failed attempt tracking with progressive timeouts, and logout.
//!
//! ## Exported functions
//! * [`local_log_in`] — Full password-based login flow: verifies user existence, checks password via Argon2,
//!   updates last login timestamp, initialises paths, creates session token
//! * [`log_with_code`] — Login using a recovery code; verifies the code against stored Argon2 hashes,
//!   marks code as used, and starts a session. Returns whether only one code remains
//! * [`change_last_login`] — Updates the `last_login` timestamp for a given user in a database transaction
//! * [`check_error_count`] — Increments the `password_errors` counter for a user; every 5 failures
//!   sets a progressive `ending_block_timestamp` timeout (30 s × multiplier)
//! * [`zero_error_count`] — Resets `password_errors` to 0 after a successful login
//! * [`get_timeout`] — Retrieves the current `ending_block_timestamp` for a user, used by the caller
//!   to determine whether the account is temporarily blocked
//! * [`session_operations`] — Generates a new UUIDv4 session token, stores its SHA-256 hash in
//!   `session_data`, and saves the raw token to the system keyring
//! * [`check_if_user_logged_in`] — Reads the session token from the keyring, hashes it, and queries
//!   `session_data`; returns a [`SessionState`] variant: `LoggedIn`, `Expired`, or `NotLoggedIn`
//! * [`local_logout`] — Clears the active user config, deletes the session row from `session_data`,
//!   and removes the token from the system keyring
//!
//! ## Key design decisions
//! Password is never stored in plain form — only an Argon2id PHC string is kept in the database and
//! verified at login time. The session token is a random UUID stored raw in the system keyring;
//! only its SHA-256 / Base32 hash lives in the database, so a keyring leak does not expose the DB
//! directly. Recovery codes are Crockford-Base32-encoded random bytes, hashed with Argon2id; each
//! code is single-use and marked with a `used_at` timestamp. All sensitive byte arrays
//! (`password`, code bytes, KEK bytes) are zeroized immediately after use.
//!
//! ## Dependencies
//! - `argon2` — Password and recovery-code hash verification
//! - `sha2` — SHA-256 digest of session UUID before DB storage
//! - `base32` — Crockford encoding for session token hash and recovery code representation
//! - `rusqlite` — SQLite access via `users_data`, `session_data`, and `recovery_keys` tables
//! - `keyring` — OS keyring storage for the raw session token
//! - `zeroize` — Secure memory wiping of passwords and key material
//! - `uuid` — UUIDv4 generation for session tokens and user identification

use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use rusqlite::{Connection, OptionalExtension, named_params};
use sha2::{Digest, Sha256};

use zeroize::Zeroize;

use crate::{ProgramFiles, errors, utils};

pub fn local_log_in(
    username: String,
    password: zeroize::Zeroizing<String>,
    users_db: &mut rusqlite::Connection,
    paths: &crate::config::ProgramFiles,
) -> Result<(uuid::Uuid, crate::config::ProgramFiles, Connection), crate::errors::Error> {
    check_if_user_exists(&username, users_db)?;
    let hash = users_db
        .query_row(
            "SELECT password_hash FROM users_data WHERE username = :username",
            rusqlite::params![username],
            |row| {
                let hash: String = row.get(0)?;
                Ok(hash)
            },
        )
        .inspect_err(|e| {
            tracing::error!(
                task = "logging",
                status = "error",
                error = ?e,
                "failed to fetch password_hash from users_data"
            )
        })
        .context("Couldnt get password_hash FROM users_data db ")?;

    let password_hash = PasswordHash::new(&hash)
        .inspect_err(|e| tracing::error!(
            task = "logging",
            status = "error",
            error = ?e,
            "failed to parse password hash from db"
        ))
        .context("CoPrepared using GPT-5.2 Thinkinguldnt create a password hash from password given by user in login")?;

    let password_verified = Argon2::default()
        .verify_password(&password.as_bytes(), &password_hash)
        .is_ok();

    if password_verified {
        crate::utils::log_helper(
            "logging",
            "success",
            Some(crate::utils::Format::Display(&username)),
            "password verified successfully",
        );
    } else {
        tracing::error!(
            task = "logging",
            status = "error",
            %username,
            "password verification failed"
        );
        return Err(crate::errors::Error::WrongPassword);
    }

    let user_uuid: String = users_db
        .query_row(
            "SELECT user_id FROM users_data WHERE username = :name",
            named_params! {
                ":name": &username,
            },
            |row| row.get("user_id"),
        )
        .inspect_err(|e| {
            tracing::error!(
                task = "logging",
                status = "error",
                error = ?e,
                %username,
                "failed to get user_id for username"
            )
        })
        .context("no user with this id")?;
    //delete session keyring token,
    let user_uuid = uuid::Uuid::parse_str(&user_uuid).context("failed to parse uuid")?;
    change_last_login(users_db, &user_uuid)?;
    let paths = crate::services::auth::register::after_validation(&user_uuid, paths)?;
    session_operations(&users_db, user_uuid)?;
    let users_db = crate::services::storage::db_creation::get_connection(&paths)?;
    crate::utils::log_helper(
        "logging",
        "success",
        Some(crate::utils::Format::Display(&username)),
        "user logged in succesfully",
    );
    Ok((user_uuid, paths, users_db))
}

pub fn log_with_code(
    paths: &crate::config::ProgramFiles,
    mut code: String,
    users_db: &rusqlite::Connection,
    user_id: uuid::Uuid,
) -> Result<(crate::config::ProgramFiles, Connection, bool), crate::errors::Error> {
    let mut found = 0;
    let count: i8 = users_db
        .query_row(
            "SELECT COUNT(*) FROM recovery_keys WHERE user_id = :id and used_at IS NULL",
            named_params! {":id": user_id.to_string()},
            |row| row.get::<_, i8>(0),
        )
        .context("Failed to get count of recovery codes left")?;
    let mut one_code = false;
    if count == 1 {
        one_code = true;
    }
    let mut stmt = users_db
        .prepare("SELECT code_hash FROM recovery_keys WHERE user_id = :id AND used_at IS NULL")
        .inspect_err(|e| {
            tracing::error!(
                task = "logging with code",
                status = "error",
                error = ?e,
                %user_id,
                "failed to prepare statement for recovery_keys"
            )
        })
        .context("failed to prepare statement")?;
    let mut handle = stmt
        .query(named_params! {
            ":id":user_id.to_string()
        })
        .inspect_err(|e| {
            tracing::error!(
                task = "logging with code",
                status = "error",
                error = ?e,
                %user_id,
                "failed to get handle to recovery codes"
            )
        })
        .context("failed to get handle to codes")?;

    if let Some(mut decoded) = base32::decode(base32::Alphabet::Crockford, &code) {
        let argon2 = Argon2::default();

        while let Some(row) = handle.next().context("failed to get next row")? {
            let mut hash: String = row
                .get(0)
                .inspect_err(|e| {
                    tracing::error!(
                        task = "logging with code",
                        status = "error",
                        error = ?e,
                        %user_id,
                        "failed to get recovery code hash from db"
                    )
                })
                .context("failed to get hash")?;
            let phc = PasswordHash::new(&hash)
                .inspect_err(|e| {
                    tracing::error!(
                        task = "logging with code",
                        status = "error",
                        error = ?e,
                        %user_id,
                        "failed to parse recovery hash to PHC"
                    )
                })
                .context("failed to parse hash from db to phc")?;
            if argon2.verify_password(&decoded, &phc).is_ok() {
                found += 1;
                users_db
                    .execute(
                        "UPDATE recovery_keys SET used_at = :time WHERE code_hash = :h",
                        named_params! {
                            ":time": utils::get_time(),
                            ":h": hash
                        },
                    )
                    .inspect_err(|e| {
                        tracing::error!(
                            task = "logging with code",
                            status = "error",
                            error = ?e,
                            %user_id,
                            "failed to mark recovery code as used"
                        )
                    })
                    .context("Failed to mark code as used")?;
            }
            hash.zeroize();

            code.zeroize();
            if found > 0 {
                let paths = crate::services::auth::register::after_validation(&user_id, paths)?;
                session_operations(&users_db, user_id)?;
                let users_db = crate::services::storage::db_creation::get_connection(&paths)?;
                decoded.zeroize();
                crate::utils::log_helper(
                    "logging with code",
                    "success",
                    None::<crate::utils::Format<String>>,
                    "user logged in using recovery code",
                );
                return Ok((paths, users_db, one_code));
            }
        }
    } else {
        tracing::error!(
            task = "logging with code",
            status = "error",
            "failed to decode recovery code from user"
        );
        return Err(errors::Error::InternalError(
            "Failed to decode code".to_string(),
        ));
    }
    tracing::error!(
        task = "logging with code",
        status = "error",
        user_id = %user_id,
        "no matching recovery code found, treating as wrong password",
    );
    Err(crate::errors::Error::WrongPassword)
}

fn check_if_user_exists(
    username: &str,
    users_db: &rusqlite::Connection,
) -> Result<(), crate::errors::Error> {
    let exists = users_db
        .query_row(
            "SELECT username FROM users_data WHERE username = :name",
            rusqlite::params![username],
            |_row| Ok(()),
        )
        .optional()
        .context("database error while logging in, couldnt check if user exists")?
        .is_some();
    if exists {
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
        .inspect_err(|e| {
            tracing::error!(
                task = "change last login",
                status = "error",
                error = ?e,
                "failed to create transaction for updating last_login"
            )
        })
        .context("failed to create transaction")?;
    tx.execute(
        "UPDATE users_data SET last_login = :time WHERE user_id = :id",
        named_params! {
            ":time": timestamp,
            ":id": current_user_id.to_string(),
        },
    )
    .inspect_err(|e| {
        tracing::error!(
            task = "change last login",
            status = "error",
            error = ?e,
            "failed to execute last_login update"
        )
    })
    .context("Failed to update users_id")?;
    tx.commit()
        .inspect_err(|e| {
            tracing::error!(
                task = "change last login",
                status = "error",
                error = ?e,
                "failed to commit transaction for last_login"
            )
        })
        .context("failed to commit transaction, rolling back")?;
    crate::utils::log_helper(
        "change last login",
        "success",
        Some(crate::utils::Format::Display(&current_user_id.to_string())),
        "last_login updated successfully",
    );
    Ok(())
}

pub fn check_error_count(
    users_db: &mut Connection,
    user_uuid: &uuid::Uuid,
) -> Result<i64, crate::errors::Error> {
    let mut end_of_timeout: i64 = 0;
    let tx = users_db
        .transaction()
        .inspect_err(|e| {
            tracing::error!(
                task = "check error count",
                status = "error",
                error = ?e,
                "failed to create transaction in checking errors"
            )
        })
        .context("failed to create transaction in checking errors")?;

    let statement: i64 = tx
        .query_row(
            "SELECT password_errors FROM users_data WHERE user_id = :user_id",
            named_params!(
                ":user_id": user_uuid.to_string(),
            ),
            |row| row.get(0),
        )
        .inspect_err(|e| {
            tracing::error!(
                task = "check error count",
                status = "error",
                error = ?e,
                "failed to fetch current password_errors"
            )
        })
        .context("failed to get statement")?;

    tx.execute(
        "UPDATE users_data SET password_errors = :new_count WHERE user_id = :id",
        rusqlite::named_params! {
            ":new_count":statement+1,
            ":id": user_uuid.to_string(),
        },
    )
    .inspect_err(|e| {
        tracing::error!(
            task = "check error count",
            status = "error",
            error = ?e,
            "failed to increment error count in usersdb"
        )
    })
    .context("failed to increment error count in usersdb")?;

    if (statement + 1) % 5 == 0 {
        let multiplier = (statement + 1) / 5;
        end_of_timeout = 30 * multiplier * 1000;
        end_of_timeout = crate::utils::get_time() + end_of_timeout;
        tx.execute(
            "UPDATE users_data SET ending_block_timestamp = :end WHERE user_id = :id",
            rusqlite::named_params! {
                ":end": end_of_timeout,
                ":id": user_uuid.to_string()
            },
        )
        .inspect_err(|e| {
            tracing::error!(
                task = "check error count",
                status = "error",
                error = ?e,
                "failed to set ending block timestamp in users data"
            )
        })
        .context("failed to set ending block timestamp in users data")?;
    }

    tx.commit()
        .inspect_err(|e| {
            tracing::error!(
                task = "check error count",
                status = "error",
                error = ?e,
                "failed to commit transaction in checking errors"
            )
        })
        .context("failed to commit transaciton in checking erros")?;
    crate::utils::log_helper(
        "check error count",
        "success",
        Some(crate::utils::Format::Display(&user_uuid.to_string())),
        "password error count updated",
    );
    Ok(end_of_timeout)
}

pub fn zero_error_count(
    users_db: &Connection,
    uuid: &uuid::Uuid,
) -> Result<(), crate::errors::Error> {
    users_db
        .execute(
            "UPDATE users_data SET password_errors = 0 WHERE user_id = :id",
            named_params! {
                ":id": uuid.to_string(),
            },
        )
        .inspect_err(|e| {
            tracing::error!(
                task = "zero error count",
                status = "error",
                error = ?e,
                "failed to reset password_errors to 0"
            )
        })
        .context("Failed to set error count to 0")?;
    crate::utils::log_helper(
        "zero error count",
        "success",
        Some(crate::utils::Format::Display(&uuid.to_string())),
        "password_errors reset to 0",
    );
    Ok(())
}

pub fn get_timeout(users_db: &Connection, uuid: &uuid::Uuid) -> Result<i64, crate::errors::Error> {
    let timeout = users_db
        .query_row(
            "SELECT ending_block_timestamp FROM users_data WHERE user_id = :id",
            named_params! {
                ":id": uuid.to_string(),
            },
            |row| row.get(0),
        )
        .inspect_err(|e| {
            tracing::error!(
                task = "get timeout",
                status = "error",
                error = ?e,
                "failed to get ending_block_timestamp"
            )
        })
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
        &Sha256::digest(session_uuid.to_string().as_bytes()),
    );
    let expires_at = crate::utils::get_time() / 1000 + crate::constants::SESSION_TOKEN_TIME_ALIVE;
    let mut stmt = users_db
        .prepare("INSERT INTO session_data(hashed_token, user_id, expires_at) VALUES (:session_hash, :uid, :expires);")
        .inspect_err(|e| tracing::error!(
            task = "session operations",
            status = "error",
            error = ?e,
            "failed to prepare insert statement to session_data"
        ))
        .context("failed to prepare insert statement to session_data")?;
    stmt.execute(named_params! {
        ":session_hash": session_uuid_hash,
        ":uid": user_id.to_string(),
        ":expires": expires_at,
    })
    .inspect_err(|e| {
        tracing::error!(
            task = "session operations",
            status = "error",
            error = ?e,
            "failed to insert data into session_data table"
        )
    })
    .context("failed to insert data into session_data table")?;
    let keyring_entry = keyring::Entry::new("llava_desktop", "session_token")
        .inspect_err(|e| {
            tracing::error!(
                task = "session operations",
                status = "error",
                error = ?e,
                "failed to create keyring entry"
            )
        })
        .context("failed to create keyring entry")?;
    keyring_entry
        .set_password(&session_uuid.to_string())
        .inspect_err(|e| {
            tracing::error!(
                task = "session operations",
                status = "error",
                error = ?e,
                "failed to set secret in keyring"
            )
        })
        .context("failed to set secret in keyring")?;
    crate::utils::log_helper(
        "session operations",
        "success",
        Some(crate::utils::Format::Display(&user_id.to_string())),
        "session token created and stored",
    );
    Ok(())
}
#[derive(Debug, serde::Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SessionState {
    LoggedIn { user_id: String },
    Expired,
    NotLoggedIn,
}

pub fn check_if_user_logged_in(
    users_db: &Connection,
    paths: &ProgramFiles,
) -> Result<SessionState, crate::errors::Error> {
    let keyring_entry = keyring::Entry::new("llava_desktop", "session_token")
        .inspect_err(|e| {
            tracing::error!(
                task = "check session",
                status = "error",
                error = ?e,
                "failed to create keyring entry"
            )
        })
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
            if expires_at > (crate::utils::get_time() / 1000) {
                crate::utils::log_helper(
                    "check session",
                    "success",
                    Some(crate::utils::Format::Display(&user_id)),
                    "user is logged in",
                );
                let parsed_user_id =
                    &uuid::Uuid::parse_str(&user_id).context("failed to parse ID")?;
                crate::config::change_active_user(parsed_user_id, &paths)?;
                Ok(SessionState::LoggedIn { user_id })
            } else {
                let _ = keyring_entry.delete_credential();
                crate::utils::log_helper(
                    "check session",
                    "success",
                    None::<crate::utils::Format<String>>,
                    "session expired, token deleted",
                );
                Ok(SessionState::Expired)
            }
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            let _ = keyring_entry.delete_credential();
            crate::utils::log_helper(
                "check session",
                "success",
                None::<crate::utils::Format<String>>,
                "no session found, token deleted",
            );
            Ok(SessionState::NotLoggedIn)
        }
        Err(e) => {
            tracing::error!(
                task = "check session",
                status = "error",
                error = ?e,
                "database error while checking session"
            );
            Err(crate::errors::Error::InternalError(e.to_string()))
        }
    }
}

pub fn local_logout(
    user_uuid: String,
    users_db: &Connection,
    paths: &crate::config::ProgramFiles,
) -> Result<(), crate::errors::Error> {
    crate::config::change_active_user(&uuid::Uuid::nil(), paths)?;
    users_db
        .execute(
            "DELETE FROM session_data WHERE user_id = :id",
            named_params! { ":id": user_uuid },
        )
        .inspect_err(|err| tracing::error!(task = "local logout", status="error", error = ?err, "database error while deleting session data"))
        .context("failed to delete session from db")?;

    // remove token from keyring
    let entry = keyring::Entry::new("llava_desktop", "session_token").inspect_err(|err| tracing::error!(task="logout", status="error", error=?err, "keyring error while creating entry"))
        .context("failed to create keyring entry")?;
    let _ = entry.delete_credential();
    crate::utils::log_helper(
        "local logout",
        "success",
        None::<crate::utils::Format<String>>,
        "user logged out successfully",
    );

    Ok(())
}
//workds only after register test
#[test]
fn login_test() {
    let mut users_db = rusqlite::Connection::open_in_memory().unwrap();
    users_db
        .execute_batch(crate::constants::LOCAL_LOGIN_DB_SCHEMA)
        .unwrap();
    let paths = crate::config::ProgramFiles::init_in_base().unwrap();
    let username = "twelth".to_string();
    let password = zeroize::Zeroizing::from("ToJestTest!".to_string());
    let password_r = zeroize::Zeroizing::from("ToJestTest!".to_string());

    crate::services::auth::register::register_user_offline(
        username.clone(),
        password.clone(),
        password_r,
        &paths,
        &mut users_db,
    );

    let home_path = std::env::temp_dir();
    local_log_in(username, password, &mut users_db, &paths).unwrap();
}
