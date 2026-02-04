//! modules for useful tools

use anyhow::Context;
use rusqlite::{Connection, named_params};
pub fn getting_user_input(buffer: &mut String) {
    println!("Podaj treść");
    std::io::stdin()
        .read_line(buffer)
        .expect("getting input failed");
    //im using expect cuz it wont be used in application only for testing
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
    conn: &Connection,
    username: &str,
) -> Result<uuid::Uuid, crate::errors::Error> {
    let mut stmt = conn
        .prepare("SELECT username, user_id FROM users_data")
        .unwrap();

    let uuid_str: String = conn
        .query_row(
            "SELECT user_id FROM users_data WHERE username = :n",
            named_params! {
                ":n": username,
            },
            |row| row.get(0),
        )
        .context("Failed to get uuid from database")?;

    // Try to parse the UUID
    let uuid = uuid::Uuid::parse_str(&uuid_str).context("failed to parse uuid")?;

    Ok(uuid)
}
