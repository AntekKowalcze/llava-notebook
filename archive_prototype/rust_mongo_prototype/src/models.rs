//! module for saving data models

use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
///data model of Note
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Note {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub note_id: Option<ObjectId>,
    pub created_at: String,
    pub title: String,
    pub summary: String,
    pub content: String,
}
