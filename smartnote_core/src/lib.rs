pub mod config;
pub mod constans;
pub mod errors;
pub mod models;
pub mod services;
pub mod utils;

pub use config::{AppState, ProgramFiles};
pub use services::auth::logging::local_log_in;
pub use services::auth::register::register_user_offline;
pub use services::logger::configure_logger;
//dodać reszte potrzebnych funkcji
//TODO now pisać test
