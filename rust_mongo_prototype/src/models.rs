//! module for saving data models
#[derive(serde::Serialize, serde::Deserialize, Debug)]
///data model of Note
pub struct Note {
    pub created_at: String,
    pub title: String,
    pub summary: String,
    pub content: String,
}
