//! # Application entry point
//! **Purpose**: Bootstraps the Tauri application: resolves filesystem paths, opens the
//! local login database, resolves/generates the device id, builds the single
//! `tauri::Builder` chain (plugins, custom protocols, managed state, commands), and
//! hands control to Tauri's event loop via `.run()`.
//!
//! ## Exported items
//! * [`main`] — Binary entry point. Not called directly anywhere else; invoked by the
//!   generated Tauri runtime shim (`#[cfg_attr(mobile, tauri::mobile_entry_point)]`
//!   makes this the entry point on mobile targets too).
//!
//! ## Key design decisions
//! This file must contain **exactly one** `tauri::Builder` chain, ending in **exactly
//! one** `.run(...)` call. `.run()` blocks until the app exits and consumes the
//! builder by value, so a second chain or a leftover scaffold block after it is
//! either a compile error (moved value) or dead code that silently never executes.
//! This bit us once already with the `attachment://` protocol registration living in
//! an unreferenced scaffold `main.rs` — see `commands::attachments::protocol` for the
//! writeup. If you're pasting in example code from docs, put it in a scratch file
//! outside `src-tauri/src`, not here.
//!
//! `AppState` fields are populated incrementally: `users_db` and `device_id` are set
//! here at startup since they don't depend on a logged-in user; `notes_db` and
//! `notes_key` are populated later, on login (see the auth commands), since they
//! require the user's credentials to derive the decryption key.
//!
//! ## Dependencies
//! - `tauri` — `Builder`, `Manager`, `Emitter`, application lifecycle
//! - `llava_core` — `ProgramFiles`, `AppState`, local login DB, device id, logger, settings
//! - `commands` — all `#[tauri::command]` handlers plus the `attachment://` protocol registration

//Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use llava_core::local_auth::connect_or_create_local_login_db;
use llava_core::ProgramFiles;
use tauri::Emitter;
use tauri::Manager;
mod commands;
mod protocols;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn main() {
    let program_paths: ProgramFiles =
        llava_core::ProgramFiles::init().expect("failed creating program pahts");
    let user_db = connect_or_create_local_login_db(&program_paths.local_login_database_path)
        .expect("error while creating locla login db");
    let _logger_worker =
        Some(llava_core::configure_logger(&program_paths.logs_path).expect("failed logger"));
    let device_id = llava_core::get_device_id(&user_db, &program_paths.device_id_path)
        .expect("big error while reading device id");
    println!("{}", device_id);

    let mut builder = tauri::Builder::default();
    builder = builder.plugin(tauri_plugin_opener::init());
    builder = builder.plugin(tauri_plugin_clipboard_manager::init());
    builder = protocols::protocol::register(builder);

    let mut state: llava_core::AppState =
        llava_core::AppState::init().expect("couldnt create state struct");

    state.users_db = std::sync::Mutex::from(Some(user_db));
    state.device_id = std::sync::Mutex::from(Some(device_id));

    state.paths = std::sync::Mutex::from(Some(program_paths));

    builder
        // Runs once, after plugins/state are wired but before the window is shown.
        // Loads the persisted user config (if any), pushes it to the frontend via the
        // `config-updated` event, and starts the background connection-status monitor.
        .setup(|app: &mut tauri::App| {
            app.manage(state);
            let app_state = app.state::<llava_core::AppState>();

            if let Ok(paths_guard) = app_state.paths.lock() {
                if let Some(paths) = paths_guard.as_ref() {
                    if let Ok(state_config) = llava_core::settings::get_config_for_state(paths) {
                        if let Ok(mut config_guard) = app_state.user_config.lock() {
                            *config_guard = Some(state_config.clone());
                        }
                        let _ = app.emit("config-updated", &state_config);
                    }
                }
                let handle = app.handle().clone();

                crate::commands::utils::start_connection_monitor(handle.clone());
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    crate::commands::sync::sync::run_sync_loop(handle).await;
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::local_auth::register_command,
            commands::local_auth::login_command,
            commands::local_auth::check_if_user_exists,
            commands::local_auth::check_timeout_before_submit,
            commands::local_auth::change_password,
            commands::local_auth::log_with_code,
            commands::local_auth::check_login_on_start,
            commands::local_auth::local_logout_command,
            commands::utils::get_username_from_uuid,
            commands::dashboard::get_dashboard_data,
            commands::settings::get_config_data,
            commands::settings::get_config_state,
            commands::settings::update_settings,
            commands::settings::get_methapone_map,
            commands::settings::load_backup_config,
            commands::settings::get_logfile_content,
            commands::settings::get_recovery_codes,
            commands::settings::change_username,
            commands::online_auth::register_user_online,
            commands::online_auth::online_logout,
            commands::online_auth::get_email_from_id,
            commands::online_auth::login_online,
            commands::online_auth::try_login_if_connected_with_server,
            commands::notes::create_note::create_note,
            commands::sliding_panel::get_panel_data,
            crate::commands::utils::get_connection_status,
            commands::notes::note_operations::get_note_content,
            commands::notes::note_operations::save_note,
            commands::notes::note_operations::toggle_note_sync,
            commands::notes::note_operations::get_note_object,
            commands::tags::tags::add_tag_to_note,
            commands::tags::tags::remove_tag_from_note,
            commands::tags::tags::get_all_tags,
            commands::tags::tags::get_all_tags_for_note,
            commands::tags::tags::remove_tag,
            commands::notes::note_operations::change_note_title,
            commands::note_managing_views::get_all_notes_data,
            commands::notes::note_operations::toggle_note_encryption,
            commands::notes::note_operations::remove_note,
            commands::notes::note_operations::hard_delete_note,
            commands::notes::note_operations::restore_note,
            commands::note_managing_views::get_all_removed_notes_data,
            commands::attachments::create_attachment::create_attachment,
            commands::attachments::clean_attachments::clean_attachments,
            commands::sync::sync::synchronize_all,
            commands::ai::ai::ai_request,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
