//! Just for tests, wont be needed after taurii added
mod config;
mod constants;
mod errors;
mod models;
mod services;
mod utils;
fn main() {
    //1. logowanie albo rejestracja
    //tworzenie ścieżek i baz
    //program dziala

    let program_files_paths = crate::config::ProgramFiles::init().unwrap();
    let sqlite_connection =
        crate::services::storage::db_creation::get_connection(&program_files_paths);
}
