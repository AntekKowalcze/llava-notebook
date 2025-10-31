//! module for saving data models

use serde::{Deserialize, Serialize};
///data model of Note
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Note {
    pub created_at: String,
    pub title: String,
    pub summary: String,
    pub content: String,
}
