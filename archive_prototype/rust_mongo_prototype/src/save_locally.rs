//! module responsible for saving notes locally
use crate::models::Note;
use std::sync::Arc;
use tokio::sync::RwLock;
/// adding note to vector
pub async fn save_locally(
    inserting_object: crate::models::Note,
    local_note_storage: Arc<RwLock<Vec<Note>>>,
) {
    let mut local_note_to_write: tokio::sync::RwLockWriteGuard<'_, Vec<Note>> =
        local_note_storage.write().await;
    local_note_to_write.push(inserting_object)
}
