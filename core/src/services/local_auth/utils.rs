use anyhow::Context;

pub fn check_if_first_start(users_db: &rusqlite::Connection) -> Result<bool, crate::errors::Error> {
    let exists: i64 = users_db
        .query_row("SELECT EXISTS(SELECT 1 FROM users_data);", [], |row| {
            row.get(0)
        })
        .context("database error while checking if its first run off app")?;
    let is_first_login = exists == 0;
    Ok(is_first_login)
}
