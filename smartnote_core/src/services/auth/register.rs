//! Module responsible for registering user
//! in this modules important data is encrypted, and keys for notes encryption are also created
use crate::constants::*;
use crate::utils::{Format, log_helper};
use anyhow::Context;
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use chacha20poly1305::{
    ChaCha20Poly1305,
    aead::{Aead, AeadCore, KeyInit},
};
use core::task;
use rusqlite::{Connection, OptionalExtension, params};
use zeroize::Zeroize;

use crate::config::ProgramFiles;
///function responsible for registering user offilne and adding it encrypted to local db
pub fn register_user_offline(
    username: String,
    password: zeroize::Zeroizing<String>,
    paths: &crate::config::ProgramFiles,
    conn: &mut Connection,
) -> Result<(), crate::errors::Error> {
    let username = username.trim().to_string();
    validate_username(&username, &conn)?;
    let password = password.as_str().trim();
    password_validation(password)?;
    let notes_key: chacha20poly1305::Key = ChaCha20Poly1305::generate_key(&mut OsRng);
    let (password_hash, salt, encrypted_notes_key, nonce_for_key_wrap) =
        generate_enctypted_keys(password, notes_key)?;
    let new_user = crate::services::auth::auth_data_models::local_user::LocalUser {
        user_id: uuid::Uuid::new_v4(),
        username: username,
        password_hash: password_hash,
        password_salt: salt,
        notes_key: encrypted_notes_key,
        nonce_notes_key: nonce_for_key_wrap,
        is_online_linked: false,
        online_account_email: None,
        device_id: crate::config::get_device_id()?,
        created_at: crate::utils::get_time(),
        last_login: crate::utils::get_time(),
    };

    let tx = conn.transaction().context(
        "Couldnt insert user into database, transaction failed while registering a user",
    )?;

    tx.execute(
        LOCAL_USER_DB_INSERT_SQL_SCHEMA,
        rusqlite::named_params! {
        ":user_id": new_user.user_id.to_string(),
         ":username":new_user.username ,
         ":password_hash":new_user.password_hash,
         ":password_salt":new_user.password_salt,
         ":notes_key":new_user.notes_key,
         ":nonce_notes_key":new_user.nonce_notes_key,
         ":is_online_linked": new_user.is_online_linked,
         ":online_account_email":new_user.online_account_email,
         ":device_id": new_user.device_id.to_string(),
         ":created_at":new_user.created_at,
         ":last_login":new_user.last_login,
          },
    )
    .context("Couldnt insert user into database, transaction failed while registering a user")?;
    tx.commit().context(
        "Couldnt insert user into database, transaction failed while registering a user",
    )?;
    crate::config::change_active_user(new_user.user_id, &paths)?; //narazie tutaj, moze po logowaniu damy to wszystko do jednej funkcji
    log_helper(
        "registering",
        "success",
        Some(Format::Display(&new_user.username)),
        "User successfully registered",
    );
    Ok(())
    //TODO add function after login/register which changes paths current user etc.
}

///this function generates encrypted keys
fn generate_enctypted_keys(
    //reuse on password change
    password: &str,
    mut notes_key: chacha20poly1305::Key,
) -> Result<(String, String, Vec<u8>, Vec<u8>), crate::errors::Error> {
    let salt: SaltString = SaltString::generate(&mut OsRng); //generating salt for password
    let argon2 = Argon2::default(); //creating argon2 instance
    let mut kek_bytes = [0u8; KEY_ENCRYPTED_KEY_LENGTH]; // Can be any desired size
    argon2
        .hash_password_into(
            password.as_bytes(),
            salt.as_str().as_bytes(),
            &mut kek_bytes,
        )
        .context("couldnt hash password into key encrypted key in registering ")?; //creating derived key from password, which will be used to encrypt notes_key 

    // Hash password to PHC string ($argon2id$v=19$...)
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .context("Couldnt hash a password in registering a user")?
        .to_string(); //hasing password 
    log_helper(
        "password encryption",
        "success",
        None::<Format<String>>,
        "password encrypted successfully",
    );

    //generate random key
    let kek = ChaCha20Poly1305::new(&kek_bytes.into());
    let nonce_for_key_wrap = ChaCha20Poly1305::generate_nonce(&mut OsRng);
    let encrypted_notes_key = kek
        .encrypt(&nonce_for_key_wrap, notes_key.as_ref())
        .context("Couldnt encrypt key encrypted key while registering a user")?;

    kek_bytes.zeroize();
    notes_key.as_mut_slice().zeroize();
    Ok((
        password_hash,
        salt.to_string(),
        encrypted_notes_key,
        nonce_for_key_wrap.to_vec(),
    ))
}

///this function validates password on backend side
fn password_validation(password: &str) -> Result<(), crate::errors::Error> {
    if password.len() < MINIMAL_PASSWORD_LENGTH
        || !password.chars().any(|c| c.is_ascii_punctuation())
        || !password.chars().any(|c| c.is_ascii_uppercase())
        || !password.chars().any(|c| c.is_ascii_lowercase())
    {
        tracing::error!(
            task = "password validation",
            status = "error",
            "password didnt pass validation"
        );

        return Err(crate::errors::Error::PasswordValidation);
    }
    log_helper(
        "password validation",
        "success",
        None::<Format<String>>,
        "Password validated successfully",
    );

    Ok(())
}
///this function validate username on backend side
fn validate_username(username: &str, conn: &Connection) -> Result<(), crate::errors::Error> {
    let exists = conn
        .query_row(
            "SELECT username FROM users_data WHERE username = :name",
            params![username],
            |_row| Ok(()),
        )
        .optional()
        .context("couldnt check if username exist in username validation SQL Error")?
        .is_some();
    if exists {
        tracing::error!(task="username validation", status="error", %username, "username didnt pass validation");

        return Err(crate::errors::Error::UsernameExistsError);
    } else {
        log_helper(
            "username validation",
            "success",
            Some(Format::Display(&username)),
            "username validated successfully",
        );

        return Ok(());
    }
}

#[test]
fn test_password_validation() {
    let password = "aA#$#$#@";
    password_validation(password).unwrap();
}

#[test]
fn register_test() {
    let paths = ProgramFiles::init_in_base().unwrap();
    let home_path = std::env::temp_dir();

    let mut conn =
        crate::services::auth::database_creation::connect_or_create_local_login_db(&home_path)
            .unwrap();
    register_user_offline(
        "tescik".to_string(),
        zeroize::Zeroizing::from("ToJestTest!".to_string()),
        &paths,
        &mut conn,
    )
    .unwrap();
}
