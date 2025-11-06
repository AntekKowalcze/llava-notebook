//! module for keeping error types

///error type for connecting to database
#[derive(Debug)]
pub enum ConnectionError {
    ConnectionTimeout(tokio::time::error::Elapsed),
    UnableToConnect(mongodb::error::Error),
}

impl From<tokio::time::error::Elapsed> for ConnectionError {
    fn from(error: tokio::time::error::Elapsed) -> Self {
        ConnectionError::ConnectionTimeout(error)
    }
}

impl From<mongodb::error::Error> for ConnectionError {
    fn from(error: mongodb::error::Error) -> Self {
        ConnectionError::UnableToConnect(error)
    }
}
