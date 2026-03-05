//! # Local user register module
//! **Purpose**: This module is responsible for all actions taken while creating local account.   
//! It handles saving account data to database, create and encrypt keys and hash passwords, it also handles keyring logged in account saving
//!  ## Exported functions 
//! * [register_user_offline] - Full registration flow from validating password and username to encryption to changing state of the app
//! * [recovery_code_handling] - Function responsible for generating and handling recovery keys database
//! * [after_validation] - Function responsible for changing physical state of the app (paths)
//! * [change_password] - Function responsible for changing password, reencrypting notes key, rehashing etc.
//! ## Key design decisions
//! Password is entry for all kdf, notes key is random, encrypted and wrapped with KEK. All important data is zeroized. 
//! Recovery key behaves as password, after use user is logged in, and its needed for password change, every key reencrypts notes key
//! ## Dependencies
//! - `argon2` — Password hashing and KEK derivation
//! - `chacha20poly1305` — Authenticated encryption of key material
//! - `rusqlite` — SQLite via `users_data` and `recovery_keys` tables
//! - `zeroize` — Secure memory wiping of sensitive values
use crate::constants::*;
use crate::utils::{Format, log_helper};
use anyhow::Context;

use argon2::password_hash::rand_core::RngCore;
use argon2::{
    Argon2, PasswordHash, PasswordVerifier,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};

use chacha20poly1305::{
    ChaCha20Poly1305,
    aead::{Aead, AeadCore, KeyInit},
};
use rusqlite::{Connection, OptionalExtension, named_params, params};

use zeroize::Zeroize;

use crate::config::{ProgramFiles, change_active_user};
pub fn register_user_offline(
    username: String,
    password: zeroize::Zeroizing<String>,
    password_repeated: zeroize::Zeroizing<String>,
    paths: &crate::config::ProgramFiles,
    users_db: &mut Connection,
) -> Result<(uuid::Uuid, ProgramFiles, Connection, Vec<String>), crate::errors::Error> {
    let username = username.trim().to_string();
    validate_username(&username, &users_db)?;
    let password = password.as_str().trim();
    let password_repeated = password_repeated.as_str().trim();
    password_validation(password, password_repeated)?;
    let notes_key: chacha20poly1305::Key = ChaCha20Poly1305::generate_key(&mut OsRng); //Creating of chacha poly key for encrypting notes
    let (password_hash, salt, encrypted_notes_key, nonce_for_key_wrap) =
        generate_enctypted_keys(password, notes_key)?;
    let new_user = crate::services::auth::auth_data_models::local_user::LocalUser {
        user_id: uuid::Uuid::new_v4(),
        username: username,
        password_hash: password_hash, //SALT ALREADY IN PHC STRING
        notes_key: encrypted_notes_key,
        nonce_notes_key: nonce_for_key_wrap,
        kek_salt: salt,
        is_online_linked: false,
        online_account_email: None,
        device_id: crate::config::get_device_id(&users_db, &paths.device_id_path)?,
        created_at: crate::utils::get_time(),
        last_login: crate::utils::get_time(),
        password_errors: 0,
        ending_block_timestamp: 0,
    };

    let tx = users_db.transaction().context(
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
         ":kek_salt": new_user.kek_salt,
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
    crate::config::change_active_user(&new_user.user_id, &paths)?;
    log_helper(
        "registering",
        "success",
        Some(Format::Display(&new_user.username)),
        "User successfully registered",
    );

    let codes = recovery_code_handling(&new_user.username, users_db, password)?; //get recovery codes as strings

    let paths = after_validation(&new_user.user_id, paths)?;
    crate::services::auth::logging::session_operations(&users_db, new_user.user_id)?;
    let users_db = crate::services::storage::db_creation::get_connection(&paths)?; //get connection for note database
    Ok((new_user.user_id, paths, users_db, codes))
}

fn generate_enctypted_keys(
    //reuse on password change
    password: &str,
    mut notes_key: chacha20poly1305::Key,
) -> Result<(String, String, Vec<u8>, Vec<u8>), crate::errors::Error> {
    let salt: SaltString = SaltString::generate(&mut OsRng); //generating salt for password
    let argon2 = Argon2::default(); //creating argon2 instance
    let mut kek_bytes = [0u8; KEY_ENCRYPTED_KEY_LENGTH]; //creating empty array to store key to chachapoly instance

    argon2
        .hash_password_into(
            password.as_bytes(),
            salt.as_str().as_bytes(),
            &mut kek_bytes,
        )
        .context("couldnt hash password into key encrypted key in registering ")?; //fill key for chachapoly with bytes

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
    let kek = ChaCha20Poly1305::new(&kek_bytes.into()); //create chachapoly instance with kek_bytes as key to decrypt
    let nonce_for_key_wrap = ChaCha20Poly1305::generate_nonce(&mut OsRng); //get nonce (value which makes every chachapoly different)
    let encrypted_notes_key = kek
        .encrypt(&nonce_for_key_wrap, notes_key.as_ref())
        .inspect_err(|e| tracing::error!(
        task = "password encryption",
        status = "error",
        error = %e,
        "failed to encrypt notes key with KEK"
    ))
        .context("Couldnt encrypt key encrypted key while registering a user")?; //encrypt notes key with nonce and kek (kek is key for chachapoly we are encrypting with)

    kek_bytes.zeroize();
    notes_key.as_mut_slice().zeroize();
    Ok((
        password_hash,
        salt.to_string(),
        encrypted_notes_key,
        nonce_for_key_wrap.to_vec(),
    ))
}

//to generate more keys just run this funciton
pub fn recovery_code_handling(
    username: &str,
    users_db: &rusqlite::Connection,
    password: &str,
) -> Result<Vec<String>, crate::errors::Error> {
    let user_uuid = crate::utils::get_user_uuid(users_db, &username)?;
    let mut user_visible_codes: Vec<String> = Vec::new();
    let arg = Argon2::default();
    let (notes_key, nonce, kek_salt) = users_db
        .query_row(
            "SELECT notes_key, nonce_notes_key, kek_salt FROM users_data WHERE user_id = ?", //Z recovery key a nie z usera
            [&user_uuid.to_string()],
            |row| {
                let notes_key: Vec<u8> = row.get(0)?;
                let nonce: Vec<u8> = row.get(1)?;
                let kek: String = row.get(2)?;
                Ok((notes_key, nonce, kek))
            },
        )
        .context("Failed to get user encryption data from database")?; //get nonce, encrypted notes_key, and salt used to get kek_bytes
    let mut kek_bytes = [0u8; 32]; //create kek bytes empty array
    Argon2::default()
        .hash_password_into(password.as_bytes(), kek_salt.as_bytes(), &mut kek_bytes)
         .inspect_err(|e| tracing::error!(
        task = "recovery code handling",
        status = "error",
        error = %e,
        "failed to derive KEK from password"
    ))
        .context("Failed to derive kek")?; //recreate kek_bytes from password and salt from db

    let kek = ChaCha20Poly1305::new(&kek_bytes.into()); //create chachapoly instance with kek_bytes as key
    let nonce_arr = chacha20poly1305::Nonce::from_slice(&nonce); //recreate nonce from slice
    let mut decrypted_notes_key = kek
        .decrypt(nonce_arr, notes_key.as_ref())
         .inspect_err(|e| tracing::error!(
        task = "recovery code handling",
        status = "error",
        error = %e,
        "failed to decrypt notes key"
    ))
        .map_err(|_| anyhow::anyhow!("Failed to decrypt notes_key"))
        .context("notes_key decryption failed")?; //get notes key for next steps
    kek_bytes.zeroize();
        crate::utils::log_helper("recovery code handling", "success", None::<Format<String>>, "successfully decrypted notes key");
    for _ in 0..NUMBER_OF_KEYS {
        let (mut key, user_readable, wrapped_key, nonce, kdf_salt) =
            generate_recovery_code(&arg, &decrypted_notes_key)?;
        user_visible_codes.push(user_readable);
        users_db.execute(
            "INSERT INTO recovery_keys (user_id, code_hash, used_at, wrapped_notes_key, wrapped_notes_key_nonce, recovery_kdf_salt)  VALUES (:id, :hash, NULL, :wnk, :wnkn, :rks)",
            named_params! {
                ":id": user_uuid.to_string(),
                ":hash": key,
                ":wnk": wrapped_key, //notes key encrypted with recovery code (shorthand)
                ":wnkn": nonce, 
                ":rks": kdf_salt //salt for getting kek bytes
            },
        ).inspect_err(|_| log_helper("handling recovery codes", "error", None::<Format<String>>, "Failed generating recovery keys")).context("failed to insert key into db")?;
        key.zeroize();
    }
    decrypted_notes_key.zeroize();
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
    notes_key: &[u8], //notes_key decrypted
) -> Result<(String, String, Vec<u8>, Vec<u8>, String), crate::errors::Error> {
    let salt: SaltString = SaltString::generate(&mut OsRng); //generating salt for code encryption

    let mut code_bytes = [0u8; RECOVERY_CODE_LENGTH];

    OsRng.fill_bytes(&mut code_bytes); //fill code bytes

    let code_hashed = argon_instance
        .hash_password(&code_bytes, &salt)
        .context("failed to hash key")?
        .to_string();
    //hash code
    let kdf_salt = SaltString::generate(&mut OsRng); // separate salt for KDF (to get kek_bytes for encrypting notes_key)
    let mut recovery_kek_bytes = [0u8; KEY_ENCRYPTED_KEY_LENGTH];
    argon_instance
        .hash_password_into(
            &code_bytes,
            kdf_salt.as_str().as_bytes(),
            &mut recovery_kek_bytes,
        )
         .inspect_err(|e| tracing::error!(
        task = "generate recovery code",
        status = "error",
        error = %e,
        "failed to hash recovery code"
    ))
        .context("failed to derive recovery KEK")?; //creating key for chachapoly instance

    let recovery_kek = ChaCha20Poly1305::new(&recovery_kek_bytes.into()); //create chachapoly instance with recovery kek bytes as key
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
    let wrapped_key = recovery_kek
        .encrypt(&nonce, notes_key)
         .inspect_err(|e| tracing::error!(
        task = "generate recovery code",
        status = "error",
        error = %e,
        "failed to wrap notes key with recovery KEK"
    ))
        .context("failed to wrap notes_key")?; //notes_key encrypted with KEK derived from raw code bytes via Argon2

    recovery_kek_bytes.zeroize();
    let readable_code = base32::encode(base32::Alphabet::Crockford, &code_bytes); //make bytes to letters and numbers
    code_bytes.zeroize();
    log_helper(
    "generate recovery code",
    "success",
    None::<Format<String>>,
    "recovery code generated successfully",
);
    Ok((
        code_hashed,
        readable_code,
        wrapped_key,
        nonce.to_vec(),
        kdf_salt.to_string(),
    ))
}
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
    
    Ok(())
}
fn validate_username(username: &str, users_db: &Connection) -> Result<(), crate::errors::Error> {
    let exists = users_db
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
    change_active_user(&user_uuid, paths).inspect_err(|e| tracing::error!(
        task = "after validation",
        status = "error",
        error = ?e,
        "failed to change active user"
    ))?;
    let paths = crate::config::get_paths(paths.app_home.clone(), user_uuid).inspect_err(|e| tracing::error!(
        task = "after validation",
        status = "error",
        error = ?e,
        "failed to get paths for new user"
    ))?;

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

pub fn change_password(
    users_db: &Connection,
    username: String,
    password: String,
    password_repeated: String,
    mut code: String,
) -> Result<(), crate::errors::Error> {
    let user_uuid = crate::utils::get_user_uuid(users_db, &username)?;
    let password = password.as_str().trim();
    let password_repeated = password_repeated.as_str().trim();
    password_validation(&password, &password_repeated)?;

    let mut found = 0;
    let mut stmt = users_db
        .prepare("SELECT code_hash FROM recovery_keys WHERE user_id = :id AND used_at IS NOT NULL")
        .context("failed to prepare statement")?;
    let mut handle = stmt
        .query(named_params! {
            ":id":user_uuid.to_string()
        })
        .context("failed to get handle to codes")?;

    if let Some(mut decoded) = base32::decode(base32::Alphabet::Crockford, &code) {
        let argon2 = Argon2::default();
        while let Some(row) = handle.next().context("failed to get next row")? {
            let  hash: String = row.get(0).context("failed to get hash")?;
            let phc: PasswordHash<'_> =
                argon2::PasswordHash::new(&hash).context("failed to parse hash from db to phc")?;
            if argon2.verify_password(&decoded, &phc).is_ok() {
                found += 1;
                users_db
                    .execute(
                        "UPDATE recovery_keys SET used_at = :time WHERE code_hash = :h",
                        named_params! {
                            ":time": crate::utils::get_time(),
                            ":h": hash
                        },
                    )
                    .context("Failed to mark code as used")?;
            }

           
            code.zeroize();
            if found > 0 {
                 let (wrapped_key, nonce, kdf_salt) = users_db.query_row("SELECT wrapped_notes_key, wrapped_notes_key_nonce, recovery_kdf_salt FROM recovery_keys WHERE code_hash = :hash", named_params! {
            ":hash": hash
        }, |row|{
    
       let wrapped_key: Vec<u8> = row.get(0)?;
                let nonce: Vec<u8> = row.get(1)?;
                let kdf_salt: String = row.get(2)?;
                Ok((wrapped_key, nonce, kdf_salt))
        }
    
    ).context("failed to obtain crypto meta for used code")?;
    //For decryption
let mut kek_bytes = [0u8; 32];
 Argon2::default()
        .hash_password_into(&decoded, kdf_salt.as_bytes(), &mut kek_bytes)
         .inspect_err(|e| tracing::error!(
        task = "change password",
        status = "error",
        error = %e,
        "failed to derive KEK from recovery code"
    ))
        .context("Failed to derive kek")?;
    
     decoded.zeroize();
let nonce = chacha20poly1305::Nonce::from_slice(&nonce);
let kek = ChaCha20Poly1305::new(&kek_bytes.into()); //create chachapoly instance with kek_bytes as key
 let mut decrypted_notes_key = kek
        .decrypt(nonce, wrapped_key.as_ref())
         .inspect_err(|e| tracing::error!(
        task = "change password",
        status = "error",
        error = %e,
        "failed to decrypt notes key"
    ))
        .map_err(|_| anyhow::anyhow!("Failed to decrypt notes_key"))
        .context("notes_key decryption failed while changing password")?; //get notes key for next steps
    kek_bytes.zeroize();
//NEW VALUES
let mut new_kek_bytes =[0u8; 32];
let new_kdf_salt = SaltString::generate(&mut OsRng);
argon2.hash_password_into(password.as_bytes(), new_kdf_salt.as_str().as_bytes(), &mut new_kek_bytes).context("failed to fill kek bytes with new password")?;
let password_salt = SaltString::generate(&mut OsRng);
let hashed_password = argon2.hash_password(password.as_bytes(), &password_salt).context("failed to hash password")?;
let new_kek = ChaCha20Poly1305::new(&new_kek_bytes.into());
let new_nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
let wrapped_key = new_kek
            .encrypt(&new_nonce, decrypted_notes_key.as_ref())
            .context("failed to wrap notes_key")?;
    new_kek_bytes.zeroize(); 
let mut stmt = users_db.prepare("UPDATE users_data SET password_hash = :ph, notes_key = :nk, nonce_notes_key = :nnk, kek_salt = :kdf_salt, password_errors = 0 WHERE user_id = :uuid").context("failed to prepare update after changing password")?;
stmt.execute(named_params! {
    ":ph": &hashed_password.to_string(),
    ":nk": wrapped_key,
    ":nnk": new_nonce.to_vec(),
    ":kdf_salt": new_kdf_salt.to_string(),
    ":uuid": user_uuid.to_string()
}).context("failed updating users data table after changing password")?;
log_helper(
    "change password",
    "success",
    None::<Format<String>>,
    "password changed successfully, user data updated",
);
decrypted_notes_key.zeroize();
break;
            }
        }

        //getting code bytes
        //
       
        // let argon2 = Argon2::default();
    }

    

    Ok(())
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

    let mut users_db =
        crate::services::auth::database_creation::connect_or_create_local_login_db(&home_path)
            .unwrap();
    register_user_offline(
        "tescik".to_string(),
        zeroize::Zeroizing::from("ToJestTest!".to_string()),
        zeroize::Zeroizing::from("ToJestTest!".to_string()),
        &paths,
        &mut users_db,
    )
    .unwrap();
}

#[test]
fn generate_codes() {
    let paths = ProgramFiles::init_in_base().unwrap();
    let mut users_db = rusqlite::Connection::open_in_memory().unwrap();

    // Initialize schema manually
    users_db.execute_batch(LOCAL_LOGIN_DB_SCHEMA).unwrap();

    register_user_offline(
        "tescik".to_string(),
        zeroize::Zeroizing::from("ToJestTest!".to_string()),
        zeroize::Zeroizing::from("ToJestTest!".to_string()),
        &paths,
        &mut users_db,
    )
    .unwrap();

    let u_uuid: String = users_db
        .query_row(
            "SELECT user_id FROM users_data WHERE username = :name;",
            named_params! {
                ":name": "tescik".to_string(),
            },
            |row| row.get(0),
        )
        .unwrap();

    let _u_uuid = uuid::Uuid::parse_str(&u_uuid).unwrap();
    let keys = recovery_code_handling("tescik", &users_db, "ToJestTest!").unwrap();
    println!("keys: {:#?}", keys);
}
