use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use chacha20poly1305::{
    ChaCha20Poly1305,
    aead::{Aead, AeadCore, KeyInit},
};
use rusqlite::Connection;

use crate::config::ProgramFiles;

fn register_user_offline(
    username: String,
    password: String,
    paths: &crate::config::ProgramFiles,
    mut conn: Connection,
) -> Result<(), crate::errors::Error> {
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
        device_id: crate::config::get_device_id(paths)?,
        created_at: crate::utils::get_time(),
        last_login: crate::utils::get_time(),
    };
    let tx = conn.transaction()?;

    tx.execute(
        r#"INSERT INTO users_data (
        user_id, username,
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
        ":user_id": &new_user.user_id,
         ":username":new_user.username ,
         ":password_hash":new_user.password_hash,
         ":password_salt":new_user.password_salt,
         ":notes_key":new_user.notes_key,
         ":nonce_notes_key":new_user.nonce_notes_key,
         ":is_online_linked": new_user.is_online_linked,
         ":online_account_email":new_user.online_account_email,
         ":device_id": new_user.device_id,
         ":created_at":new_user.created_at,
         ":last_login":new_user.last_login,
          },
    )?;
    tx.commit()?;
    crate::services::logger::log_success("Successfully added a user to a database");
    crate::config::change_active_user(new_user.user_id, &paths)?; //narazie tutaj, moze po logowaniu damy to wszystko do jednej funkcji
    Ok(())
}

fn register_user_online() {}

fn generate_enctypted_keys(
    //reuse on password change
    password: String,
    notes_key: chacha20poly1305::Key,
) -> Result<(String, String, Vec<u8>, Vec<u8>), crate::errors::Error> {
    let salt = SaltString::generate(&mut OsRng); //generating salt for password
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

    //generate random key
    let kek = ChaCha20Poly1305::new(&kek_bytes.into());
    let nonce_for_key_wrap = ChaCha20Poly1305::generate_nonce(&mut OsRng);
    let encrypted_notes_key = kek.encrypt(&nonce_for_key_wrap, notes_key.as_ref())?;

    Ok((
        password_hash,
        salt.to_string(),
        encrypted_notes_key,
        nonce_for_key_wrap.to_vec(),
    ))
}

#[test]
fn register_test() {
    let paths = ProgramFiles::init().unwrap();
    let mut conn =
        crate::services::auth::database_creation::connect_or_create_local_login_db(&paths).unwrap();
    register_user_offline("second_user".to_string(), "test".to_string(), &paths, conn).unwrap();
}
