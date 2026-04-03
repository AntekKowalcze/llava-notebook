use zeroize::Zeroizing;
pub fn register(
    username: String,
    password: String,
    password_repeated: String,
    paths: &llava_core::ProgramFiles,
    users_db: &mut rusqlite::Connection,
) -> Result<
    (
        uuid::Uuid,
        llava_core::ProgramFiles,
        rusqlite::Connection,
        Vec<String>,
    ),
    llava_core::Error,
> {
    let password_zeroized = Zeroizing::from(password);
    let password_repeated_zeroized = Zeroizing::from(password_repeated);

    llava_core::local_auth::register_user_offline(
        username,
        password_zeroized,
        password_repeated_zeroized,
        paths,
        users_db,
    )
}

pub fn login(
    username: String,
    password: String,
    paths: &llava_core::ProgramFiles,
    users_db: &mut rusqlite::Connection,
) -> Result<(uuid::Uuid, llava_core::ProgramFiles, rusqlite::Connection), llava_core::Error> {
    let password_zeroized = Zeroizing::from(password);

    llava_core::local_auth::local_log_in(username.clone(), password_zeroized, users_db, paths)
        .map_err(|e| match &e {
            llava_core::Error::WrongPassword => {
                if let Ok(user_uuid) = llava_core::get_user_uuid(users_db, &username) {
                    if let Ok(end_of_timeout) =
                        llava_core::local_auth::check_error_count(users_db, &user_uuid)
                    {
                        if end_of_timeout > llava_core::get_time() {
                            let timeout_duration = end_of_timeout - llava_core::get_time();
                            return llava_core::Error::AccountLocked(timeout_duration);
                        }
                    }
                }
                e
            }
            _ => llava_core::Error::FatalError,
        })
}

pub fn log_with_code(
    code: String,
    username: &str,
    paths: &llava_core::ProgramFiles,
    users_db: &rusqlite::Connection,
) -> Result<(llava_core::ProgramFiles, rusqlite::Connection, bool), llava_core::Error> {
    let user_uuid = llava_core::get_user_uuid(users_db, username)?;
    llava_core::local_auth::log_with_code(paths, code, users_db, user_uuid)
}

pub fn check_timeout(
    username: &str,
    users_db: &rusqlite::Connection,
) -> Result<i64, llava_core::Error> {
    let user_uuid = llava_core::get_user_uuid(users_db, username).map_err(|e| match e {
        llava_core::Error::UserNotExists => llava_core::Error::UserNotExists,
        _ => llava_core::Error::FatalError,
    })?;

    match llava_core::local_auth::get_timeout(users_db, &user_uuid) {
        Ok(end_of_timeout) => {
            if end_of_timeout > llava_core::get_time() {
                Ok(end_of_timeout - llava_core::get_time())
            } else {
                Ok(0)
            }
        }
        Err(_) => Err(llava_core::Error::FatalError),
    }
}
