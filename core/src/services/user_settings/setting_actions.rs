use std::path::PathBuf;

use anyhow::Context;
use rusqlite::named_params;

pub fn logfile_contents(logfile_path: &PathBuf) -> Result<String, crate::errors::Error> {
    Ok(std::fs::read_to_string(logfile_path)?)
}

pub fn change_username(
    user_id: &uuid::Uuid,
    new_username: &str,
    users_db: &rusqlite::Connection,
) -> Result<(), crate::errors::Error> {
    users_db
        .execute(
            "UPDATE users_data SET username = :username WHERE user_id = :id;",
            named_params! {":username": new_username, ":id": user_id.to_string()},
        )
        .context("Failed to update username in database")?;
    Ok(())
}
