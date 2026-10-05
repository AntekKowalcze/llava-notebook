//! # Synchronization command module
//!
//! **Purpose**: This module orchestrates the client-side cloud synchronization
//! workflow between the Tauri application state, the local SQLite database,
//! the remote synchronization service, and attachment storage.
//!
//! It performs full synchronization cycles, refreshes expired access tokens,
//! processes synchronization instructions, applies local database operations
//! atomically, retries failed note operations, and periodically runs automatic
//! synchronization when online sync is enabled.
//!
//! ## Exports
//!
//! * [`synchronize_all`] — Executes a complete synchronization cycle, including
//!   connection checks, token refresh, server synchronization checks, uploads,
//!   downloads, deletions, retries, and final synchronization-state updates.
//! * [`refresh_access_token`] — Obtains a new access token for an online account
//!   and stores it in [`AppState`].
//! * [`run_sync_loop`] — Runs the background synchronization loop that
//!   periodically triggers [`synchronize_all`] when the application is
//!   configured for online synchronization.
//!
//! ## Key design decisions
//!
//! Synchronization is disabled while local mode is active. It is also skipped
//! when the user has explicitly disabled the `online.sync` setting. These
//! checks happen before any network or database synchronization work is
//! started.
//!
//! The synchronization workflow is split into a server decision phase and a
//! client execution phase. The server first determines which notes and
//! attachments must be uploaded, downloaded, synchronized, or deleted.
//! [`process_sync_response`] then converts those instructions into filesystem
//! changes, network operations, and queued [`DbOperation`] values.
//!
//! Local SQLite mutations produced by one synchronization response are applied
//! through a single transaction. This prevents a synchronization batch from
//! leaving the local database in a partially updated state when one operation
//! fails.
//!
//! Access-token expiration is handled transparently. When a synchronization
//! request reports [`llava_core::Error::OnlineSessionExpired`], the module
//! refreshes the online session and retries the synchronization request with
//! the new token.
//!
//! Newly uploaded notes require an additional synchronization pass because
//! creating a cloud note assigns its server-side MongoDB identifier and
//! initial cloud version. The second pass allows attachments and subsequent
//! note state to be evaluated using the newly established cloud identity.
//!
//! Failed note synchronizations are collected and retried once more after the
//! main synchronization processing completes. Notes that still fail after the
//! retry are marked with [`DbOperation::MarkNoteError`] in the local database.
//!
//! Synchronization responses can contain both filesystem operations and
//! database operations. Files are prepared before the corresponding database
//! transaction is committed, so database state is not updated to reference
//! content that could not be successfully downloaded or written.
//!
//! Attachment uploads and downloads are delegated to the core synchronization
//! layer. This command module coordinates the operations and state transitions
//! but does not directly implement the S3 transfer protocol.
//!
//! The background synchronization loop uses a fixed interval and checks local
//! configuration and authentication state before starting a synchronization
//! cycle. Synchronization errors are logged without terminating the background
//! loop, allowing later iterations to continue operating.
//!
//! Progress and exceptional synchronization states are communicated to the
//! frontend through Tauri events such as `sync_progress` and
//! `quota_exceeded`.
//!
//! ## Dependencies
//!
//! * [`tauri`] — Tauri commands, managed application state, application handles,
//!   event emission, and access to application-managed state from background
//!   tasks.
//! * [`reqwest`] — HTTP client used for synchronization and authentication
//!   requests.
//! * [`tokio`] — Asynchronous synchronization operations, timers, and
//!   background execution.
//! * [`serde`] — Serialization of synchronization progress states.
//! * [`llava_core::sync`] — Core synchronization logic, synchronization data
//!   structures, database operations, and server communication.
//! * [`llava_core::online_auth`] — Online session validation and access-token
//!   refresh logic.
//! * [`llava_core`] — Provides [`AppState`], application errors, and shared
//!   configuration structures.
//! * [`crate::commands::utils`] — Provides connectivity checks before network
//!   synchronization.

use llava_core::online_auth::AccessToken;
use llava_core::{sync::DbOperation, AppState, ProgramFiles};
use serde::Serialize;
use tauri::AppHandle;
use tauri::Emitter;
struct SyncProcessResult {
    failed_notes: Vec<String>,
    newly_uploaded_notes: Vec<String>,
}
#[derive(Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
enum SyncResult {
    Done,
    InProgress,
    Error,
}
use tauri::Manager;

/// Sent after a sync pass changed local data, so open views can refresh.
#[derive(Serialize, Clone)]
struct SyncFinishedPayload {
    /// Notes inserted or overwritten from the cloud, plus conflict copies.
    changed_note_ids: Vec<String>,
    /// Notes removed locally because they were deleted in the cloud.
    removed_note_ids: Vec<String>,
}

/// Only one sync cycle may run at a time. The 60 s timer, the first sync
/// after login and a manual trigger can otherwise overlap; both runs then
/// act on the same server answer and, for example, insert a note downloaded
/// from the cloud twice.
static SYNC_RUNNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Held while a sync cycle (or an operation that must not race with one, such
/// as deleting local data on logout) is running.
pub struct SyncRunGuard;

impl SyncRunGuard {
    fn try_acquire() -> Option<SyncRunGuard> {
        if SYNC_RUNNING.swap(true, std::sync::atomic::Ordering::AcqRel) {
            None
        } else {
            Some(SyncRunGuard)
        }
    }
}

impl Drop for SyncRunGuard {
    fn drop(&mut self) {
        SYNC_RUNNING.store(false, std::sync::atomic::Ordering::Release);
    }
}

/// Waits (up to `max_wait`) for a running sync cycle to finish and takes the
/// sync lock, so nothing else syncs until the guard is dropped.
pub async fn acquire_sync_lock(
    max_wait: std::time::Duration,
) -> Result<SyncRunGuard, llava_core::Error> {
    let deadline = tokio::time::Instant::now() + max_wait;

    loop {
        if let Some(guard) = SyncRunGuard::try_acquire() {
            return Ok(guard);
        }

        if tokio::time::Instant::now() >= deadline {
            return Err(llava_core::Error::SyncFailed);
        }

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}

/// Reads a `"on"`/`"off"` style user setting; `None` when unavailable.
fn config_value(state: &tauri::State<'_, AppState>, key: &str) -> Option<String> {
    state
        .user_config
        .lock()
        .ok()
        .and_then(|config| config.as_ref().and_then(|map| map.get(key).cloned()))
}

/// Whether online sync is switched off in settings.
pub fn is_online_sync_off(state: &tauri::State<'_, AppState>) -> bool {
    config_value(state, "online.sync")
        .map(|value| value == "off")
        .unwrap_or(false)
}

#[tauri::command]
pub async fn synchronize_all(
    state: tauri::State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), llava_core::Error> {
    let is_local_only: bool = match state.user_config.lock() {
        Ok(config) => config
            .as_ref()
            .and_then(|map| map.get("local.mode"))
            .map(|value| value == "on")
            .unwrap_or(false),
        Err(_) => true,
    };
    if is_local_only {
        return Err(llava_core::Error::InternalError(
            "cannot sync in local mode".to_string(),
        ));
    } else if is_online_sync_off(&state) {
        return Ok(());
    }

    let Some(_run_guard) = SyncRunGuard::try_acquire() else {
        tracing::debug!(task = "sync", "sync already running, skipping this trigger");
        return Ok(());
    };

    sync_cycle(state, app_handle).await
}

/// One complete sync cycle. The caller must hold the [`SyncRunGuard`].
pub async fn sync_cycle(
    state: tauri::State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), llava_core::Error> {
    let _ = app_handle.emit("sync_progress", SyncResult::InProgress);
    let result = async {
        crate::commands::utils::check_connection_before_request(state.clone())?;

        let client = state.server_client.clone();

        let mut access_token: AccessToken = {
            let guard = state
                .access_token
                .lock()
                .map_err(|_| llava_core::Error::LockError)?;

            guard
                .as_ref()
                .ok_or(llava_core::Error::NotLoggedIn)?
                .clone()
        };

        let online_id: String = {
            let guard = state
                .online_user_id
                .lock()
                .map_err(|_| llava_core::Error::LockError)?;

            guard.as_ref().ok_or(llava_core::Error::LockError)?.clone()
        };

        let notes_to_sync = get_notes_to_sync(&state)?;

        let first_response = match llava_core::sync::sync(
            client.clone(),
            notes_to_sync,
            &access_token,
            true,
        )
        .await
        {
            Ok(response) => response,

            Err(llava_core::Error::OnlineSessionExpired) => {
                access_token = refresh_access_token(&state, &client, &online_id).await?;

                let notes_to_sync = get_notes_to_sync(&state)?;

                llava_core::sync::sync(client.clone(), notes_to_sync, &access_token, true).await?
            }

            Err(err) => return Err(err),
        };
        if first_response.quota_exceeded {
            let _ = app_handle.emit("quota_exceeded", ());
        }

        let mut process_result = process_sync_response(
            state.clone(),
            &app_handle,
            client.clone(),
            first_response,
            access_token.clone(),
            online_id.clone(),
        )
        .await?;

        if !process_result.newly_uploaded_notes.is_empty() {
            let second_notes =
                get_notes_by_local_ids(&state, &process_result.newly_uploaded_notes)?;

            if !second_notes.is_empty() {
                let second_response = match llava_core::sync::sync(
                    client.clone(),
                    second_notes,
                    &access_token,
                    false,
                )
                .await
                {
                    Ok(response) => response,

                    Err(llava_core::Error::OnlineSessionExpired) => {
                        access_token = refresh_access_token(&state, &client, &online_id).await?;

                        let second_notes =
                            get_notes_by_local_ids(&state, &process_result.newly_uploaded_notes)?;

                        llava_core::sync::sync(client.clone(), second_notes, &access_token, false)
                            .await?
                    }

                    Err(err) => return Err(err),
                };

                let second_result = process_sync_response(
                    state.clone(),
                    &app_handle,
                    client.clone(),
                    second_response,
                    access_token.clone(),
                    online_id.clone(),
                )
                .await?;

                process_result
                    .failed_notes
                    .extend(second_result.failed_notes);
            }
        }

        let failed_notes = deduplicate_ids(process_result.failed_notes);

        if failed_notes.is_empty() {
            return Ok(());
        }

        let retry_notes = get_notes_by_local_ids(&state, &failed_notes)?;

        if retry_notes.is_empty() {
            return Ok(());
        }

        let retry_response =
            match llava_core::sync::sync(client.clone(), retry_notes, &access_token, false).await {
                Ok(response) => response,

                Err(llava_core::Error::OnlineSessionExpired) => {
                    access_token = refresh_access_token(&state, &client, &online_id).await?;

                    let retry_notes = get_notes_by_local_ids(&state, &failed_notes)?;

                    llava_core::sync::sync(client.clone(), retry_notes, &access_token, false)
                        .await?
                }

                Err(err) => return Err(err),
            };

        let retry_result = process_sync_response(
            state.clone(),
            &app_handle,
            client,
            retry_response,
            access_token,
            online_id,
        )
        .await?;

        let retry_failed_notes = deduplicate_ids(retry_result.failed_notes);

        if !retry_failed_notes.is_empty() {
            let operations = retry_failed_notes
                .into_iter()
                .map(|local_id| DbOperation::MarkNoteError { local_id })
                .collect::<Vec<_>>();

            let mut notes_db_guard = state
                .notes_db
                .lock()
                .map_err(|_| llava_core::Error::LockError)?;

            let notes_db = notes_db_guard
                .as_mut()
                .ok_or(llava_core::Error::LockError)?;

            llava_core::sync::execute_db_operations(notes_db, operations)?;
        }
        Ok::<(), llava_core::Error>(())
    }
    .await;
    match result {
        // Another sync (e.g. from a second device) holds the server-side lock.
        // Nothing went wrong; the next tick tries again.
        Err(llava_core::Error::SyncInProgress) => {
            let _ = app_handle.emit("sync_progress", SyncResult::Done);
            Ok(())
        }
        Err(err) => {
            let _ = app_handle.emit("sync_progress", SyncResult::Error);
            Err(err)
        }
        Ok(()) => {
            let _ = app_handle.emit("sync_progress", SyncResult::Done);
            Ok(())
        }
    }
}

pub async fn refresh_access_token(
    state: &tauri::State<'_, AppState>,
    client: &reqwest::Client,
    online_id: &str,
) -> Result<AccessToken, llava_core::Error> {
    let new_access_token =
        llava_core::online_auth::check_if_logged_in_online(online_id, client.clone()).await?;

    {
        let mut guard = state
            .access_token
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        *guard = Some(new_access_token.clone());
    }

    Ok(new_access_token)
}

fn get_notes_to_sync(
    state: &tauri::State<'_, AppState>,
) -> Result<Vec<llava_core::sync::CheckNoteSyncStatus>, llava_core::Error> {
    let notes_db_guard = state
        .notes_db
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;

    let notes_db = notes_db_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;

    llava_core::sync::get_all_notes_to_sync(notes_db)
}

fn get_notes_by_local_ids(
    state: &tauri::State<'_, AppState>,
    local_ids: &[String],
) -> Result<Vec<llava_core::sync::CheckNoteSyncStatus>, llava_core::Error> {
    if local_ids.is_empty() {
        return Ok(Vec::new());
    }

    let notes_db_guard = state
        .notes_db
        .lock()
        .map_err(|_| llava_core::Error::LockError)?;

    let notes_db = notes_db_guard
        .as_ref()
        .ok_or(llava_core::Error::LockError)?;

    let all_notes = llava_core::sync::get_all_notes_to_sync(notes_db)?;

    Ok(all_notes
        .into_iter()
        .filter(|note| local_ids.contains(&note.local_id))
        .collect())
}

async fn process_sync_response(
    state: tauri::State<'_, AppState>,
    app_handle: &AppHandle,
    client: reqwest::Client,
    mut next_steps: llava_core::sync::CheckSyncResponse,
    access_token: AccessToken,
    online_id: String,
) -> Result<SyncProcessResult, llava_core::Error> {
    let mut db_operations = Vec::<DbOperation>::new();

    {
        let notes_db_guard = state
            .notes_db
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        let notes_db = notes_db_guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?;

        llava_core::sync::drop_instructions_for_local_only_notes(notes_db, &mut next_steps)?;
    }

    let paths: ProgramFiles = {
        let guard = state
            .paths
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        guard.as_ref().ok_or(llava_core::Error::LockError)?.clone()
    };

    let user_id: String = {
        let guard = state
            .current_user
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?
            .to_string()
    };

    let mut notes_to_upload = Vec::<llava_core::sync::NoteForUpload>::new();

    let mut attachments_to_upload = Vec::<(llava_core::sync::AttachmentForUpload, String)>::new();

    let (
        notes_to_download_operations,
        notes_hard_delete_operations,
        attachments_hard_delete_operations,
        newly_uploaded_notes,
    ) = {
        let notes_db_guard = state
            .notes_db
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        let notes_db = notes_db_guard
            .as_ref()
            .ok_or(llava_core::Error::LockError)?;

        let notes_to_download_operations = llava_core::sync::handle_notes_to_download(
            next_steps.notes_to_download.clone(),
            notes_db,
            &paths.notes_path,
            &paths.delete_tmp_path,
            user_id,
        )?;

        let mut newly_uploaded_notes = Vec::new();

        for local_id in &next_steps.to_upload {
            match llava_core::sync::get_note_for_upload(notes_db, local_id) {
                Ok(note) => {
                    let is_new = note.mongo_id.as_ref().map_or(true, |id| id.is_empty());

                    if is_new {
                        newly_uploaded_notes.push(note.local_id.clone());
                    }

                    notes_to_upload.push(note);
                }

                Err(err) => {
                    tracing::error!(
                        task = "sync",
                        local_id = %local_id,
                        error = ?err,
                        "failed to prepare note for upload"
                    );
                }
            }
        }

        for upload_data in &next_steps.attachments_to_upload {
            match llava_core::sync::get_attachment_for_upload(notes_db, &upload_data.attachment_id)
            {
                Ok(attachment) => {
                    attachments_to_upload.push((attachment, upload_data.upload_url.clone()));
                }

                Err(err) => {
                    tracing::error!(
                        task = "sync",
                        attachment_id = %upload_data.attachment_id,
                        error = ?err,
                        "failed to prepare attachment for upload"
                    );
                }
            }
        }

        let notes_hard_delete_operations = llava_core::sync::handle_notes_to_hard_delete(
            next_steps.notes_to_hard_delete.clone(),
            notes_db,
        )?;

        let attachments_hard_delete_operations =
            llava_core::sync::handle_attachments_to_hard_delete(
                notes_db,
                next_steps.attachments_to_hard_delete.clone(),
            )?;

        (
            notes_to_download_operations,
            notes_hard_delete_operations,
            attachments_hard_delete_operations,
            newly_uploaded_notes,
        )
    };

    db_operations.extend(notes_to_download_operations);

    db_operations.extend(notes_hard_delete_operations);

    db_operations.extend(attachments_hard_delete_operations);

    let server_operations = llava_core::sync::execute_server_operations(
        client,
        attachments_to_upload,
        notes_to_upload,
        next_steps.clone(),
        access_token,
        online_id.clone(),
        &paths.assets_path,
    )
    .await?;

    db_operations.extend(server_operations);

    db_operations.extend(llava_core::sync::handle_notes_synced(
        next_steps.notes_synced,
    ));

    db_operations.extend(llava_core::sync::handle_attachment_synced(
        next_steps.attachments_synced,
        online_id,
    ));

    let mut changed_note_ids = Vec::new();
    let mut removed_note_ids = Vec::new();
    for operation in &db_operations {
        match operation {
            DbOperation::InsertNote { local_id, .. }
            | DbOperation::UpdateNoteFromCloud { local_id, .. }
            | DbOperation::InsertConflictCopy { local_id, .. } => {
                changed_note_ids.push(local_id.clone())
            }
            DbOperation::DeleteNote { local_id } => removed_note_ids.push(local_id.clone()),
            _ => {}
        }
    }

    {
        let mut notes_db_guard = state
            .notes_db
            .lock()
            .map_err(|_| llava_core::Error::LockError)?;

        let notes_db = notes_db_guard
            .as_mut()
            .ok_or(llava_core::Error::LockError)?;

        llava_core::sync::execute_db_operations(notes_db, db_operations)?;
    }

    if !changed_note_ids.is_empty() || !removed_note_ids.is_empty() {
        let _ = app_handle.emit(
            "sync_finished",
            SyncFinishedPayload {
                changed_note_ids,
                removed_note_ids,
            },
        );
    }

    Ok(SyncProcessResult {
        failed_notes: next_steps.notes_failed,
        newly_uploaded_notes,
    })
}

fn deduplicate_ids(ids: Vec<String>) -> Vec<String> {
    let mut result = Vec::with_capacity(ids.len());

    for id in ids {
        if !result.contains(&id) {
            result.push(id);
        }
    }

    result
}

/// Upper bound for one background sync cycle. The HTTP client has no request
/// timeout, so a stalled connection would otherwise keep the sync lock forever
/// and every later tick would be skipped silently.
const SYNC_CYCLE_TIMEOUT: tokio::time::Duration = tokio::time::Duration::from_secs(300);

/// Why a background sync should not run right now, if any reason applies.
fn background_sync_skip_reason(state: &tauri::State<'_, AppState>) -> Option<&'static str> {
    if config_value(state, "local.mode").as_deref() == Some("on") {
        return Some("local mode is on");
    }

    if is_online_sync_off(state) {
        return Some("online sync is off");
    }

    // A poisoned lock is treated like "not logged in" so the loop survives.
    let has_user_id = state
        .online_user_id
        .lock()
        .map(|user_id| user_id.is_some())
        .unwrap_or(false);

    if !has_user_id {
        return Some("not logged in online");
    }

    None
}

async fn run_background_sync(app_handle: &AppHandle) {
    let state: tauri::State<'_, AppState> = app_handle.state::<AppState>();

    if let Some(reason) = background_sync_skip_reason(&state) {
        tracing::debug!(task = "auto sync", reason, "background sync skipped");
        return;
    }

    match tokio::time::timeout(
        SYNC_CYCLE_TIMEOUT,
        synchronize_all(state, app_handle.clone()),
    )
    .await
    {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            tracing::error!(
                task = "auto sync",
                status = "error",
                %e,
                "error"
            );
        }
        Err(_) => {
            tracing::error!(
                task = "auto sync",
                status = "error",
                "sync cycle timed out and was cancelled"
            );
            let _ = app_handle.emit("sync_progress", SyncResult::Error);
        }
    }
}

pub async fn run_sync_loop(app_handle: AppHandle) {
    let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(60));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        run_background_sync(&app_handle).await;
    }
}
pub async fn first_sync(app_handle: AppHandle) {
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    run_background_sync(&app_handle).await;
}
