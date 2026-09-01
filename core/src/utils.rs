//! # Utility functions module
//!
//! **Purpose**: This module provides shared utility functions used throughout
//! the application for time handling, structured logging, local user data
//! lookup, account-link status management, hostname retrieval, and network
//! connectivity checks.
//!
//! ## Exports
//!
//! * [`get_time`] — Returns the current UTC time as a Unix timestamp in
//!   milliseconds.
//! * [`Format`] — Specifies whether additional logging data should be formatted
//!   using [`std::fmt::Display`] or [`std::fmt::Debug`].
//! * [`log_helper`] — Provides a common wrapper for structured informational
//!   logging with optional additional context.
//! * [`get_user_uuid`] — Retrieves and parses the local UUID associated with a
//!   username.
//! * [`get_username_from_uuid`] — Retrieves the local username associated with
//!   a user UUID.
//! * [`get_host_name`] — Retrieves the hostname of the current device.
//! * [`get_online_id`] — Retrieves the online account identifier linked to a
//!   local user.
//! * [`get_email_from_online_id`] — Retrieves the email address associated with
//!   an online account identifier.
//! * [`is_online_linked`] — Checks whether a local user has an online account
//!   linked.
//! * [`change_account_link_status`] — Marks the local user account as linked
//!   to an online account.
//! * [`is_device_connected_to_server`] — Checks whether the application can
//!   establish a TCP connection to the configured application server.
//! * [`is_device_online`] — Checks whether the device can establish a TCP
//!   connection to an external network endpoint.
//! * [`check_connection`] — Performs the server connectivity and general
//!   internet connectivity checks concurrently.
//!
//! ## Key design decisions
//!
//! Time is represented as Unix milliseconds using UTC so that timestamps are
//! independent of the device's local timezone and can be consistently stored
//! and compared across systems.
//!
//! Database helper functions operate directly on the local SQLite connection
//! and map database or data-conversion failures to the application's
//! [`crate::errors::Error`] type.
//!
//! [`log_helper`] centralises a small amount of repeated structured logging
//! logic while allowing callers to choose between `Display` and `Debug`
//! formatting for optional diagnostic data.
//!
//! Connectivity checks use short three-second TCP timeouts so network
//! availability checks do not block the application for an excessive amount
//! of time. [`check_connection`] executes both checks concurrently.
//!
//! The general online connectivity check uses a fixed external endpoint rather
//! than an application-level HTTP request. This keeps the check independent
//! from the availability of the application's own server.
//!
//! ## Dependencies
//!
//! * [`chrono`] — UTC timestamp generation.
//! * [`anyhow`] — Adds context to database, UUID, and hostname errors before
//!   they are converted into application-level errors.
//! * [`rusqlite`] — Access to the local SQLite user database.
//! * [`tracing`] — Structured application logging.
//! * [`uuid`] — Parsing and formatting user identifiers.
//! * [`hostname`] — Retrieval of the current device hostname.
//! * [`tokio`] — Asynchronous TCP connectivity checks and concurrent execution.
//! * [`crate::constants`] — Provides the configured server endpoint used for
//!   connectivity checks.
//! * [`crate::errors`] — Application-level error types returned by fallible
//!   utility functions.

use std::time::Duration;

use anyhow::Context;
use rusqlite::{Connection, OptionalExtension, named_params};

///gets time in UTC timestamp i64
pub fn get_time() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
pub enum Format<'a, T> {
    Display(&'a T),
    Debug(&'a T),
}

pub fn log_helper<T>(task: &str, status: &str, additional_info: Option<Format<T>>, context: &str)
where
    T: std::fmt::Display + std::fmt::Debug,
{
    match additional_info {
        Some(Format::Display(v)) => tracing::info!(task = task, status = status, %v, context),
        Some(Format::Debug(v)) => tracing::info!(task = task, status = status, ?v, context),
        None => tracing::info!(task = task, status = status, context),
    }
}

pub fn get_user_uuid(
    users_db: &Connection,
    username: &str,
) -> Result<uuid::Uuid, crate::errors::Error> {
    let uuid_str_opt = users_db
        .query_row(
            "SELECT user_id FROM users_data WHERE username = :n",
            named_params! {
                ":n": username,
            },
            |row| row.get(0),
        )
        .optional()
        .context("rusqlite error")?;

    let uuid_str: String = uuid_str_opt.ok_or(crate::errors::Error::UserNotExists)?;
    let uuid = uuid::Uuid::parse_str(&uuid_str).context("failed to parse uuid")?;

    Ok(uuid)
}

pub fn get_username_from_uuid(
    users_db: &Connection,
    user_uuid: String,
) -> Result<String, crate::errors::Error> {
    let username: String = users_db
        .query_row(
            "SELECT username FROM users_data WHERE user_id = :id;",
            named_params! {
                ":id": user_uuid,
            },
            |row| row.get::<_, String>(0),
        )
        .context("Failed to get user uuid from database")?;
    Ok(username)
}

pub fn get_host_name() -> Result<String, crate::errors::Error> {
    let hostname = hostname::get()
        .context("failed to get hostname")?
        .to_string_lossy()
        .to_string();
    Ok(hostname)
}

pub fn get_online_id(
    user_id: &uuid::Uuid,
    users_db: &Connection,
) -> Result<String, crate::errors::Error> {
    let online_id = users_db
        .query_row(
            "SELECT online_account_id FROM users_data WHERE user_id = :id;",
            named_params!(":id": user_id.to_string()),
            |row| row.get(0),
        )
        .optional()
        .context("failed to get online user_id")?;

    online_id.ok_or(crate::errors::Error::NotLoggedIn)
}

pub fn get_email_from_online_id(
    online_id: &str,
    users_db: &Connection,
) -> Result<String, crate::errors::Error> {
    let email = users_db
        .query_row(
            "SELECT online_account_email FROM users_data WHERE online_account_id = :id;",
            named_params!(":id": online_id),
            |row| row.get(0),
        )
        .context("failed to get online user_id")?;
    Ok(email)
}
pub fn is_online_linked(
    user_id: &uuid::Uuid,
    users_db: &Connection,
) -> Result<bool, crate::errors::Error> {
    let linked: i64 = users_db
        .query_row(
            "SELECT is_online_linked FROM users_data WHERE user_id = :id;",
            named_params!(":id": user_id.to_string()),
            |row| row.get(0),
        )
        .context("failed to get is_online_linked")?;
    Ok(linked == 1)
}

pub fn change_account_link_status(
    users_data: &Connection,
    user_id: &uuid::Uuid,
) -> Result<(), crate::errors::Error> {
    users_data
        .execute(
            "UPDATE users_data SET is_online_linked = 1 WHERE user_id = :id",
            named_params! {":id": user_id.to_string()},
        )
        .context("failed to update linked in users_data")?;
    Ok(())
}
pub async fn is_device_connected_to_server() -> bool {
    tokio::time::timeout(
        Duration::from_secs(3),
        reqwest::get(crate::constants::SERVER_ADDRESS_TO_PING),
    )
    .await
    .ok()
    .and_then(Result::ok)
    .map(|response| response.status().is_success())
    .unwrap_or(false)
}

pub async fn is_device_online() -> bool {
    tokio::time::timeout(
        Duration::from_secs(3),
        tokio::net::TcpStream::connect("142.250.120.113:80"),
    )
    .await
    .map(|res| res.is_ok())
    .unwrap_or(false)
}

pub async fn check_connection() -> Result<(bool, bool), crate::errors::Error> {
    Ok(tokio::join!(
        is_device_connected_to_server(),
        is_device_online()
    ))
}
