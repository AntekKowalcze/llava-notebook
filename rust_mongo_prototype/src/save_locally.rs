//! module responsible for saving notes locally
/// adding note to vector
pub fn save_locally(
    inserting_object: crate::models::Note,
    local_note_storage: &mut Vec<crate::models::Note>,
) {
    local_note_storage.push(inserting_object);
}
