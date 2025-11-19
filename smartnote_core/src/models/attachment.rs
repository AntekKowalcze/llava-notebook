//! Modul containing Attachement struct
use std::path::PathBuf;
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Attachment {
    #[serde(with = "uuid::serde::simple")]
    pub attachment_id: uuid::Uuid,
    #[serde(with = "uuid::serde::simple")]
    pub note_local_id: uuid::Uuid,

    pub filename: String,
    pub mime_type: String,
    pub size_bytes: i64,

    pub local_path: PathBuf,
    pub cloud_key: Option<String>,

    pub checksum_encrypted: String,
    pub encrypted: bool,
    pub crypto_meta: Option<serde_json::Value>,

    pub sync_state: crate::services::storage::db_creation::SyncState,

    pub created_at: i64,
    pub updated_at: i64,
}
