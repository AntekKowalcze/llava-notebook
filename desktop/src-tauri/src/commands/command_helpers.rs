use anyhow::anyhow;
use indexmap::IndexMap;
use llava_core::AppState;
use zeroize::Zeroize;
pub fn change_state_after_login(
    state: &tauri::State<'_, AppState>,
    user_uuid: uuid::Uuid,
    notes_conn: rusqlite::Connection,
    paths: llava_core::ProgramFiles,
    username: String,
    user_config: IndexMap<String, String>,
    mut notes_key: chacha20poly1305::Key,
) -> Result<(), llava_core::Error> {
    cleanup_expired_notes(&paths.delete_tmp_path, &notes_conn);
    *state
        .current_user
        .lock()
        .map_err(|_| anyhow!("couldnt edit current user"))? = Some(user_uuid);
    *state
        .notes_db
        .lock()
        .map_err(|_| anyhow!("Couldnt edit notes db in state"))? = Some(notes_conn);

    *state
        .paths
        .lock()
        .map_err(|_| anyhow!("Couldnt edit paths in state"))? = Some(paths);
    *state
        .username
        .lock()
        .map_err(|_| anyhow!("Couldnt edit username in state"))? = Some(username);
    *state
        .user_config
        .lock()
        .map_err(|_| anyhow!("Couldnt edit user_config in state"))? = Some(user_config);
    *state
        .notes_key
        .lock()
        .map_err(|_| anyhow!("Couldnt edit kek_bytes in state"))? = Some(notes_key);
    notes_key.zeroize();
    Ok(())
}
pub fn cleanup_expired_notes(tmp_deleted_path: &std::path::Path, notes_db: &rusqlite::Connection) {
    if let Err(err) = llava_core::clean::hard_deletes_terminated_notes(notes_db, tmp_deleted_path) {
        tracing::error!("Failed to cleanup expired notes: {err}");
    }
}
