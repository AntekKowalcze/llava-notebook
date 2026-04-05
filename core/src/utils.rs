//! modules for useful tools

use anyhow::Context;
use rusqlite::{Connection, OptionalExtension, named_params};
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
