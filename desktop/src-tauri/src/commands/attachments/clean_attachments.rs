use regex::Regex;
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

    let re = attachment_url_regex();

    let used_attachment_ids: HashSet<String> = re
        .captures_iter(&content)
        .filter_map(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        .collect();

    let attachments = get_attachments_for_note(notes_db, &note_id)?;

    let candidates: Vec<String> = attachments
        .into_iter()
        .map(|(attachment_id, _)| attachment_id)
        .filter(|attachment_id| !used_attachment_ids.contains(attachment_id))
        .collect();

    if candidates.is_empty() {
        return Ok(());
    }

    // Another note may still show the image (a conflict copy keeps pointing at
    // the original note's attachments), so only delete what no note uses.
    let notes_key = {
        let guard = state
            .notes_key
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;
        guard.as_ref().copied()
    };
    let used_elsewhere = attachment_ids_used_by_other_notes(notes_db, notes_key, &note_id);

    for attachment_id in candidates {
        if used_elsewhere.contains(&attachment_id) {
            continue;
        }

        // One failing attachment must not stop the rest from being cleaned.
        if let Err(err) = delete_attachment(notes_db, attachment_id.clone()) {
            tracing::error!(
                task = "clean attachments",
                attachment_id = %attachment_id,
                error = ?err,
                "failed to delete unused attachment"
            );
        }
    }

    Ok(())
}

/// Matches both resolved attachment URL formats and captures the id:
///   attachment://localhost/<uuid>          (Linux/macOS)
///   http(s)://attachment.localhost/<uuid>  (Windows)
fn attachment_url_regex() -> Regex {
    let uuid_pattern =
        r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}";
    Regex::new(&format!(
        r"(?:attachment://localhost/|https?://attachment\.localhost/)({uuid_pattern})"
    ))
    .unwrap()
}

/// Attachment ids referenced by the content of every note except `skip_note_id`.
/// Notes that cannot be read or decrypted are skipped, which can only make
/// cleanup delete more, so callers use this together with a "referenced here"
/// check for the note being edited.
pub(crate) fn attachment_ids_used_by_other_notes(
    notes_db: &rusqlite::Connection,
    notes_key: Option<chacha20poly1305::Key>,
    skip_note_id: &str,
) -> HashSet<String> {
    let attachment_url = attachment_url_regex();
    let mut used = HashSet::new();

    let Ok(mut stmt) = notes_db
        .prepare("SELECT local_id, content_path, encrypted FROM notes WHERE local_id != ?1")
    else {
        return used;
    };

    let rows = stmt.query_map(rusqlite::params![skip_note_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, bool>(2)?,
        ))
    });
    let Ok(rows) = rows else {
        return used;
    };

    for (local_id, content_path, encrypted) in rows.flatten() {
        let Ok(mut content) =
            llava_core::storage::get_note_content(&std::path::PathBuf::from(&content_path))
        else {
            continue;
        };

        if encrypted {
            let Some(key) = notes_key.as_ref() else {
                continue;
            };
            match llava_core::crypto_operations::decrypt_note(key, content, &local_id, notes_db) {
                Ok(plain) => content = plain,
                Err(_) => continue,
            }
        }

        for caps in attachment_url.captures_iter(&content) {
            if let Some(id) = caps.get(1) {
                used.insert(id.as_str().to_string());
            }
        }
    }

    used
}
