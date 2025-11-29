use std::str::FromStr;

use crate::utils::{Format, log_helper};
use anyhow::Context;
use argon2::{
    Argon2, PasswordHash, PasswordVerifier,
    password_hash::{self, SaltString},
};
use rusqlite::{OptionalExtension, named_params};
use zeroize::Zeroize;

pub fn local_log_in(
    username: String,
    mut password: zeroize::Zeroizing<String>,
    conn: &rusqlite::Connection,
) -> Result<uuid::Uuid, crate::errors::Error> {
    check_if_user_exists(&username, conn)?;
    //get hash and salt from db for this username, then hash given password again and check if hashes are the same if yes log in
    //if no return error wrong password,
    //there will be function to do thing after login, change active user, get paths, load notes,

    let hash = conn
        .query_row(
            "SELECT password_hash, password_salt FROM users_data WHERE username = :username",
            rusqlite::params![username],
            |row| {
                let hash: String = row.get(0)?;

                Ok(hash)
            },
        )
        .context("Couldnt get password_hash, password_salt FROM users_data db ")?;

    let password_hash = PasswordHash::new(&hash)
        .context("Couldnt create a password hash from password given by user in login")?;
    let password_verified = Argon2::default()
        .verify_password(&password.as_bytes(), &password_hash)
        .is_ok();
    log_helper(
        "logging",
        "success",
        Some(Format::Display(&username)),
        "password verified succesfully",
    );

    if password_verified {
        crate::services::logger::log_success("logged successfully");
    } else {
        crate::services::logger::log_error("Logging failed", crate::errors::Error::WrongPassword);
        return Err(crate::errors::Error::WrongPassword);
    }
    //after logging add resetting password for local account, and then start online accounts,
    log_helper(
        "logging",
        "success",
        Some(Format::Display(&username)),
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
    Ok(user_uuid)
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
        log_helper(
            "logging",
            "success",
            Some(Format::Display(&username)),
            "user exists, can log in",
        );

        return Ok(());
    } else {
        tracing::error!(task="checking if user exists in db", status="error", %username, "user do not exists in database, cant log in");
        return Err(crate::errors::Error::UserNotExists);
    }
}

#[test]
fn login_test() {
    let username = "twelth".to_string();
    let password = zeroize::Zeroizing::from("ToJestTest!".to_string());
    let home_path = std::env::temp_dir();
    let conn =
        crate::services::auth::database_creation::connect_or_create_local_login_db(&home_path)
            .unwrap();
    local_log_in(username, password, &conn).unwrap();
}
//TODO add one test which tests everything
