//! Modul containing Attachement struct
use std::path::PathBuf;
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Attachment {
    pub attachment_id: uuid::Uuid,
    pub note_local_id: uuid::Uuid,

    pub filename: String,
    pub mime_type: String,
    pub size_bytes: i64,

    pub local_path: PathBuf,
    pub cloud_key: Option<String>,

    pub checksum_encrypted: String,
    pub encrypted: bool,
    pub crypto_meta: String,

    pub sync_state: crate::services::storage::db_creation::SyncState,

    pub created_at: i64,
    pub updated_at: i64,
}

//TODO add documentation to all files and update old documentation, then add logs in file where logs are not added, then just put server on server, host mongodb locally on server and create binaries for aplitacion and share them to peoples
//but before deploying to people test app on windows