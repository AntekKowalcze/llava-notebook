use std::collections::HashSet;

use llava_core::{
    attachments::{delete_attachment, get_attachments_for_note},
    AppState,
};

#[tauri::command]
pub async fn clean_attachments(
    content: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), llava_core::Error> {
    let notes_db_guard = state
        .notes_db
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;

    let notes_db = notes_db_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;

    let current_note_guard = state
        .current_note
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;

    let note_id = current_note_guard
        .as_ref()
        .ok_or(llava_core::Error::NoteNotFound)?;

    let note_id = note_id.to_string();

    let used_attachment_ids: HashSet<String> = content
        .match_indices("(attachment://")
        .filter_map(|(index, _)| {
            let start = index + "(attachment://".len();
            let end = start + 36;

            if end <= content.len() {
                Some(content[start..end].to_string())
            } else {
                None
            }
        })
        .collect();

    let attachments = get_attachments_for_note(notes_db, &note_id)?;

    for (attachment_id, _) in attachments {
        if !used_attachment_ids.contains(&attachment_id) {
            delete_attachment(notes_db, attachment_id)?;
        }
    }

    Ok(())
}
