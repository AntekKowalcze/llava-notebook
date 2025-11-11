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
