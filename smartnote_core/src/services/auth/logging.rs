use rusqlite::OptionalExtension;

fn local_log_in(
    username: String,
    password: String,
    conn: &rusqlite::Connection,
) -> Result<(), crate::errors::Error> {
    check_if_user_exists(&username, conn)?;
    //get hash and salt from db for this username, then hash given password again and check if hashes are the same if yes log in
    //if no return error wrong password,
    //there will be function to do thing after login, change active user, get paths, load notes,

    //after logging add resetting password for local account, and then start online accounts,
    crate::services::logger::log_success("logged succesfully");
    Ok(())
}

fn check_if_user_exists(
    username: &str,
    conn: &rusqlite::Connection,
) -> Result<(), crate::errors::Error> {
    let exists = conn
        .query_row(
            "SELECT username FROM users_data WHERE username = :name",
            rusqlite::params![username],
            |_row| Ok(()),
        )
        .optional()?
        .is_some();
    if exists {
        crate::services::logger::log_success("username exists, all correct");
        return Ok(());
    } else {
        crate::services::logger::log_error(
            "username does not exists failed",
            crate::errors::Error::UserNotExists,
        );
        return Err(crate::errors::Error::UserNotExists);
    }
}
