//! module where Aplication custom errors are stored
use serde::Serialize;

use thiserror::Error;

#[derive(Debug, Error)]
///contains errors for sqlite errors and errors for file operations
/// using this error for easier implementation
pub enum Error {
    #[error("File operation error: {0}")]
    FileOperationError(#[from] std::io::Error),
    #[error("Password didnt pass validation")]
    PasswordValidation,

    #[error("Username already exists")]
    UsernameExistsError,

    #[error("User dont exist")]
    UserNotExists,
    #[error("Passwords dont match")]
    WrongPassword,

    #[error("Note name already exists")]
    NoteNameExistsError,

    #[error("Note name after sanitization is empty")]
    NoteNameError,

    #[error("file with this notename already exists")]
    FileAlreadyExists,

    #[error("Title too long")]
    TitleTooLong,

    #[error("name too long")]
    NoteNameToLong,
    #[error("Current user not found in active user file")]
    CurrentUserNotFound,
    #[error("Device ID can't be red from file")]
    DeviceIdErorr,
    #[error(transparent)]
    InternalError(#[from] anyhow::Error),
    #[error("Fatal error couldnt find home directory for app")]
    FatalError, //komunikat panic i wyjście
}

// Manual Serialize - uses the Display from thiserror
impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
         match self {
            Error::InternalError(_) => serializer.serialize_str("Internal error"),
            _ => serializer.serialize_str(&self.to_string()),
        }
    }
}
//TODO dodać usuwanie pliku device id przy jakim kolwiek błędzie, pobieranie go z lokalnej bazy danych aby zapisać ponownie
