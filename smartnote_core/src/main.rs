//! Just for tests, wont be needed after taurii added
mod config;
mod errors;
mod models;
mod services;
mod utils;

fn main() {
    let program_files_paths = crate::config::ProgramFiles::init();
    let sqlite_connection =
        crate::services::storage::db_creation::get_connection(&program_files_paths);
}
