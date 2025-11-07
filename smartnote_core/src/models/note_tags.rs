#[derive(serde::Serialize, serde::Deserialize)]
pub struct NoteTags {
    #[serde(with = "uuid::serde::simple")]
    pub note_local_id: uuid::Uuid,
    #[serde(with = "uuid::serde::simple")]
    pub tag_id: uuid::Uuid,
    pub created_at: i64,
}
