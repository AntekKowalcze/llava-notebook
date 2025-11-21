//! module where Aplication custom errors are stored
use serde::Serialize;

use thiserror::Error;

#[derive(Debug, Error)]
///contains errors for sqlite errors and errors for file operations
/// using this error for easier implementation
pub enum Error {
    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),

    #[error("File operation error: {0}")]
    FileOperationError(#[from] std::io::Error),

    #[error("Deserialization error: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("Uuid Parsing error: {0}")]
    UuidParsingError(#[from] uuid::Error),

    #[error("Error while hashing password: {0}")]
    ArgonHashError(#[from] argon2::password_hash::Error),

    #[error("Error while derivation of key in Argon: {0}")]
    ArgonKeyDeriviationError(#[from] argon2::Error),

    #[error("Error while encrypting main key for user: {0}")]
    ChaChaEncyptionError(#[from] chacha20poly1305::Error),

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

    #[error("Title too long")]
    TitleTooLong,

    #[error("name too long")]
    NoteNameToLong,
}

// Manual Serialize - uses the Display from thiserror
impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
//TODO dodać usuwanie pliku device id przy jakim kolwiek błędzie, pobieranie go z lokalnej bazy danych aby zapisać ponownie
