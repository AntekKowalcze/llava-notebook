use anyhow::anyhow;
use llava_core::AppState;
#[tauri::command]
pub async fn get_dashboard_data(
    user_uuid: String,
    state: tauri::State<'_, AppState>,
) -> Result<llava_core::stats::DashboardData, llava_core::Error> {
    let users_db_guard: std::sync::MutexGuard<'_, Option<rusqlite::Connection>> = state
        .users_db
        .lock()
        .map_err(|_| anyhow!("error while gettnig users_db from state"))?;

    let users_db = users_db_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;

    let notes_db_guard = state
        .notes_db
        .lock()
        .map_err(|_| anyhow!("Couldnt edit notes db in state"))?;
    let notes_db = notes_db_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;

    let notes_key = {
        let notes_key_guard = state
            .notes_key
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        *notes_key_guard
            .as_ref()
            .ok_or(llava_core::Error::NoKeyToDecryptANote)?
    };

    llava_core::stats::get_dashboard_stats(user_uuid, notes_db, users_db, &notes_key)
}

/// Whether the cloud part of [`StorageUsage`] could be read.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CloudStorageStatus {
    Available,
    /// Local-only account, sync switched off, or not logged in online.
    NotConnected,
    /// Offline, or the server does not report usage (yet).
    Unavailable,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageUsage {
    local: llava_core::stats::LocalStorageUsage,
    cloud: Option<llava_core::stats::CloudStorageUsage>,
    cloud_status: CloudStorageStatus,
}

/// Space taken by the user's notes and attachments on this device and, when
/// logged in online, the cloud attachment quota.
#[tauri::command]
pub async fn get_storage_usage(
    state: tauri::State<'_, AppState>,
) -> Result<StorageUsage, llava_core::Error> {
    let local = {
        let notes_db_guard = state
            .notes_db
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;
        let notes_db = notes_db_guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?;

        llava_core::stats::get_local_storage_usage(notes_db)?
    };

    let (cloud, cloud_status) = match fetch_cloud_usage(&state).await {
        Ok(Some(usage)) => (Some(usage), CloudStorageStatus::Available),
        Ok(None) => (None, CloudStorageStatus::Unavailable),
        Err(llava_core::Error::NotLoggedIn) => (None, CloudStorageStatus::NotConnected),
        Err(_) => (None, CloudStorageStatus::Unavailable),
    };

    Ok(StorageUsage {
        local,
        cloud,
        cloud_status,
    })
}

async fn fetch_cloud_usage(
    state: &tauri::State<'_, AppState>,
) -> Result<Option<llava_core::stats::CloudStorageUsage>, llava_core::Error> {
    if crate::commands::sync::sync::is_online_sync_off(state) {
        return Err(llava_core::Error::NotLoggedIn);
    }

    let access_token = {
        let token = state
            .access_token
            .lock()
            .map_err(|_| llava_core::Error::LockError)?
            .clone();
        let has_online_id = state
            .online_user_id
            .lock()
            .map_err(|_| llava_core::Error::LockError)?
            .is_some();

        match token {
            Some(token) if has_online_id => token,
            _ => return Err(llava_core::Error::NotLoggedIn),
        }
    };

    crate::commands::utils::check_connection_before_request(state.clone())?;

    // An expired token is reported as unavailable, not refreshed here: refresh
    // tokens are single-use, so refreshing in parallel with the background
    // sync could invalidate the sync's refresh and log the user out. The
    // background sync refreshes the token within a minute.
    llava_core::stats::fetch_cloud_storage_usage(state.server_client.clone(), &access_token).await
}
