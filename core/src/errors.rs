//! module where Application custom errors are stored

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize, Clone)]
pub enum Error {
    #[error("Password didn't pass validation")]
    PasswordValidation,

    #[error("Username already exists")]
    UsernameExistsError,

    #[error("User doesn't exist")]
    UserNotExists,

    #[error("Wrong password")]
    WrongPassword,

    #[error("Account locked until timestamp {0}")]
    AccountLocked(i64),

    #[error("Note name already exists")]
    NoteNameExistsError,

    #[error("Note name after sanitization is empty")]
    NoteNameError,

    #[error("File with this note name already exists")]
    FileAlreadyExists,

    #[error("Title too long")]
    TitleTooLong,

    #[error("Name too long")]
    NoteNameTooLong,

    #[error("Current user not found in active user file")]
    CurrentUserNotFound,

    #[error("Device ID can't be read from file")]
    DeviceIdError,

    #[error("Fatal error couldn't find home directory for app")]
    FatalError,

    #[error("File operation error: {0}")]
    FileOperationError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::FileOperationError(err.to_string())
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Error::InternalError(err.to_string())
    }
}
