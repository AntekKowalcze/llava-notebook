mod config;
mod services;
mod utils;

fn main() {
    let program_files_paths = crate::config::ProgramFiles::init();
    let sqlite_connection = crate::services::storage::get_connection(&program_files_paths);
}
