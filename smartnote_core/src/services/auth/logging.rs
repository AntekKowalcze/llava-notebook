use argon2::{
    Argon2, PasswordHash, PasswordVerifier,
    password_hash::{self, SaltString},
};
use rusqlite::OptionalExtension;
use zeroize::Zeroize;

fn local_log_in(
    username: String,
    mut password: zeroize::Zeroizing<String>,
    conn: &rusqlite::Connection,
) -> Result<(), crate::errors::Error> {
    check_if_user_exists(&username, conn)?;
    //get hash and salt from db for this username, then hash given password again and check if hashes are the same if yes log in
    //if no return error wrong password,
    //there will be function to do thing after login, change active user, get paths, load notes,

    let hash = conn.query_row(
        "SELECT password_hash, password_salt FROM users_data WHERE username = :username",
        rusqlite::params![username],
        |row| {
            let hash: String = row.get(0)?;

            Ok(hash)
        },
    )?;

    let password_hash = PasswordHash::new(&hash)?;
    let password_verified = Argon2::default()
        .verify_password(&password.as_bytes(), &password_hash)
        .is_ok();

    println!("{password_verified :?} Password");
    if password_verified {
        crate::services::logger::log_success("logged successfully");
    } else {
        crate::services::logger::log_error("Logging failed", crate::errors::Error::WrongPassword);
        return Err(crate::errors::Error::WrongPassword);
    }
    //TODO reduce error variants and put them in distinct enums, for example, create Login error which will be enum with types of WrongPassword, WrongUserName, PasswordvalidationError Etc.
    //after logging add resetting password for local account, and then start online accounts,
    crate::services::logger::log_success("logged succesfully");
    Ok(())
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
        .optional()?
        .is_some();
    if exists {
        crate::services::logger::log_success("username exists, all correct");
        return Ok(());
    } else {
        crate::services::logger::log_error(
            "username does not exists failed",
            crate::errors::Error::UserNotExists,
        );
        return Err(crate::errors::Error::UserNotExists);
    }
}

#[test]
fn login_test() {
    let paths = crate::config::ProgramFiles::init().unwrap();
    let username = "eight".to_string();
    let password = zeroize::Zeroizing::from("ToJestTest!".to_string());
    let conn =
        crate::services::auth::database_creation::connect_or_create_local_login_db(&paths).unwrap();
    local_log_in(username, password, &conn).unwrap();
}
//TODO add one test which tests everything
