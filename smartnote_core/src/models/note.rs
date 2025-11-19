//! Module contains Note struct
use std::path::PathBuf;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Note {
    #[serde(with = "uuid::serde::simple")]
    pub local_id: uuid::Uuid,
    pub mongo_id: Option<String>,
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

    pub version: i64, //zmieniać wersje o 1 co zapis, porównywać na czasie ostatniej modyfikacji, wersja tylko pomocniczo
    pub cloud_version: Option<i64>,
    pub sync_state: crate::services::storage::db_creation::SyncState,

    pub encrypted: bool,
    pub crypto_meta: Option<serde_json::Value>,
}
