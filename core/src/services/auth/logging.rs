use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use rusqlite::{Connection, OptionalExtension, named_params};
use sha2::{Digest, Sha256};

use zeroize::Zeroize;

use crate::{errors, utils};

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

    let user_uuid: String = conn
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

    let user_uuid = uuid::Uuid::parse_str(&user_uuid).context("failed to parse uuid")?;
    change_last_login(conn, &user_uuid)?;
    let paths = crate::services::auth::register::after_validation(&user_uuid, paths)?;
    session_operations(&conn, user_uuid)?;
    let conn = crate::services::storage::db_creation::get_connection(&paths)?;
    crate::utils::log_helper(
        "logging",
        "success",
        Some(crate::utils::Format::Display(&username)),
        "user logged in succesfully",
    );
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
        while let Ok(Some(row)) = handle.next() {
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
                let conn = crate::services::storage::db_creation::get_connection(&paths)?;
                decoded.zeroize();
                crate::utils::log_helper(
                    "logging with code",
                    "success",
                    None::<crate::utils::Format<String>>,
                    "user logged in using recovery code",
                );
                return Ok((paths, conn));
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
    conn: &mut Connection,
    user_uuid: &uuid::Uuid,
) -> Result<i64, crate::errors::Error> {
    let mut end_of_timeout: i64 = 0;
    let tx = conn
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

pub fn zero_error_count(conn: &Connection, uuid: &uuid::Uuid) -> Result<(), crate::errors::Error> {
    conn.execute(
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

pub fn get_timeout(conn: &Connection, uuid: &uuid::Uuid) -> Result<i64, crate::errors::Error> {
    let timeout = conn
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
    let expires_at = crate::utils::get_time() / 100 + crate::constants::SESSION_TOKEN_TIME_ALIVE;
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
            if expires_at > (crate::utils::get_time() / 100) {
                crate::utils::log_helper(
                    "check session",
                    "success",
                    Some(crate::utils::Format::Display(&user_id)),
                    "user is logged in",
                );
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
