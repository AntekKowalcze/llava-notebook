//! # Application error types
//! **Purpose**: Defines the single [`Error`] enum used across the entire codebase.
//! All modules convert their internal errors into this type, which is also serialisable
//! so Tauri commands can forward errors directly to the frontend as JSON.
//!
//! `std::io::Error` and `anyhow::Error` both convert into this type via `From` impls —
//! `io::Error` maps to `FileOperationError` and `anyhow::Error` maps to `InternalError`.
//!
//! ## Dependencies
//! - `thiserror` — Derives `Error` and formats `#[error(...)]` messages
//! - `serde` — `Serialize` impl for Tauri frontend error forwarding

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize, Clone)]
pub enum Error {
    #[error("Password didn't pass validation")]
    PasswordValidation,

    #[error("Username already exists")]
    UsernameExistsError,

    #[error("Email already used")]
    EmailAlreadyUsed,

    #[error("User doesn't exist")]
    UserNotExists,

    #[error("Wrong password")]
    WrongPassword,

    #[error("Wrong credentials")]
    WrongCredentials,

    #[error("Account locked until timestamp {0}")]
    AccountLocked(i64),

    #[error("Note name already exists")]
    NoteNameExistsError,

    #[error("Note name after sanitization is empty")]
    NoteNameError,

    #[error("Code not found")]
    CodeNotFound,

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

    #[error("Couldn't lock state, check deadlocks.")]
    LockError,

    #[error("Email not verified.")]
    WrongEmail,

    #[error("Not logged in online")]
    NotLoggedIn,

    #[error("No internet connection")]
    NoInternetConnection,

    #[error("Server not responding")]
    ServerNotAvailable,

    #[error("Refresh token expired")]
    OnlineSessionExpired,

    #[error("User is not owner of this note")]
    UserIsNotOwner,

    #[error("Note not found")]
    NoteNotFound,

    #[error("Encryption Error")]
    NoKeyToDecryptANote,

    #[error("Invalid tag name")]
    InvalidTagName,

    #[error("Tag already exists")]
    TagAlreadyExists,

    #[error("Invalid mime type")]
    InvalidMimeType,

    #[error("Sync failed")]
    SyncFailed,

    #[error("Attachment upload failed")]
    FailedToUploadAttachment,
    #[error("Attachment download failed")]
    AttachmendDownloadFailed,

    #[error("Ai request erorr")]
    AiError,
    
    #[error("Request error")]
    RequestError((u16, String)),

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
