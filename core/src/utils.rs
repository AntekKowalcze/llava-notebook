//! modules for useful tools

use std::time::Duration;

use anyhow::Context;
use rusqlite::{Connection, OptionalExtension, named_params};

#[allow(dead_code)]
pub fn getting_user_input(buffer: &mut String) {
    println!("Podaj treść");
    std::io::stdin()
        .read_line(buffer)
        .expect("getting input failed");
}
///gets time in UTC timestamp i64
pub fn get_time() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
pub enum Format<'a, T> {
    Display(&'a T),
    Debug(&'a T),
}

pub fn log_helper<T>(task: &str, status: &str, additional_info: Option<Format<T>>, context: &str)
where
    T: std::fmt::Display + std::fmt::Debug,
{
    match additional_info {
        Some(Format::Display(v)) => tracing::info!(task = task, status = status, %v, context),
        Some(Format::Debug(v)) => tracing::info!(task = task, status = status, ?v, context),
        None => tracing::info!(task = task, status = status, context),
    }
}

pub fn get_user_uuid(
    users_db: &Connection,
    username: &str,
) -> Result<uuid::Uuid, crate::errors::Error> {
    let uuid_str_opt = users_db
        .query_row(
            "SELECT user_id FROM users_data WHERE username = :n",
            named_params! {
                ":n": username,
            },
            |row| row.get(0),
        )
        .optional()
        .context("rusqlite error")?;

    let uuid_str: String = uuid_str_opt.ok_or(crate::errors::Error::UserNotExists)?;
    println!("{:?} uuid String", uuid_str);
    let uuid = uuid::Uuid::parse_str(&uuid_str).context("failed to parse uuid")?;

    Ok(uuid)
}

pub fn get_username_from_uuid(
    users_db: &Connection,
    user_uuid: String,
) -> Result<String, crate::errors::Error> {
    let username: String = users_db
        .query_row(
            "SELECT username FROM users_data WHERE user_id = :id;",
            named_params! {
                ":id": user_uuid,
            },
            |row| row.get::<_, String>(0),
        )
        .context("Failed to get user uuid from database")?;
    Ok(username)
}

pub fn get_host_name() -> Result<String, crate::errors::Error> {
    let hostname = hostname::get()
        .context("failed to get hostname")?
        .to_string_lossy()
        .to_string();
    Ok(hostname)
}

pub fn get_online_id(
    user_id: &uuid::Uuid,
    users_db: &Connection,
) -> Result<String, crate::errors::Error> {
    let online_id = users_db
        .query_row(
            "SELECT online_account_id FROM users_data WHERE user_id = :id;",
            named_params!(":id": user_id.to_string()),
            |row| row.get(0),
        )
        .optional()
        .context("failed to get online user_id")?;

    online_id.ok_or(crate::errors::Error::NotLoggedIn)
}

pub fn get_email_from_online_id(
    online_id: &str,
    users_db: &Connection,
) -> Result<String, crate::errors::Error> {
    let email = users_db
        .query_row(
            "SELECT online_account_email FROM users_data WHERE online_account_id = :id;",
            named_params!(":id": online_id),
            |row| row.get(0),
        )
        .context("failed to get online user_id")?;
    Ok(email)
}
pub fn is_online_linked(
    user_id: &uuid::Uuid,
    users_db: &Connection,
) -> Result<bool, crate::errors::Error> {
    let linked: i64 = users_db
        .query_row(
            "SELECT is_online_linked FROM users_data WHERE user_id = :id;",
            named_params!(":id": user_id.to_string()),
            |row| row.get(0),
        )
        .context("failed to get is_online_linked")?;
    Ok(linked == 1)
}

pub fn change_account_link_status(
    users_data: &Connection,
    user_id: &uuid::Uuid,
) -> Result<(), crate::errors::Error> {
    users_data
        .execute(
            "UPDATE users_data SET is_online_linked = 1 WHERE user_id = :id",
            named_params! {":id": user_id.to_string()},
        )
        .context("failed to update linked in users_data")?;
    Ok(())
}
pub async fn is_device_connected_to_server() -> bool {
    tokio::time::timeout(
        Duration::from_secs(3),
        tokio::net::TcpStream::connect(crate::constants::SERVER_ADDRESS_TO_PING),
    )
    .await
    .map(|res| res.is_ok())
    .unwrap_or(false)
}

pub async fn is_device_online() -> bool {
    tokio::time::timeout(
        Duration::from_secs(3),
        tokio::net::TcpStream::connect("142.250.120.113:80"),
    )
    .await
    .map(|res| res.is_ok())
    .unwrap_or(false)
}

pub async fn check_connection() -> Result<(bool, bool), crate::errors::Error> {
    Ok(tokio::join!(
        is_device_connected_to_server(),
        is_device_online()
    ))
}
