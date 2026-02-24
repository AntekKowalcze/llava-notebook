use anyhow::anyhow;
use llava_core::AppState;
pub fn chagne_state_after_login(
    state: &tauri::State<'_, AppState>,
    user_uuid: uuid::Uuid,
    notes_conn: rusqlite::Connection,
    paths: llava_core::ProgramFiles,
    username: String,
) -> Result<(), llava_core::Error> {
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
    Ok(())
}
