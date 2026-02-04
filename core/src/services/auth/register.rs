//! Module responsible for registering user
//! in this modules important data is encrypted, and keys for notes encryption are also created
use crate::constants::*;
use crate::services::auth::utils;
use crate::utils::{Format, log_helper};
use anyhow::Context;
use argon2::password_hash::rand_core::RngCore;
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use chacha20poly1305::{
    ChaCha20Poly1305,
    aead::{Aead, AeadCore, KeyInit},
};
use core::task;
use rusqlite::{Connection, OptionalExtension, named_params, params};
use serde_json::to_string;
use std::path::Path;
use uuid::Uuid;
use zeroize::Zeroize;

use crate::config::{ProgramFiles, change_active_user};
///function responsible for registering user offilne and adding it encrypted to local db
pub fn register_user_offline(
    username: String,
    password: zeroize::Zeroizing<String>,
    password_repeated: zeroize::Zeroizing<String>,
    paths: &crate::config::ProgramFiles,
    conn: &mut Connection,
) -> Result<(uuid::Uuid, ProgramFiles, Connection), crate::errors::Error> {
    let username = username.trim().to_string();
    validate_username(&username, &conn)?;
    let password = password.as_str().trim();
    let password_repeated = password_repeated.as_str().trim();
    password_validation(password, password_repeated)?;
    let notes_key: chacha20poly1305::Key = ChaCha20Poly1305::generate_key(&mut OsRng);
    let (password_hash, encrypted_notes_key, nonce_for_key_wrap) =
        generate_enctypted_keys(password, notes_key)?;
    let new_user = crate::services::auth::auth_data_models::local_user::LocalUser {
        user_id: uuid::Uuid::new_v4(),
        username: username,
        password_hash: password_hash, //SALT ALREADY IN PHC STRING
        notes_key: encrypted_notes_key,
        nonce_notes_key: nonce_for_key_wrap,
        is_online_linked: false,
        online_account_email: None,
        device_id: crate::config::get_device_id(&conn, &paths.device_id_path)?,
        created_at: crate::utils::get_time(),
        last_login: crate::utils::get_time(),
        password_errors: 0,
        ending_block_timestamp: 0,
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
         ":notes_key":new_user.notes_key,
         ":nonce_notes_key":new_user.nonce_notes_key,
         ":is_online_linked": new_user.is_online_linked,
         ":online_account_email":new_user.online_account_email,
         ":device_id": new_user.device_id.to_string(),
         ":created_at":new_user.created_at,
         ":last_login":new_user.last_login,
         ":password_errors":new_user.password_errors,
         ":ending_block_timestamp":new_user.ending_block_timestamp,//timestamp
          },
    )
    .context("Couldnt insert user into database, transaction failed while registering a user")?;
    tx.commit().context(
        "Couldnt insert user into database, transaction failed while registering a user",
    )?;
    crate::config::change_active_user(&new_user.user_id, &paths)?; //narazie tutaj, moze po logowaniu damy to wszystko do jednej funkcji
    log_helper(
        "registering",
        "success",
        Some(Format::Display(&new_user.username)),
        "User successfully registered",
    );
    let paths = after_validation(&new_user.user_id, paths)?;
    let conn = crate::services::storage::db_creation::get_connection(&paths)?;
    Ok((new_user.user_id, paths, conn))
}

///this function generates encrypted keys
fn generate_enctypted_keys(
    //reuse on password change
    password: &str,
    mut notes_key: chacha20poly1305::Key,
) -> Result<(String, Vec<u8>, Vec<u8>), crate::errors::Error> {
    let salt: SaltString = SaltString::generate(&mut OsRng); //generating salt for password
    let argon2 = Argon2::default(); //creating argon2 instance
    let mut kek_bytes = [0u8; KEY_ENCRYPTED_KEY_LENGTH];
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

    //generate random key for notes
    let kek = ChaCha20Poly1305::new(&kek_bytes.into());
    let nonce_for_key_wrap = ChaCha20Poly1305::generate_nonce(&mut OsRng);
    let encrypted_notes_key = kek
        .encrypt(&nonce_for_key_wrap, notes_key.as_ref())
        .context("Couldnt encrypt key encrypted key while registering a user")?;

    kek_bytes.zeroize();
    notes_key.as_mut_slice().zeroize();
    Ok((
        password_hash,
        encrypted_notes_key,
        nonce_for_key_wrap.to_vec(),
    ))
}

pub fn recovery_code_handling(
    user_uuid: &uuid::Uuid,
    users_db: &rusqlite::Connection,
) -> Result<(Vec<String>), crate::errors::Error> {
    let mut user_visible_codes: Vec<String> = Vec::new();
    let arg = Argon2::default();
    for _ in 0..NUMBER_OF_KEYS {
        let (mut key, user_readable) = generate_recovery_code(&arg)?;
        user_visible_codes.push(user_readable);
        users_db.execute(
            "INSERT INTO recovery_keys (user_id, code_hash, used_at)  VALUES (:id, :hash, NULL)",
            named_params! {
                ":id": user_uuid.to_string(),
                ":hash": key,
            },
        ).inspect_err(|_| log_helper("handling recovery codes", "error", None::<Format<String>>, "Failed generating recovery keys")).context("failed to insert key into db")?;
        key.zeroize();
    }
    log_helper(
        "handling recovery codes",
        "success",
        None::<Format<String>>,
        "successfully generated and inserted recovery keys",
    );
    Ok(user_visible_codes)
}

fn generate_recovery_code(
    argon_instance: &Argon2<'_>,
) -> Result<(String, String), crate::errors::Error> {
    let salt: SaltString = SaltString::generate(&mut OsRng); //generating salt for password

    let mut key_bytes = [0u8; RECOVERY_CODE_LENGTH];
    OsRng.fill_bytes(&mut key_bytes);
    let key = argon_instance
        .hash_password(&key_bytes, &salt)
        .context("failed to hash key")?
        .to_string();
    salt.to_string().zeroize();
    let readable_code = base32::encode(base32::Alphabet::Crockford, &key_bytes);
    key_bytes.zeroize();
    Ok((key, readable_code))
}
///this function validates password on backend side
fn password_validation(
    password: &str,
    password_repeated: &str,
) -> Result<(), crate::errors::Error> {
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
    if password != password_repeated {
        return Err(crate::errors::Error::PasswordValidation);
    }
    log_helper(
        "password validation",
        "success",
        None::<Format<String>>,
        "Password validated successfully",
    );
    //TODO dodać fronted wyświetlający kody + możę ściaganie do pliku tych kodów + zapomniałeś hasła przy logowaniu i pytanie o zalogowanie
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

pub fn after_validation(
    user_uuid: &uuid::Uuid,
    paths: &crate::config::ProgramFiles,
) -> Result<ProgramFiles, crate::errors::Error> {
    change_active_user(&user_uuid, paths)?;
    let paths = crate::config::get_paths(paths.app_home.clone(), user_uuid)?;

    let tmp_nil_path = paths
        .app_home
        .join("llava/users/00000000-0000-0000-0000-000000000000");
    println!("{:?}", tmp_nil_path);
    if tmp_nil_path.exists() {
        std::fs::remove_dir_all(
            paths
                .app_home
                .join("llava/users/00000000-0000-0000-0000-000000000000"),
        )
        .context("failed while deleting nil uuid starting folder")?;
        log_helper(
            "after login",
            "success",
            None::<Format<String>>,
            "Successfully deltetd nic folder",
        );
    }
    Ok(paths)
}

#[test]
fn test_password_validation() {
    let password = "aA#$#$#@";
    let password_repeated = "aA#$#$#@";
    password_validation(password, password_repeated).unwrap();
}

#[test]
fn register_test() {
    let paths = ProgramFiles::init_in_base().unwrap();
    let home_path = std::env::temp_dir().join(LOCAL_USERS_DB);

    let mut conn =
        crate::services::auth::database_creation::connect_or_create_local_login_db(&home_path)
            .unwrap();
    register_user_offline(
        "tescik".to_string(),
        zeroize::Zeroizing::from("ToJestTest!".to_string()),
        zeroize::Zeroizing::from("ToJestTest!".to_string()),
        &paths,
        &mut conn,
    )
    .unwrap();
}

#[test]
fn generate_codes() {
    let paths = ProgramFiles::init_in_base().unwrap();
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();

    // Initialize schema manually
    conn.execute_batch(LOCAL_LOGIN_DB_SCHEMA).unwrap();

    register_user_offline(
        "tescik".to_string(),
        zeroize::Zeroizing::from("ToJestTest!".to_string()),
        zeroize::Zeroizing::from("ToJestTest!".to_string()),
        &paths,
        &mut conn,
    )
    .unwrap();

    let u_uuid: String = conn
        .query_row(
            "SELECT user_id FROM users_data WHERE username = :name;",
            named_params! {
                ":name": "tescik".to_string(),
            },
            |row| row.get(0),
        )
        .unwrap();

    let u_uuid = uuid::Uuid::parse_str(&u_uuid).unwrap();
    let keys = recovery_code_handling(&u_uuid, &conn).unwrap();
    println!("keys: {:#?}", keys);
    //    recovery_code_handling();
}
