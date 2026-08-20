use anyhow::Context;
use rusqlite::Connection;
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Tag {
    pub tag_id: uuid::Uuid,
    pub owner_id: uuid::Uuid,
    pub name: String,
    pub color: String,
    pub created_at: i64,
}
