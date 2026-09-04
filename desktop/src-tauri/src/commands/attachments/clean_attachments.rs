use std::collections::HashSet;
use regex::Regex;

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

    // Matches both resolved formats:
    //   attachment://localhost/<uuid>          (Linux/macOS)
    //   http(s)://attachment.localhost/<uuid>  (Windows)
    let uuid_pattern = r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}";
    let re = Regex::new(&format!(
        r"(?:attachment://localhost/|https?://attachment\.localhost/)({uuid_pattern})"
    )).unwrap();

    let used_attachment_ids: HashSet<String> = re
        .captures_iter(&content)
        .filter_map(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        .collect();

    let attachments = get_attachments_for_note(notes_db, &note_id)?;

    for (attachment_id, _) in attachments {
        if !used_attachment_ids.contains(&attachment_id) {
            delete_attachment(notes_db, attachment_id)?;
        }
    }

    Ok(())
}