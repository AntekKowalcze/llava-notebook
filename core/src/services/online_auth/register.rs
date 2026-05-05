use crate::constants::SERVER_ADDRESS;
use crate::services::online_auth::models::online_account::{
    AccessToken, ArgonParams, RegisterDevicePayload, RegisterRequest, RegisterUserPayload, Tokens,
};
use anyhow::Context;
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use chacha20poly1305::{AeadCore, ChaCha20Poly1305, Key, KeyInit, aead::Aead};
use regex::Regex;
use rusqlite::{Connection, named_params};
use zeroize::Zeroize;
pub async fn register(
    client: &reqwest::Client,
    email: String,
    password: zeroize::Zeroizing<String>,
    password_repeated: zeroize::Zeroizing<String>,
    device_id: uuid::Uuid,
    notes_key: &chacha20poly1305::Key,
) -> Result<(AccessToken, String), crate::errors::Error> {
    verify_email(&email)?;
    crate::services::local_auth::register::password_validation(&password, &password_repeated)?;

    let argon2 = Argon2::default();
    let password_salt = SaltString::generate(&mut OsRng);
    let password_hashed = argon2
        .hash_password(password.as_bytes(), &password_salt)
        .context("failed to generate password hash")?
        .to_string();

    let kek_salt = SaltString::generate(&mut OsRng);
    let mut kek_bytes = [0u8; 32];
    argon2
        .hash_password_into(
            password.as_bytes(),
            kek_salt.as_str().as_bytes(),
            &mut kek_bytes,
        )
        .context("failed to derive online KEK")?;

    let kek = ChaCha20Poly1305::new(Key::from_slice(&kek_bytes));
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
    let wrapped_notes_key = kek
        .encrypt(&nonce, notes_key.as_slice())
        .map_err(|_| anyhow::anyhow!("failed to encrypt notes key with online KEK"))
        .context("failed to wrap notes_key for online registration")?;

    kek_bytes.zeroize();
    let params = argon2::Params::default();
    let argon_params = ArgonParams {
        m_cost: params.m_cost(),
        t_cost: params.t_cost(),
        p_cost: params.p_cost(),
    };
    let user = RegisterUserPayload {
        email,
        password_hash: password_hashed,
        kek_salt: kek_salt.to_string(),
        master_key_enc: wrapped_notes_key,
        master_key_nonce: nonce.to_vec(),
        argon2_params: argon_params,
    };
    let device = RegisterDevicePayload {
        device_id: device_id,
        device_name: crate::utils::get_host_name().context("failed to get hostname")?,
    };
    let request = RegisterRequest {
        user: user,
        device: device,
    };
    let res = client
        .post(format!("{}auth/register", SERVER_ADDRESS))
        .json(&request)
        .send()
        .await
        .map_err(|err| crate::errors::Error::from_reqwest_error(&err))?;
    if !res.status().is_success() {
        let status = res.status().as_u16();
        if status == 409 {
            return Err(crate::errors::Error::EmailAlreadyUsed);
        }
        let err = res.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("server error: {}", err).into());
    }

    let tokens = res
        .json::<Tokens>()
        .await
        .context("failed to parse response")?;
    let entry = keyring::Entry::new(
        "llava_desktop",
        &format!("refresh_token_id:{}", &tokens.user_id),
    )
    .context("failed to create keyring entry")?;
    entry
        .set_password(&tokens.refresh_token.0)
        .context("failed to store refresh token in keyring")?;
    Ok((tokens.access_token, tokens.user_id))
}

fn verify_email(email: &str) -> Result<(), crate::errors::Error> {
    let re = Regex::new(
        r"[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*@(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?",
    ).unwrap(); //regex from RFC2822 so it can not panic 
    if !re.is_match(email) {
        Err(crate::errors::Error::WrongEmail)
    } else {
        Ok(())
    }
}

pub fn change_email_in_database(
    email: &str,
    users_db: &Connection,
    id: &str,
    user_id: String,
) -> Result<(), crate::errors::Error> {
    users_db
        .execute(
            "UPDATE users_data SET online_account_email = :mail, online_account_id = :online_id, is_online_linked = 1 WHERE user_id = :id;",
            named_params! {
                ":mail": email,
                ":online_id": id,
                ":id": user_id
            },
        )
        .inspect_err(|err| println!("{:?}", err))
        .context("failed to update mail in users_db locally")?;
    Ok(())
}
