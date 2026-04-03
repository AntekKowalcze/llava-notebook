//Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use llava_core::local_auth::connect_or_create_local_login_db;
use llava_core::ProgramFiles;
use tauri::Manager;
mod commands;
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn main() {
    let program_paths: ProgramFiles =
        llava_core::ProgramFiles::init().expect("failed creating program pahts");

    // if cfg!(not(debug_assertions)) {
    // llava_core::ProgramFiles::init().expect("failed creating program pahts");
    // }
    //  else {
    // let path = std::env::temp_dir().join("llava_test");
    // if path.exists() {
    //     std::fs::remove_dir_all(path).expect("PROBABLY LLAVA_TEST IS NOT EXISTING JUST CREATE IT SO IT COULD BE DELETED WITH NO ERROR");
    // }
    // IF YOU NEED RESTART UNCOMMENT THIS LINE

    //     llava_core::ProgramFiles::init_in_base().expect("failed creating program pahts")
    // };
    let user_db = connect_or_create_local_login_db(&program_paths.local_login_database_path)
        .expect("error while creating locla login db");
    // let _logger_worker = if cfg!(not(debug_assertions)) {
    //     Some(llava_core::configure_logger(&program_paths.logs_path).expect("failed logger"))
    // } else {
    //     println!("Tryb DEV: Logger plikowy wyłączony");
    //     None
    // };
    let _logger_worker =
        Some(llava_core::configure_logger(&program_paths.logs_path).expect("failed logger"));
    let device_id = llava_core::get_device_id(&user_db, &program_paths.device_id_path)
        .expect("big error while reading device id");
    println!("{}", device_id);
    let mut builder = tauri::Builder::default();
    builder = builder.plugin(tauri_plugin_opener::init());
    builder = builder.plugin(tauri_plugin_clipboard_manager::init());

    // #[cfg(debug_assertions)]
    // {
    //     builder = builder.plugin(tauri_plugin_devtools::init());
    // }
    let mut state: llava_core::AppState =
        llava_core::AppState::init().expect("couldnt create state struct");

    state.users_db = std::sync::Mutex::from(Some(user_db));
    state.device_id = std::sync::Mutex::from(Some(device_id));

    // state.current_user = std::sync::Mutex::from(Some(
    //     llava_core::config::read_current_user(&program_paths.active_user_path)
    //         .expect("error while reading current user"),
    // ));
    // let mut notes_db =
    //     llava_core::get_connection(&program_paths).expect("Error while creating database for user");
    // state.connection = std::sync::Mutex::from(Some(notes_db));
    state.paths = std::sync::Mutex::from(Some(program_paths));

    builder
        .setup(|app| {
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::local_auth::register_command, //register locally
            commands::local_auth::login_command,    //login locally
            commands::local_auth::check_if_user_exists,
            commands::local_auth::check_timeout_before_submit,
            commands::local_auth::change_password,
            commands::local_auth::log_with_code,
            commands::local_auth::check_login_on_start,
            commands::local_auth::local_logout_command,
            commands::utils::get_username_from_uuid,
            commands::dashboard::get_dashboard_data,
            commands::settings::get_config_data,
            commands::settings::update_settings,
            commands::settings::get_methapone_map,
            commands::settings::load_backup_config,
            commands::settings::get_logfile_content,
            commands::settings::get_recovery_codes,
            commands::settings::change_username
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
