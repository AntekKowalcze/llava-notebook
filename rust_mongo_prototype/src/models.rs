//! module for saving data models
#[derive(serde::Serialize, serde::Deserialize, Debug)]
///data model of Note
pub struct Note {
    pub note_id: Option<u64>,
    pub created_at: String,
    pub title: String,
    pub summary: String,
    pub content: String,
}
