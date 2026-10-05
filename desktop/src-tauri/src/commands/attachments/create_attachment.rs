use llava_core::{AppState, ProgramFiles};

#[tauri::command]
pub async fn create_attachment(
    file: Vec<u8>,
    file_name: String,
    mime_type: String,
    // The note the editor is showing. Optional for older callers, which fall
    // back to the globally tracked current note; passing it avoids attaching
    // the file to the wrong note when the user switches notes mid-upload.
    note_id: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<String, llava_core::Error> {
    if !is_allowed_mime_type(&mime_type) {
        return Err(llava_core::Error::InvalidMimeType);
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
    let current_note_id: uuid::Uuid = match note_id {
        Some(id) => {
            let note_id = uuid::Uuid::parse_str(&id)
                .map_err(|_| llava_core::Error::InternalError("invalid note id".to_string()))?;

            let user_id = {
                let guard = state
                    .current_user
                    .lock()
                    .map_err(|_| llava_core::Error::LockError)?;
                guard
                    .as_ref()
                    .ok_or(llava_core::Error::LockError)?
                    .to_string()
            };

            if !llava_core::storage::verify_note_owner(&user_id, &note_id.to_string(), notes_db)? {
                return Err(llava_core::Error::UserIsNotOwner);
            }

            note_id
        }
        None => {
            let cn_guard = state
                .current_note
                .lock()
                .map_err(|_| llava_core::Error::LockError)?;
            *cn_guard.as_ref().ok_or(llava_core::Error::LockError)?
        }
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
        "image/png" | "image/jpeg" | "image/webp" | "image/gif" | "application/pdf" | "text/plain"
    );
}

/// Largest local image file we are willing to read into memory.
const MAX_LOCAL_IMAGE_BYTES: u64 = 100 * 1024 * 1024;

fn is_local_image_extension(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "webp" | "gif"
            )
        })
        .unwrap_or(false)
}

/// Returns the bytes of a local image file so the editor can feed it into the
/// normal upload flow (`create_attachment`). Needed because a file copied or
/// dragged out of a file manager reaches the webview only as a path, never as
/// file data. Only common image types are read, so a pasted path cannot be used
/// to pull in an arbitrary file.
#[tauri::command]
pub async fn read_local_image(path: String) -> Result<tauri::ipc::Response, llava_core::Error> {
    let path = std::path::PathBuf::from(path);

    if !is_local_image_extension(&path) {
        return Err(llava_core::Error::InvalidMimeType);
    }

    let metadata = tokio::fs::metadata(&path)
        .await
        .map_err(|e| llava_core::Error::InternalError(format!("cannot read image file: {e}")))?;

    if !metadata.is_file() || metadata.len() > MAX_LOCAL_IMAGE_BYTES {
        return Err(llava_core::Error::InternalError(
            "image file is missing or too large".to_string(),
        ));
    }

    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| llava_core::Error::InternalError(format!("cannot read image file: {e}")))?;

    Ok(tauri::ipc::Response::new(bytes))
}
