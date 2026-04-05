//! Module contains Note struct
use std::path::PathBuf;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Note {
    #[serde(with = "uuid::serde::simple")]
    pub local_id: uuid::Uuid,
    pub mongo_id: Option<String>, //change to ObjectId with serde rename tag check user model
    #[serde(with = "uuid::serde::simple")]
    pub owner_id: uuid::Uuid,

    pub name: String,
    pub title: String,
    pub summary: String,
    pub content_path: PathBuf,

    pub created_at: i64,
    pub updated_at: i64,
    pub is_deleted: bool,
    pub deleted_at: Option<i64>,

    pub version: i64, //change versions by 1 every save, compare at the time of last modification, version only for auxiliary purposes
    pub cloud_version: Option<i64>,
    pub sync_state: crate::services::storage::db_creation::SyncState,

    pub encrypted: bool,

    pub crypto_meta: Option<serde_json::Value>,
}
//add here vec of attachments, but attachment stuct should be changed or rethinked so its good to go
//add vector clock, so its good for sync
//add this for ai summary, migrate db and change database structure
// "ai_summary": "<binary>",
//   "ai_summary_nonce": "<binary>",
//   "ai_summary_updated_at": "timestamp",
