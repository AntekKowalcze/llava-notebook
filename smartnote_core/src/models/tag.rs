#[derive(serde::Serialize, serde::Deserialize)]
pub struct Tag {
    #[serde(with = "uuid::serde::simple")]
    pub tag_id: uuid::Uuid,
    #[serde(with = "uuid::serde::simple")]
    pub owner_id: uuid::Uuid,
    pub name: String,
    pub color: String,
    pub created_at: i64,
}
