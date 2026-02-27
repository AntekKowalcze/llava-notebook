//Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use llava_core::{config::get_device_id, ProgramFiles};
use tauri::Manager;
mod commands;
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn main() {
    let program_paths: ProgramFiles = if cfg!(not(debug_assertions)) {
        llava_core::ProgramFiles::init().expect("failed creating program pahts")
    } else {
        // let path = std::env::temp_dir().join("llava_test");
        // if path.exists() {
        //     std::fs::remove_dir_all(path).expect("PROBABLY LLAVA_TEST IS NOT EXISTING JUST CREATE IT SO IT COULD BE DELETED WITH NO ERROR");
        // }
        // IF YOU NEED RESTART UNCOMMENT THIS LINE

        llava_core::ProgramFiles::init_in_base().expect("failed creating program pahts")
    };
    let user_db =
        llava_core::connect_or_create_local_login_db(&program_paths.local_login_database_path)
            .expect("error while creating locla login db");
    let _logger_worker = if cfg!(not(debug_assertions)) {
        Some(llava_core::configure_logger(&program_paths.logs_path).expect("failed logger"))
    } else {
        println!("Tryb DEV: Logger plikowy wyłączony");
        None
    };
    let device_id = get_device_id(&user_db, &program_paths.device_id_path)
        .expect("big error while reading device id");
    println!("{}", device_id);
    let mut builder = tauri::Builder::default();
    #[cfg(debug_assertions)]
    {
        builder = builder.plugin(tauri_plugin_devtools::init());
    }
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
    // this line shall be done again after logging/register

    builder
        .setup(|app| {
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::commands::register_command, //register locally
            commands::commands::login_command,    //login locally
            commands::commands::check_if_user_exists,
            commands::commands::check_timeout_before_submit,
            commands::commands::change_password,
            commands::commands::log_with_code,
            commands::commands::check_login_on_start,
            commands::commands::get_username_from_uuid,
            commands::commands::local_logout_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
