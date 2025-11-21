//! Module responsible for registering user
//! in this modules important data is encrypted, and keys for notes encryption are also created

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use chacha20poly1305::{
    ChaCha20Poly1305,
    aead::{Aead, AeadCore, KeyInit},
};
use rusqlite::{Connection, OptionalExtension, params};
use zeroize::Zeroize;

use crate::config::ProgramFiles;
///function responsible for registering user offilne and adding it encrypted to local db
fn register_user_offline(
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
    let mut new_user = crate::services::auth::auth_data_models::local_user::LocalUser {
        user_id: uuid::Uuid::new_v4(),
        username: username,
        password_hash: password_hash,
        password_salt: salt,
        notes_key: encrypted_notes_key,
        nonce_notes_key: nonce_for_key_wrap,
        is_online_linked: false,
        online_account_email: None,
        device_id: crate::config::get_device_id(paths)?,
        created_at: crate::utils::get_time(),
        last_login: crate::utils::get_time(),
    };

    let tx = conn.transaction()?;

    tx.execute(
        r#"INSERT INTO users_data (
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
       :last_login)"#,
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
    )?;
    tx.commit()?;
    crate::config::change_active_user(new_user.user_id, &paths)?; //narazie tutaj, moze po logowaniu damy to wszystko do jednej funkcji

    crate::services::logger::log_success("Successfully added a user to a database");

    Ok(())
}

fn register_user_online() {}
///this function generates encrypted keys
fn generate_enctypted_keys(
    //reuse on password change
    password: &str,
    mut notes_key: chacha20poly1305::Key,
) -> Result<(String, String, Vec<u8>, Vec<u8>), crate::errors::Error> {
    let salt: SaltString = SaltString::generate(&mut OsRng); //generating salt for password
    let argon2 = Argon2::default(); //creating argon2 instance
    let mut kek_bytes = [0u8; 32]; // Can be any desired size
    argon2.hash_password_into(
        password.as_bytes(),
        salt.as_str().as_bytes(),
        &mut kek_bytes,
    )?; //creating derived key from password, which will be used to encrypt notes_key 

    // Hash password to PHC string ($argon2id$v=19$...)
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)?
        .to_string(); //hasing password 
    //TODO sprawdzic typy w bazach , zrobić spójne

    //generate random key
    let kek = ChaCha20Poly1305::new(&kek_bytes.into());
    let nonce_for_key_wrap = ChaCha20Poly1305::generate_nonce(&mut OsRng);
    let encrypted_notes_key = kek.encrypt(&nonce_for_key_wrap, notes_key.as_ref())?;

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
    if password.len() < 8
        || !password.chars().any(|c| c.is_ascii_punctuation())
        || !password.chars().any(|c| c.is_ascii_uppercase())
        || !password.chars().any(|c| c.is_ascii_lowercase())
    {
        crate::services::logger::log_error(
            "password not validated",
            crate::errors::Error::PasswordValidation,
        );

        return Err(crate::errors::Error::PasswordValidation);
    }
    crate::services::logger::log_success("Password validated successfully");

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
        .optional()?
        .is_some();
    if exists {
        crate::services::logger::log_error(
            "username validation failed",
            crate::errors::Error::UsernameExistsError,
        );
        return Err(crate::errors::Error::UsernameExistsError);
    } else {
        crate::services::logger::log_success("passed username validation");
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
    let paths = ProgramFiles::init().unwrap();
    let mut conn =
        crate::services::auth::database_creation::connect_or_create_local_login_db(&paths).unwrap();
    register_user_offline(
        "seventh".to_string(),
        zeroize::Zeroizing::from("ToJestTest!".to_string()),
        &paths,
        &mut conn,
    )
    .unwrap();
}

//TODO kiedyś dodać rollback przy usuwaniu:
//delete_note wykonuje rename file i update DB; ale jeśli fs::rename się uda a DB update fails, masz niespójność — zapakuj te operacje w transakcję logiczną (najpierw db update, potem fs op). Lepsza sekwencja: przygotuj plik tmp, update db to PendingDeleted, potem rename, potem final commit. Przy błędach — rollback.
