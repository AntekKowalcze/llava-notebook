use llava_core::{AppState, ProgramFiles};

#[tauri::command]
pub async fn create_attachment(
    file: Vec<u8>,
    file_name: String,
    mime_type: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, llava_core::Error> {

   if !is_allowed_mime_type(&mime_type){
    return Err(llava_core::Error::InvalidMimeType)
   }

    let notes_key = {
        let notes_key_guard = state
            .notes_key
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        *notes_key_guard
            .as_ref()
            .ok_or(llava_core::Error::NoKeyToDecryptANote)?
    };

    let program_paths: ProgramFiles = {
        let guard = state
            .paths
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        guard.as_ref().ok_or(llava_core::Error::LockError)?.clone()
    };

    let notes_db_guard = state
        .notes_db
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;
    let notes_db = notes_db_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;
    let current_note_id: uuid::Uuid = {
        let cn_guard = state
            .current_note
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;
        *cn_guard.as_ref().ok_or(llava_core::Error::LockError)?
    };
    let is_encrypted =
        llava_core::storage::check_if_note_is_encrypted(&current_note_id.to_string(), notes_db)?;
    let is_synced =
        llava_core::storage::check_if_note_is_synced(&current_note_id.to_string(), notes_db)?;

    let attachment = llava_core::attachments::create_attachment(
        &notes_key,
        &program_paths.assets_path,
        notes_db,
        current_note_id.to_string(),
        file_name,
        mime_type,
        is_encrypted,
        file,
        is_synced,
    )?;

    Ok(attachment.attachment_id.to_string())
}

fn is_allowed_mime_type(mime_type: &str) -> bool {
   return matches!(
        mime_type,
        "image/png"
            | "image/jpeg"
            | "image/webp"
            | "application/pdf"
            | "text/plain"
    )
}